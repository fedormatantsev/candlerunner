use std::{collections::HashSet, sync::Arc};

use chrono::prelude::*;
use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::components;
use crate::models::{instruments::Figi, market_data::DataAvailability};

use super::requirements_collector::{Ranges, RequirementsCollector};

pub struct MarketDataSyncPeriodic {
    tinkoff_client: Arc<components::TinkoffClient>,
    mongo: Arc<components::Mongo>,
    strategy_cache: Arc<components::StrategyCache>,
    max_chunks_per_instrument: usize,
}

impl ComponentName for MarketDataSyncPeriodic {
    fn component_name() -> &'static str {
        "market-data-sync"
    }
}

impl Periodic for MarketDataSyncPeriodic {
    type State = ();

    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> periodic_component::PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;
            let mongo = resolver.resolve::<components::Mongo>().await?;
            let strategy_cache = resolver.resolve::<components::StrategyCache>().await?;

            let max_chunks_per_instrument = config.get_u64("max_chunks_per_instrument")? as usize;

            let periodic = Self {
                tinkoff_client,
                mongo,
                strategy_cache,
                max_chunks_per_instrument,
            };

            Ok((periodic, ()))
        })
    }

    fn step<'periodic>(
        &'periodic mut self,
        state: Arc<Self::State>,
    ) -> periodic_component::PeriodicFuture<'periodic, Self::State> {
        Box::pin(async move {
            let strategies = self.strategy_cache.state();
            let mut collector = RequirementsCollector::default();

            for (def, strategy) in strategies.values() {
                strategy.data_requirements().iter().for_each(|figi| {
                    collector.push(figi.clone(), def.time_from(), def.time_to());
                });
            }

            let data_ranges = collector.finalize();

            for (figi, ranges) in data_ranges {
                match self.sync_market_data_ranges(&figi, ranges).await {
                    Ok(_) => (),
                    Err(err) => println!(
                        "Failed to retrieve market data for {}: {}",
                        figi.0,
                        err.to_string()
                    ),
                }
            }

            Ok(state)
        })
    }
}

impl MarketDataSyncPeriodic {
    async fn sync_candle_data(&self, figi: &Figi, date: Date<Utc>) -> anyhow::Result<()> {
        let request_ts = date.and_hms(0, 0, 0);

        let candles = self
            .tinkoff_client
            .get_candles(&figi, request_ts, date.succ().and_hms(0, 0, 0))
            .await?;

        let mut cursor: Option<DateTime<Utc>> = None;

        let mut one_minute_candles = 0usize;
        let mut one_hour_candles = 0usize;
        let mut one_day_candles = 0usize;

        for (ts, _) in candles.iter() {
            if let Some(cursor) = cursor {
                if cursor.hour() < ts.hour() {
                    one_hour_candles += 1;
                    one_minute_candles += 1;
                } else if cursor.minute() < ts.minute() {
                    one_minute_candles += 1;
                }
            } else {
                one_day_candles += 1;
            }

            cursor = Some(*ts);
        }

        self.mongo.write_candles(&figi, candles).await?;
        self.mongo
            .write_candle_data_availability(
                &figi,
                date,
                DataAvailability::new(one_minute_candles, one_hour_candles, one_day_candles),
            )
            .await?;

        Ok(())
    }

    async fn sync_market_data_ranges(&self, figi: &Figi, ranges: Ranges) -> anyhow::Result<()> {
        let availability = self.mongo.read_candle_data_availability(&figi).await?;

        let days = ranges
            .into_iter()
            .fold(HashSet::<Date<Utc>>::default(), |mut days, range| {
                let mut from = range.0.date();
                let to = range.1.date().succ();

                while from < to {
                    if let None = availability.get(&from) {
                        days.insert(from);
                    }

                    from = from.succ();
                }

                days
            });

        let mut chunks_fetched = 0;

        for date in days {
            if chunks_fetched > self.max_chunks_per_instrument {
                break;
            }

            let fetch_result = self.sync_candle_data(&figi, date).await;

            if let Err(err) = fetch_result {
                println!(
                    "Failed to fetch candles data for {} at {}: {}",
                    figi.0,
                    date,
                    err.to_string(),
                );
                continue;
            }

            chunks_fetched += 1;
        }

        Ok(())
    }
}

pub type MarketDataSync = PeriodicComponent<MarketDataSyncPeriodic>;
