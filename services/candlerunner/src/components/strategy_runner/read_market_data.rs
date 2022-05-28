use std::collections::{BTreeMap, HashMap};

use chrono::{prelude::*, Duration};

use crate::components;
use crate::models::instruments::Figi;
use crate::models::market_data::{Candle, CandleResolution, DataAvailability};

fn interpolate(
    candle_resolution: CandleResolution,
    data: BTreeMap<DateTime<Utc>, Candle>,
) -> BTreeMap<DateTime<Utc>, Candle> {
    let now = Utc::now();
    let candle_interval = candle_resolution.interval();

    let mut result: BTreeMap<DateTime<Utc>, Candle> = Default::default();

    for (ts, candle) in data {
        let ts = get_candle_ts(candle_interval, ts);
        if (now - ts) < candle_interval {
            continue;
        }

        let interpolated_candle = result.entry(ts).or_insert(candle);

        interpolated_candle.high = interpolated_candle.high.max(candle.high);
        interpolated_candle.low = interpolated_candle.low.min(candle.low);
        // interpolated_candle.open stays as is
        interpolated_candle.close = candle.close;
        interpolated_candle.volume += candle.volume;
    }

    result
}

fn get_candle_ts(candle_interval: Duration, ts: DateTime<Utc>) -> DateTime<Utc> {
    let midnight = ts.date().and_hms(0, 0, 0);
    let since_midnight = ts - midnight;
    let n_candles = (since_midnight.num_minutes() / candle_interval.num_minutes()) as i32;

    midnight + candle_interval * n_candles
}

pub async fn read_market_data(
    mongo: &components::Mongo,
    requirements: &[Figi],
    time_from: DateTime<Utc>,
    time_to: DateTime<Utc>,
    candle_resolution: CandleResolution,
) -> anyhow::Result<BTreeMap<DateTime<Utc>, HashMap<Figi, Candle>>> {
    let date_to = time_to.date();
    let mut last_available_ts = time_to;

    for figi in requirements {
        let availability_timeline = mongo.read_candle_data_availability(figi).await?;

        let mut cur_date = time_from.date();

        while cur_date < date_to {
            let availability = availability_timeline
                .get(&cur_date)
                .unwrap_or(&DataAvailability::Unavailable);

            match availability {
                DataAvailability::Unavailable => {
                    last_available_ts = last_available_ts.min(cur_date.and_hms(0, 0, 0));
                    break;
                }
                DataAvailability::Available => (),
                DataAvailability::PartiallyAvailable { cursor } => {
                    last_available_ts = last_available_ts.min(cursor.clone());
                    break;
                }
            }

            cur_date = cur_date.succ();
        }
    }

    if last_available_ts <= time_from {
        return Ok(Default::default());
    }

    let mut packed_candles: BTreeMap<DateTime<Utc>, HashMap<Figi, Candle>> = Default::default();

    for figi in requirements {
        let data = mongo
            .read_candles(figi, time_from, last_available_ts)
            .await?;
        let candles_timeline = interpolate(candle_resolution, data);

        for (ts, candle) in candles_timeline {
            let pack = packed_candles.entry(ts).or_default();
            pack.insert(figi.clone(), candle);
        }
    }

    let filtered_candles: BTreeMap<DateTime<Utc>, HashMap<Figi, Candle>> = packed_candles
        .into_iter()
        .filter(|(_, pack)| pack.len() == requirements.len())
        .collect();

    Ok(filtered_candles)
}
