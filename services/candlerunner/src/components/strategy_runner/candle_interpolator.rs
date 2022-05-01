use std::collections::{BTreeMap, HashMap};

use chrono::{prelude::*, Duration};

use crate::models::instruments::Figi;
use crate::models::market_data::{Candle, CandleResolution};

pub struct CandleInterpolator {
    candle_interval: Duration,
    data: BTreeMap<DateTime<Utc>, HashMap<Figi, Candle>>,
}

impl CandleInterpolator {
    pub fn new(candle_resolution: CandleResolution) -> Self {
        Self {
            candle_interval: candle_resolution.interval(),
            data: Default::default(),
        }
    }

    pub fn insert_candle_data(&mut self, figi: &Figi, data: BTreeMap<DateTime<Utc>, Candle>) {
        let now = Utc::now();

        for (ts, candle) in data {
            let ts = self.get_candle_ts(ts);
            if (now - ts) < self.candle_interval {
                continue;
            }

            let val = self.data.entry(ts).or_default();
            let interpolated_candle = val.entry(figi.clone()).or_insert(candle);

            interpolated_candle.high = interpolated_candle.high.max(candle.high);
            interpolated_candle.low = interpolated_candle.low.min(candle.low);
            // interpolated_candle.open stays as is
            interpolated_candle.close = candle.close;
            interpolated_candle.volume += candle.volume;
        }
    }

    fn get_candle_ts(&self, ts: DateTime<Utc>) -> DateTime<Utc> {
        let midnight = ts.date().and_hms(0, 0, 0);
        let since_midnight = ts - midnight;
        let n_candles = (since_midnight.num_minutes() / self.candle_interval.num_minutes()) as i32;

        midnight + self.candle_interval * n_candles
    }

    pub fn data(self) -> BTreeMap<DateTime<Utc>, HashMap<Figi, Candle>> {
        return self.data;
    }
}
