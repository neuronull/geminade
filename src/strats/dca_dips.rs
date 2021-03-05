use log::{debug, error, info, warn};
use std::thread;
use std::time::Duration;

use gemini_rust::{GeminiClient, OrderPlacerAPI, OrderStatusAPI, SymbolDetail, Ticker};

use crate::strats::order_util;
use crate::strats::strat;
use crate::strats::strat_util;
use order_util::*;
use strat::Execute;
use strat_util::*;

pub struct DipStrat {
    pub usd_per_window: u32,
    dip_interval: u32,
    n_trades: u32,
    buckets: Vec<f32>,
    usd_per_trade: f32,
}

impl DipStrat {
    pub fn new(usd_per_window: u32, dip_interval: u32, dip_bot: u32) -> DipStrat {
        let mut d = DipStrat {
            usd_per_window,
            dip_interval,
            n_trades: dip_bot / dip_interval,
            buckets: vec![],
            usd_per_trade: usd_per_window as f32 / (dip_bot / dip_interval) as f32,
        };
        for _ in 1..=d.n_trades {
            d.buckets.push(0.0);
        }
        d
    }
}

impl Execute for DipStrat {
    fn execute(
        &mut self,
        client: &GeminiClient,
        symbol: &str,
        orders: &mut Vec<u64>,
        _dt: &SymbolDetail,
        tk: &Ticker,
    ) -> bool {
        // add to the buckets
        for bucket in &mut self.buckets {
            *bucket = self.usd_per_trade;
        }
        debug!("woke up, buckets: {:#?}", self.buckets);

        let ask: f32 = tk.ask.parse().unwrap();

        // check on status of limit orders
        for order_id in orders.iter_mut() {
            let status = client.order_status(Some(*order_id), None).unwrap();
            let client_order_id = status.client_order_id.unwrap();
            let mut bucket_num: usize = client_order_id.parse().unwrap();
            bucket_num -= 1;

            // cancel live orders and add remainder to that bucket's next trade
            if status.is_live {
                let remaining_amt: f32 = status.remaining_amount.parse().unwrap();
                let remaining_usd = get_usd(remaining_amt, status.price.parse().unwrap());
                self.buckets[bucket_num] += remaining_usd;
                info!(
                        "{{{}}}: {} order id={} has ${:.2} remaining on it. cancelling and adding to bucket {}% = ${:.2}",
                        symbol, client_order_id, status.order_id,
                        remaining_usd, client_order_id, self.buckets[bucket_num]
                    );
                let status = client.cancel_order(*order_id).unwrap();
                if !status.is_cancelled {
                    error!(
                        "{{{}}}: error cancelling order! ({})",
                        symbol,
                        status.reason.unwrap_or("".to_string())
                    );
                }

            // add remainder of cancelled orders to that bucket's next trade
            } else if status.is_cancelled {
                let remaining_amt: f32 = status.remaining_amount.parse().unwrap();
                let remaining_usd = get_usd(remaining_amt, status.price.parse().unwrap());
                if remaining_usd > 0.0 {
                    self.buckets[bucket_num] += remaining_usd;
                    info!(
                            "{{{}}}: {} order id={} cancelled, had ${:.2} remaining on it, adding to bucket {}% = ${:.2}",
                            symbol, client_order_id,  status.order_id,
                            remaining_usd, client_order_id, self.buckets[bucket_num]
                        );
                }
            // reset that bucket
            } else {
                info!(
                    "{{{}}}: {} order id={} fulfilled! {} @ ${}",
                    symbol,
                    client_order_id,
                    status.order_id,
                    status.executed_amount,
                    status.avg_execution_price
                );
                self.buckets[bucket_num] = self.usd_per_trade;
            }
        }
        orders.clear();

        // place a market order to be fulfilled now
        {
            let price = ask;
            let amount = get_amount(price, self.usd_per_trade);
            info!(
                "{{{}}}: n_trades={} usd_per_trade {:.2}",
                symbol, self.n_trades, self.usd_per_trade
            );
            let status = new_taker_order(&client, "", symbol, amount, price);
            if status.is_cancelled {
                // TODO retry in some minutes ? for now just add the amount back in
                self.buckets[0] += self.usd_per_trade;
            }
            thread::sleep(Duration::from_millis(10))
        }

        info!("buckets: {:#?}", &self.buckets);

        // place limit orders on the dips
        for (i, bucket) in self.buckets.iter().enumerate() {
            let pct = (i + 1) * self.dip_interval as usize;
            let price = ask * (1.0 - (pct as f32 / 100.0));
            let amount = get_amount(price, *bucket);
            let status = new_maker_order(
                &client,
                &format!("{}", i + 1).to_owned(),
                symbol,
                amount,
                price,
            );
            if status.is_cancelled {
                warn!(
                    "{{{}}}: order was cancelled! ({})",
                    symbol,
                    status.reason.unwrap_or("".to_string())
                );
            } else {
                info!(
                    "{{{}}}: {}% order id={} {:.6} @ ${:.2}",
                    symbol, pct, status.order_id, amount, price
                );
                let id: u64 = status.order_id.parse().unwrap();
                orders.push(id);
            }
            thread::sleep(Duration::from_millis(10))
        }
        debug!("{{{}}}: order_ids: {:?}", symbol, orders);
        true
    }
}
