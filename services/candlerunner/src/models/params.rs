use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::instance_id::InstanceId;
use crate::models::instruments::Figi;
use crate::models::namespaces::get_params_set_ns;

use crate::utils::id_generator::IdGenerator;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub enum ParamValue {
    Instrument(Figi),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl ParamValue {
    pub fn as_instrument(&self) -> Option<&Figi> {
        if let ParamValue::Instrument(val) = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        if let ParamValue::Integer(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if let ParamValue::Float(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        if let ParamValue::Boolean(val) = self {
            Some(*val)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

impl InstanceId for HashMap<String, ParamValue> {
    fn id(&self) -> uuid::Uuid {
        let mut generator = IdGenerator::default();

        let mut sorted_params: Vec<(&String, &ParamValue)> = self.iter().collect();
        sorted_params.sort_by(|lhs, rhs| (*lhs).0.cmp(rhs.0));

        for (param_name, param_value) in sorted_params {
            generator.add("paramName", param_name.as_bytes());

            match param_value {
                ParamValue::Instrument(figi) => generator.add("figi", figi.0.as_bytes()),
                ParamValue::Integer(i) => generator.add("integer", i.to_le_bytes()),
                ParamValue::Float(f) => generator.add("float", f.to_le_bytes()),
                ParamValue::Boolean(b) => {
                    let val = match b {
                        true => [1u8],
                        false => [0u8],
                    };
                    generator.add("boolean", val);
                }
            }
        }

        generator.generate(get_params_set_ns())
    }
}
