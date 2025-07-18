use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::BigDecimal as Decimal;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerdableKlineData {
    #[serde(rename = "t")]
    pub start_time: u64,
    #[serde(rename = "T")]
    pub end_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "f")]
    pub first_trade_id: i32,
    #[serde(rename = "L")]
    pub last_trade_id: i32,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "n")]
    pub trade_count: u64,
    #[serde(rename = "q")]
    pub quote_volume: String,
}

impl From<SerdableKlineData> for KlineData {
    fn from(data: SerdableKlineData) -> Self {
        KlineData {
            start_time: DateTime::from_timestamp_millis(data.start_time as i64).unwrap(),
            end_time: DateTime::from_timestamp_millis(data.end_time as i64).unwrap(),
            symbol: data.symbol,
            interval: data.interval,
            first_trade_id: data.first_trade_id,
            last_trade_id: data.last_trade_id,
            open: data.open.parse::<Decimal>().unwrap(),
            high: data.high.parse::<Decimal>().unwrap(),
            low: data.low.parse::<Decimal>().unwrap(),
            close: data.close.parse::<Decimal>().unwrap(),
            volume: data.volume.parse::<Decimal>().unwrap(),
            trade_count: Some(data.trade_count as i32),
            quote_volume: Some(data.quote_volume.parse::<Decimal>().unwrap()),
            created_at: None,
            update_at: None,
        }
    }
}

impl From<KlineData> for SerdableKlineData {
    fn from(data: KlineData) -> Self {
        SerdableKlineData {
            start_time: data.start_time.timestamp_millis() as u64,
            end_time: data.end_time.timestamp_millis() as u64,
            symbol: data.symbol,
            interval: data.interval,
            first_trade_id: data.first_trade_id,
            last_trade_id: data.last_trade_id,
            open: data.open.to_string(),
            close: data.close.to_string(),
            high: data.high.to_string(),
            low: data.low.to_string(),
            volume: data.volume.to_string(),
            trade_count: data.trade_count.unwrap_or(0) as u64,
            quote_volume: data.quote_volume.unwrap_or_default().to_string(),
        }
    }
}

/// Represents a single Kline (candlestick) data point for a specific symbol and interval.
#[derive(FromRow, Debug, Clone)]
pub struct KlineData {
    /// The start time of the Kline interval.
    pub start_time: DateTime<Utc>,
    /// The end time of the Kline interval.
    pub end_time: DateTime<Utc>,
    /// The trading symbol (e.g., "BTCUSDT").
    pub symbol: String,
    /// The interval of the Kline data (e.g., "1m", "1h").
    pub interval: String,
    /// The ID of the first trade in this Kline interval.
    pub first_trade_id: i32,
    /// The ID of the last trade in this Kline interval.
    pub last_trade_id: i32,
    /// The opening price for the interval.
    pub open: Decimal,
    /// The highest price reached during the interval.
    pub high: Decimal,
    /// The lowest price reached during the interval.
    pub low: Decimal,
    /// The closing price for the interval.
    pub close: Decimal,
    /// The total volume of the base asset traded during the interval.
    pub volume: Decimal,
    /// The total number of trades during the interval.
    pub trade_count: Option<i32>,
    /// The total volume of the quote asset traded during the interval.
    pub quote_volume: Option<Decimal>,
    /// The timestamp when this record was created in the database.
    pub created_at: Option<DateTime<Utc>>,
    /// The timestamp when this record was last updated in the database.
    pub update_at: Option<DateTime<Utc>>,
}

impl KlineData {
    /// Creates a new `KlineData` instance.
    ///
    /// # Arguments
    ///
    /// * `start_time` - The start time of the Kline interval as a Unix timestamp.
    /// * `end_time` - The end time of the Kline interval as a Unix timestamp.
    /// * `symbol` - The trading symbol.
    /// * `interval` - The Kline interval.
    /// * `first_trade_id` - The ID of the first trade.
    /// * `last_trade_id` - The ID of the last trade.
    /// * `open` - The opening price.
    /// * `high` - The highest price.
    /// * `low` - The lowest price.
    /// * `close` - The closing price.
    /// * `volume` - The trading volume.
    /// * `trade_count` - The number of trades.
    /// * `quote_volume` - The quote asset volume.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start_time: &u64,
        end_time: &u64,
        symbol: &str,
        interval: &str,
        first_trade_id: i32,
        last_trade_id: i32,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
        trade_count: Option<i32>,
        quote_volume: Option<Decimal>,
    ) -> Self {
        KlineData {
            start_time: DateTime::from_timestamp_millis(*start_time as i64).unwrap(),
            end_time: DateTime::from_timestamp_millis(*end_time as i64).unwrap(),
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            first_trade_id,
            last_trade_id,
            open,
            high,
            low,
            close,
            volume,
            trade_count,
            quote_volume,
            created_at: None,
            update_at: None,
        }
    }

    /// Inserts a new `KlineData` record into the database.
    ///
    /// # Arguments
    ///
    /// * `pool` - The database connection pool.
    pub async fn add(&self, pool: &sqlx::PgPool) -> Result<Self, sqlx::Error> {
        let kline = sqlx::query_as!(
            KlineData,
            r#"
            INSERT INTO kline_data (
                start_time, end_time, symbol, interval, first_trade_id, last_trade_id,
                open, high, low, close, volume, trade_count, quote_volume
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
            self.start_time,
            self.end_time,
            self.symbol,
            self.interval,
            self.first_trade_id,
            self.last_trade_id,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
            self.trade_count,
            self.quote_volume
        )
        .fetch_one(pool)
        .await?;
        Ok(kline)
    }

    /// Retrieves a `KlineData` record from the database.
    ///
    /// # Arguments
    ///
    /// * `pool` - The database connection pool.
    /// * `start_time` - The start time of the Kline interval.
    /// * `end_time` - The end time of the Kline interval.
    /// * `symbol` - The trading symbol.
    /// * `interval` - The Kline interval.
    pub async fn get(
        pool: &sqlx::PgPool,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        symbol: &str,
        interval: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let kline = sqlx::query_as!(
            KlineData,
            r#"
            SELECT * FROM kline_data
            WHERE start_time > $1 AND end_time <= $2 AND symbol = $3 AND interval = $4
            "#,
            start_time,
            end_time,
            symbol,
            interval
        )
        .fetch_optional(pool)
        .await?;
        Ok(kline)
    }

    /// Updates an existing `KlineData` record in the database.
    ///
    /// # Arguments
    ///
    /// * `pool` - The database connection pool.
    pub async fn update(&self, pool: &sqlx::PgPool) -> Result<Self, sqlx::Error> {
        let kline = sqlx::query_as!(
            KlineData,
            r#"
            UPDATE kline_data
            SET
                end_time = $1,
                first_trade_id = $2,
                last_trade_id = $3,
                open = $4,
                high = $5,
                low = $6,
                close = $7,
                volume = $8,
                trade_count = $9,
                quote_volume = $10,
                update_at = NOW()
            WHERE start_time = $11 AND symbol = $12 AND interval = $13
            RETURNING *
            "#,
            self.end_time,
            self.first_trade_id,
            self.last_trade_id,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
            self.trade_count,
            self.quote_volume,
            self.start_time,
            self.symbol,
            self.interval
        )
        .fetch_one(pool)
        .await?;
        Ok(kline)
    }

    /// Inserts a new `KlineData` record or updates an existing one if a conflict occurs.
    ///
    /// A conflict is determined by the unique constraint on `(start_time, symbol, interval)`.
    ///
    /// # Arguments
    ///
    /// * `pool` - The database connection pool.
    pub async fn upsert(&self, pool: &sqlx::PgPool) -> Result<Self, sqlx::Error> {
        // Upsert by using on conflict clause
        let kline = sqlx::query_as!(
            KlineData,
            r#"
            INSERT INTO kline_data (
                start_time, end_time, symbol, interval, first_trade_id, last_trade_id,
                open, high, low, close, volume, trade_count, quote_volume
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (start_time, symbol, interval) DO UPDATE
            SET
                end_time = EXCLUDED.end_time,
                first_trade_id = EXCLUDED.first_trade_id,
                last_trade_id = EXCLUDED.last_trade_id,
                open = EXCLUDED.open,
                high = EXCLUDED.high,
                low = EXCLUDED.low,
                close = EXCLUDED.close,
                volume = EXCLUDED.volume,
                trade_count = EXCLUDED.trade_count,
                quote_volume = EXCLUDED.quote_volume,
                update_at = NOW()
            RETURNING *
            "#,
            self.start_time,
            self.end_time,
            self.symbol,
            self.interval,
            self.first_trade_id,
            self.last_trade_id,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
            self.trade_count,
            self.quote_volume
        )
        .fetch_one(pool)
        .await?;
        Ok(kline)
    }
}
