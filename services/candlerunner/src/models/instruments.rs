#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Figi(pub String);

#[derive(Debug, Clone)]
pub struct Ticker(pub String);

#[derive(Debug)]
pub struct Instrument {
    pub figi: Figi,
    pub ticker: Ticker,
    pub display_name: String,
}
