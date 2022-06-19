use serde::{de::DeserializeOwned, Serialize};

use crate::models::instance_id::InstanceId;

pub trait ExtractIndicatorValue {
    type ValueType;

    fn extract_value(&self) -> Option<Self::ValueType>;
}

pub trait Indicator: InstanceId {
    type Input;
    type ValueType;
    type State: ExtractIndicatorValue<ValueType = Self::ValueType>
        + Serialize
        + DeserializeOwned
        + Default;

    fn update(&self, state: Self::State, input: &Self::Input) -> Self::State;
}
