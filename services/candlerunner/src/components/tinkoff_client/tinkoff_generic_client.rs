use chrono::prelude::*;

use crate::models::account::Account;
use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::CandleTimeline;

#[async_trait::async_trait]
pub trait TinkoffGenericClient {
    async fn get_instruments(&self) -> anyhow::Result<Vec<Instrument>>;
    async fn get_candles(
        &self,
        figi: &Figi,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<CandleTimeline>;
    
    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>>;
}
