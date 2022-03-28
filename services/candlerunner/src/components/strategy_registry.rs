use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use component_store::prelude::*;

use crate::models::strategy::{
    CreateStrategyError, Strategy, StrategyDefinition, StrategyFactory, StrategyInstanceDefinition,
};
use crate::strategies;

pub struct Definitions<'registry> {
    inner: hash_map::Values<'registry, String, Box<dyn StrategyFactory>>,
}

impl<'registry> Iterator for Definitions<'registry> {
    type Item = &'registry StrategyDefinition;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|factory| factory.definition())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.inner.count()
    }
}

#[derive(Default)]
struct Builder {
    factories: HashMap<String, Box<dyn StrategyFactory>>,
}

impl Builder {
    pub fn register<T: StrategyFactory>(mut self, factory: T) -> Self {
        let name = factory.strategy_name().to_string();

        self.factories.insert(name, Box::new(factory));
        self
    }

    pub fn build(self) -> HashMap<String, Box<dyn StrategyFactory>> {
        self.factories
    }
}

pub struct StrategyRegistry {
    factories: HashMap<String, Box<dyn StrategyFactory>>,
}

impl StrategyRegistry {
    pub fn definitions(&self) -> Definitions<'_> {
        Definitions {
            inner: self.factories.values(),
        }
    }

    pub fn instantiate_strategy(
        &self,
        def: StrategyInstanceDefinition,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError> {
        let factory = self.factories.get(def.strategy_name()).ok_or_else(|| {
            CreateStrategyError::StrategyNotFound(def.strategy_name().to_string())
        })?;

        factory.create(def.params())
    }

    async fn new(_: ComponentResolver, _: Box<dyn ConfigProvider>) -> Result<Self, ComponentError> {
        Ok(Self {
            factories: Builder::default()
                .register(strategies::BuyAndHoldFactory::default())
                .build(),
        })
    }
}

impl InitComponent for StrategyRegistry {
    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Self, ComponentError>> {
        Box::pin(Self::new(resolver, config))
    }
}

impl ShutdownComponent for StrategyRegistry {}

impl ComponentName for StrategyRegistry {
    fn component_name() -> &'static str {
        "strategy-registry"
    }
}

impl Component for StrategyRegistry {}
