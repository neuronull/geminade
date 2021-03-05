use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Api {
    pub url: String,
    pub key: String,
    pub sec: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct StratStatic {
    pub symbol: String,
    pub sandbox: Option<bool>,
    pub usd_per_trade: f32,
    pub trade_day: u32,
    pub trade_hr: u32,
    pub trade_min: u32,
    pub trade_sec: u32,
    pub immediate: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StratDips {
    pub symbol: String,
    pub sandbox: Option<bool>,
    pub usd_per_window: u32,
    pub trade_day: u32,
    pub trade_hr: u32,
    pub trade_min: u32,
    pub trade_sec: u32,
    pub immediate: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Cfg {
    pub api: Api,
    pub sandbox: Api,
    // TODO make these optional:
    pub strat_static: Vec<StratStatic>,
    pub strat_dips: Vec<StratDips>,
}

impl Cfg {
    pub fn new() -> Result<Self, ConfigError> {
        let mut c = Config::new();
        c.merge(File::with_name("cfg")).unwrap();
        c.try_into()
    }
}
