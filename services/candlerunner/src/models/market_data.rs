use std::collections::{BTreeMap, HashMap};

use chrono::{prelude::*, Duration};
use serde::{Deserialize, Serialize};

use crate::models::instruments::Figi;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub close: f64,

    /// Volume in lots
    pub volume: u64,
}

pub type CandleTimeline = BTreeMap<DateTime<Utc>, Candle>;
pub type CandlePack = HashMap<Figi, Candle>;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CandleResolution {
    OneMinute,
    OneHour,
    OneDay,
}

impl ToString for CandleResolution {
    fn to_string(&self) -> String {
        match self {
            CandleResolution::OneMinute => "oneMinute".to_string(),
            CandleResolution::OneHour => "oneHour".to_string(),
            CandleResolution::OneDay => "oneDay".to_string(),
        }
    }
}

impl From<CandleResolution> for Duration {
    fn from(val: CandleResolution) -> Self {
        match val {
            CandleResolution::OneMinute => Duration::minutes(1),
            CandleResolution::OneHour => Duration::hours(1),
            CandleResolution::OneDay => Duration::days(1),
        }
    }
}

/// Data availability on a trading day
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataAvailability {
    /// Data was not fetched from data provider
    Unavailable,

    /// Data for complete day is available
    Available,

    /// Data is partially available [00:00; `available_up_to`)
    PartiallyAvailable { available_up_to: DateTime<Utc> },
}
