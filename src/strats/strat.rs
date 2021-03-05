use log::info;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration as Duration2;
use std::time::Instant;

use gemini_rust::{GeminiClient, SymbolDetail, Ticker};

use crate::strats::strat_util;
use strat_util::*;

pub trait Execute {
    fn execute(
        &mut self,
        client: &GeminiClient,
        symbol: &str,
        orders: &mut Vec<u64>,
        dt: &SymbolDetail,
        tk: &Ticker,
    ) -> bool;
}

pub struct Strat {
    client_mutex: Arc<Mutex<GeminiClient>>,
    symbol: String,
    cond_: Arc<(Mutex<bool>, Condvar)>,
    day: i64,
    hr: u32,
    min: u32,
    sec: u32,
}

impl Strat {
    pub fn new(
        client_mutex: Arc<Mutex<GeminiClient>>,
        symbol: &str,
        cond_: Arc<(Mutex<bool>, Condvar)>,
        day: i64,
        hr: u32,
        min: u32,
        sec: u32,
    ) -> Strat {
        Strat {
            client_mutex,
            symbol: symbol.to_owned(),
            cond_,
            day,
            hr,
            min,
            sec,
        }
    }

    pub fn run<T: Execute>(&self, ctx: &mut T, immediate: Option<bool>) {
        let retry_interval = Duration2::new(15, 0);

        let mut sleep_interval = if immediate.unwrap_or(false) {
            Duration2::new(0, 0)
        } else {
            get_dur_until_next_target_date(self.day, self.hr, self.min, self.sec)
        };

        let mut orders: Vec<u64> = vec![];
        let mut run = true;
        let mut is_retry = false;
        let mut start = Instant::now();

        let seconds = sleep_interval.as_secs() % 60;
        let minutes = (sleep_interval.as_secs() / 60) % 60;
        let hours = (sleep_interval.as_secs() / 60) / 60;
        info!(
            "{{{}}}: Waking up in {}:{}:{}",
            self.symbol, hours, minutes, seconds
        );

        while run {
            run = sleep_or_die(
                &self.client_mutex,
                &self.symbol,
                &self.cond_,
                &orders,
                sleep_interval,
            );
            if !run {
                info!("{{{}}}: terminated", self.symbol);
                return;
            }

            if !is_retry {
                start = Instant::now();
            }
            is_retry = false;

            let client = self.client_mutex.lock().unwrap();

            let info = get_info(&client, &self.symbol);
            if info.is_none() {
                sleep_interval = retry_interval;
                is_retry = true;
                continue;
            }
            let (dt, tk) = info.unwrap();

            if ctx.execute(&client, &self.symbol, &mut orders, &dt, &tk) == false {
                sleep_interval = retry_interval;
                is_retry = true;
                continue;
            }

            let dur = Instant::now() - start;
            sleep_interval = get_dur_until_next_target_date(self.day, self.hr, self.min, self.sec);
            let seconds = sleep_interval.as_secs() % 60;
            let minutes = (sleep_interval.as_secs() / 60) % 60;
            let hours = (sleep_interval.as_secs() / 60) / 60;
            info!(
                "{{{}}}: Took {:?}. Waking up in {}:{}:{}",
                self.symbol, dur, hours, minutes, seconds
            );
        }
    }
}
