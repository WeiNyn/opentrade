use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use std::fmt::Debug;

#[derive(FromRow, Debug)]
pub struct KlineData {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub symbol: String,
    pub interval: String,
    pub first_trade_id: i32,
    pub last_trade_id: i32,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trade_count: Option<i32>,
    pub quote_volume: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
    pub update_at: Option<DateTime<Utc>>,
}