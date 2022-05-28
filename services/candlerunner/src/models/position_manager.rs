use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::account::AccountId;
use crate::models::instance_id::InstanceId;
use crate::models::instruments::Figi;
use crate::models::params::{ParamDefinition, ParamError, ParamValue};
use crate::models::positions::AccountPositions;
use crate::models::strategy::StrategyExecutionContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionManagerDefinition {
    position_manager_name: String,
    params: Vec<ParamDefinition>,
}

impl PositionManagerDefinition {
    pub fn new(position_manager_name: String, params: Vec<ParamDefinition>) -> Self {
        Self {
            position_manager_name,
            params,
        }
    }

    pub fn params(&self) -> &[ParamDefinition] {
        self.params.as_ref()
    }

    pub fn position_manager_name(&self) -> &str {
        self.position_manager_name.as_ref()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PositionManagerInstanceOptions {
    Realtime { account_id: AccountId },
    Backtest,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PositionManagerInstanceDefinition {
    position_manager_name: String,
    strategies: Vec<Uuid>,
    options: PositionManagerInstanceOptions,
    params: HashMap<String, ParamValue>,
}

static mut POSITION_MANAGER_INSTANCE_NS: Option<Uuid> = None;

fn get_position_manager_instance_ns() -> &'static Uuid {
    unsafe {
        POSITION_MANAGER_INSTANCE_NS.get_or_insert_with(|| {
            Uuid::new_v5(&Uuid::NAMESPACE_OID, b"Position Manager Instance Id")
        })
    }
}

// TODO: extract params id logic for strategy/position manager instances
impl InstanceId for PositionManagerInstanceDefinition {
    fn id(&self) -> Uuid {
        let mut bytes: Vec<u8> = Default::default();
        bytes.extend_from_slice(self.position_manager_name.as_bytes());
        bytes.push(0);

        let mut sorted_params: Vec<(String, ParamValue)> = self
            .params
            .iter()
            .map(|(param_name, param_value)| (param_name.clone(), param_value.clone()))
            .collect();

        sorted_params.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

        for (param_name, param_value) in sorted_params {
            bytes.extend_from_slice(param_name.as_bytes());
            bytes.push(0);

            match param_value {
                ParamValue::Instrument(figi) => {
                    bytes.extend_from_slice(b"Figi:");
                    bytes.extend_from_slice(figi.0.as_bytes());
                    bytes.push(0);
                }

                ParamValue::Integer(i) => {
                    bytes.extend_from_slice(b"Integer:");
                    bytes.extend_from_slice(&i.to_le_bytes());
                    bytes.push(0);
                }

                ParamValue::Float(f) => {
                    bytes.extend_from_slice(b"Float:");
                    bytes.extend_from_slice(&f.to_le_bytes());
                    bytes.push(0);
                }

                ParamValue::Boolean(b) => {
                    bytes.extend_from_slice(b"Boolean:");
                    let val = match b {
                        true => 1u8,
                        false => 0u8,
                    };
                    bytes.extend_from_slice(&[val, 0u8]);
                }
            }
        }

        let mut sorted_strategies = self.strategies.clone();
        sorted_strategies.sort();

        bytes.extend_from_slice(b"Strategies:");
        for strategy in sorted_strategies {
            bytes.extend_from_slice(strategy.as_bytes());
            bytes.push(0);
        }
        bytes.push(0);

        Uuid::new_v5(get_position_manager_instance_ns(), &bytes)
    }
}

impl PositionManagerInstanceDefinition {
    pub fn position_manager_name(&self) -> &str {
        self.position_manager_name.as_ref()
    }

    pub fn params(&self) -> &HashMap<String, ParamValue> {
        &self.params
    }

    pub fn strategies(&self) -> &[Uuid] {
        self.strategies.as_ref()
    }

    pub fn options(&self) -> &PositionManagerInstanceOptions {
        &self.options
    }
}

pub enum OrderDirection {
    Buy,
    Sell,
}

pub enum OrderType {
    Limit,
    Market,
}

pub struct Order {
    direction: OrderDirection,
    order_type: OrderType,
    lots: u32,
    instrument: Figi,
}

pub trait PositionManager: Sync + Send + 'static {
    fn execute(
        &self,
        ts: DateTime<Utc>,
        strategies: HashMap<uuid::Uuid, StrategyExecutionContext>,
        positions: &AccountPositions,
    ) -> Vec<Order>;
}

#[derive(Error, Debug)]
pub enum InstantiatePositionManagerError {
    #[error("Position manager `{0}` is not found")]
    NotFound(String),
    #[error("Failed to instantiate position manager: {0}")]
    FailedToInstantiate(String),
    #[error("Params validation failed")]
    ParamValidationFailed {
        #[from]
        source: ParamError,
    },
    #[error("Strategy {0} is unknown")]
    UnknownStrategy(Uuid),
}

pub trait PositionManagerFactory: Sync + Send + 'static {
    fn position_manager_name(&self) -> &'_ str;
    fn definition(&self) -> &'_ PositionManagerDefinition;
    fn create(
        &self,
        params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn PositionManager>, InstantiatePositionManagerError>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PositionManagerExecutionState {
    cursor: DateTime<Utc>,
}

impl PositionManagerExecutionState {
    pub fn new(cursor: DateTime<Utc>) -> Self {
        Self { cursor }
    }

    pub fn cursor(&self) -> DateTime<Utc> {
        self.cursor
    }
}
