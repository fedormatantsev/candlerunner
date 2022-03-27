use std::sync::Arc;

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::components;

pub struct InstrumentSyncPeriodic {
    tinkoff_client: Arc<components::TinkoffClient>,
    mongo: Arc<components::Mongo>,
}

impl ComponentName for InstrumentSyncPeriodic {
    fn component_name() -> &'static str {
        "instrument-sync"
    }
}

impl Periodic for InstrumentSyncPeriodic {
    type State = ();

    fn create(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> periodic_component::PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;
            let mongo = resolver.resolve::<components::Mongo>().await?;

            let periodic = Self {
                tinkoff_client,
                mongo,
            };

            Ok((periodic, ()))
        })
    }

    fn step(&mut self, state: Arc<Self::State>) -> periodic_component::PeriodicFuture<Self::State> {
        let tinkoff_client = self.tinkoff_client.clone();
        let mongo = self.mongo.clone();

        Box::pin(async move {
            let instruments = tinkoff_client.get_instruments().await?;
            mongo.write_instruments(instruments).await?;

            Ok(state)
        })
    }
}

pub type InstrumentSync = PeriodicComponent<InstrumentSyncPeriodic>;
