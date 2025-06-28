use chrono::{DateTime, Utc};
use sqlx::types::BigDecimal as Decimal;
use sqlx::FromRow;
use std::fmt::Debug;

#[derive(FromRow, Debug, Clone)]
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

impl KlineData {
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
            start_time: DateTime::<Utc>::from_timestamp(*start_time as i64, 0).unwrap(),
            end_time: DateTime::<Utc>::from_timestamp(*end_time as i64, 0).unwrap(),
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

    pub async fn add(
        &self,
        pool: &sqlx::PgPool,
    ) -> Result<Self, sqlx::Error> {
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

    pub async fn update(
        &self,
        pool: &sqlx::PgPool,
    ) -> Result<Self, sqlx::Error> {
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
}