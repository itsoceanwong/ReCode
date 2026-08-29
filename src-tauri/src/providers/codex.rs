//! Codex rollout JSONL parser with per-stream cumulative delta reconstruction.
//!
//! Codex can interleave multiple SessionState streams (parent + reviewer).
//! Recover stream identity via: previous_total = total - last.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use walkdir::WalkDir;

use crate::db::Db;
use crate::paths;
use crate::pricing;

const MAX_BASELINES: usize = 32;

fn is_manageable_codex_thread(thread_source: Option<&str>) -> bool {
    match thread_source {
        None => true, // legacy / missing: allow (index filter cleans later)
        Some("user") => true,
        Some(_) => false,
    }
}

#[cfg(test)]
mod thread_source_tests {
    use super::is_manageable_codex_thread;

    #[test]
    fn only_user_or_missing_are_manageable() {
        assert!(is_manageable_codex_thread(Some("user")));
        assert!(is_manageable_codex_thread(None));
        assert!(!is_manageable_codex_thread(Some("subagent")));
        assert!(!is_manageable_codex_thread(Some("guardian_review")));
    }
}

#[derive(Debug, Clone, Default)]
struct TokenUsage {
    input_tokens: i64,
    cached_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
    reasoning_output_tokens: i64,
    total_tokens: i64,
}

impl TokenUsage {
    fn from_value(v: &Value) -> Option<Self> {
        if !v.is_object() {
            return None;
        }
        let get = |keys: &[&str]| -> i64 {
            for k in keys {
                if let Some(n) = v.get(*k).and_then(|x| x.as_i64()) {
                    return n.max(0);
                }
                if let Some(n) = v.get(*k).and_then(|x| x.as_f64()) {
                    return n.max(0.0) as i64;
                }
            }
            0
        };
        let out = Self {
            input_tokens: get(&["input_tokens"]),
            cached_input_tokens: get(&["cached_input_tokens"]),
            cache_creation_input_tokens: get(&[
                "cache_creation_input_tokens",
                "cache_write_input_tokens",
            ]),
            output_tokens: get(&["output_tokens"]),
            reasoning_output_tokens: get(&["reasoning_output_tokens"]),
            total_tokens: get(&["total_tokens"]),
        };
        if out.input_tokens
            + out.cached_input_tokens
            + out.cache_creation_input_tokens
            + out.output_tokens
            + out.reasoning_output_tokens
            + out.total_tokens
            == 0
        {
            return None;
        }
        Some(out)
    }

    fn signature(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.input_tokens,
            self.cached_input_tokens,
            self.cache_creation_input_tokens,
            self.output_tokens,
            self.reasoning_output_tokens,
            self.total_tokens
        )
    }

    fn saturating_sub(&self, other: &Self) -> Option<Self> {
        if self.input_tokens < other.input_tokens
            || self.cached_input_tokens < other.cached_input_tokens
            || self.cache_creation_input_tokens < other.cache_creation_input_tokens
            || self.output_tokens < other.output_tokens
            || self.reasoning_output_tokens < other.reasoning_output_tokens
            || self.total_tokens < other.total_tokens
        {
            return None;
        }
        Some(Self {
            input_tokens: self.input_tokens - other.input_tokens,
            cached_input_tokens: self.cached_input_tokens - other.cached_input_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens
                - other.cache_creation_input_tokens,
            output_tokens: self.output_tokens - other.output_tokens,
            reasoning_output_tokens: self.reasoning_output_tokens - other.reasoning_output_tokens,
            total_tokens: self.total_tokens - other.total_tokens,
        })
    }

    fn diff_from(&self, previous: &Self) -> Self {
        Self {
            input_tokens: (self.input_tokens - previous.input_tokens).max(0),
            cached_input_tokens: (self.cached_input_tokens - previous.cached_input_tokens).max(0),
            cache_creation_input_tokens: (self.cache_creation_input_tokens
                - previous.cache_creation_input_tokens)
                .max(0),
            output_tokens: (self.output_tokens - previous.output_tokens).max(0),
            reasoning_output_tokens: (self.reasoning_output_tokens
                - previous.reasoning_output_tokens)
                .max(0),
            total_tokens: (self.total_tokens - previous.total_tokens).max(0),
        }
    }

    /// Non-cached input kept separate from cache read.
    fn normalize(mut self) -> Self {
        self.input_tokens = (self.input_tokens - self.cached_input_tokens).max(0);
        self.total_tokens = self.input_tokens
            + self.cached_input_tokens
            + self.cache_creation_input_tokens
            + self.output_tokens
            + self.reasoning_output_tokens;
        self
    }
}

#[derive(Debug, Default)]
pub struct UsageDeltaState {
    last_total: Option<TokenUsage>,
    baselines: Vec<TokenUsage>,
}

impl UsageDeltaState {
    pub fn consume(&mut self, last: Option<TokenUsage>, total: Option<TokenUsage>) -> Option<TokenUsage> {
        let total = total?;
        if let Some(idx) = self.find_baseline(&total) {
            self.touch_at(idx, total);
            return None; // duplicate snapshot
        }

        if let Some(ref last_u) = last {
            if let Some(expected_prev) = total.saturating_sub(last_u) {
                if let Some(idx) = self.find_baseline(&expected_prev) {
                    self.touch_at(idx, total);
                    return Some(last_u.clone());
                }
                // First observation for this stream lineage
                self.touch(total);
                return Some(last_u.clone());
            }
        }

        if let Some(ref active) = self.last_total {
            if total.total_tokens >= active.total_tokens {
                let delta = total.diff_from(active);
                if last
                    .as_ref()
                    .map(|l| delta.total_tokens <= l.total_tokens)
                    .unwrap_or(true)
                {
                    let idx = self.find_baseline(active);
                    self.touch_at(idx.unwrap_or(usize::MAX), total);
                    return Some(delta);
                }
            }
        }

        self.touch(total.clone());
        last.or(Some(total))
    }

    fn find_baseline(&self, usage: &TokenUsage) -> Option<usize> {
        let sig = usage.signature();
        self.baselines.iter().position(|b| b.signature() == sig)
    }

    fn touch_at(&mut self, index: usize, usage: TokenUsage) {
        if index < self.baselines.len() {
            self.baselines.remove(index);
        } else if let Some(dup) = self.find_baseline(&usage) {
            self.baselines.remove(dup);
        }
        self.baselines.push(usage.clone());
        while self.baselines.len() > MAX_BASELINES {
            self.baselines.remove(0);
        }
        self.last_total = Some(usage);
    }

    fn touch(&mut self, usage: TokenUsage) {
        let idx = self.find_baseline(&usage).unwrap_or(usize::MAX);
        self.touch_at(idx, usage);
    }
}

#[derive(Default)]
pub struct CodexProvider {
    offsets: HashMap<PathBuf, u64>,
    delta_states: HashMap<PathBuf, UsageDeltaState>,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discover_files() -> Vec<PathBuf> {
        let mut files = Vec::new();
        let roots = [
            paths::codex_dir().join("sessions"),
            paths::codex_dir().join("archived_sessions"),
        ];
        for root in roots {
            if !root.exists() {
                continue;
            }
            for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("rollout-") && name.ends_with(".jsonl") {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        files.sort();
        files
    }

    pub fn scan_all(&mut self, db: &Db) -> Result<usize> {
        let mut count = 0;
        for path in Self::discover_files() {
            count += self.scan_file(db, &path)?;
        }
        Ok(count)
    }

    pub fn scan_file(&mut self, db: &Db, path: &Path) -> Result<usize> {
        let mut file = File::open(path)
            .with_context(|| format!("open {}", path.display()))?;
        let offset = self.offsets.get(path).copied().unwrap_or(0);
        let meta_len = file.metadata()?.len();
        if offset > meta_len {
            // truncated / replaced
            self.offsets.insert(path.to_path_buf(), 0);
            self.delta_states.remove(path);
        }
        let start = self.offsets.get(path).copied().unwrap_or(0);
        file.seek(SeekFrom::Start(start))?;
        let mut reader = BufReader::new(file);

        let mut session_id = String::new();
        let mut model = String::from("unknown");
        let mut cwd: Option<String> = None;
        let mut manageable_session = true;
        let mut written = 0usize;
        let mut line_no: u64 = 0;
        let mut bytes_read = start;

        let state = self
            .delta_states
            .entry(path.to_path_buf())
            .or_default();

        // When resuming mid-file, line_no for dedup must be unique; use byte offset in key.
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            bytes_read += n as u64;
            line_no += 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            let typ = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let payload = obj.get("payload").cloned().unwrap_or(Value::Null);
            let ts = parse_ts(obj.get("timestamp"));

            match typ {
                "session_meta" => {
                    if let Some(id) = payload.get("id").and_then(|v| v.as_str()) {
                        session_id = id.to_string();
                    }
                    if let Some(c) = payload.get("cwd").and_then(|v| v.as_str()) {
                        cwd = Some(c.to_string());
                    }
                    let thread_source = payload.get("thread_source").and_then(|v| v.as_str());
                    manageable_session = is_manageable_codex_thread(thread_source);
                    if manageable_session && !session_id.is_empty() {
                        let model_ref = if model == "unknown" {
                            None
                        } else {
                            Some(model.as_str())
                        };
                        let _ = db.upsert_session(
                            &session_id,
                            "codex",
                            cwd.as_deref(),
                            model_ref,
                            ts,
                        );
                    }
                }
                "turn_context" => {
                    if let Some(m) = payload.get("model").and_then(|v| v.as_str()) {
                        model = m.to_string();
                    }
                    if let Some(c) = payload.get("cwd").and_then(|v| v.as_str()) {
                        cwd = Some(c.to_string());
                    }
                }
                "event_msg" => {
                    let ptype = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if ptype != "token_count" {
                        continue;
                    }
                    let info = match payload.get("info") {
                        Some(i) => i,
                        None => continue,
                    };
                    let last = info
                        .get("last_token_usage")
                        .and_then(TokenUsage::from_value);
                    let total = info
                        .get("total_token_usage")
                        .and_then(TokenUsage::from_value);

                    // Rate-limit snapshot (optional)
                    upsert_codex_limits(db, info, ts);

                    let Some(delta) = state.consume(last, total) else {
                        continue;
                    };
                    let delta = delta.normalize();
                    if delta.input_tokens
                        + delta.cached_input_tokens
                        + delta.cache_creation_input_tokens
                        + delta.output_tokens
                        + delta.reasoning_output_tokens
                        == 0
                    {
                        continue;
                    }

                    let sid = if session_id.is_empty() {
                        format!("codex-{}", path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"))
                    } else {
                        session_id.clone()
                    };
                    let file_key = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("rollout");
                    let dedup_key = format!("codex:{sid}:{file_key}:{line_no}");

                    let price = pricing::matcher(db, &model);
                    let cost = pricing::compute_cost(
                        &price,
                        delta.input_tokens,
                        delta.output_tokens,
                        delta.cached_input_tokens,
                        delta.cache_creation_input_tokens,
                    );
                    let inserted = db.insert_usage_exact(
                        "codex",
                        &model,
                        ts,
                        delta.input_tokens,
                        delta.output_tokens,
                        delta.cached_input_tokens,
                        delta.cache_creation_input_tokens,
                        delta.reasoning_output_tokens,
                        cost,
                        Some(&sid),
                        None,
                        "jsonl",
                        &dedup_key,
                    )?;
                    if inserted {
                        written += 1;
                    }
                    if manageable_session {
                        let model_ref = if model == "unknown" {
                            None
                        } else {
                            Some(model.as_str())
                        };
                        let _ = db.upsert_session(&sid, "codex", cwd.as_deref(), model_ref, ts);
                    }
                }
                _ => {}
            }
        }

        self.offsets.insert(path.to_path_buf(), bytes_read);
        Ok(written)
    }
}

fn parse_ts(v: Option<&Value>) -> i64 {
    match v {
        Some(Value::String(s)) => DateTime::parse_from_rfc3339(s)
            .map(|d| d.timestamp())
            .or_else(|_| {
                s.parse::<DateTime<Utc>>()
                    .map(|d| d.timestamp())
                    .map_err(|_| ())
            })
            .unwrap_or_else(|_| Utc::now().timestamp()),
        Some(Value::Number(n)) => n.as_i64().unwrap_or_else(|| Utc::now().timestamp()),
        _ => Utc::now().timestamp(),
    }
}

fn upsert_codex_limits(db: &Db, info: &Value, ts: i64) {
    // Look for rate limit snapshots keyed by limit_window_seconds
    let candidates = [
        info.get("rate_limits"),
        info.get("rate_limit"),
        info.get("limits"),
        Some(info),
    ];
    for cand in candidates.into_iter().flatten() {
        if let Some(arr) = cand.as_array() {
            for item in arr {
                apply_limit_item(db, item, ts);
            }
        } else if cand.is_object() {
            // Maybe map of windows
            if let Some(obj) = cand.as_object() {
                for (k, v) in obj {
                    if k.contains("18000") || k == "five_hour" {
                        apply_window(db, "five_hour", v, ts);
                    } else if k.contains("604800") || k == "seven_day" {
                        apply_window(db, "seven_day", v, ts);
                    } else {
                        apply_limit_item(db, v, ts);
                    }
                }
            }
        }
    }
}

fn apply_limit_item(db: &Db, item: &Value, ts: i64) {
    let secs = item
        .get("limit_window_seconds")
        .and_then(|v| v.as_i64())
        .or_else(|| item.get("window_seconds").and_then(|v| v.as_i64()));
    let kind = match secs {
        Some(18000) => "five_hour",
        Some(604800) => "seven_day",
        _ => return,
    };
    apply_window(db, kind, item, ts);
}

fn apply_window(db: &Db, kind: &str, v: &Value, ts: i64) {
    let used = v
        .get("used_percent")
        .or_else(|| v.get("used_percentage"))
        .and_then(|x| x.as_f64());
    let resets = v
        .get("reset_at")
        .or_else(|| v.get("resets_at"))
        .and_then(|x| x.as_i64());
    if used.is_none() && resets.is_none() {
        return;
    }
    let _ = db.upsert_limit("codex", kind, used, resets, false);
    let _ = ts;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(input: i64, cached: i64, out: i64, total: i64) -> TokenUsage {
        TokenUsage {
            input_tokens: input,
            cached_input_tokens: cached,
            cache_creation_input_tokens: 0,
            output_tokens: out,
            reasoning_output_tokens: 0,
            total_tokens: total,
        }
    }

    #[test]
    fn no_double_count_interleaved_streams() {
        let mut state = UsageDeltaState::default();
        // Stream A first turn: last=10, total=10
        let d1 = state
            .consume(Some(u(10, 0, 5, 15)), Some(u(10, 0, 5, 15)))
            .unwrap();
        assert_eq!(d1.input_tokens, 10);

        // Stream B first turn interleaved: last=7, total=7
        let d2 = state
            .consume(Some(u(7, 0, 3, 10)), Some(u(7, 0, 3, 10)))
            .unwrap();
        assert_eq!(d2.input_tokens, 7);

        // Stream A second turn: last=4, total=14 (=10+4)
        let d3 = state
            .consume(Some(u(4, 0, 2, 6)), Some(u(14, 0, 7, 21)))
            .unwrap();
        assert_eq!(d3.input_tokens, 4);

        // Duplicate snapshot of stream A total -> None
        let d4 = state.consume(Some(u(0, 0, 0, 0)), Some(u(14, 0, 7, 21)));
        assert!(d4.is_none());
    }
}
