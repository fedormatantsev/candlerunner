use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use component_store::prelude::*;

use crate::components;
use crate::models::strategy::{
    CreateStrategyError, ParamType, ParamValue, Strategy, StrategyDefinition, StrategyFactory,
    StrategyInstanceDefinition,
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
    instrument_cache: Arc<components::InstrumentCache>,
}

impl StrategyRegistry {
    pub fn definitions(&self) -> Definitions<'_> {
        Definitions {
            inner: self.factories.values(),
        }
    }

    pub fn validate_instance_definition(
        &self,
        instance_definition: &StrategyInstanceDefinition,
    ) -> Result<(), CreateStrategyError> {
        let factory = self
            .factories
            .get(instance_definition.strategy_name())
            .ok_or_else(|| {
                CreateStrategyError::StrategyNotFound(
                    instance_definition.strategy_name().to_string(),
                )
            })?;

        let instruments = self.instrument_cache.state();

        for (param_name, _) in instance_definition.params() {
            if factory
                .definition()
                .params()
                .iter()
                .find(|expected_param| (*expected_param).name() == param_name)
                .is_none()
            {
                return Err(CreateStrategyError::InvalidParam(param_name.to_string()));
            }
        }

        for expected_param in factory.definition().params() {
            let actual_value = instance_definition
                .params()
                .get(expected_param.name())
                .ok_or_else(|| {
                    CreateStrategyError::ParamMissing(expected_param.name().to_string())
                })?;

            let actual_type = ParamType::from(actual_value);

            if *expected_param.param_type() != actual_type {
                return Err(CreateStrategyError::ParamTypeMismatch(
                    expected_param.name().to_string(),
                ));
            }

            if let ParamValue::Instrument(ref figi) = actual_value {
                if !instruments.contains_key(figi) {
                    return Err(CreateStrategyError::FailedToInstantiateStrategy(format!(
                        "Instrument `{}` not found in cache",
                        figi.0
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn instantiate_strategy(
        &self,
        instance_definition: StrategyInstanceDefinition,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError> {
        let factory = self
            .factories
            .get(instance_definition.strategy_name())
            .ok_or_else(|| {
                CreateStrategyError::StrategyNotFound(
                    instance_definition.strategy_name().to_string(),
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
                .register(strategies::BuyAndHoldFactory::default())
                .build(),
            instrument_cache: resolver.resolve::<components::InstrumentCache>().await?,
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
