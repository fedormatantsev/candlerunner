use std::collections::HashMap;

use crate::models::instruments::Figi;

pub enum ParamType {
    Instrument,
    Integer,
    Float,
    Boolean,
}

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

pub struct StrategyInstanceDefinition {
    pub strategy_name: String,
    pub params: HashMap<String, ParamValue>,
}

pub trait StrategyDefinition {
    fn params(&self) -> &'_ Vec<ParamDefinition>;
    fn strategy_name(&self) -> &'_ str;
    fn strategy_description(&self) -> &'_ str;
}

pub trait Strategy {}

pub enum CreateStrategyError {
    StrategyNotFound(String),
    ParamMissing(String),
    ParamTypeMismatch(String),
    FailedToCreateStrategy { source: anyhow::Error },
}

pub trait StrategyFactory: Sync + Send + 'static {
    fn strategy_name(&self) -> &'_ str;
    fn definition(&self) -> &'_ dyn StrategyDefinition;
    fn create(
        &self,
        params: HashMap<String, ParamValue>,
    ) -> Result<Box<dyn Strategy>, CreateStrategyError>;
}
