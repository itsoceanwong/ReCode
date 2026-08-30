use anyhow::Result;
use serde::Deserialize;

use crate::db::Db;
use crate::models::PricingRow;

#[derive(Debug, Deserialize)]
struct SeedRow {
    model: String,
    input_pm: f64,
    output_pm: f64,
    cache_read_pm: f64,
    cache_write_pm: f64,
}

const OBSOLETE_DEFAULT_MODELS: &[&str] = &[
    "claude-sonnet-4",
    "claude-opus-4",
    "claude-haiku-4",
    "gpt-5",
    "gpt-4.1",
    "o3",
    "o4-mini",
];

/// Upsert official seed rates and remove obsolete previous defaults.
pub fn sync_pricing_seed(db: &Db, seed_json: &str) -> Result<()> {
    let rows: Vec<SeedRow> = serde_json::from_str(seed_json)?;
    for row in rows {
        db.set_pricing(&PricingRow {
            model: row.model,
            input_pm: row.input_pm,
            output_pm: row.output_pm,
            cache_read_pm: row.cache_read_pm,
            cache_write_pm: row.cache_write_pm,
        })?;
    }
    for model in OBSOLETE_DEFAULT_MODELS {
        db.delete_pricing(model)?;
    }
    Ok(())
}

pub fn matcher(db: &Db, model: &str) -> PricingRow {
    db.find_pricing(model).ok().flatten().unwrap_or(PricingRow {
        model: model.to_string(),
        input_pm: 0.0,
        output_pm: 0.0,
        cache_read_pm: 0.0,
        cache_write_pm: 0.0,
    })
}

pub fn compute_cost(
    pricing: &PricingRow,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
) -> f64 {
    (input as f64 * pricing.input_pm
        + output as f64 * pricing.output_pm
        + cache_read as f64 * pricing.cache_read_pm
        + cache_write as f64 * pricing.cache_write_pm)
        / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    const NEW_SEED: &str = r#"[
      {"model":"gpt-5.6-sol","input_pm":4.0,"output_pm":20.0,"cache_read_pm":0.4,"cache_write_pm":5.0},
      {"model":"claude-sonnet-5","input_pm":2.0,"output_pm":10.0,"cache_read_pm":0.2,"cache_write_pm":2.5}
    ]"#;

    #[test]
    fn cost_math() {
        let p = PricingRow {
            model: "t".into(),
            input_pm: 1.0,
            output_pm: 2.0,
            cache_read_pm: 0.1,
            cache_write_pm: 0.5,
        };
        let c = compute_cost(&p, 1_000_000, 1_000_000, 1_000_000, 1_000_000);
        assert!((c - 3.6).abs() < 1e-9);
    }

    #[test]
    fn sync_upserts_rates_and_removes_obsolete_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).unwrap();
        db.set_pricing(&PricingRow {
            model: "gpt-5".into(),
            input_pm: 1.25,
            output_pm: 10.0,
            cache_read_pm: 0.125,
            cache_write_pm: 1.25,
        })
        .unwrap();
        db.set_pricing(&PricingRow {
            model: "custom-local".into(),
            input_pm: 9.0,
            output_pm: 9.0,
            cache_read_pm: 9.0,
            cache_write_pm: 9.0,
        })
        .unwrap();
        db.set_pricing(&PricingRow {
            model: "gpt-5.6-sol".into(),
            input_pm: 99.0,
            output_pm: 99.0,
            cache_read_pm: 99.0,
            cache_write_pm: 99.0,
        })
        .unwrap();

        sync_pricing_seed(&db, NEW_SEED).unwrap();
        let rows = db.get_pricing().unwrap();
        let by = |m: &str| rows.iter().find(|r| r.model == m).cloned();

        assert!(by("gpt-5").is_none(), "obsolete default removed");
        assert_eq!(by("custom-local").unwrap().input_pm, 9.0);
        let sol = by("gpt-5.6-sol").unwrap();
        assert!((sol.input_pm - 4.0).abs() < 1e-9);
        assert!((sol.output_pm - 20.0).abs() < 1e-9);
        assert!(by("claude-sonnet-5").is_some());
    }

    #[test]
    fn matcher_prefers_exact_gpt_56_alias_over_luna_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).unwrap();
        let seed = include_str!("../resources/pricing-seed.json");
        sync_pricing_seed(&db, seed).unwrap();
        let row = matcher(&db, "gpt-5.6");
        assert_eq!(row.model, "gpt-5.6");
        assert!((row.input_pm - 4.0).abs() < 1e-9);
    }
}
