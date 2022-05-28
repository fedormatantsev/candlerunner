use std::{collections::HashMap, sync::Arc};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};
use uuid::Uuid;

use crate::{
    components,
    models::{strategy::{Strategy, StrategyInstanceDefinition}, instance_id::InstanceId},
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
    type State = HashMap<Uuid, (StrategyInstanceDefinition, Arc<dyn Strategy>)>;

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

    async fn step(
        &mut self,
        prev_state: Arc<<Self as Periodic>::State>,
    ) -> anyhow::Result<Arc<<Self as Periodic>::State>> {
        let defs = self.mongo.read_strategy_instances().await?;

        let mut inserted = 0;
        let mut failed = 0;

        let mut instantiate =
            |id: Uuid, def: StrategyInstanceDefinition, state: &mut <Self as Periodic>::State| {
                match self.registry.instantiate_strategy(def.clone()) {
                    Ok(instance) => {
                        state.insert(id, (def, instance));
                        inserted += 1;
                    }
                    Err(err) => {
                        println!("Failed to instantiate strategy: {}", err);
                        failed += 1;
                    }
                };
            };

        let new_state =
            defs.into_iter()
                .fold(<Self as Periodic>::State::default(), |mut state, def| {
                    let id = def.id();

                    match prev_state.get(&id) {
                        Some(item) => {
                            state.insert(id, item.clone());
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
    }
}

pub type StrategyCache = PeriodicComponent<StrategyCachePeriodic>;
