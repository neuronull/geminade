use log::{info, warn};

use gemini_rust::{GeminiClient, SymbolDetail, Ticker};

use crate::strats::order_util;
use crate::strats::strat;
use crate::strats::strat_util;
use order_util::new_taker_order;
use strat::Execute;
use strat_util::*;

pub struct StaticStrat {
    pub usd_per_trade: f32,
}

impl StaticStrat {
    pub fn new(usd_per_trade: f32) -> StaticStrat {
        StaticStrat { usd_per_trade }
    }
}

impl Execute for StaticStrat {
    fn execute(
        &mut self,
        client: &GeminiClient,
        symbol: &str,
        orders: &mut Vec<u64>,
        _dt: &SymbolDetail,
        tk: &Ticker,
    ) -> bool {
        let ask: f32 = tk.ask.parse().unwrap();

        let amount = get_amount(ask, self.usd_per_trade);
        let order_status = new_taker_order(&client, "", symbol, amount, ask);

        if order_status.is_cancelled
            && order_status.reason == Some("ImmediateOrCancelWouldPost".to_owned())
        {
            warn!(
                "{{{}}}: order cancelled as would post, retry in 15s",
                symbol
            );
            return false;
        }
        info!(
            "{{{}}}: limit order id={} {:.6} @ ${:.2}",
            symbol, order_status.order_id, amount, ask
        );
        if orders.len() == 0 {
            orders.push(order_status.order_id.parse::<u64>().unwrap());
        } else {
            orders[0] = order_status.order_id.parse::<u64>().unwrap();
        }
        return true;
    }
}
