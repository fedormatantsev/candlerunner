use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use chrono::{prelude::*, Duration};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};
use uuid::Uuid;

use crate::{
    components,
    models::{
        position_manager::{PositionManagerExecutionState, PositionManagerInstanceOptions},
        strategy::StrategyExecutionContext,
    },
};

pub struct PositionManagerRunnerPeriodic {
    position_manager_cache: Arc<components::PositionManagerCache>,
    positions_cache: Arc<components::PositionsCache>,
    mongo: Arc<components::Mongo>,
    max_execution_context_age: Duration,
}

impl ComponentName for PositionManagerRunnerPeriodic {
    fn component_name() -> &'static str {
        "position-manager-runner"
    }
}

impl Periodic for PositionManagerRunnerPeriodic {
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

impl PositionManagerRunnerPeriodic {
    async fn new(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> Result<(Self, <Self as Periodic>::State), ComponentError> {
        let position_manager_cache = resolver
            .resolve::<components::PositionManagerCache>()
            .await?;
        let positions_cache = resolver.resolve::<components::PositionsCache>().await?;
        let mongo = resolver.resolve::<components::Mongo>().await?;
        let max_execution_context_age = config
            .get_i64("max_execution_context_age")
            .map(Duration::seconds)?;

        Ok((
            Self {
                position_manager_cache,
                positions_cache,
                mongo,
                max_execution_context_age,
            },
            <Self as Periodic>::State::default(),
        ))
    }

    async fn step(
        &mut self,
        prev_state: Arc<<Self as Periodic>::State>,
    ) -> anyhow::Result<Arc<<Self as Periodic>::State>> {
        let position_manager_cache = self.position_manager_cache.state();

        for (position_manager_id, (position_manager_instance_def, position_manager)) in
            position_manager_cache.iter()
        {
            let account_id = match position_manager_instance_def.options() {
                PositionManagerInstanceOptions::Realtime { account_id } => account_id.to_owned(),
                PositionManagerInstanceOptions::Backtest => continue,
            };

            let execution_state = self
                .mongo
                .read_position_manager_execution_state(position_manager_id)
                .await;

            let execution_state = match execution_state {
                Ok(state) => state,
                Err(err) => {
                    println!(
                        "Failed to read position manager execution state: {}",
                        err.to_string()
                    );
                    continue;
                }
            };

            let execution_state = match execution_state {
                Some(state) => state,
                None => {
                    let time_from = Utc::now() - self.max_execution_context_age;
                    let state = PositionManagerExecutionState::new(time_from);
                    if let Err(err) = self
                        .mongo
                        .write_position_manager_execution_state(position_manager_id, &state)
                        .await
                    {
                        println!(
                            "Failed to write position manager execution state: {}",
                            err.to_string()
                        );
                        continue;
                    }

                    state
                }
            };

            let strategy_ids = position_manager_instance_def.strategies();
            let mut time_to = None;

            for strategy_id in strategy_ids {
                let strategy_execution_state = self
                    .mongo
                    .read_strategy_execution_state(strategy_id)
                    .await?;

                match strategy_execution_state {
                    Some(state) => {
                        time_to.replace(time_to.unwrap_or(state.cursor()).min(state.cursor()));
                    }
                    None => {
                        time_to = None;
                        break;
                    }
                }
            }

            let time_to = match time_to {
                Some(val) => val,
                None => continue,
            };

            let mut all_execution_contexts: BTreeMap<
                DateTime<Utc>,
                HashMap<Uuid, StrategyExecutionContext>,
            > = Default::default();

            let time_from = execution_state
                .cursor()
                .min(Utc::now() - self.max_execution_context_age);

            for strategy_id in strategy_ids {
                let execution_contexts = self
                    .mongo
                    .read_strategy_execution_contexts(strategy_id, time_from, Some(time_to))
                    .await?;

                execution_contexts.into_iter().for_each(|(ts, ctx)| {
                    let entry = all_execution_contexts.entry(ts).or_default();
                    entry.insert(strategy_id.clone(), ctx);
                });
            }

            for (ts, contexts) in all_execution_contexts {
                let positions_cache = self.positions_cache.state();
                let positions = positions_cache.get(&account_id);

                if let Some(positions) = positions {
                    let _orders = position_manager.execute(ts, contexts, positions);
                } else {
                    println!("Unable to find positions for account `{}`", account_id.0);
                }
            }

            self.mongo
                .write_position_manager_execution_state(
                    position_manager_id,
                    &PositionManagerExecutionState::new(time_to.max(execution_state.cursor())),
                )
                .await?;
        }

        return Ok(prev_state);
    }
}

pub type PositionManagerRunner = PeriodicComponent<PositionManagerRunnerPeriodic>;
