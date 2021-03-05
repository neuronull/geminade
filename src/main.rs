use log::debug;
use log::info;
use log4rs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use gemini_rust::GeminiClient;

mod cfg;
use cfg::Cfg;

mod strats;
use strats::dca_dips::*;
use strats::dca_static::*;
use strats::strat::Strat;

fn main() {
    log4rs::init_file("logcfg.yml", Default::default()).unwrap();

    let cfg = Cfg::new().unwrap();

    let cl = GeminiClient::new(&cfg.api.url, &cfg.api.key, &cfg.api.sec);
    let sn = GeminiClient::new(&cfg.sandbox.url, &cfg.sandbox.key, &cfg.sandbox.sec);

    let client = Arc::new(Mutex::new(cl));
    let sandbox = Arc::new(Mutex::new(sn));

    let cond = Arc::new((Mutex::new(false), Condvar::new()));

    let mut handles = vec![];

    for w_cfg in cfg.strat_static {
        let client = Arc::clone(&client);
        let sandbox = Arc::clone(&sandbox);
        info!(
            "{{{}}}: static {} : buy ${:.2} every week",
            w_cfg.symbol,
            match w_cfg.sandbox.unwrap_or(false) {
                true => &cfg.sandbox.url,
                false => &cfg.api.url,
            },
            w_cfg.usd_per_trade,
        );
        let cond_ = Arc::clone(&cond);
        let handle = thread::spawn(move || {
            let strat = Strat::new(
                match w_cfg.sandbox.unwrap_or(false) {
                    true => sandbox,
                    false => client,
                },
                &w_cfg.symbol,
                cond_,
                w_cfg.trade_day as i64,
                w_cfg.trade_hr,
                w_cfg.trade_min,
                w_cfg.trade_sec,
            );
            let mut ctx = StaticStrat::new(w_cfg.usd_per_trade);
            strat.run(&mut ctx, w_cfg.immediate);
        });
        handles.push(handle);
    }

    for w_cfg in cfg.strat_dips {
        let client = Arc::clone(&client);
        let sandbox = Arc::clone(&sandbox);
        info!(
            "{{{}}}: dips {} : buy ${:.2} every week",
            w_cfg.symbol,
            match w_cfg.sandbox.unwrap_or(false) {
                true => &cfg.sandbox.url,
                false => &cfg.api.url,
            },
            w_cfg.usd_per_window,
        );
        let cond_ = Arc::clone(&cond);
        let handle = thread::spawn(move || {
            let strat = Strat::new(
                match w_cfg.sandbox.unwrap_or(false) {
                    true => sandbox,
                    false => client,
                },
                &w_cfg.symbol,
                cond_,
                w_cfg.trade_day as i64,
                w_cfg.trade_hr,
                w_cfg.trade_min,
                w_cfg.trade_sec,
            );
            let mut ctx = DipStrat::new(w_cfg.usd_per_window, 1, 15);
            strat.run(&mut ctx, w_cfg.immediate);
        });
        handles.push(handle);
    }

    // wait for CTL-C
    {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        info!("Initiated. Ctrl-C to exit and terminate open orders.");

        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(5));
        }
        info!("Received Ctrl-C signal, terminating workers.");
    }

    // signal threads to die
    let (lock, cvar) = &*cond;
    {
        debug!("getting lock on die condvar");
        let mut die = lock.lock().unwrap();
        *die = true;
    }
    debug!("die is true, notify_all");
    cvar.notify_all();
    debug!("join threads");

    // join threads
    for handle in handles {
        handle.join().unwrap();
    }
}
