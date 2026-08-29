//! Claude provider: statusfile parse + JSONL backfill.
//! Primary accurate tokens come from OTLP (otlp.rs).

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

use crate::db::Db;
use crate::paths;

#[derive(Default)]
pub struct ClaudeProvider {
    status_offset: u64,
    last_context: HashMap<String, (i64, i64)>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_statusfile(&mut self, db: &Db) -> Result<usize> {
        let path = paths::recode_statusfile();
        if !path.exists() {
            return Ok(0);
        }
        let mut file = File::open(&path)?;
        let len = file.metadata()?.len();
        if self.status_offset > len {
            self.status_offset = 0;
        }
        file.seek(SeekFrom::Start(self.status_offset))?;
        let mut reader = BufReader::new(file);
        let mut written = 0usize;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            self.status_offset += n as u64;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            let ts = obj
                .get("t")
                .and_then(|v| v.as_i64())
                .map(|ms| ms / 1000)
                .unwrap_or_else(|| Utc::now().timestamp());

            if let Some(rl) = obj.get("rate_limits") {
                upsert_rate_limit(db, rl, "five_hour");
                upsert_rate_limit(db, rl, "seven_day");
            }

            let session_id = obj
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let model = obj
                .get("model")
                .and_then(|m| {
                    m.get("id")
                        .or_else(|| m.get("display_name"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("claude")
                .to_string();

            if let Some(cw) = obj.get("context_window") {
                let input = cw
                    .get("total_input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let output = cw
                    .get("total_output_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let prev = self.last_context.get(&session_id).copied().unwrap_or((0, 0));
                if input >= prev.0 && output >= prev.1 && (input > prev.0 || output > prev.1) {
                    let di = input - prev.0;
                    let dout = output - prev.1;
                    let minute = ts / 60;
                    let dedup = format!("statusline:{session_id}:{model}:{minute}");
                    db.upsert_usage_delta(
                        "claude",
                        &model,
                        ts,
                        di,
                        dout,
                        0,
                        0,
                        0,
                        0.0,
                        Some(&session_id),
                        Some("main"),
                        "statusline",
                        &dedup,
                    )?;
                    written += 1;
                }
                self.last_context.insert(session_id.clone(), (input, output));
            }

            let _ = db.upsert_session(&session_id, "claude", None, Some(&model), ts);
        }
        Ok(written)
    }
}

fn upsert_rate_limit(db: &Db, rl: &Value, kind: &str) {
    let node = match kind {
        "five_hour" => rl.get("five_hour"),
        "seven_day" => rl.get("seven_day"),
        _ => None,
    };
    let Some(node) = node else { return };
    let used = node
        .get("used_percentage")
        .or_else(|| node.get("used_percent"))
        .and_then(|v| v.as_f64());
    let resets = node.get("resets_at").and_then(|v| v.as_i64());
    if used.is_none() && resets.is_none() {
        return;
    }
    let _ = db.upsert_limit("claude", kind, used, resets, false, None);
}

#[allow(dead_code)]
pub fn backfill_jsonl(_db: &Db, _root: Option<PathBuf>) -> Result<usize> {
    Ok(0)
}
