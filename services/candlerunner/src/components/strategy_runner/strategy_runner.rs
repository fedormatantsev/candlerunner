use std::sync::Arc;

use chrono::{prelude::*, Duration};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};

use crate::components;
use crate::models::strategy::{
    Strategy, StrategyExecutionContext, StrategyExecutionError, StrategyExecutionState,
    StrategyExecutionStatus, StrategyInstanceDefinition,
};

use super::read_market_data;

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

    async fn exec_strategy(
        &self,
        strategy_id: &uuid::Uuid,
        strategy_definition: &StrategyInstanceDefinition,
        strategy: &dyn Strategy,
    ) -> anyhow::Result<()> {
        let execution_state = self
            .mongo
            .read_strategy_execution_state(strategy_id)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to read execution state for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                )
            })?;

        let execution_state = execution_state.unwrap_or_else(|| {
            StrategyExecutionState::new(
                StrategyExecutionStatus::Running,
                strategy_definition.time_from(),
            )
        });

        if execution_state.status() != StrategyExecutionStatus::Running {
            return Ok(());
        }

        let execution_contexts = self
            .mongo
            .read_strategy_execution_contexts(
                strategy_id,
                execution_state.cursor() - Duration::seconds(10),
                strategy_definition.time_to(),
            )
            .await?;

        let (time_from, mut prev_context) = execution_contexts
            .into_iter()
            .last()
            .map(|(ts, ctx)| (ts, Some(ctx)))
            .unwrap_or_else(|| (strategy_definition.time_from(), None));

        let time_to = match strategy_definition.time_to() {
            Some(ts) => ts,
            None => Utc::now(),
        };

        let data_requirements = strategy.data_requirements();
        let packed_candles = read_market_data::read_market_data(
            self.mongo.as_ref(),
            data_requirements,
            time_from,
            time_to,
            strategy_definition.resolution(),
        )
        .await?;

        let mut contexts: Vec<(DateTime<Utc>, StrategyExecutionContext)> = Default::default();

        // TODO: execute strategies in chunks
        for (ts, candles) in packed_candles {
            let execution_result = strategy.execute(ts, candles, prev_context);

            match execution_result {
                Ok(ctx) => {
                    contexts.push((ts, ctx.clone()));
                    prev_context = Some(ctx);
                }
                Err(err) => {
                    match err {
                        StrategyExecutionError::NonFixableFailure => {
                            if let Err(err) = self
                                .mongo
                                .write_strategy_execution_state(
                                    strategy_id,
                                    &StrategyExecutionState::new(
                                        StrategyExecutionStatus::Failed,
                                        ts,
                                    ),
                                )
                                .await
                            {
                                println!(
                                    "Failed to update execution state for strategy {}: {}",
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

        let cursor = contexts
            .iter()
            .last()
            .map(|(k, _)| k.clone())
            .unwrap_or(execution_state.cursor());

        let status = match strategy_definition.time_to() {
            Some(time_to) => {
                if cursor < time_to {
                    StrategyExecutionStatus::Running
                } else {
                    StrategyExecutionStatus::Finished
                }
            }
            None => StrategyExecutionStatus::Running,
        };

        self.mongo
            .write_strategy_execution_state(
                strategy_id,
                &StrategyExecutionState::new(status, cursor),
            )
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to update execution status for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                )
            })?;

        self.mongo
            .write_strategy_execution_contexts(strategy_id, contexts)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to write strategy execution contexts for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                )
            })?;

        Ok(())
    }

    async fn step(
        &mut self,
        prev_state: Arc<<Self as Periodic>::State>,
    ) -> anyhow::Result<Arc<<Self as Periodic>::State>> {
        let strategies = self.strategy_cache.state();

        for (strategy_id, (def, strategy)) in strategies.iter() {
            if let Err(err) = self
                .exec_strategy(strategy_id, def, strategy.as_ref())
                .await
            {
                println!("Failed to execute strategy: {}", err.to_string());
            }
        }

        return Ok(prev_state);
    }
}

pub type StrategyRunner = PeriodicComponent<StrategyRunnerPeriodic>;
