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

pub fn seed_pricing_if_empty(db: &Db, seed_json: &str) -> Result<()> {
    if !db.get_pricing()?.is_empty() {
        return Ok(());
    }
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
}
