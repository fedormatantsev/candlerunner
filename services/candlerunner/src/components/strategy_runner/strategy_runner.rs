use std::sync::Arc;

use chrono::prelude::*;

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};

use crate::components;
use crate::models::strategy::{
    Strategy, StrategyExecution, StrategyExecutionError, StrategyExecutionStatus,
    StrategyInstanceDefinition, StrategyState,
};

use super::read_market_data;

pub struct StrategyRunnerPeriodic {
    strategy_cache: Arc<components::StrategyCache>,
    mongo: Arc<components::Mongo>,
}

impl ComponentName for StrategyRunnerPeriodic {
    fn component_name() -> &'static str {
        "strategyRunner"
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

    async fn init_execution(
        &self,
        strategy_id: &uuid::Uuid,
        strategy_definition: &StrategyInstanceDefinition,
    ) -> anyhow::Result<(StrategyExecution, StrategyState)> {
        let mut execution = self
            .mongo
            .read_strategy_execution(strategy_id)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to read execution for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                )
            })?
            .unwrap_or_else(|| {
                StrategyExecution::new(
                    StrategyExecutionStatus::Running,
                    strategy_definition.time_from(),
                )
            });

        let states = self
            .mongo
            .read_strategy_state(
                strategy_id,
                execution.last_execution_timestamp(),
                strategy_definition.time_to(),
            )
            .await?;

        let (last_execution_timestamp, last_state) = states
            .into_iter()
            .last()
            .map(|(ts, state)| (ts, state))
            .unwrap_or_else(|| {
                (
                    execution.last_execution_timestamp(),
                    StrategyState::default(),
                )
            });

        if last_execution_timestamp > execution.last_execution_timestamp() {
            execution.set_last_execution_timestamp(last_execution_timestamp);
        }

        Ok((execution, last_state))
    }

    async fn exec_strategy(
        &self,
        strategy_id: &uuid::Uuid,
        strategy_definition: &StrategyInstanceDefinition,
        strategy: &dyn Strategy,
    ) -> anyhow::Result<()> {
        let (mut execution, mut last_state) = self
            .init_execution(strategy_id, strategy_definition)
            .await?;

        if execution.status() != StrategyExecutionStatus::Running {
            return Ok(());
        }

        let time_to = match strategy_definition.time_to() {
            Some(ts) => ts,
            None => Utc::now(),
        };

        let data_requirements = strategy.data_requirements();
        let packed_candles = read_market_data::read_market_data(
            self.mongo.as_ref(),
            data_requirements,
            execution.last_execution_timestamp(),
            time_to,
            strategy_definition.resolution(),
        )
        .await?;

        let mut states: Vec<(DateTime<Utc>, StrategyState)> = Default::default();

        // TODO: execute strategies in chunks
        for (ts, candles) in packed_candles {
            let state = strategy.execute(ts, candles, last_state);

            match state {
                Ok(state) => {
                    states.push((ts, state.clone()));
                    last_state = state;
                    execution.set_last_execution_timestamp(ts);
                }
                Err(err) => {
                    if let StrategyExecutionError::CriticalFailure = err {
                        execution.set_status(StrategyExecutionStatus::Failed);
                    }

                    println!("Strategy execution failed: {}", err);
                    break;
                }
            }
        }

        self.mongo
            .write_strategy_execution(strategy_id, &execution)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to update execution status for strategy {}: {}",
                    strategy_id,
                    err.to_string()
                )
            })?;

        self.mongo
            .write_strategy_state(strategy_id, states)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to write strategy states for strategy {}: {}",
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
                println!("Failed to execute strategy: {}", err);
            }
        }

        Ok(prev_state)
    }
}

pub type StrategyRunner = PeriodicComponent<StrategyRunnerPeriodic>;
