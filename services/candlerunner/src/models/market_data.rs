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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataAvailability {
    one_minute_candles: usize,
    one_hour_candles: usize,
    one_day_candles: usize,
}

impl DataAvailability {
    pub fn new(one_minute_candles: usize, one_hour_candles: usize, one_day_candles: usize) -> Self {
        Self {
            one_minute_candles,
            one_hour_candles,
            one_day_candles,
        }
    }
}
