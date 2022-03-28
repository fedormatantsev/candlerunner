use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::instruments::Figi;

pub enum ParamType {
    Instrument,
    Integer,
    Float,
    Boolean,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ParamValue {
    Instrument(Figi),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

pub struct ParamDefinition {
    name: String,
    description: String,
    param_type: ParamType,
    default: Option<ParamValue>,
}

impl ParamDefinition {
    pub fn new<N: ToString, D: ToString>(
        name: N,
        description: D,
        param_type: ParamType,
        default: Option<ParamValue>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            param_type,
            default,
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn param_type(&self) -> &ParamType {
        &self.param_type
    }

    pub fn default(&self) -> &Option<ParamValue> {
        &self.default
    }
}

#[derive(Serialize, Deserialize)]
pub struct StrategyInstanceDefinition {
    strategy_name: String,
    params: HashMap<String, ParamValue>,
}

static mut STRATEGY_INSTANCE_NS: Option<Uuid> = None;

fn get_strategy_instance_ns() -> &'static Uuid {
    unsafe {
        STRATEGY_INSTANCE_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"Strategy Instance Id"))
    }
}

impl StrategyInstanceDefinition {
    pub fn new<N: ToString>(strategy_name: N, params: HashMap<String, ParamValue>) -> Self {
        Self {
            strategy_name: strategy_name.to_string(),
            params,
        }
    }

    pub fn id(&self) -> Uuid {
        let mut bytes: Vec<u8> = Default::default();
        bytes.extend_from_slice(self.strategy_name.as_bytes());
        bytes.push(0);

        let mut sorted_params: Vec<(String, ParamValue)> = self
            .params
            .iter()
            .map(|(param_name, param_value)| (param_name.clone(), param_value.clone()))
            .collect();

        sorted_params.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

        for (param_name, param_value) in &self.params {
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

        Uuid::new_v5(get_strategy_instance_ns(), &bytes)
    }

    pub fn strategy_name(&self) -> &str {
        &self.strategy_name
    }

    pub fn params(&self) -> &HashMap<String, ParamValue> {
        &self.params
    }
}

pub struct StrategyDefinition {
    params: Vec<ParamDefinition>,
    strategy_name: String,
    strategy_description: String,
}

impl StrategyDefinition {
    pub fn new<N: ToString, D: ToString>(
        params: Vec<ParamDefinition>,
        strategy_name: N,
        strategy_description: D,
    ) -> Self {
        Self {
            params,
            strategy_name: strategy_name.to_string(),
            strategy_description: strategy_description.to_string(),
        }
    }

    pub fn params(&self) -> &[ParamDefinition] {
        &self.params
    }

    pub fn strategy_name(&self) -> &str {
        &self.strategy_name
    }

    pub fn strategy_description(&self) -> &str {
        &self.strategy_description
    }
}

pub trait Strategy: Send + Sync + 'static {}

#[derive(Error, Debug)]
pub enum CreateStrategyError {
    #[error("Strategy `{0}` is not found")]
    StrategyNotFound(String),
    #[error("Strategy parameter `{0}` is not specified")]
    ParamMissing(String),
    #[error("Strategy parameter `{0}` is of wrong type")]
    ParamTypeMismatch(String),
    #[error("Failed to instantiate strategy")]
    FailedToInstantiateStrategy { source: anyhow::Error },
}

pub trait StrategyFactory: Sync + Send + 'static {
    fn strategy_name(&self) -> &'_ str;
    fn definition(&self) -> &'_ StrategyDefinition;
    fn create(
        &self,
        params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError>;
}
