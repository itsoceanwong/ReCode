//! Cursor usage — best-effort read of local `state.vscdb` (read-only).
//!
//! Source: `%APPDATA%/Cursor/User/globalStorage/state.vscdb` (and OS equivalents),
//! table `cursorDiskKV`, keys `bubbleId:…` (tokenCount) and `composerData:…`
//! (context-meter fallback). Marked approximate (`origin=jsonl`). Never reads
//! auth / accessToken keys.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::db::Db;
use crate::paths;
use crate::pricing::{self, compute_cost};

const LOOKBACK_DAYS: i64 = 90;

#[derive(Debug, Clone, Serialize)]
pub struct CursorStatus {
    pub enabled: bool,
    pub db_found: bool,
    pub db_path: Option<String>,
    pub last_inserted: u64,
    pub schema_ok: bool,
    pub detail: Option<String>,
}

pub struct CursorProvider {
    enabled: bool,
    last_mtime: Option<SystemTime>,
    last_inserted: u64,
    last_detail: Option<String>,
    last_schema_ok: bool,
}

impl CursorProvider {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_mtime: None,
            last_inserted: 0,
            last_detail: None,
            last_schema_ok: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn invalidate_cache(&mut self) {
        self.last_mtime = None;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn status(&self) -> CursorStatus {
        let path = paths::cursor_state_db();
        CursorStatus {
            enabled: self.enabled,
            db_found: path.is_some(),
            db_path: path.map(|p| p.display().to_string()),
            last_inserted: self.last_inserted,
            schema_ok: self.last_schema_ok,
            detail: self.last_detail.clone(),
        }
    }

    pub fn scan(&mut self, db: &Db) -> Result<usize> {
        if !self.enabled {
            self.last_detail = Some("disabled".into());
            return Ok(0);
        }
        let Some(src) = paths::cursor_state_db() else {
            self.last_schema_ok = false;
            self.last_detail = Some("state.vscdb not found".into());
            return Ok(0);
        };

        let mtime = std::fs::metadata(&src)
            .ok()
            .and_then(|m| m.modified().ok());
        if mtime.is_some() && mtime == self.last_mtime && self.last_inserted > 0 {
            return Ok(0);
        }

        let conn = match open_cursor_db(&src) {
            Ok(c) => c,
            Err(e) => {
                self.last_detail = Some(format!("open failed: {e}"));
                self.last_schema_ok = false;
                return Ok(0);
            }
        };
        conn.execute_batch("PRAGMA query_only = ON;")?;

        if !schema_ok(&conn) {
            self.last_schema_ok = false;
            self.last_detail = Some("cursorDiskKV / bubbleId schema missing".into());
            return Ok(0);
        }
        self.last_schema_ok = true;

        let floor_ms = (chrono::Utc::now().timestamp() - LOOKBACK_DAYS * 86_400) * 1000;
        let mut inserted = ingest_bubbles(db, &conn, floor_ms)?;
        if inserted == 0 {
            inserted += ingest_composer_meta(db, &conn, floor_ms)?;
        }

        self.last_mtime = mtime;
        self.last_inserted = inserted as u64;
        self.last_detail = Some(format!("scanned {}", src.display()));
        Ok(inserted)
    }
}

fn open_cursor_db(src: &Path) -> Result<Connection> {
    let tmp = paths::recode_dir().join("cursor-state-ro.vscdb");
    if let Err(e) = std::fs::copy(src, &tmp) {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        return Connection::open_with_flags(src, flags)
            .with_context(|| format!("open {} (copy failed: {e})", src.display()));
    }
    let _ = std::fs::copy(
        PathBuf::from(format!("{}-wal", src.display())),
        PathBuf::from(format!("{}-wal", tmp.display())),
    );
    let _ = std::fs::copy(
        PathBuf::from(format!("{}-shm", src.display())),
        PathBuf::from(format!("{}-shm", tmp.display())),
    );
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Connection::open_with_flags(&tmp, flags)
        .with_context(|| format!("open copy {}", tmp.display()))
}

fn schema_ok(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='cursorDiskKV'",
        [],
        |r| r.get(0),
    )
    .unwrap_or(false)
}

fn ingest_bubbles(db: &Db, conn: &Connection, floor_ms: i64) -> Result<usize> {
    let mut stmt = conn.prepare(
        r#"
SELECT
  key,
  COALESCE(json_extract(value, '$.tokenCount.inputTokens'), 0),
  COALESCE(json_extract(value, '$.tokenCount.outputTokens'), 0),
  json_extract(value, '$.modelInfo.modelName'),
  CAST(json_extract(value, '$.createdAt') AS TEXT),
  CAST(json_extract(value, '$.requestId') AS TEXT)
FROM cursorDiskKV
WHERE key LIKE 'bubbleId:%'
  AND (
    COALESCE(json_extract(value, '$.tokenCount.inputTokens'), 0) > 0
    OR COALESCE(json_extract(value, '$.tokenCount.outputTokens'), 0) > 0
  )
"#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let mut n = 0usize;
    for row in rows {
        let (key, input, output, model, created_raw, request_id) = row?;
        if let Some(ms) = created_at_ms(created_raw.as_deref()) {
            if ms < floor_ms {
                continue;
            }
        }
        let model_name = model
            .filter(|m| !m.is_empty() && m != "default")
            .unwrap_or_else(|| "cursor-auto".into());
        let pricing = pricing::matcher(db, &model_name);
        let cost = compute_cost(&pricing, input, output, 0, 0);
        let ts = parse_created_at(created_raw.as_deref());
        let dedup = format!(
            "cursor:bubble:{}",
            request_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(key.as_str())
        );
        if db.insert_usage_exact(
            "cursor",
            &model_name,
            ts,
            input.max(0),
            output.max(0),
            0,
            0,
            0,
            cost,
            Some(&key),
            None,
            "jsonl",
            &dedup,
        )? {
            n += 1;
        }
    }
    Ok(n)
}

fn ingest_composer_meta(db: &Db, conn: &Connection, floor_ms: i64) -> Result<usize> {
    let mut stmt = conn.prepare(
        r#"
SELECT
  substr(key, length('composerData:') + 1),
  COALESCE(json_extract(value, '$.promptTokenBreakdown.totalUsedTokens'), 0),
  COALESCE(json_extract(value, '$.contextTokensUsed'), 0),
  CAST(json_extract(value, '$.createdAt') AS TEXT),
  CAST(json_extract(value, '$.modelName') AS TEXT)
FROM cursorDiskKV
WHERE key >= 'composerData:' AND key < 'composerData;'
"#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    let mut n = 0usize;
    for row in rows {
        let (composer_id, used, ctx, created_raw, model) = row?;
        let tokens = if used > 0 { used } else { ctx };
        if tokens <= 0 || composer_id.is_empty() {
            continue;
        }
        if let Some(ms) = created_at_ms(created_raw.as_deref()) {
            if ms < floor_ms {
                continue;
            }
        }
        let model_name = model
            .filter(|m| !m.is_empty() && m != "default")
            .unwrap_or_else(|| "cursor-auto".into());
        let ts = parse_created_at(created_raw.as_deref());
        let pricing = pricing::matcher(db, &model_name);
        let cost = compute_cost(&pricing, tokens, 0, 0, 0);
        let dedup = format!("cursor:composer:{composer_id}");
        if db.insert_usage_exact(
            "cursor",
            &model_name,
            ts,
            tokens,
            0,
            0,
            0,
            0,
            cost,
            Some(&composer_id),
            None,
            "jsonl",
            &dedup,
        )? {
            n += 1;
        }
    }
    Ok(n)
}

fn created_at_ms(raw: Option<&str>) -> Option<i64> {
    let s = raw?.trim();
    if s.is_empty() || s == "null" {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        if n > 10_000_000_000_000 {
            return Some(n / 1_000_000);
        }
        if n > 10_000_000_000 {
            return Some(n);
        }
        return Some(n * 1000);
    }
    if let Ok(f) = s.parse::<f64>() {
        let n = f as i64;
        if n > 10_000_000_000 {
            return Some(n);
        }
        return Some(n * 1000);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    None
}

fn parse_created_at(raw: Option<&str>) -> i64 {
    created_at_ms(raw)
        .map(|ms| ms / 1000)
        .unwrap_or_else(|| chrono::Utc::now().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn bubble_rows_insert_once() {
        let dir = std::env::temp_dir().join(format!(
            "recode-cursor-ut-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.vscdb");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value TEXT);
INSERT INTO cursorDiskKV(key,value) VALUES
 ('bubbleId:comp-1:msg-1', '{"tokenCount":{"inputTokens":100,"outputTokens":50},"modelInfo":{"modelName":"gpt-4.1"},"createdAt":"2026-08-01T00:00:00Z","requestId":"req-1"}');
"#,
            )
            .unwrap();
        }
        let recode_db = dir.join("recode.db");
        let db = Db::open(&recode_db).unwrap();
        let src = Connection::open(&path).unwrap();
        let n = ingest_bubbles(&db, &src, 0).unwrap();
        assert_eq!(n, 1);
        let n2 = ingest_bubbles(&db, &src, 0).unwrap();
        assert_eq!(n2, 0, "dedup should skip second insert");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn composer_fallback_when_no_bubble_tokens() {
        let dir = std::env::temp_dir().join(format!(
            "recode-cursor-ut2-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.vscdb");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value TEXT);
INSERT INTO cursorDiskKV(key,value) VALUES
 ('composerData:abc', '{"promptTokenBreakdown":{"totalUsedTokens":0},"contextTokensUsed":321,"createdAt":"2026-08-02T00:00:00Z","modelName":"claude-4-sonnet"}');
"#,
            )
            .unwrap();
        }
        let db = Db::open(&dir.join("recode.db")).unwrap();
        let src = Connection::open(&path).unwrap();
        let n = ingest_composer_meta(&db, &src, 0).unwrap();
        assert_eq!(n, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
