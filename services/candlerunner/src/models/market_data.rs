use std::collections::BTreeMap;

use chrono::{prelude::*, Duration};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Candle {
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub close: f64,

    /// Volume in lots
    pub volume: u64,
}

pub type CandleTimeline = BTreeMap<DateTime<Utc>, Candle>;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum CandleResolution {
    OneMinute,
    OneHour,
    OneDay,
}

impl CandleResolution {
    pub fn interval(&self) -> Duration {
        match self {
            CandleResolution::OneMinute => Duration::minutes(1),
            CandleResolution::OneHour => Duration::hours(1),
            CandleResolution::OneDay => Duration::days(1),
        }
    }
}

/// Data availability on a trading day
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataAvailability {
    /// Data was not fetched from data provider
    Unavailable,

    /// Data for complete day is available
    Available,

    /// Data is available from 00:00 up to cursor
    PartiallyAvailable { cursor: DateTime<Utc> },
}
