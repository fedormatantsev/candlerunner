use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use component_store::prelude::*;

use crate::models::strategy::{
    CreateStrategyError, ParamValue, Strategy, StrategyDefinition, StrategyFactory,
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

    pub fn create_strategy(
        &self,
        name: &String,
        params: HashMap<String, ParamValue>,
    ) -> Result<Box<dyn Strategy>, CreateStrategyError> {
        let factory = self
            .factories
            .get(name)
            .ok_or_else(|| CreateStrategyError::StrategyNotFound(name.to_owned()))?;
            
        factory.create(params)
    }

    async fn new(_: ComponentResolver, _: Box<dyn ConfigProvider>) -> Result<Self, ComponentError> {
        Ok(Self {
            factories: Default::default(),
        })
    }
}

impl CreateComponent for StrategyRegistry {
    fn create(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<std::sync::Arc<Self>, ComponentError>> {
        Box::pin(async move { Ok(Arc::new(Self::new(resolver, config).await?)) })
    }
}

impl DestroyComponent for StrategyRegistry {}

impl ComponentName for StrategyRegistry {
    fn component_name() -> &'static str {
        "strategy-registry"
    }
}

impl Component for StrategyRegistry {}
