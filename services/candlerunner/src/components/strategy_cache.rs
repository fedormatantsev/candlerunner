use std::{collections::HashMap, sync::Arc};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};
use uuid::Uuid;

use crate::{
    components,
    models::strategy::{Strategy, StrategyInstanceDefinition},
};

pub struct StrategyCachePeriodic {
    mongo: Arc<components::Mongo>,
    registry: Arc<components::StrategyRegistry>,
}

impl ComponentName for StrategyCachePeriodic {
    fn component_name() -> &'static str {
        "strategy-cache"
    }
}

impl Periodic for StrategyCachePeriodic {
    type State = HashMap<Uuid, Arc<dyn Strategy>>;

    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(Self::new(resolver, config))
    }

    fn step(&mut self, prev_state: Arc<Self::State>) -> PeriodicFuture<Self::State> {
        let mongo = self.mongo.clone();
        let registry = self.registry.clone();

        Box::pin(async move {
            let defs = mongo.read_strategy_instance_defs().await?;

            let mut inserted = 0;
            let mut failed = 0;

            let mut instantiate =
                |id: Uuid, def: StrategyInstanceDefinition, state: &mut Self::State| {
                    match registry.instantiate_strategy(def) {
                        Ok(instance) => {
                            state.insert(id, instance);
                            inserted += 1;
                        }
                        Err(err) => {
                            println!("Failed to instantiate strategy: {}", err);
                            failed += 1;
                        }
                    };
                };

            let new_state = defs
                .into_iter()
                .fold(Self::State::default(), |mut state, def| {
                    let id = def.id();

                    match prev_state.get(&id) {
                        Some(instance) => {
                            state.insert(id, instance.clone());
                            return state;
                        }
                        None => {
                            instantiate(id, def, &mut state);
                            return state;
                        }
                    }
                });

            let removed = prev_state.len() - (new_state.len() - inserted);

            println!(
                "Updated strategy cache: {} inserted, {} removed, {} failed, {} total",
                inserted,
                removed,
                failed,
                new_state.len()
            );

            Ok(Arc::new(new_state))
        })
    }
}

impl StrategyCachePeriodic {
    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<(Self, <Self as Periodic>::State), ComponentError> {
        let mongo = resolver.resolve::<components::Mongo>().await?;
        let registry = resolver.resolve::<components::StrategyRegistry>().await?;

        Ok((
            Self { mongo, registry },
            <Self as Periodic>::State::default(),
        ))
    }
}

pub type StrategyCache = PeriodicComponent<StrategyCachePeriodic>;
