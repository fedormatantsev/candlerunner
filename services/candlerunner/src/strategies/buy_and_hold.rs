use std::collections::HashMap;
use std::sync::Arc;

use crate::models::instruments::Figi;
use crate::models::strategy::{
    CreateStrategyError, ParamDefinition, ParamType, ParamValue, Strategy, StrategyDefinition,
    StrategyFactory,
};

const PARAM_NAME_INSTRUMENT: &str = "Instrument";

pub struct BuyAndHold {
    _figi: Figi,
}

impl BuyAndHold {
    pub fn new(figi: Figi) -> Self { Self { _figi: figi } }
}

impl Strategy for BuyAndHold {}

pub struct BuyAndHoldFactory {
    definition: StrategyDefinition,
}

impl Default for BuyAndHoldFactory {
    fn default() -> Self {
        Self {
            definition: StrategyDefinition::new(
                vec![ParamDefinition::new(
                    PARAM_NAME_INSTRUMENT,
                    "Instrument to buy",
                    ParamType::Instrument,
                    None,
                )],
                "BuyAndHold",
                "Buys instrument on day 1 and keeps it for the whole period of time",
            ),
        }
    }
}

impl StrategyFactory for BuyAndHoldFactory {
    fn strategy_name(&self) -> &str {
        "BuyAndHold"
    }

    fn definition(&self) -> &StrategyDefinition {
        &self.definition
    }

    fn create(
        &self,
        params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError> {
        let instrument_param = params
            .get(PARAM_NAME_INSTRUMENT)
            .ok_or_else(|| CreateStrategyError::ParamMissing(PARAM_NAME_INSTRUMENT.to_string()))?;

        let figi = match instrument_param {
            ParamValue::Instrument(figi) => figi.clone(),
            _ => return Err(CreateStrategyError::ParamTypeMismatch(PARAM_NAME_INSTRUMENT.to_string())),
        };

        Ok(Arc::new(BuyAndHold::new(figi)))
    }
}
