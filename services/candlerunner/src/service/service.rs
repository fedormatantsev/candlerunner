use std::sync::Arc;

use component_store::{ComponentName, ComponentStore};
use thiserror::Error;
use tonic::{Request, Response, Status};

use crate::components;
use crate::generated::candlerunner_api::{
    self,
    candlerunner_service_server::{CandlerunnerService, CandlerunnerServiceServer},
};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Failed to resolve component {0}")]
    FailedToResolveComponent(String),
}

pub struct Service {
    instruments_cache: Arc<components::InstrumentCache>,
    strategy_registry: Arc<components::StrategyRegistry>,
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

        Ok(CandlerunnerServiceServer::new(Self {
            instruments_cache,
            strategy_registry,
        }))
    }
}

#[tonic::async_trait]
impl CandlerunnerService for Service {
    async fn instruments(
        &self,
        request: Request<candlerunner_api::InstrumentsRequest>,
    ) -> Result<Response<candlerunner_api::InstrumentsResponse>, Status> {
        let instruments = self.instruments_cache.state();

        let instruments: Vec<candlerunner_api::Instrument> = instruments
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
        request: Request<candlerunner_api::StrategyDefinitionsRequest>,
    ) -> Result<Response<candlerunner_api::StrategyDefinitionsResponse>, Status> {
        todo!()
    }

    async fn instantiate_strategy(
        &self,
        request: Request<candlerunner_api::InstantiateStrategyRequest>,
    ) -> Result<Response<candlerunner_api::InstantiateStrategyResponse>, Status> {
        todo!()
    }
}
