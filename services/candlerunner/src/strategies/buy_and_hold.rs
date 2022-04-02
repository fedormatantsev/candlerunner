use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;

use crate::models::instruments::Figi;
use crate::models::strategy::{
    CreateStrategyError, InstrumentDataRequirement, ParamDefinition, ParamType, ParamValue,
    Strategy, StrategyDefinition, StrategyFactory,
};

const PARAM_NAME_INSTRUMENT: &str = "Instrument";

pub struct BuyAndHold {
    _figi: Figi,
    data_requirements: [InstrumentDataRequirement; 1],
}

impl BuyAndHold {
    pub fn new(figi: Figi, time_from: DateTime<Utc>, time_to: Option<DateTime<Utc>>) -> Self {
        Self {
            _figi: figi.clone(),
            data_requirements: [InstrumentDataRequirement {
                figi,
                time_from,
                time_to,
            }],
        }
    }
}

impl Strategy for BuyAndHold {
    fn data_requirements(&self) -> &[InstrumentDataRequirement] {
        &self.data_requirements
    }
}

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
        time_from: DateTime<Utc>,
        time_to: Option<DateTime<Utc>>,
    ) -> Result<Arc<dyn Strategy>, CreateStrategyError> {
        let instrument_param = params
            .get(PARAM_NAME_INSTRUMENT)
            .ok_or_else(|| CreateStrategyError::ParamMissing(PARAM_NAME_INSTRUMENT.to_string()))?;

        let figi = match instrument_param {
            ParamValue::Instrument(figi) => figi.clone(),
            _ => {
                return Err(CreateStrategyError::ParamTypeMismatch(
                    PARAM_NAME_INSTRUMENT.to_string(),
                ))
            }
        };

        Ok(Arc::new(BuyAndHold::new(figi, time_from, time_to)))
    }
}
