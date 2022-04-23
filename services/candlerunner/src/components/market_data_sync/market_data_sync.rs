use std::{collections::HashSet, sync::Arc};

use chrono::prelude::*;
use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::{components, models::instruments::Figi};

use super::requirements_collector::RequirementsCollector;

pub struct MarketDataSyncPeriodic {
    tinkoff_client: Arc<components::TinkoffClient>,
    mongo: Arc<components::Mongo>,
    strategy_cache: Arc<components::StrategyCache>,
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
        _: Box<dyn ConfigProvider>,
    ) -> periodic_component::PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;
            let mongo = resolver.resolve::<components::Mongo>().await?;
            let strategy_cache = resolver.resolve::<components::StrategyCache>().await?;

            let periodic = Self {
                tinkoff_client,
                mongo,
                strategy_cache,
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

            for strategy in strategies.values() {
                strategy
                    .data_requirements()
                    .iter()
                    .for_each(|req| collector.push(req));
            }

            let data_ranges = collector.finalize();

            for (figi, ranges) in data_ranges {
                let days =
                    ranges
                        .into_iter()
                        .fold(HashSet::<Date<Utc>>::default(), |mut days, range| {
                            let mut from = range.0.date();
                            let to = range.1.date().succ();

                            while from < to {
                                days.insert(from.clone());
                                from = from.succ();
                            }

                            days
                        });

                for date in days {
                    let fetch_result = self.fetch_candle_data(&figi, date).await;
                    if let Err(err) = fetch_result {
                        println!(
                            "Failed to fetch candles data for {}: {}",
                            figi.0,
                            err.to_string(),
                        );
                    }
                }
            }

            Ok(state)
        })
    }
}

impl MarketDataSyncPeriodic {
    async fn fetch_candle_data(&self, figi: &Figi, date: Date<Utc>) -> anyhow::Result<()> {
        let available = self.mongo.get_candle_data_availability(&figi, date).await?;
        if available {
            return Ok(());
        }

        let candles = self
            .tinkoff_client
            .get_candles(&figi, date.and_hms(0, 0, 0), date.succ().and_hms(0, 0, 0))
            .await?;

        self.mongo.write_candles(&figi, candles).await?;
        self.mongo.set_candle_data_availability(&figi, date).await?;

        Ok(())
    }
}

pub type MarketDataSync = PeriodicComponent<MarketDataSyncPeriodic>;
