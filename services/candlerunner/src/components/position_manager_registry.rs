use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use component_store::prelude::*;

use crate::components;
use crate::models::position_manager::{
    InstantiatePositionManagerError, PositionManager, PositionManagerDefinition, PositionManagerFactory,
    PositionManagerInstanceDefinition,
};
use crate::position_managers;

pub struct Definitions<'registry> {
    inner: hash_map::Values<'registry, String, Box<dyn PositionManagerFactory>>,
}

impl<'registry> Iterator for Definitions<'registry> {
    type Item = &'registry PositionManagerDefinition;

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
    factories: HashMap<String, Box<dyn PositionManagerFactory>>,
}

impl Builder {
    pub fn register<T: PositionManagerFactory>(mut self, factory: T) -> Self {
        let name = factory.position_manager_name().to_string();

        self.factories.insert(name, Box::new(factory));
        self
    }

    pub fn build(self) -> HashMap<String, Box<dyn PositionManagerFactory>> {
        self.factories
    }
}

pub struct PositionManagerRegistry {
    factories: HashMap<String, Box<dyn PositionManagerFactory>>,
    param_validator: Arc<components::ParamValidator>,
    strategy_cache: Arc<components::StrategyCache>,
}

impl PositionManagerRegistry {
    pub fn definitions(&self) -> Definitions<'_> {
        Definitions {
            inner: self.factories.values(),
        }
    }

    pub fn validate_instance_definition(
        &self,
        instance_definition: &PositionManagerInstanceDefinition,
    ) -> Result<(), InstantiatePositionManagerError> {
        let factory = self
            .factories
            .get(instance_definition.position_manager_name())
            .ok_or_else(|| {
                InstantiatePositionManagerError::NotFound(
                    instance_definition.position_manager_name().to_owned(),
                )
            })?;

        let strategies = self.strategy_cache.state();

        for strategy_id in instance_definition.strategies() {
            if !strategies.contains_key(strategy_id) {
                return Err(InstantiatePositionManagerError::UnknownStrategy(
                    strategy_id.to_owned(),
                ));
            }
        }

        self.param_validator
            .validate(factory.definition().params(), instance_definition.params())?;

        Ok(())
    }

    pub fn instantiate_position_manager(
        &self,
        instance_definition: PositionManagerInstanceDefinition,
    ) -> Result<Arc<dyn PositionManager>, InstantiatePositionManagerError> {
        let factory = self
            .factories
            .get(instance_definition.position_manager_name())
            .ok_or_else(|| {
                InstantiatePositionManagerError::NotFound(
                    instance_definition.position_manager_name().to_owned(),
                )
            })?;

        factory.create(instance_definition.params())
    }

    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<Self, ComponentError> {
        Ok(Self {
            factories: Builder::default()
                .register(position_managers::QuorumManagerFactory::default())
                .build(),
            param_validator: resolver.resolve::<components::ParamValidator>().await?,
            strategy_cache: resolver.resolve::<components::StrategyCache>().await?,
        })
    }
}

impl InitComponent for PositionManagerRegistry {
    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Self, ComponentError>> {
        Box::pin(Self::new(resolver, config))
    }
}

impl ShutdownComponent for PositionManagerRegistry {}

impl ComponentName for PositionManagerRegistry {
    fn component_name() -> &'static str {
        "position-manager-registry"
    }
}

impl Component for PositionManagerRegistry {}
