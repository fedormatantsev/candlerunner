use std::collections::HashMap;
use std::sync::Arc;

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent, PeriodicCreateFuture, PeriodicFuture};

use crate::components;
use crate::models::instruments::{Figi, Instrument};

pub struct InstrumentCachePeriodic {
    mongo: Arc<components::Mongo>,
}

impl ComponentName for InstrumentCachePeriodic {
    fn component_name() -> &'static str {
        "instrument-cache"
    }
}

impl Periodic for InstrumentCachePeriodic {
    type State = HashMap<Figi, Instrument>;

    fn init(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let mongo = resolver.resolve::<components::Mongo>().await?;

            // Ensures that instrument-sync is initialized before cache initialization.
            // Otherwise, we might get empty cache on startup with fresh new db.
            let _ = resolver.resolve::<components::InstrumentSync>().await?;

            let periodic = InstrumentCachePeriodic { mongo };
            let init_state = Self::State::default();

            Ok((periodic, init_state))
        })
    }

    fn step(&mut self, _: Arc<Self::State>) -> PeriodicFuture<Self::State> {
        Box::pin(async move {
            let instruments: HashMap<_, _> = self.mongo
                .read_instruments()
                .await?
                .into_iter()
                .map(|instrument| (instrument.figi.clone(), instrument))
                .collect();

            Ok(Arc::new(instruments))
        })
    }
}

pub type InstrumentCache = PeriodicComponent<InstrumentCachePeriodic>;
