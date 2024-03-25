use crossterm::terminal::ClearType;
use crossterm::terminate::Clear;
use crossterm::{
    execute,
    style::{Color, Print, SetForegroundColor},
};
use futures::{SkinExt, StreamExt};
use rust_decimal::Decimal;
use serde::Derserialize;
use serde::Serialize;
use std::collection::BtreeMap;
use std::io::stdout;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

const API_BASE: &str = "";
const WSS_URL: &str = "";
const SYMBOL: &str = "";

type Price = Decimal;
type Quantity = Decimal;

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    pub asks: BTreeMap<Decimal, Decimal>,
    pub bids: BTreeMap<Decimal, Deciaml>,
    pub last_update_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookSnapshot {
    pub asks: Vec<(Decimal, Decimal)>,
    pub bids: Vec<(Decimal, Decimal)>,
    pub last_update_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DepthMessage {
    data: DepthEvent,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DepthEvent {
    /// The type of event.
    #[serde(rename(deserialize = "e"))]
    event_type: String,
    /// The time the event was emitted.
    #[serde(rename(deserialize = "E"))]
    event_time: i64,
    /// The market symbol.
    #[serde(rename(deserialize = "s"))]
    symbol: String,
    /// Changes to the depth on the asks side.
    #[serde(rename(deserialize = "a"))]
    pub asks: Vec<(Price, Quantity)>,
    /// Changes to the depth on the bids side.
    #[serde(rename(deserialize = "b"))]
    pub bids: Vec<(Price, Quantity)>,
    /// The first id of the aggregated events.
    #[serde(rename(deserialize = "U"))]
    pub first_update_id: u64,
    /// The last id of the aggregated events.
    #[serde(rename(deserialize = "u"))]
    pub last_update_id: u64,
}

#[tokio::main]
async fn main() {
    let (stream, _) = connect_async(WSS_URL).await.expect("Failed to connect");
    let (mut write, read) = stream.split();

    let subscribe_message = serde_json::json!({
        "method": "SUBSCRIBE",
        "params": [format!("{SYMBOL}@depth")]
    })
    .to_string();

    write.send(Message::text(subscribe_message)).await.unwrap();

    let order_book = Arc::new(Mutex::new(Orderbook {
        asks: BTreeMap::new(),
        bids: BTreeMap::new(),
        last_update_id: 0,
    }));
    let events = Arc::new(Mutex::new(Vec::new()));

    let read_future = read.for_each(|message| async {
        let data = message.unwrap().into_data();
        let Ok(message) = serde_json::from_slice::<DepthMessage>(&data) else {
            println!("Error parsing message ");
            return;
        };
        events.lock().await.push(message.data);
    });
}
