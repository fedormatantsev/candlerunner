use std::collections::BTreeMap;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Candle {
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub close: f64,

    /// Volume in lots
    pub volume: u64,
}

pub type CandleTimeline = BTreeMap<DateTime<Utc>, Candle>;
