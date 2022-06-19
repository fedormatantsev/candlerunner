use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::account::AccountId;
use crate::models::indicator::{ExtractIndicatorValue, Indicator};
use crate::models::instance_id::InstanceId;
use crate::models::instruments::Figi;
use crate::models::market_data::{CandlePack, CandleResolution};
use crate::models::namespaces;
use crate::models::params::{ParamDefinition, ParamError, ParamValue};

use crate::utils::id_generator::IdGenerator;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderSettings {
    account_id: AccountId,

    buy_threshold: Option<f64>,
    sell_threshold: Option<f64>,

    /// Offset in ticks from order price to stop loss price
    stop_loss_offset: u32,

    /// Offset in ticks from order price to take profit price
    take_profit_offset: u32,

    /// Length of interval in candles
    interval_length: u32,
}

impl InstanceId for PlaceOrderSettings {
    fn id(&self) -> Uuid {
        let mut generator = IdGenerator::default();
        generator.add("accountId", self.account_id.0.as_bytes());
        generator.add_opt(
            "buyThreshold",
            self.buy_threshold.map(|val| val.to_le_bytes()),
        );
        generator.add_opt(
            "sellThreshold",
            self.sell_threshold.map(|val| val.to_le_bytes()),
        );

        generator.add("stopLossOffset", self.stop_loss_offset.to_le_bytes());
        generator.add("takeProfitOffset", self.take_profit_offset.to_le_bytes());
        generator.add("interval_length", self.interval_length.to_le_bytes());

        generator.generate(namespaces::get_place_order_settings_ns())
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategyInstanceDefinition {
    strategy_name: String,
    params: HashMap<String, ParamValue>,
    time_from: DateTime<Utc>,
    time_to: Option<DateTime<Utc>>,
    resolution: CandleResolution,
    place_order_settings: Option<PlaceOrderSettings>,
}

impl InstanceId for StrategyInstanceDefinition {
    fn id(&self) -> Uuid {
        let mut generator = IdGenerator::default();
        generator.add("strategyName", self.strategy_name.as_bytes());
        generator.add("params", self.params().id().as_bytes());
        generator.add("timeFrom", &self.time_from.timestamp().to_le_bytes());
        generator.add_opt(
            "timeTo",
            self.time_to.map(|val| val.timestamp().to_le_bytes()),
        );
        
        generator.add("resolution", self.resolution.to_string().as_bytes());

        generator.add_opt(
            "placeOrderSettings",
            self.place_order_settings
                .as_ref()
                .map(|val| val.id().as_bytes().to_owned()),
        );

        generator.generate(namespaces::get_strategy_instance_ns())
    }
}

impl StrategyInstanceDefinition {
    pub fn new<N: ToString>(
        strategy_name: N,
        params: HashMap<String, ParamValue>,
        time_from: DateTime<Utc>,
        time_to: Option<DateTime<Utc>>,
        resolution: CandleResolution,
        place_order_settings: Option<PlaceOrderSettings>,
    ) -> Self {
        Self {
            strategy_name: strategy_name.to_string(),
            params,
            time_from,
            time_to,
            resolution,
            place_order_settings,
        }
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

    pub fn place_order_settings(&self) -> &Option<PlaceOrderSettings> {
        &self.place_order_settings
    }
}

#[derive(Serialize, Deserialize)]
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
    CriticalFailure,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub enum StrategyExecutionStatus {
    Running,
    Finished,
    Failed,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyExecution {
    status: StrategyExecutionStatus,
    last_execution_timestamp: DateTime<Utc>,
}

impl StrategyExecution {
    pub fn new(status: StrategyExecutionStatus, last_execution_time: DateTime<Utc>) -> Self {
        Self {
            status,
            last_execution_timestamp: last_execution_time,
        }
    }

    pub fn status(&self) -> StrategyExecutionStatus {
        self.status
    }

    pub fn last_execution_timestamp(&self) -> DateTime<Utc> {
        self.last_execution_timestamp
    }

    pub fn set_status(&mut self, status: StrategyExecutionStatus) {
        self.status = status;
    }

    pub fn set_last_execution_timestamp(&mut self, last_execution_timestamp: DateTime<Utc>) {
        self.last_execution_timestamp = last_execution_timestamp;
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyState {
    indicators: HashMap<uuid::Uuid, bson::Bson>,
    signals: HashMap<Figi, f64>,
}

impl StrategyState {
    pub fn set_signal(&mut self, instrument: Figi, value: f64) {
        self.signals.insert(instrument, value);
    }

    pub fn update_indicator<I: Indicator>(
        &mut self,
        indicator: &I,
        input: &I::Input,
    ) -> Option<I::ValueType> {
        let id = indicator.id();
        let state = match self.indicators.get(&id) {
            Some(serialized) => {
                bson::from_bson::<I::State>(serialized.to_owned()).unwrap_or_else(|err| {
                    println!("Failed to deserialize indicator state: {}", err);
                    I::State::default()
                })
            }
            None => I::State::default(),
        };

        let next_state = indicator.update(state, input);
        let value = next_state.extract_value();

        match bson::to_bson(&next_state) {
            Ok(serialized) => {
                self.indicators.insert(id, serialized);
            }
            Err(err) => println!("Failed to serialize indicator state: {}", err),
        }

        value
    }
}

pub trait Strategy: Send + Sync + 'static {
    fn data_requirements(&self) -> &[Figi];
    fn execute(
        &self,
        ts: DateTime<Utc>,
        candles_pack: CandlePack,
        state: StrategyState,
    ) -> Result<StrategyState, StrategyExecutionError>;
}

#[derive(Error, Debug)]
pub enum InstantiateStrategyError {
    #[error("Strategy `{0}` is not found")]
    NotFound(String),
    #[error("Failed to instantiate strategy: {0}")]
    FailedToInstantiate(String),
    #[error("Params validation failed")]
    ParamValidationFailed {
        #[from]
        source: ParamError,
    },
}

pub trait StrategyFactory: Sync + Send + 'static {
    fn strategy_name(&self) -> &'_ str;
    fn definition(&self) -> &'_ StrategyDefinition;
    fn create(
        &self,
        params: &HashMap<String, ParamValue>,
    ) -> Result<Arc<dyn Strategy>, InstantiateStrategyError>;
}
