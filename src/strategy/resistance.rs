use rand::{Rng, thread_rng};
use tokio_tungstenite::connect_async;
use futures_util::{StreamExt, SinkExt};
use chrono::{DateTime, Utc};
use colored::Colorize;

use std::sync::Arc;

use crate::dydx::{InternalAccount, Markets, Position, Side, OrderType, Exposure, TradeData};

use crate::analysis::Matrix;

pub const WINDOW_SIZE: usize = 10;
pub const DEVIATIONS: f64 = 1.0;
pub const SENSITIVITY: f64 = 0.0002;

enum Orderbook {
    Bid,
    Ask,
}

pub struct Resistance;

impl Resistance {

    pub(crate) async fn run(account: InternalAccount, market: Markets, exposure: Exposure, testnet: bool) -> anyhow::Result<()> {

        let now = Utc::now().timestamp() as u64;
        let position_id = account.position_id();
        let acc = Arc::new(account);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(f64, Orderbook)>(30);


        // <------------------------------------------------------------->
        //            This thread manages the DYDX orderbook
        // <------------------------------------------------------------->

        tokio::spawn(async move {

            let url = {
                if testnet { url::Url::parse("wss://api.stage.dydx.exchange/v3/ws").unwrap() }
                else { url::Url::parse("wss://api.dydx.exchange/v3/ws").unwrap() }
            };

            let (socket, _response) = connect_async(url).await?;
            let (mut write, read) = socket.split();

            let message = market.orderbook_feed_message();
            write.send(message).await?;

            let read_future = read.for_each(|message| async {
                if let Err(e) = message { println!("{} {} {:?}", "[-]".red().bold(), "Failed to read message with error:".red(), e); }
                else {
                    let data: Result<serde_json::Value, serde_json::Error> = serde_json::from_slice(message.unwrap().into_data().as_slice());
                    if let Ok(d) = data {
                        let asks = &d["contents"]["asks"];
                        let bids = &d["contents"]["bids"];
                        if !asks[0].is_object() && !asks[0].is_null() {
                            for e in asks.as_array().unwrap() {
                                let price = e[0].as_str().unwrap().parse::<f64>().unwrap();
                                let size = e[1].as_str().unwrap().parse::<f64>().unwrap();
                                if size > 0.0 { tx.clone().send((price, Orderbook::Ask)).await; }
                            }
                        }
                        if !bids[0].is_object() && !bids[0].is_null() {
                        for e in bids.as_array().unwrap() {
                                let price = e[0].as_str().unwrap().parse::<f64>().unwrap();
                                let size = e[1].as_str().unwrap().parse::<f64>().unwrap();
                                if size > 0.0 { tx.clone().send((price, Orderbook::Bid)).await; }
                            }
                        }
                    }
                }
            });
            read_future.await;
            Ok::<(), anyhow::Error>(())
        });


        let mut c = 0;
        let mut t = [0u64; WINDOW_SIZE];
        let mut p = [0f64; WINDOW_SIZE];

        let mut anchor = Utc::now().timestamp() as u64;
        let mut best_ask = f64::MAX;
        let mut best_bid = 0f64;
        let mut recent_price = 0f64;
        let mut most_recent_timestamp = anchor;
        let mut open_short = false;
        let mut open_long = false;

        while t[WINDOW_SIZE-1] == 0 {
            if let Ok((price, orderbook)) = rx.try_recv() {
                let snapshot = Utc::now().timestamp() as u64;
                if snapshot != most_recent_timestamp {
                    if most_recent_timestamp == anchor { most_recent_timestamp = snapshot; }
                    else {
                        p[c] = (best_ask + best_bid)/2f64;
                        t[c] = most_recent_timestamp - anchor;
                        most_recent_timestamp = snapshot;
                        c += 1;

                        best_ask = f64::MAX;
                        best_bid = 0f64;
                    }

                }
                match orderbook {
                    Orderbook::Ask => {
                        if best_ask > price {
                            best_ask = price;
                        }
                    }
                    _ => {
                        if best_bid < price {
                            best_bid = price;
                        }
                    }
                }
            }
        }

        let mut matrix = Matrix::new(t, p);
        let mut r = matrix.resistances(DEVIATIONS);

        println!("{} {}", "[i]".yellow().bold(), "The bot has gathered sufficient data and will begin trading".yellow());
        println!("{:?}", r);
        println!("{:?}", matrix);

        /*
        // TODO: Look at the windows. THey are not optimized at all
        loop {
            // Populate buffer.
            while let Ok((price, timestamp)) = rx.try_recv() {
                t[c] = timestamp - anchor;
                p[c] = price;
                most_recent_timestamp = timestamp - anchor;
                c += 1;
            
                // Refresh support lines.
                if c == WINDOW_SIZE {
                    anchor = timestamp;
                    matrix = Matrix::new(t, p);
                    r = matrix.resistances(DEVIATIONS);
                    println!("{:?}", r);
                    c = 0;
                }
            }

            while let Ok(price) = o_rx.try_recv() {

            }

            // If we are currently
            // 1. Above our top resistance line,
            // 2. Moving downwards,
            // 3. Without an open short position,
            // 4. Close to the resistance line...
            // ... we sell

            let mrt = most_recent_timestamp as f64;
            let threshold = r.upper_alpha() + (r.beta() * mrt);
            let itm = threshold <= most_recent_price;
            let moving = most_recent_price < recent_price;
            let target = SENSITIVITY * most_recent_price;
            let proximity = target <= (most_recent_price - threshold);

            if itm && moving && !open_short && proximity {
                let tacc = acc.clone();
                let random_id = thread_rng().gen::<u128>();
                let order_size = market.default_order_size().to_string(); // TODO
                let position = Position::new(format!("{}", threshold as u128), order_size, Side::SELL);
                let trade_data = TradeData::new(None, None, Some(false));
                let trade_response: Result<serde_json::Value, anyhow::Error> = tokio::spawn(async move { 
                    let response = tacc.open_order(
                        market,
                        position,
                        OrderType::MARKET,
                        "0.01".to_string(),
                        format!("{}", random_id),
                        position_id,
                        false,
                        trade_data,
                        testnet
                    ).await?;
                    Ok(serde_json::from_str(&response).unwrap())
                }).await.unwrap();

                if let Ok(value) = trade_response {
                    if let Some(price) = value["order"]["price"].as_str() {
                        println!("{} {} {}", "[+]".green().bold(), "Placed sell order at".green(), price);
                        if open_long { open_long = false; }
                        else { open_short = true; }
                    }
                    else {
                        println!("{} {} {:?}", "[-]".red().bold(), "Error opening sell order. Reason:".red(), value);
                    }
                }
            }

            // If we are currently
            // 1. Below our bottom resistance line,
            // 2. Moving upwards,
            // 3. Without an open long position,
            // 4. Close to the resistance line...
            // ... we buy

            let threshold = r.lower_alpha() + (r.beta() * mrt);
            let itm = threshold >= most_recent_price;
            let moving = most_recent_price > recent_price;
            let target = SENSITIVITY * threshold;
            let proximity = target <= (threshold - most_recent_price); // Check

            if itm && moving && !open_short && proximity {
                let tacc = acc.clone();
                let random_id = thread_rng().gen::<u128>();
                let order_size = market.default_order_size().to_string(); //TODO
                let position = Position::new(format!("{}", threshold as u128), order_size, Side::BUY);
                let trade_data = TradeData::new(None, None, Some(false));
                let trade_response: Result<serde_json::Value, anyhow::Error> = tokio::spawn(async move { 
                    let response = tacc.open_order(
                        market,
                        position,
                        OrderType::MARKET,
                        "0.01".to_string(),
                        format!("{}", random_id),
                        position_id,
                        false,
                        trade_data,
                        testnet
                    ).await?;
                    Ok(serde_json::from_str(&response).unwrap())
                }).await.unwrap();

                if let Ok(value) = trade_response {
                    if let Some(price) = value["order"]["price"].as_str() {
                        println!("{} {} {}", "[+]".green().bold(), "Placed buy order at".green(), price);
                        if open_short { open_short = false; }
                        else { open_long = true; }
                    }
                    else {
                        println!("{} {} {:?}", "[-]".red().bold(), "Error opening buy order. Reason:".red(), value);
                    }
                }
            }
        }
    */
        Ok(())
    }
}

