use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::instruments::Figi;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamValue {
    Instrument(Figi),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Error)]
pub enum ParamError {
    #[error("Parameter `{0}` is not specified")]
    ParamMissing(String),
    #[error("Invalid parameter `{0}`")]
    InvalidParam(String),
    #[error("Parameter `{0}` is of wrong type")]
    ParamTypeMismatch(String),
    #[error("Parameter `{0}` has unknown instrument value `{1}`")]
    UnknownInstrument(String, String),
}
