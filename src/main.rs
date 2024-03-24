use crossterm::terminate::Clear;
use crossterm::terminal::ClearType;
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

#[derive (Debug, Deserialize)]
pub struct OrderBook {
    pub asks : BtreeMap<Decimal, Decimal>,
    pub bids : BtreeMap<Decimal, Deciaml>,
    pub last_update_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookSnapshot {
     pub asks: Vec<(Decimal, Decimal)>,
     pub bids: Vec<(Decimal, Decimal)>,
     pub last_update_id: String,
}
