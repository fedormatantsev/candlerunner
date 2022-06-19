use std::collections::HashSet;
use std::sync::Arc;

use chrono::prelude::*;
use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::components;
use crate::models::instruments::Figi;
use crate::models::market_data::DataAvailability;

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

    fn step(
        &mut self,
        state: Arc<Self::State>,
    ) -> periodic_component::PeriodicFuture<'_, Self::State> {
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
                    Err(err) => println!("Failed to retrieve market data for {}: {}", figi.0, err),
                }
            }

            Ok(state)
        })
    }
}

impl MarketDataSyncPeriodic {
    async fn sync_candle_data(&self, figi: &Figi, cursor: DateTime<Utc>) -> anyhow::Result<()> {
        let candles = self
            .tinkoff_client
            .get_candles(figi, cursor, cursor.date().and_hms(23, 59, 59))
            .await?;

        let today = Utc::now().date();

        let availability = if cursor.date() == today {
            let available_up_to = candles.keys().last().cloned().unwrap_or(cursor);

            DataAvailability::PartiallyAvailable { available_up_to }
        } else {
            DataAvailability::Available
        };

        self.mongo.write_candles(figi, candles).await?;
        self.mongo
            .write_candle_data_availability(figi, cursor.date(), availability)
            .await?;

        Ok(())
    }

    async fn sync_market_data_ranges(&self, figi: &Figi, ranges: Ranges) -> anyhow::Result<()> {
        let availability = self.mongo.read_candle_data_availability(figi).await?;

        let cursors =
            ranges
                .into_iter()
                .fold(HashSet::<DateTime<Utc>>::default(), |mut cursors, range| {
                    let mut from = range.0.date();
                    let to = range.1.date().succ();

                    while from < to {
                        let availability = availability
                            .get(&from)
                            .cloned()
                            .unwrap_or(DataAvailability::Unavailable);

                        let cursor = match availability {
                            DataAvailability::Unavailable => Some(from.and_hms(0, 0, 0)),
                            DataAvailability::Available => None,
                            DataAvailability::PartiallyAvailable {
                                available_up_to: cursor,
                            } => Some(cursor),
                        };

                        if let Some(cursor) = cursor {
                            cursors.insert(cursor);
                        }

                        from = from.succ();
                    }

                    cursors
                });

        let mut chunks_fetched = 0;

        for cursor in cursors {
            if chunks_fetched > self.max_chunks_per_instrument {
                break;
            }

            let fetch_result = self.sync_candle_data(figi, cursor).await;

            if let Err(err) = fetch_result {
                println!(
                    "Failed to fetch candles data for {} at {}: {}",
                    figi.0,
                    cursor.date(),
                    err,
                );
                continue;
            }

            chunks_fetched += 1;
        }

        Ok(())
    }
}

pub type MarketDataSync = PeriodicComponent<MarketDataSyncPeriodic>;
