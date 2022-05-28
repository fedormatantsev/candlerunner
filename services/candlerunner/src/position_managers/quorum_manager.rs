use std::{collections::HashMap, sync::Arc};

use chrono::prelude::*;
use uuid::Uuid;

use crate::models::{
    params::{ParamDefinition, ParamType, ParamValue},
    position_manager::{
        InstantiatePositionManagerError, Order, PositionManager, PositionManagerDefinition,
        PositionManagerFactory,
    },
    positions::AccountPositions,
    strategy::StrategyExecutionContext,
};

const PARAM_NAME_BUY_THRESHOLD: &str = "Buy Threshold";
const PARAM_NAME_SELL_THRESHOLD: &str = "Sell Threshold";

pub struct QuorumManager {}

impl PositionManager for QuorumManager {
    fn execute(
        &self,
        _ts: DateTime<chrono::Utc>,
        _strategies: HashMap<Uuid, StrategyExecutionContext>,
        _positions: &AccountPositions,
    ) -> Vec<Order> {
        todo!()
    }
}

pub struct QuorumManagerFactory {
    definition: PositionManagerDefinition,
}

impl Default for QuorumManagerFactory {
    fn default() -> Self {
        Self {
            definition: PositionManagerDefinition::new(
                "QuorumManager".to_owned(),
                vec![
                    ParamDefinition::new(
                        PARAM_NAME_BUY_THRESHOLD,
                        "Cumulative signal level to buy instrument",
                        ParamType::Float,
                        None,
                    ),
                    ParamDefinition::new(
                        PARAM_NAME_SELL_THRESHOLD,
                        "Cumulative signal level to sell instrument",
                        ParamType::Float,
                        None,
                    ),
                ],
            ),
        }
    }
}

impl PositionManagerFactory for QuorumManagerFactory {
    fn position_manager_name(&self) -> &str {
        "QuorumManager"
    }

    fn definition(&self) -> &PositionManagerDefinition {
        &self.definition
    }

    fn create(
        &self,
        _params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn PositionManager>, InstantiatePositionManagerError> {
        Ok(Arc::new(QuorumManager {}))
    }
}
