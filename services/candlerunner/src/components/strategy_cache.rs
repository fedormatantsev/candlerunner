use std::sync::Arc;

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};

use crate::components;

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
    type State = ();

    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(Self::new(resolver, config))
    }

    fn step(&mut self, state: Arc<Self::State>) -> PeriodicFuture<Self::State> {
        //let mongo = self.mongo;

        Box::pin(async move { Ok(state) })
    }
}

impl StrategyCachePeriodic {
    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<(Self, <Self as Periodic>::State), ComponentError> {
        let mongo = resolver.resolve::<components::Mongo>().await?;
        let registry = resolver.resolve::<components::StrategyRegistry>().await?;

        Ok((Self { mongo, registry }, ()))
    }
}

pub type StrategyCache = PeriodicComponent<StrategyCachePeriodic>;
