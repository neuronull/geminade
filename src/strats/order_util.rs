use log::{debug, error, info, warn};

use gemini_rust::{GeminiClient, OrderPlacerAPI, OrderStatus, OrderStatusAPI};

use crate::strats::strat_util;
use strat_util::get_usd;

fn new_order(
    client: &GeminiClient,
    order_id: &str,
    type_: &str,
    symbol: &str,
    amount: f32,
    price: f32,
) -> OrderStatus {
    let os = client.new_order(
        symbol,
        amount,
        price,
        "buy",
        "exchange limit",
        order_id,
        &vec![type_],
    );
    if os.is_none() {
        error!(
            "{{{}}}: {} order {} failed ${:.2} {:.6} @ ${}",
            symbol,
            type_,
            order_id,
            get_usd(amount, price),
            amount,
            price,
        );
        let mut r = OrderStatus::default();
        r.is_cancelled = true;
        return r;
    }
    let order_status = os.unwrap();

    debug!("{{{}}}: {:?}", symbol, order_status);

    if order_status.is_cancelled {
        warn!(
            "{{{}}}: {} order {} cancelled ${:.2} {:.8} @ ${}",
            symbol,
            type_,
            order_status.order_id,
            get_usd(amount, price),
            amount,
            price,
            // TODO print reason &order_status.reason.unwrap_or("".to_string())
        );
    } else {
        info!(
            "{{{}}}: {} order {} success ${:.2} {:.8} @ ${}",
            symbol,
            type_,
            order_status.order_id,
            get_usd(
                order_status.original_amount.parse().unwrap(),
                order_status.price.parse().unwrap()
            ),
            order_status.original_amount,
            order_status.price,
        );
    }
    return order_status;
}

pub fn new_maker_order(
    client: &GeminiClient,
    order_id: &str,
    symbol: &str,
    amount: f32,
    price: f32,
) -> OrderStatus {
    new_order(client, order_id, "maker-or-cancel", symbol, amount, price)
}

pub fn new_taker_order(
    client: &GeminiClient,
    order_id: &str,
    symbol: &str,
    amount: f32,
    price: f32,
) -> OrderStatus {
    new_order(
        client,
        order_id,
        "immediate-or-cancel",
        symbol,
        amount,
        price,
    )
}

pub fn cancel_orders_if_open(client: &GeminiClient, order_ids: &Vec<u64>) -> bool {
    let mut ret = true;
    for order_id in order_ids {
        let os = client.order_status(Some(*order_id), None);
        if os.is_none() {
            warn!("No order status for order_id: {}", order_id);
            ret = false;
            continue;
        }
        let order_status = os.unwrap();
        if order_status.is_live {
            let status = client.cancel_order(*order_id).unwrap();
            if !status.is_cancelled {
                error!(
                    "{{{}}}: error cancelling limit order! ({})",
                    status.symbol,
                    status.reason.unwrap_or("".to_string())
                );
                ret = false;
            } else {
                info!(
                    "{{{}}}: {} order id={} cancelled.",
                    status.symbol,
                    status.client_order_id.unwrap_or("".to_string()),
                    status.order_id
                );
            }
        }
    }
    ret
}
