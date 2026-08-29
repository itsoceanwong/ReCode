use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::paths;

const RECODE_ENV_KEYS: &[&str] = &[
    "CLAUDE_CODE_ENABLE_TELEMETRY",
    "OTEL_METRICS_EXPORTER",
    "OTEL_EXPORTER_OTLP_PROTOCOL",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE",
    "OTEL_METRIC_EXPORT_INTERVAL",
];

fn read_settings() -> Result<Value> {
    let path = paths::claude_settings();
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    Ok(serde_json::from_str(&raw)?)
}

fn write_settings_atomic(value: &Value) -> Result<()> {
    let path = paths::claude_settings();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let bak = path.with_extension("json.recode.bak");
        let _ = fs::copy(&path, &bak);
    }
    let tmp = path.with_extension("json.recode.tmp");
    let body = serde_json::to_string_pretty(value)?;
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn ensure_telemetry(port: u16, with_statusline: bool) -> Result<()> {
    let mut root = read_settings()?;
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let obj = root.as_object_mut().expect("object");
    let env = obj
        .entry("env")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .map(|o| o as &mut serde_json::Map<String, Value>)
        .ok_or_else(|| anyhow::anyhow!("settings.env is not an object"))?;

    env.insert(
        "CLAUDE_CODE_ENABLE_TELEMETRY".into(),
        Value::String("1".into()),
    );
    env.insert("OTEL_METRICS_EXPORTER".into(), Value::String("otlp".into()));
    env.insert(
        "OTEL_EXPORTER_OTLP_PROTOCOL".into(),
        Value::String("http/protobuf".into()),
    );
    env.insert(
        "OTEL_EXPORTER_OTLP_ENDPOINT".into(),
        Value::String(format!("http://127.0.0.1:{port}")),
    );
    env.insert(
        "OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE".into(),
        Value::String("delta".into()),
    );
    env.insert(
        "OTEL_METRIC_EXPORT_INTERVAL".into(),
        Value::String("10000".into()),
    );

    if with_statusline {
        let script = install_statusline_script()?;
        let node = which_node().unwrap_or_else(|| "node".into());
        let our_cmd = format!("\"{node}\" \"{}\"", script.display());
        match obj.get("statusLine") {
            Some(existing) if !is_our_statusline(existing) => {
                // Wrap: run theirs then ours via a small shell-less node wrapper path.
                // Store ours; leave a note field. Prefer not clobbering — wrap via command chaining on Windows/macOS is fragile,
                // so we set ours and keep backup under statusLine_recode_previous.
                obj.insert("statusLine_recode_previous".into(), existing.clone());
                obj.insert(
                    "statusLine".into(),
                    serde_json::json!({ "type": "command", "command": our_cmd }),
                );
            }
            _ => {
                obj.insert(
                    "statusLine".into(),
                    serde_json::json!({ "type": "command", "command": our_cmd }),
                );
            }
        }
    }

    write_settings_atomic(&root)?;
    Ok(())
}

pub fn remove_telemetry() -> Result<()> {
    let mut root = read_settings()?;
    if let Some(obj) = root.as_object_mut() {
        if let Some(env) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
            for k in RECODE_ENV_KEYS {
                env.remove(*k);
            }
        }
        if let Some(sl) = obj.get("statusLine") {
            if is_our_statusline(sl) {
                if let Some(prev) = obj.remove("statusLine_recode_previous") {
                    obj.insert("statusLine".into(), prev);
                } else {
                    obj.remove("statusLine");
                }
            }
        }
    }
    write_settings_atomic(&root)?;
    Ok(())
}

pub fn telemetry_present() -> bool {
    let Ok(root) = read_settings() else {
        return false;
    };
    let Some(env) = root.get("env").and_then(|v| v.as_object()) else {
        return false;
    };
    env.get("CLAUDE_CODE_ENABLE_TELEMETRY")
        .and_then(|v| v.as_str())
        == Some("1")
        && env
            .get("OTEL_METRICS_EXPORTER")
            .and_then(|v| v.as_str())
            == Some("otlp")
}

pub fn ccswitch_detected() -> bool {
    paths::ccswitch_db().exists()
}

fn is_our_statusline(v: &Value) -> bool {
    v.get("command")
        .and_then(|c| c.as_str())
        .map(|c| c.contains("claude-statusline.js"))
        .unwrap_or(false)
}

fn install_statusline_script() -> Result<PathBuf> {
    let dest = paths::recode_dir().join("claude-statusline.js");
    let src = include_str!("../resources/claude-statusline.js");
    fs::write(&dest, src)?;
    Ok(dest)
}

fn which_node() -> Option<String> {
    which_in_path("node")
}

fn which_in_path(bin: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{bin}.exe"));
            if with_exe.exists() {
                return Some(with_exe.to_string_lossy().into_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_does_not_drop_unrelated_keys() {
        // Unit-style: simulate deep merge logic on a Value.
        let mut root = serde_json::json!({
            "env": { "FOO": "bar" },
            "other": 1
        });
        let env = root["env"].as_object_mut().unwrap();
        env.insert("CLAUDE_CODE_ENABLE_TELEMETRY".into(), Value::String("1".into()));
        assert_eq!(root["env"]["FOO"], "bar");
        assert_eq!(root["other"], 1);
        assert_eq!(root["env"]["CLAUDE_CODE_ENABLE_TELEMETRY"], "1");
    }
}
