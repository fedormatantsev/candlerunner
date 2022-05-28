use serde::{Deserialize, Serialize};

use crate::models::instruments::Figi;

#[derive(Clone, Serialize, Deserialize)]
pub struct Position {
    pub figi: Figi,
    pub lots: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Currency {
    pub iso_currency: String,
    pub amount: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AccountPositions {
    pub currencies: Vec<Currency>,
    pub positions: Vec<Position>,
}
