use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;

use crate::models::instruments::Figi;
use crate::models::market_data::CandlePack;
use crate::models::params::{ParamDefinition, ParamError, ParamType, ParamValue};
use crate::models::strategy::{
    InstantiateStrategyError, Strategy, StrategyDefinition, StrategyExecutionError,
    StrategyFactory, StrategyState,
};

const PARAM_NAME_INSTRUMENT: &str = "instrument";

pub struct BuyAndHold {
    figi: Figi,
    data_requirements: [Figi; 1],
}

impl BuyAndHold {
    pub fn new(figi: Figi) -> Self {
        Self {
            figi: figi.clone(),
            data_requirements: [figi],
        }
    }
}

impl Strategy for BuyAndHold {
    fn data_requirements(&self) -> &[Figi] {
        &self.data_requirements
    }

    fn execute(
        &self,
        _ts: DateTime<Utc>,
        _candles: CandlePack,
        mut state: StrategyState,
    ) -> Result<StrategyState, StrategyExecutionError> {
        state.set_signal(self.figi.to_owned(), 1.0f64);
        Ok(state)
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
    ) -> Result<Arc<dyn Strategy>, InstantiateStrategyError> {
        let instrument_param = params
            .get(PARAM_NAME_INSTRUMENT)
            .ok_or_else(|| ParamError::ParamMissing(PARAM_NAME_INSTRUMENT.to_string()))?;

        let figi = instrument_param
            .as_instrument()
            .ok_or_else(|| ParamError::ParamTypeMismatch(PARAM_NAME_INSTRUMENT.to_owned()))?;

        Ok(Arc::new(BuyAndHold::new(figi.to_owned())))
    }
}
