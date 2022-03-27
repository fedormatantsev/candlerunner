use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use component_store::prelude::*;

use crate::models::strategy::{
    CreateStrategyError, Strategy, StrategyDefinition, StrategyFactory, StrategyInstanceDefinition,
};

pub struct StrategyRegistry {
    factories: HashMap<String, Box<dyn StrategyFactory>>,
}

pub struct Definitions<'registry> {
    inner: hash_map::Values<'registry, String, Box<dyn StrategyFactory>>,
}

impl<'registry> Iterator for Definitions<'registry> {
    type Item = &'registry dyn StrategyDefinition;

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
        let factory = self
            .factories
            .get(&def.strategy_name)
            .ok_or_else(|| CreateStrategyError::StrategyNotFound(def.strategy_name))?;

        factory.create(def.params)
    }

    async fn new(_: ComponentResolver, _: Box<dyn ConfigProvider>) -> Result<Self, ComponentError> {
        Ok(Self {
            factories: Default::default(),
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
