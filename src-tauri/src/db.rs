use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::models::{
    AutocontinueLog, DashboardData, DashboardSummary, LimitWindow, PricingRow, SessionView,
    UsageAggregate,
};

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open sqlite {}", path.display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        db.seed_settings()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS usage_events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  source        TEXT NOT NULL,
  model         TEXT NOT NULL,
  ts            INTEGER NOT NULL,
  input         INTEGER NOT NULL DEFAULT 0,
  output        INTEGER NOT NULL DEFAULT 0,
  cache_read    INTEGER NOT NULL DEFAULT 0,
  cache_write   INTEGER NOT NULL DEFAULT 0,
  reasoning     INTEGER NOT NULL DEFAULT 0,
  cost_usd      REAL    NOT NULL DEFAULT 0,
  session_id    TEXT,
  query_source  TEXT,
  origin        TEXT NOT NULL,
  dedup_key     TEXT UNIQUE
);
CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_events(ts);
CREATE INDEX IF NOT EXISTS idx_usage_src_model ON usage_events(source, model);

CREATE TABLE IF NOT EXISTS limit_windows (
  source       TEXT NOT NULL,
  window_kind  TEXT NOT NULL,
  used_percent REAL,
  resets_at    INTEGER,
  is_manual    INTEGER NOT NULL DEFAULT 0,
  updated_at   INTEGER NOT NULL,
  PRIMARY KEY (source, window_kind)
);

CREATE TABLE IF NOT EXISTS sessions (
  id                    TEXT PRIMARY KEY,
  source                TEXT NOT NULL,
  cwd                   TEXT,
  model                 TEXT,
  auto_continue_enabled INTEGER NOT NULL DEFAULT 0,
  continue_prompt       TEXT,
  target_kind           TEXT,
  target_ref            TEXT,
  last_seen             INTEGER
);

CREATE TABLE IF NOT EXISTS pricing (
  model          TEXT PRIMARY KEY,
  input_pm       REAL NOT NULL,
  output_pm      REAL NOT NULL,
  cache_read_pm  REAL NOT NULL,
  cache_write_pm REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS autocontinue_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT,
  fired_at   INTEGER NOT NULL,
  target     TEXT,
  status     TEXT NOT NULL,
  detail     TEXT
);
"#,
        )?;
        Ok(())
    }

    fn seed_settings(&self) -> Result<()> {
        let defaults = [
            (
                "default_prompt",
                "read the history, continue on the work",
            ),
            ("continue_offset_seconds", "120"),
            ("otlp_port", "0"),
            ("telemetry_enabled", "false"),
            ("notify_only", "false"),
            ("cursor_enabled", "true"),
        ];
        let conn = self.conn.lock().expect("db lock");
        for (key, value) in defaults {
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
        Ok(())
    }

    pub fn get_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_dashboard(&self) -> Result<DashboardData> {
        let now = chrono::Utc::now().timestamp();
        let day_start = now - (now.rem_euclid(86_400));
        let conn = self.conn.lock().expect("db lock");

        let summary = conn.query_row(
            "SELECT COALESCE(SUM(input + output + cache_read + cache_write + reasoning), 0),
                    COALESCE(SUM(cost_usd), 0)
             FROM usage_events WHERE ts >= ?1",
            params![day_start],
            |row| {
                Ok(DashboardSummary {
                    today_tokens: row.get::<_, i64>(0)?,
                    today_cost_usd: row.get::<_, f64>(1)?,
                })
            },
        )?;

        let mut limit_stmt = conn.prepare(
            "SELECT source, window_kind, used_percent, resets_at, is_manual, updated_at
             FROM limit_windows ORDER BY source, window_kind",
        )?;
        let limits = limit_stmt
            .query_map([], |row| {
                Ok(LimitWindow {
                    source: row.get(0)?,
                    window_kind: row.get(1)?,
                    used_percent: row.get(2)?,
                    resets_at: row.get(3)?,
                    is_manual: row.get::<_, i64>(4)? != 0,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut log_stmt = conn.prepare(
            "SELECT id, session_id, fired_at, target, status, detail
             FROM autocontinue_log ORDER BY fired_at DESC LIMIT 20",
        )?;
        let recent_autocontinue = log_stmt
            .query_map([], |row| {
                Ok(AutocontinueLog {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    fired_at: row.get(2)?,
                    target: row.get(3)?,
                    status: row.get(4)?,
                    detail: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DashboardData {
            limits,
            summary,
            recent_autocontinue,
        })
    }

    pub fn get_usage(
        &self,
        from: i64,
        to: i64,
        group_by: &str,
    ) -> Result<Vec<UsageAggregate>> {
        let conn = self.conn.lock().expect("db lock");
        let sql = if group_by == "model" {
            "SELECT source, model,
                    SUM(input), SUM(output), SUM(cache_read), SUM(cache_write), SUM(reasoning),
                    SUM(cost_usd),
                    CASE
                      WHEN SUM(CASE WHEN origin = 'otlp' THEN 1 ELSE 0 END) > 0 THEN 'otlp'
                      WHEN SUM(CASE WHEN origin = 'statusline' THEN 1 ELSE 0 END) > 0 THEN 'statusline'
                      ELSE 'jsonl'
                    END
             FROM usage_events
             WHERE ts >= ?1 AND ts <= ?2
             GROUP BY source, model
             ORDER BY source, model"
        } else {
            "SELECT source, '',
                    SUM(input), SUM(output), SUM(cache_read), SUM(cache_write), SUM(reasoning),
                    SUM(cost_usd),
                    CASE
                      WHEN SUM(CASE WHEN origin = 'otlp' THEN 1 ELSE 0 END) > 0 THEN 'otlp'
                      WHEN SUM(CASE WHEN origin = 'statusline' THEN 1 ELSE 0 END) > 0 THEN 'statusline'
                      ELSE 'jsonl'
                    END
             FROM usage_events
             WHERE ts >= ?1 AND ts <= ?2
             GROUP BY source
             ORDER BY source"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params![from, to], |row| {
                Ok(UsageAggregate {
                    source: row.get(0)?,
                    model: row.get(1)?,
                    input: row.get(2)?,
                    output: row.get(3)?,
                    cache_read: row.get(4)?,
                    cache_write: row.get(5)?,
                    reasoning: row.get(6)?,
                    cost_usd: row.get(7)?,
                    origin: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_sessions(&self) -> Result<Vec<SessionView>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT id, source, cwd, model, auto_continue_enabled, continue_prompt,
                    target_kind, target_ref, last_seen
             FROM sessions ORDER BY last_seen DESC, id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SessionView {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    cwd: row.get(2)?,
                    model: row.get(3)?,
                    auto_continue_enabled: row.get::<_, i64>(4)? != 0,
                    continue_prompt: row.get(5)?,
                    target_kind: row.get(6)?,
                    target_ref: row.get(7)?,
                    last_seen: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_usage_delta(
        &self,
        source: &str,
        model: &str,
        ts: i64,
        input: i64,
        output: i64,
        cache_read: i64,
        cache_write: i64,
        reasoning: i64,
        cost_usd: f64,
        session_id: Option<&str>,
        query_source: Option<&str>,
        origin: &str,
        dedup_key: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO usage_events
             (source, model, ts, input, output, cache_read, cache_write, reasoning,
              cost_usd, session_id, query_source, origin, dedup_key)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(dedup_key) DO UPDATE SET
               input = input + excluded.input,
               output = output + excluded.output,
               cache_read = cache_read + excluded.cache_read,
               cache_write = cache_write + excluded.cache_write,
               reasoning = reasoning + excluded.reasoning,
               cost_usd = cost_usd + excluded.cost_usd",
            params![
                source,
                model,
                ts,
                input,
                output,
                cache_read,
                cache_write,
                reasoning,
                cost_usd,
                session_id,
                query_source,
                origin,
                dedup_key
            ],
        )?;
        Ok(())
    }

    pub fn insert_usage_exact(
        &self,
        source: &str,
        model: &str,
        ts: i64,
        input: i64,
        output: i64,
        cache_read: i64,
        cache_write: i64,
        reasoning: i64,
        cost_usd: f64,
        session_id: Option<&str>,
        query_source: Option<&str>,
        origin: &str,
        dedup_key: &str,
    ) -> Result<bool> {
        let conn = self.conn.lock().expect("db lock");
        let changed = conn.execute(
            "INSERT OR IGNORE INTO usage_events
             (source, model, ts, input, output, cache_read, cache_write, reasoning,
              cost_usd, session_id, query_source, origin, dedup_key)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                source,
                model,
                ts,
                input,
                output,
                cache_read,
                cache_write,
                reasoning,
                cost_usd,
                session_id,
                query_source,
                origin,
                dedup_key
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn upsert_session(
        &self,
        id: &str,
        source: &str,
        cwd: Option<&str>,
        model: Option<&str>,
        last_seen: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO sessions (id, source, cwd, model, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               cwd = COALESCE(excluded.cwd, sessions.cwd),
               model = COALESCE(excluded.model, sessions.model),
               last_seen = excluded.last_seen",
            params![id, source, cwd, model, last_seen],
        )?;
        Ok(())
    }

    pub fn set_session_autocontinue(
        &self,
        id: &str,
        enabled: bool,
        prompt: Option<&str>,
        target_kind: Option<&str>,
        target_ref: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "UPDATE sessions SET
               auto_continue_enabled = ?2,
               continue_prompt = ?3,
               target_kind = COALESCE(?4, target_kind),
               target_ref = COALESCE(?5, target_ref)
             WHERE id = ?1",
            params![id, enabled as i64, prompt, target_kind, target_ref],
        )?;
        Ok(())
    }

    pub fn upsert_limit(
        &self,
        source: &str,
        window_kind: &str,
        used_percent: Option<f64>,
        resets_at: Option<i64>,
        is_manual: bool,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db lock");
        if !is_manual {
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT is_manual FROM limit_windows WHERE source = ?1 AND window_kind = ?2",
                    params![source, window_kind],
                    |row| row.get(0),
                )
                .ok();
            if existing == Some(1) {
                return Ok(());
            }
        }
        conn.execute(
            "INSERT INTO limit_windows (source, window_kind, used_percent, resets_at, is_manual, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(source, window_kind) DO UPDATE SET
               used_percent = excluded.used_percent,
               resets_at = excluded.resets_at,
               is_manual = excluded.is_manual,
               updated_at = excluded.updated_at",
            params![
                source,
                window_kind,
                used_percent,
                resets_at,
                is_manual as i64,
                now
            ],
        )?;
        Ok(())
    }

    pub fn clear_manual_limit(&self, source: &str, window_kind: &str) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "DELETE FROM limit_windows WHERE source = ?1 AND window_kind = ?2 AND is_manual = 1",
            params![source, window_kind],
        )?;
        Ok(())
    }

    pub fn all_limits(&self) -> Result<Vec<LimitWindow>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT source, window_kind, used_percent, resets_at, is_manual, updated_at
             FROM limit_windows",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LimitWindow {
                    source: row.get(0)?,
                    window_kind: row.get(1)?,
                    used_percent: row.get(2)?,
                    resets_at: row.get(3)?,
                    is_manual: row.get::<_, i64>(4)? != 0,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_pricing(&self) -> Result<Vec<PricingRow>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT model, input_pm, output_pm, cache_read_pm, cache_write_pm FROM pricing ORDER BY model",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PricingRow {
                    model: row.get(0)?,
                    input_pm: row.get(1)?,
                    output_pm: row.get(2)?,
                    cache_read_pm: row.get(3)?,
                    cache_write_pm: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_pricing(&self, row: &PricingRow) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO pricing (model, input_pm, output_pm, cache_read_pm, cache_write_pm)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(model) DO UPDATE SET
               input_pm = excluded.input_pm,
               output_pm = excluded.output_pm,
               cache_read_pm = excluded.cache_read_pm,
               cache_write_pm = excluded.cache_write_pm",
            params![
                row.model,
                row.input_pm,
                row.output_pm,
                row.cache_read_pm,
                row.cache_write_pm
            ],
        )?;
        Ok(())
    }

    pub fn find_pricing(&self, model: &str) -> Result<Option<PricingRow>> {
        let all = self.get_pricing()?;
        if let Some(exact) = all.iter().find(|p| p.model == model) {
            return Ok(Some(exact.clone()));
        }
        let lower = model.to_ascii_lowercase();
        if let Some(exact) = all.iter().find(|p| p.model.to_ascii_lowercase() == lower) {
            return Ok(Some(exact.clone()));
        }
        if let Some(prefix) = all.iter().find(|p| {
            lower.starts_with(&p.model.to_ascii_lowercase())
                || p.model.to_ascii_lowercase().starts_with(&lower)
        }) {
            return Ok(Some(prefix.clone()));
        }
        Ok(None)
    }

    pub fn insert_autocontinue_log(
        &self,
        session_id: Option<&str>,
        target: Option<&str>,
        status: &str,
        detail: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO autocontinue_log (session_id, fired_at, target, status, detail)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, now, target, status, detail],
        )?;
        Ok(())
    }

    pub fn enabled_autocontinue_sessions(&self) -> Result<Vec<SessionView>> {
        Ok(self
            .get_sessions()?
            .into_iter()
            .filter(|s| s.auto_continue_enabled)
            .collect())
    }
}
