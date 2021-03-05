use chrono::Duration as cDuration;
use chrono::{Datelike, NaiveTime, TimeZone, Utc};
use chrono_tz::US::Mountain;
use log::{debug, error, warn};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use gemini_rust::{GeminiClient, PublicAPI, SymbolDetail, Ticker};

use crate::strats::order_util;
use order_util::cancel_orders_if_open;

pub fn get_amount(price: f32, usd: f32) -> f32 {
    usd / price
}

pub fn get_usd(amount: f32, price: f32) -> f32 {
    price * amount
}

pub fn get_info(client: &GeminiClient, symbol: &str) -> Option<(SymbolDetail, Ticker)> {
    // get the details of the symbol
    let dt = client.symbol_detail(symbol);
    if dt.is_none() {
        error!("{{{}}}: Error getting details. retry in 15", symbol);
        return None;
    }
    let detail = dt.unwrap();
    if detail.status != "open" {
        error!("{{{}}}: is not open: {}", symbol, detail.status);
        return None;
    }
    debug!("{{{}}}: detail: {:?}", symbol, detail);

    // get the ticker of the symbol
    let tk = client.ticker(symbol);
    if tk.is_none() {
        error!("{{{}}}: Error getting ticker. retry in 15s", symbol);
        return None;
    }
    let ticker = tk.unwrap();
    debug!("{{{}}}: ticker: {:?}", symbol, ticker);

    Some((detail, ticker))
}

pub fn sleep_or_die(
    client_mutex: &Mutex<GeminiClient>,
    symbol: &str,
    cond_: &(Mutex<bool>, Condvar),
    order_ids: &Vec<u64>,
    sleep_interval: Duration,
) -> bool {
    let mut run = true;
    let (lock, cvar) = &*cond_;

    let mut die = lock.lock().unwrap();
    let result = cvar.wait_timeout(die, sleep_interval).unwrap();
    debug!("woke up");
    die = result.0;
    debug!("die= {}", die);
    if *die == true {
        warn!("{{{}}}: Terminated! Cancelling all open orders.", symbol);
        let client = client_mutex.lock().unwrap();
        cancel_orders_if_open(&client, order_ids);
        run = false;
    }
    run
}

pub fn get_dur_until_next_target_date(
    day: i64,
    hrs: u32,
    mins: u32,
    secs: u32,
) -> std::time::Duration {
    let utc = Utc::now().naive_utc();
    let now = Mountain.from_utc_datetime(&utc);

    let now_date = now.date();
    let current_day = now_date.weekday().number_from_monday() as i64;
    let sunday_correction =
        if current_day == day && now.time() < NaiveTime::from_hms(hrs, mins, secs) {
            0
        } else {
            7
        };
    let days_to_sunday = cDuration::days((7 + current_day - day) % 7 + sunday_correction);
    let target_date = (now_date + days_to_sunday).and_hms(hrs, mins, secs);
    let duration = target_date.signed_duration_since(now).to_std().unwrap();

    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    debug!(
        "Duration between {:?} and {:?} is: {}:{}:{}",
        now, target_date, hours, minutes, seconds
    );
    duration
}
