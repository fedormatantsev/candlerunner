use std::collections::BTreeMap;

use chrono::{prelude::*, Duration};

use crate::components;
use crate::models::instruments::Figi;
use crate::models::market_data::{Candle, CandlePack, CandleResolution, DataAvailability};

fn interpolate(
    candle_resolution: CandleResolution,
    data: BTreeMap<DateTime<Utc>, Candle>,
) -> BTreeMap<DateTime<Utc>, Candle> {
    let now = Utc::now();
    let candle_interval = Duration::from(candle_resolution);

    let mut result: BTreeMap<DateTime<Utc>, Candle> = Default::default();

    for (ts, candle) in data {
        let ts = align_timestamp(candle_interval, ts);
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

fn align_timestamp(candle_interval: Duration, ts: DateTime<Utc>) -> DateTime<Utc> {
    let midnight = ts.date().and_hms(0, 0, 0);
    let since_midnight = ts - midnight;
    let n_candles = (since_midnight.num_minutes() / candle_interval.num_minutes()) as i32;

    midnight + candle_interval * n_candles
}

pub async fn read_market_data(
    mongo: &components::Mongo,
    requirements: &[Figi],
    time_from: DateTime<Utc>,
    mut time_to: DateTime<Utc>,
    candle_resolution: CandleResolution,
) -> anyhow::Result<BTreeMap<DateTime<Utc>, CandlePack>> {
    let time_from = align_timestamp(candle_resolution.into(), time_from);

    for figi in requirements {
        let availability_timeline = mongo.read_candle_data_availability(figi).await?;

        let mut cur_date = time_from.date();

        while cur_date < time_to.date() {
            let availability = availability_timeline
                .get(&cur_date)
                .unwrap_or(&DataAvailability::Unavailable);

            match availability {
                DataAvailability::Unavailable => {
                    time_to = time_to.min(cur_date.and_hms(0, 0, 0));
                    break;
                }
                DataAvailability::Available => (),
                DataAvailability::PartiallyAvailable { available_up_to } => {
                    time_to = time_to.min(*available_up_to);
                    break;
                }
            }

            cur_date = cur_date.succ();
        }
    }

    if time_to <= time_from {
        return Ok(Default::default());
    }

    let mut packed_candles: BTreeMap<DateTime<Utc>, CandlePack> = Default::default();

    for figi in requirements {
        let data = mongo.read_candles(figi, time_from, time_to).await?;
        let candles_timeline = interpolate(candle_resolution, data);

        for (ts, candle) in candles_timeline {
            let pack = packed_candles.entry(ts).or_default();
            pack.insert(figi.clone(), candle);
        }
    }

    Ok(packed_candles)
}
