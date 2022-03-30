use std::collections::HashMap;
use std::sync::Arc;

use component_store::{ComponentName, ComponentStore};
use thiserror::Error;
use tonic::{Request, Response, Status};

use crate::components;
use crate::generated::candlerunner_api::{
    self,
    candlerunner_service_server::{CandlerunnerService, CandlerunnerServiceServer},
};

use crate::models::instruments::Figi;
use crate::models::strategy::{ParamType, ParamValue, StrategyInstanceDefinition};

impl TryFrom<candlerunner_api::StrategyInstanceDefinition> for StrategyInstanceDefinition {
    type Error = Status;

    fn try_from(proto: candlerunner_api::StrategyInstanceDefinition) -> Result<Self, Self::Error> {
        let mut params: HashMap<String, ParamValue> = Default::default();

        for proto_param in proto.params {
            if proto_param.param_name.is_empty() {
                return Err(Status::invalid_argument("Param name is not specified"));
            }

            let proto_value = proto_param
                .param_value
                .and_then(|v| v.value)
                .ok_or_else(|| {
                    Status::invalid_argument(format!(
                        "Parameter `{}` missing value field",
                        &proto_param.param_name
                    ))
                })?;

            let value = match proto_value {
                candlerunner_api::param_value::Value::InstrumentVal(figi) => {
                    ParamValue::Instrument(Figi(figi))
                }
                candlerunner_api::param_value::Value::IntegerVal(i) => ParamValue::Integer(i),
                candlerunner_api::param_value::Value::FloatVal(f) => ParamValue::Float(f),
                candlerunner_api::param_value::Value::BooleanVal(b) => ParamValue::Boolean(b),
            };

            params.insert(proto_param.param_name, value);
        }

        Ok(StrategyInstanceDefinition::new(proto.strategy_name, params))
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Failed to resolve component {0}")]
    FailedToResolveComponent(String),
}

pub struct Service {
    instruments_cache: Arc<components::InstrumentCache>,
    strategy_registry: Arc<components::StrategyRegistry>,
    strategy_cache: Arc<components::StrategyCache>,
    mongo: Arc<components::Mongo>,
}

impl Service {
    pub fn new(
        component_store: &ComponentStore,
    ) -> Result<CandlerunnerServiceServer<Self>, ServiceError> {
        let instruments_cache = component_store
            .resolve::<components::InstrumentCache>()
            .ok_or_else(|| {
                ServiceError::FailedToResolveComponent(
                    components::InstrumentCache::component_name().to_string(),
                )
            })?;

        let strategy_registry = component_store
            .resolve::<components::StrategyRegistry>()
            .ok_or_else(|| {
                ServiceError::FailedToResolveComponent(
                    components::StrategyRegistry::component_name().to_string(),
                )
            })?;

        let strategy_cache = component_store
            .resolve::<components::StrategyCache>()
            .ok_or_else(|| {
                ServiceError::FailedToResolveComponent(
                    components::StrategyCache::component_name().to_string(),
                )
            })?;

        let mongo = component_store
            .resolve::<components::Mongo>()
            .ok_or_else(|| {
                ServiceError::FailedToResolveComponent(
                    components::Mongo::component_name().to_string(),
                )
            })?;

        Ok(CandlerunnerServiceServer::new(Self {
            instruments_cache,
            strategy_registry,
            strategy_cache,
            mongo,
        }))
    }
}

#[tonic::async_trait]
impl CandlerunnerService for Service {
    async fn instruments(
        &self,
        _: Request<candlerunner_api::InstrumentsRequest>,
    ) -> Result<Response<candlerunner_api::InstrumentsResponse>, Status> {
        let instruments: Vec<_> = self
            .instruments_cache
            .state()
            .values()
            .cloned()
            .map(|i| candlerunner_api::Instrument {
                figi: i.figi.0,
                ticker: i.ticker.0,
                display_name: i.display_name,
            })
            .collect();

        Ok(Response::new(candlerunner_api::InstrumentsResponse {
            instruments,
        }))
    }

    async fn strategy_definitions(
        &self,
        _: Request<candlerunner_api::StrategyDefinitionsRequest>,
    ) -> Result<Response<candlerunner_api::StrategyDefinitionsResponse>, Status> {
        fn to_param_type(t: &ParamType) -> candlerunner_api::ParamType {
            match t {
                ParamType::Instrument => candlerunner_api::ParamType::Instrument,
                ParamType::Integer => candlerunner_api::ParamType::Integer,
                ParamType::Float => candlerunner_api::ParamType::Float,
                ParamType::Boolean => candlerunner_api::ParamType::Boolean,
            }
        }

        let definitions: Vec<_> = self
            .strategy_registry
            .definitions()
            .map(|def| {
                let params: Vec<_> = def
                    .params()
                    .iter()
                    .map(|p| candlerunner_api::ParamDefinition {
                        param_name: p.name().to_string(),
                        description: p.description().to_string(),
                        param_type: to_param_type(p.param_type()) as i32,
                        default_value: None,
                    })
                    .collect();

                candlerunner_api::StrategyDefinition {
                    strategy_name: def.strategy_name().to_string(),
                    description: def.strategy_description().to_string(),
                    params,
                }
            })
            .collect();

        Ok(Response::new(
            candlerunner_api::StrategyDefinitionsResponse { definitions },
        ))
    }

    async fn instantiate_strategy(
        &self,
        request: Request<candlerunner_api::InstantiateStrategyRequest>,
    ) -> Result<Response<candlerunner_api::InstantiateStrategyResponse>, Status> {
        let proto_definition = request
            .into_inner()
            .instance_definition
            .ok_or_else(|| Status::invalid_argument("Field `instance_definition` is missing"))?;

        let instance_def = StrategyInstanceDefinition::try_from(proto_definition)?;
        self.strategy_registry
            .validate_instance_definition(&instance_def)
            .map_err(|err| Status::internal(err.to_string()))?;

        self.mongo
            .write_strategy_instance(&instance_def)
            .await
            .map_err(|err| {
                Status::internal(format!("Failed to write strategy instance to db: {}", err))
            })?;

        self.strategy_cache
            .force_update(Some(std::time::Duration::from_millis(100)))
            .await;

        let id = instance_def.id();
        if !self.strategy_cache.state().contains_key(&id) {
            return Err(Status::internal("Failed to instantiate strategy"));
        }

        Ok(Response::new(
            candlerunner_api::InstantiateStrategyResponse {
                instance_id: id.to_string(),
            },
        ))
    }
}
