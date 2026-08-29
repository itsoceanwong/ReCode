use anyhow::Result;

use crate::db::Db;
use crate::models::LimitWindow;

pub fn upsert(db: &Db, win: LimitWindow) -> Result<()> {
    db.upsert_limit(
        &win.source,
        &win.window_kind,
        win.used_percent,
        win.resets_at,
        win.is_manual,
    )
}

pub fn all(db: &Db) -> Result<Vec<LimitWindow>> {
    db.all_limits()
}

pub fn next_reset(db: &Db, source: &str, kind: &str) -> Option<i64> {
    all(db)
        .ok()?
        .into_iter()
        .find(|w| w.source == source && w.window_kind == kind)
        .and_then(|w| w.resets_at)
}
