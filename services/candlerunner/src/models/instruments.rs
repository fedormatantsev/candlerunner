#[derive(Debug)]
pub struct Figi(pub String);

#[derive(Debug)]
pub struct Ticker(pub String);

#[derive(Debug)]
pub struct Instrument {
    pub figi: Figi,
    pub ticker: Ticker,
    pub display_name: String,
}
