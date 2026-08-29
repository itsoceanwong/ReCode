use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::routing::post;
use axum::Router;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::db::Db;
use crate::state::AppState;

#[derive(Clone)]
struct OtlpState {
    db: Arc<Db>,
    app: AppHandle,
}

pub struct OtlpServer;

impl OtlpServer {
    pub async fn start(app: AppHandle, state: AppState, preferred_port: u16) -> Result<u16> {
        let port = if preferred_port == 0 {
            4318
        } else {
            preferred_port
        };
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let otlp = OtlpState {
            db: Arc::clone(&state.db),
            app,
        };
        let router = Router::new()
            .route("/v1/metrics", post(handle_metrics))
            .with_state(otlp);

        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(_) if preferred_port == 0 || preferred_port == 4318 => {
                let fallback = SocketAddr::from(([127, 0, 0, 1], 0));
                tokio::net::TcpListener::bind(fallback).await?
            }
            Err(e) => return Err(e.into()),
        };
        let bound = listener.local_addr()?.port();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                eprintln!("otlp server error: {e}");
            }
        });
        Ok(bound)
    }
}

async fn handle_metrics(
    State(state): State<OtlpState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let changed = if ct.contains("json") {
        ingest_json(&state, &body).map_err(|_| StatusCode::BAD_REQUEST)?
    } else {
        // Prefer protobuf; fall back to JSON if decode fails.
        match ingest_protobuf(&state, &body) {
            Ok(n) => n,
            Err(_) => ingest_json(&state, &body).map_err(|_| StatusCode::BAD_REQUEST)?,
        }
    };

    if changed > 0 {
        let _ = state.app.emit("usage_updated", ());
    }
    Ok(StatusCode::OK)
}

fn ingest_json(state: &OtlpState, body: &[u8]) -> Result<usize> {
    let root: Value = serde_json::from_slice(body)?;
    let mut changed = 0usize;
    let resource_metrics = root
        .get("resourceMetrics")
        .or_else(|| root.get("resource_metrics"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for rm in resource_metrics {
        let scope_metrics = rm
            .get("scopeMetrics")
            .or_else(|| rm.get("scope_metrics"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for sm in scope_metrics {
            let metrics = sm
                .get("metrics")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for metric in metrics {
                changed += handle_metric_json(state, &metric)?;
            }
        }
    }
    Ok(changed)
}

fn handle_metric_json(state: &OtlpState, metric: &Value) -> Result<usize> {
    let name = metric.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let is_token = name == "claude_code.token.usage";
    let is_cost = name == "claude_code.cost.usage";
    if !is_token && !is_cost {
        return Ok(0);
    }
    let sum = metric
        .get("sum")
        .or_else(|| metric.get("gauge"))
        .cloned()
        .unwrap_or(Value::Null);
    let points = sum
        .get("dataPoints")
        .or_else(|| sum.get("data_points"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut n = 0;
    for dp in points {
        let attrs = attrs_map(&dp);
        let model = attrs
            .get("model")
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let session_id = attrs.get("session.id").cloned();
        let query_source = attrs.get("query_source").cloned();
        let type_ = attrs.get("type").cloned().unwrap_or_default();
        let ts_nano = dp
            .get("timeUnixNano")
            .or_else(|| dp.get("time_unix_nano"))
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0);
        let ts = if ts_nano > 0 {
            (ts_nano / 1_000_000_000) as i64
        } else {
            chrono::Utc::now().timestamp()
        };
        let minute = ts - ts.rem_euclid(60);

        if is_cost {
            let value = number_as_f64(&dp);
            let dedup = format!(
                "otlp:{}:{}:{}:cost",
                session_id.as_deref().unwrap_or("-"),
                model,
                minute
            );
            state.db.upsert_usage_delta(
                "claude",
                &model,
                minute,
                0,
                0,
                0,
                0,
                0,
                value,
                session_id.as_deref(),
                query_source.as_deref(),
                "otlp",
                &dedup,
            )?;
            n += 1;
            continue;
        }

        let value = number_as_i64(&dp);
        let (input, output, cache_read, cache_write) = match type_.as_str() {
            "input" => (value, 0, 0, 0),
            "output" => (0, value, 0, 0),
            "cacheRead" => (0, 0, value, 0),
            "cacheCreation" => (0, 0, 0, value),
            _ => continue,
        };
        let dedup = format!(
            "otlp:{}:{}:{}:{}",
            session_id.as_deref().unwrap_or("-"),
            model,
            minute,
            type_
        );
        state.db.upsert_usage_delta(
            "claude",
            &model,
            minute,
            input,
            output,
            cache_read,
            cache_write,
            0,
            0.0,
            session_id.as_deref(),
            query_source.as_deref(),
            "otlp",
            &dedup,
        )?;
        n += 1;
    }
    Ok(n)
}

fn attrs_map(dp: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let attrs = dp
        .get("attributes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for a in attrs {
        let key = a.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let value = a
            .get("value")
            .and_then(|v| {
                v.get("stringValue")
                    .or_else(|| v.get("string_value"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        v.get("intValue")
                            .or_else(|| v.get("int_value"))
                            .map(|x| x.to_string())
                    })
            })
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key.to_string(), value);
        }
    }
    map
}

fn number_as_i64(dp: &Value) -> i64 {
    dp.get("asInt")
        .or_else(|| dp.get("as_int"))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .or_else(|| {
            dp.get("asDouble")
                .or_else(|| dp.get("as_double"))
                .and_then(|v| v.as_f64())
                .map(|f| f.floor() as i64)
        })
        .unwrap_or(0)
        .max(0)
}

fn number_as_f64(dp: &Value) -> f64 {
    dp.get("asDouble")
        .or_else(|| dp.get("as_double"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            dp.get("asInt")
                .or_else(|| dp.get("as_int"))
                .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                .map(|i| i as f64)
        })
        .unwrap_or(0.0)
}

fn ingest_protobuf(state: &OtlpState, body: &[u8]) -> Result<usize> {
    if let Ok(s) = std::str::from_utf8(body) {
        if s.trim_start().starts_with('{') {
            return ingest_json(state, body);
        }
    }

    use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
    use opentelemetry_proto::tonic::metrics::v1::metric::Data as MetricData;
    use opentelemetry_proto::tonic::metrics::v1::number_data_point::Value as DpValue;
    use prost::Message;

    let req = ExportMetricsServiceRequest::decode(body)
        .map_err(|e| anyhow::anyhow!("otlp protobuf decode: {e}"))?;

    let mut changed = 0usize;
    for rm in req.resource_metrics {
        for sm in rm.scope_metrics {
            for metric in sm.metrics {
                let is_token = metric.name == "claude_code.token.usage";
                let is_cost = metric.name == "claude_code.cost.usage";
                if !is_token && !is_cost {
                    continue;
                }
                let points = match metric.data {
                    Some(MetricData::Sum(sum)) => sum.data_points,
                    Some(MetricData::Gauge(gauge)) => gauge.data_points,
                    _ => continue,
                };
                for dp in points {
                    let mut attrs = HashMap::new();
                    for kv in &dp.attributes {
                        let val = kv
                            .value
                            .as_ref()
                            .and_then(|v| v.value.as_ref())
                            .map(|v| match v {
                                opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s) => s.clone(),
                                opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(i) => i.to_string(),
                                opentelemetry_proto::tonic::common::v1::any_value::Value::DoubleValue(d) => d.to_string(),
                                _ => String::new(),
                            })
                            .unwrap_or_default();
                        attrs.insert(kv.key.clone(), val);
                    }
                    let model = attrs
                        .get("model")
                        .cloned()
                        .unwrap_or_else(|| "unknown".into());
                    let session_id = attrs.get("session.id").cloned();
                    let query_source = attrs.get("query_source").cloned();
                    let type_ = attrs.get("type").cloned().unwrap_or_default();
                    let ts = if dp.time_unix_nano > 0 {
                        (dp.time_unix_nano / 1_000_000_000) as i64
                    } else {
                        chrono::Utc::now().timestamp()
                    };
                    let minute = ts - ts.rem_euclid(60);

                    if is_cost {
                        let value = match dp.value {
                            Some(DpValue::AsDouble(d)) => d,
                            Some(DpValue::AsInt(i)) => i as f64,
                            None => 0.0,
                        };
                        let dedup = format!(
                            "otlp:{}:{}:{}:cost",
                            session_id.as_deref().unwrap_or("-"),
                            model,
                            minute
                        );
                        state.db.upsert_usage_delta(
                            "claude",
                            &model,
                            minute,
                            0,
                            0,
                            0,
                            0,
                            0,
                            value,
                            session_id.as_deref(),
                            query_source.as_deref(),
                            "otlp",
                            &dedup,
                        )?;
                        changed += 1;
                        continue;
                    }

                    let value = match dp.value {
                        Some(DpValue::AsInt(i)) => i.max(0),
                        Some(DpValue::AsDouble(d)) => d.floor().max(0.0) as i64,
                        None => 0,
                    };
                    let (input, output, cache_read, cache_write) = match type_.as_str() {
                        "input" => (value, 0, 0, 0),
                        "output" => (0, value, 0, 0),
                        "cacheRead" => (0, 0, value, 0),
                        "cacheCreation" => (0, 0, 0, value),
                        _ => continue,
                    };
                    let dedup = format!(
                        "otlp:{}:{}:{}:{}",
                        session_id.as_deref().unwrap_or("-"),
                        model,
                        minute,
                        type_
                    );
                    state.db.upsert_usage_delta(
                        "claude",
                        &model,
                        minute,
                        input,
                        output,
                        cache_read,
                        cache_write,
                        0,
                        0.0,
                        session_id.as_deref(),
                        query_source.as_deref(),
                        "otlp",
                        &dedup,
                    )?;
                    changed += 1;
                }
            }
        }
    }
    Ok(changed)
}
