//! Finding medicines by symptom, and the symptom suggestions.

use axum::Json;
use axum::extract::{Query, State};
use chassis::storage::AppDb;
use serde::{Deserialize, Serialize};

use super::{ApiResult, Clock};
use crate::domain::medicine::{Freshness, StockLevel};
use crate::store::medicines::{self, Medicine};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    symptom: String,
    #[serde(flatten)]
    clock: Clock,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    symptom: String,
    /// In the cabinet, switched on, in stock and not expired.
    usable: Vec<Medicine>,
    /// Known for this symptom but out of stock, expired or switched off.
    unavailable: Vec<Medicine>,
}

fn is_usable(m: &Medicine) -> bool {
    m.active
        && m.stock_level != StockLevel::Out
        && m.freshness != Freshness::Expired
        && !m.course_over
}

pub async fn by_symptom(
    State(db): State<AppDb>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<SearchResult> {
    let today = q.clock.date()?;
    let needle = q.symptom.trim().to_lowercase();
    let wanted = needle.clone();
    let all = db.read(move |conn| medicines::list(conn, today)).await?;
    let (usable, unavailable) = all
        .into_iter()
        .filter(|m| m.symptoms.iter().any(|s| s.contains(&wanted)))
        .partition(is_usable);
    Ok(Json(SearchResult {
        symptom: needle,
        usable,
        unavailable,
    }))
}
