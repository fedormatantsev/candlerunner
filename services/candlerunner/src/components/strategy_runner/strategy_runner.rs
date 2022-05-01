use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use chrono::prelude::*;

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};

use crate::{
    components,
    models::{
        instruments::Figi,
        market_data::DataAvailability,
        strategy::{
            StrategyExecutionContext, StrategyExecutionError, StrategyExecutionOutput,
            StrategyExecutionStatus,
        },
    },
};

use super::candle_interpolator::CandleInterpolator;

pub struct StrategyRunnerPeriodic {
    strategy_cache: Arc<components::StrategyCache>,
    mongo: Arc<components::Mongo>,
}

impl ComponentName for StrategyRunnerPeriodic {
    fn component_name() -> &'static str {
        "strategy-runner"
    }
}

impl Periodic for StrategyRunnerPeriodic {
    type State = ();

    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(Self::new(resolver, config))
    }

    fn step(&mut self, prev_state: Arc<Self::State>) -> PeriodicFuture<Self::State> {
        Box::pin(self.step(prev_state))
    }
}

impl StrategyRunnerPeriodic {
    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<(Self, <Self as Periodic>::State), ComponentError> {
        let strategy_cache = resolver.resolve::<components::StrategyCache>().await?;
        let mongo = resolver.resolve::<components::Mongo>().await?;

        Ok((
            Self {
                strategy_cache,
                mongo,
            },
            <Self as Periodic>::State::default(),
        ))
    }

    async fn step(
        &mut self,
        prev_state: Arc<<Self as Periodic>::State>,
    ) -> anyhow::Result<Arc<<Self as Periodic>::State>> {
        let strategies = self.strategy_cache.state();

        //
        // Collect required instruments availability
        //
        let data_requirements = strategies.iter().fold(
            HashSet::<Figi>::default(),
            |mut instruments, (_, (_, strategy))| {
                for figi in strategy.data_requirements() {
                    instruments.insert(figi.clone());
                }

                return instruments;
            },
        );

        let mut availability: HashMap<Figi, BTreeMap<Date<Utc>, DataAvailability>> =
            Default::default();

        for figi in data_requirements {
            let payload = self.mongo.read_candle_data_availability(&figi).await;
            match payload {
                Ok(data) => {
                    availability.insert(figi, data);
                }
                Err(err) => {
                    println!(
                        "Failed to get market availability for {}: {}",
                        figi.0,
                        err.to_string()
                    );
                }
            }
        }

        //
        // Execute strategies
        //
        for (strategy_id, (def, strategy)) in strategies.iter() {
            let execution_status =
                match self.mongo.read_strategy_execution_status(strategy_id).await {
                    Ok(status) => status,
                    Err(err) => {
                        println!(
                            "Failed to read strategy execution status for strategy {}: {}",
                            strategy_id,
                            err.to_string()
                        );
                        StrategyExecutionStatus::Running
                    }
                };

            if execution_status != StrategyExecutionStatus::Running {
                continue;
            }

            let execution_contexts = self
                .mongo
                .read_strategy_execution_contexts(strategy_id, def.time_from(), def.time_to())
                .await?;

            let last_ctx = execution_contexts.into_iter().last();

            let (time_from, mut prev_context) = match last_ctx {
                Some((ts, ctx)) => (ts + def.resolution().interval(), Some(ctx)),
                None => (def.time_from(), None),
            };

            let time_to = match def.time_to() {
                Some(ts) => ts,
                None => Utc::now(),
            };

            let data_requirements = strategy.data_requirements();

            let mut interpolator = CandleInterpolator::new(def.resolution());

            for figi in data_requirements.iter() {
                match self.mongo.read_candles(figi, time_from, time_to).await {
                    Ok(timeline) => interpolator.insert_candle_data(figi, timeline),
                    Err(err) => println!("Failed to retrieve candles: {}", err),
                }
            }

            let interpolated_candles = interpolator.data();
            let mut contexts: Vec<(DateTime<Utc>, StrategyExecutionContext)> = Default::default();

            for (ts, candles) in interpolated_candles {
                let execution_result = strategy.execute(ts, candles, prev_context);

                match execution_result {
                    Ok(output) => match output {
                        StrategyExecutionOutput::Unavailable => {
                            println!("Strategy output is unavailable, will retry later");
                            break;
                        }
                        StrategyExecutionOutput::Available(ctx) => {
                            contexts.push((ts, ctx.clone()));
                            prev_context = Some(ctx);
                        }
                    },
                    Err(err) => {
                        match err {
                            StrategyExecutionError::NonFixableFailure => {
                                if let Err(err) = self
                                    .mongo
                                    .write_strategy_execution_status(
                                        strategy_id,
                                        &StrategyExecutionStatus::Failed,
                                    )
                                    .await
                                {
                                    println!(
                                        "Failed to update execution status for strategy {}: {}",
                                        strategy_id,
                                        err.to_string()
                                    );
                                }
                            }
                            _ => (),
                        };

                        println!("Strategy execution failed: {}", err.to_string());
                        break;
                    }
                }
            }

            if def.time_to().is_some() {
                if let Err(err) = self
                    .mongo
                    .write_strategy_execution_status(
                        strategy_id,
                        &StrategyExecutionStatus::Finished,
                    )
                    .await
                {
                    println!(
                        "Failed to update execution status for strategy {}: {}",
                        strategy_id,
                        err.to_string()
                    );
                }
            }

            match self
                .mongo
                .write_strategy_execution_contexts(strategy_id, contexts)
                .await
            {
                Ok(_) => (),
                Err(err) => println!(
                    "Failed to write strategy execution contexts for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                ),
            }
        }

        return Ok(prev_state);
    }
}

pub type StrategyRunner = PeriodicComponent<StrategyRunnerPeriodic>;
