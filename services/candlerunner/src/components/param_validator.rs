use std::collections::HashMap;
use std::sync::Arc;

use component_store::prelude::*;

use crate::components;
use crate::models::params::{ParamDefinition, ParamError, ParamType, ParamValue};

pub struct ParamValidator {
    instrument_cache: Arc<components::InstrumentCache>,
}

impl ParamValidator {
    pub fn validate(
        &self,
        param_definitions: &[ParamDefinition],
        params: &HashMap<String, ParamValue>,
    ) -> Result<(), ParamError> {
        let instruments = self.instrument_cache.state();

        for param_name in params.keys() {
            if param_definitions
                .iter()
                .any(|expected_param| (*expected_param).name() == param_name)
            {
                return Err(ParamError::InvalidParam(param_name.to_owned()));
            }
        }

        for expected_param in param_definitions {
            let actual_value = params
                .get(expected_param.name())
                .ok_or_else(|| ParamError::ParamMissing(expected_param.name().to_owned()))?;

            let actual_type = ParamType::from(actual_value);

            if *expected_param.param_type() != actual_type {
                return Err(ParamError::ParamTypeMismatch(
                    expected_param.name().to_string(),
                ));
            }

            if let ParamValue::Instrument(ref figi) = actual_value {
                if !instruments.contains_key(figi) {
                    return Err(ParamError::UnknownInstrument(
                        expected_param.name().to_owned(),
                        figi.0.to_owned(),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn new(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> Result<Self, ComponentError> {
        Ok(Self {
            instrument_cache: resolver.resolve::<components::InstrumentCache>().await?,
        })
    }
}

impl InitComponent for ParamValidator {
    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Self, ComponentError>> {
        Box::pin(Self::new(resolver, config))
    }
}

impl ShutdownComponent for ParamValidator {}

impl ComponentName for ParamValidator {
    fn component_name() -> &'static str {
        "param-validator"
    }
}

impl Component for ParamValidator {}
