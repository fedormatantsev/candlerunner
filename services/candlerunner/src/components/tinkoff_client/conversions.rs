use crate::generated::tinkoff_invest_api;
use crate::models::instruments::{Figi, Instrument, Ticker};
use crate::models::market_data::Candle;

const NANO: f64 = 10e-9;

impl From<tinkoff_invest_api::Share> for Instrument {
    fn from(proto: tinkoff_invest_api::Share) -> Self {
        Instrument {
            figi: Figi(proto.figi),
            ticker: Ticker(proto.ticker),
            display_name: proto.name,
        }
    }
}

fn to_f64(quote: tinkoff_invest_api::Quotation) -> f64 {
    (quote.units as f64) + (quote.nano as f64) * NANO
}

impl TryFrom<tinkoff_invest_api::HistoricCandle> for Candle {
    type Error = anyhow::Error;

    fn try_from(proto: tinkoff_invest_api::HistoricCandle) -> Result<Self, Self::Error> {
        let high = proto
            .high
            .ok_or_else(|| anyhow::anyhow!("HistoricalCandle `high` field is missing"))?;
        let low = proto
            .low
            .ok_or_else(|| anyhow::anyhow!("HistoricalCandle `low` field is missing"))?;
        let open = proto
            .open
            .ok_or_else(|| anyhow::anyhow!("HistoricalCandle `open` field is missing"))?;
        let close = proto
            .close
            .ok_or_else(|| anyhow::anyhow!("HistoricalCandle `close` field is missing"))?;

        let high = to_f64(high);
        let low = to_f64(low);
        let open = to_f64(open);
        let close = to_f64(close);

        Ok(Candle {
            high,
            low,
            open,
            close,
            volume: proto.volume as u64,
        })
    }
}
