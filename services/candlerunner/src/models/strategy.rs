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

#[derive(Serialize, Deserialize)]
pub struct StrategyInstanceDefinition {
    pub strategy_name: String,
    pub params: HashMap<String, ParamValue>,
}

static mut STRATEGY_INSTANCE_NS: Option<Uuid> = None;

fn get_strategy_instance_ns() -> &'static Uuid {
    unsafe {
        STRATEGY_INSTANCE_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"Strategy Instance Id"))
    }
}

impl StrategyInstanceDefinition {
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
}

pub trait StrategyDefinition {
    fn params(&self) -> &'_ Vec<ParamDefinition>;
    fn strategy_name(&self) -> &'_ str;
    fn strategy_description(&self) -> &'_ str;
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
    fn definition(&self) -> &'_ dyn StrategyDefinition;
    fn create(
        &self,
        params: HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError>;
}
