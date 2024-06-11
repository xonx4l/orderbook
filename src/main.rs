use crossterm::{
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::stdout;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use reqwest;

const API_BASE: &str = "https://api.example.com";
const WSS_URL: &str = "wss://stream.example.com/ws";
const SYMBOL: &str = "btcusdt";

type Price = Decimal;
type Quantity = Decimal;

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    pub asks: BTreeMap<Price, Quantity>,
    pub bids: BTreeMap<Price, Quantity>,
    pub last_update_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookSnapshot {
    pub asks: Vec<(Price, Quantity)>,
    pub bids: Vec<(Price, Quantity)>,
    pub last_update_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DepthMessage {
    pub data: DepthEvent,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DepthEvent {
    #[serde(rename(deserialize = "e"))]
    event_type: String,
    #[serde(rename(deserialize = "E"))]
    event_time: i64,
    #[serde(rename(deserialize = "s"))]
    symbol: String,
    #[serde(rename(deserialize = "a"))]
    pub asks: Vec<(Price, Quantity)>,
    #[serde(rename(deserialize = "b"))]
    pub bids: Vec<(Price, Quantity)>,
    #[serde(rename(deserialize = "U"))]
    pub first_update_id: u64,
    #[serde(rename(deserialize = "u"))]
    pub last_update_id: u64,
}

#[tokio::main]
async fn main() {
    let (stream, _) = connect_async(WSS_URL).await.expect("Failed to connect");
    let (mut write, mut read) = stream.split();

    let subscribe_message = serde_json::json!({
        "method": "SUBSCRIBE",
        "params": [format!("{}@depth", SYMBOL)],
        "id": 1
    })
    .to_string();

    write.send(Message::text(subscribe_message)).await.unwrap();

    let order_book = Arc::new(Mutex::new(OrderBook {
        asks: BTreeMap::new(),
        bids: BTreeMap::new(),
        last_update_id: 0,
    }));
    let events = Arc::new(Mutex::new(Vec::new()));

    let read_future = tokio::spawn({
        let events = events.clone();

        async move {
            while let Some(message) = read.next().await {
                if let Ok(message) = message {
                    if let Ok(depth_message) = serde_json::from_slice::<DepthMessage>(&message.into_data()) {
                        events.lock().await.push(depth_message.data);
                    } else {
                        eprintln!("Error parsing depth message");
                    }
                } else {
                    eprintln!("Error receiving message");
                }
            }
        }
    });

    let bootstrap_future = tokio::spawn({
        let order_book = order_book.clone();
        let events = events.clone();

        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                let last_ws_update_id = {
                    let events = events.lock().await;
                    events.first().map(|event| event.last_update_id)
                };

                if let Some(last_ws_update_id) = last_ws_update_id {
                    let response = reqwest::get(format!("{}/depth?symbol={}", API_BASE, SYMBOL)).await;

                    if let Ok(response) = response {
                        if let Ok(snapshot) = response.json::<OrderBookSnapshot>().await {
                            if let Ok(snapshot_last_update_id) = snapshot.last_update_id.parse::<u64>() {
                                if snapshot_last_update_id >= last_ws_update_id {
                                    let mut events = events.lock().await;
                                    events.retain(|event| event.last_update_id > snapshot_last_update_id);

                                    let mut order_book = order_book.lock().await;
                                    order_book.asks = snapshot.asks.into_iter().collect();
                                    order_book.bids = snapshot.bids.into_iter().collect();
                                    order_book.last_update_id = snapshot_last_update_id;

                                    break;
                                }
                            } else {
                                eprintln!("Error parsing snapshot last update ID");
                            }
                        } else {
                            eprintln!("Error parsing order book snapshot");
                        }
                    } else {
                        eprintln!("Error fetching order book snapshot");
                    }
                }
            }
        }
    });

    let _ = tokio::try_join!(read_future, bootstrap_future);
}
