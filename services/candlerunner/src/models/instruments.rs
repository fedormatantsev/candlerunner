use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Figi(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct Instrument {
    pub figi: Figi,
    pub ticker: Ticker,
    pub display_name: String,
}
