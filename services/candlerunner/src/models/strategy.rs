use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::instruments::Figi;
use crate::models::market_data::{Candle, CandleResolution};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParamType {
    Instrument,
    Integer,
    Float,
    Boolean,
}

impl From<&ParamValue> for ParamType {
    fn from(value: &ParamValue) -> Self {
        match value {
            ParamValue::Instrument(_) => Self::Instrument,
            ParamValue::Integer(_) => Self::Integer,
            ParamValue::Float(_) => Self::Float,
            ParamValue::Boolean(_) => Self::Boolean,
        }
    }
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
    default_value: Option<ParamValue>,
}

impl ParamDefinition {
    pub fn new<N: ToString, D: ToString>(
        name: N,
        description: D,
        param_type: ParamType,
        default_value: Option<ParamValue>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            param_type,
            default_value,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn param_type(&self) -> &ParamType {
        &self.param_type
    }

    pub fn default_value(&self) -> &Option<ParamValue> {
        &self.default_value
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StrategyInstanceDefinition {
    strategy_name: String,
    params: HashMap<String, ParamValue>,
    time_from: DateTime<Utc>,
    time_to: Option<DateTime<Utc>>,
    resolution: CandleResolution,
}

static mut STRATEGY_INSTANCE_NS: Option<Uuid> = None;

fn get_strategy_instance_ns() -> &'static Uuid {
    unsafe {
        STRATEGY_INSTANCE_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"Strategy Instance Id"))
    }
}

impl StrategyInstanceDefinition {
    pub fn new<N: ToString>(
        strategy_name: N,
        params: HashMap<String, ParamValue>,
        time_from: DateTime<Utc>,
        time_to: Option<DateTime<Utc>>,
        resolution: CandleResolution,
    ) -> Self {
        Self {
            strategy_name: strategy_name.to_string(),
            params,
            time_from,
            time_to,
            resolution,
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

        bytes.extend_from_slice(b"Time from:");
        bytes.extend_from_slice(&self.time_from.timestamp().to_le_bytes());
        bytes.push(0);

        bytes.extend_from_slice(b"Time to:");
        match self.time_to {
            Some(dt) => bytes.extend_from_slice(&dt.timestamp().to_le_bytes()),
            None => bytes.extend_from_slice(b"NOW"),
        }
        bytes.push(0);

        bytes.extend_from_slice(b"Resolution:");
        match self.resolution {
            CandleResolution::OneMinute => bytes.extend_from_slice(b"OneMinute"),
            CandleResolution::OneHour => bytes.extend_from_slice(b"OneHour"),
            CandleResolution::OneDay => bytes.extend_from_slice(b"OneDay"),
        }
        bytes.push(0);

        Uuid::new_v5(get_strategy_instance_ns(), &bytes)
    }

    pub fn strategy_name(&self) -> &str {
        &self.strategy_name
    }

    pub fn params(&self) -> &HashMap<String, ParamValue> {
        &self.params
    }

    pub fn time_from(&self) -> DateTime<Utc> {
        self.time_from
    }

    pub fn time_to(&self) -> Option<DateTime<Utc>> {
        self.time_to
    }

    pub fn resolution(&self) -> CandleResolution {
        self.resolution
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

#[derive(Error, Debug)]
pub enum StrategyExecutionError {
    #[error("Failed to execute strategy; retry later")]
    ExecutionFailure,

    #[error("Failed to execute strategy")]
    NonFixableFailure,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum StrategyExecutionStatus {
    Running,
    Finished,
    Failed,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StrategyExecutionContext {
    output: f64,
    meta: bson::Document,
}

impl StrategyExecutionContext {
    pub fn new(output: f64, meta: bson::Document) -> Self {
        Self { output, meta }
    }

    /// Get the strategy execution context's output.
    pub fn output(&self) -> f64 {
        self.output
    }
}

pub enum StrategyExecutionOutput {
    Unavailable,
    Available(StrategyExecutionContext),
}

pub trait Strategy: Send + Sync + 'static {
    fn data_requirements(&self) -> &[Figi];
    fn execute(
        &self,
        ts: DateTime<Utc>,
        candles: HashMap<Figi, Candle>,
        prev_context: Option<StrategyExecutionContext>,
    ) -> Result<StrategyExecutionOutput, StrategyExecutionError>;
}

#[derive(Error, Debug)]
pub enum CreateStrategyError {
    #[error("Strategy `{0}` is not found")]
    StrategyNotFound(String),
    #[error("Strategy parameter `{0}` is not specified")]
    ParamMissing(String),
    #[error("Invalid strategy parameter `{0}`")]
    InvalidParam(String),
    #[error("Strategy parameter `{0}` is of wrong type")]
    ParamTypeMismatch(String),
    #[error("Failed to instantiate strategy: {0}")]
    FailedToInstantiateStrategy(String),
}

pub trait StrategyFactory: Sync + Send + 'static {
    fn strategy_name(&self) -> &'_ str;
    fn definition(&self) -> &'_ StrategyDefinition;
    fn create(
        &self,
        params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError>;
}
