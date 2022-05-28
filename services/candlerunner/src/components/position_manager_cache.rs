use std::{collections::HashMap, sync::Arc};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};
use uuid::Uuid;

use crate::{
    components,
    models::{
        instance_id::InstanceId,
        position_manager::{PositionManager, PositionManagerInstanceDefinition},
    },
};

pub struct PositionManagerCachePeriodic {
    mongo: Arc<components::Mongo>,
    registry: Arc<components::PositionManagerRegistry>,
}

impl ComponentName for PositionManagerCachePeriodic {
    fn component_name() -> &'static str {
        "position-manager-cache"
    }
}

impl Periodic for PositionManagerCachePeriodic {
    type State = HashMap<Uuid, (PositionManagerInstanceDefinition, Arc<dyn PositionManager>)>;

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

impl PositionManagerCachePeriodic {
    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<(Self, <Self as Periodic>::State), ComponentError> {
        let mongo = resolver.resolve::<components::Mongo>().await?;
        let registry = resolver
            .resolve::<components::PositionManagerRegistry>()
            .await?;

        Ok((
            Self { mongo, registry },
            <Self as Periodic>::State::default(),
        ))
    }

    async fn step(
        &mut self,
        prev_state: Arc<<Self as Periodic>::State>,
    ) -> anyhow::Result<Arc<<Self as Periodic>::State>> {
        let defs = self.mongo.read_position_manager_instances().await?;

        let mut inserted = 0;
        let mut failed = 0;

        let mut instantiate =
            |id: Uuid,
             def: PositionManagerInstanceDefinition,
             state: &mut <Self as Periodic>::State| {
                match self.registry.instantiate_position_manager(def.clone()) {
                    Ok(instance) => {
                        state.insert(id, (def, instance));
                        inserted += 1;
                    }
                    Err(err) => {
                        println!("Failed to instantiate position manager: {}", err);
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
            "Updated position manager cache: {} inserted, {} removed, {} failed, {} total",
            inserted,
            removed,
            failed,
            new_state.len()
        );

        Ok(Arc::new(new_state))
    }
}

pub type PositionManagerCache = PeriodicComponent<PositionManagerCachePeriodic>;
