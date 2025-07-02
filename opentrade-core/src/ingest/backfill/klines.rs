use binance_spot_connector_rust::market::klines::KlineInterval;
use chrono::{DateTime, Utc};

use crate::data_source::rest::{extract_klines_from_string, get_kline_data};
use anyhow::Result;

/// Backfills kline data for a single symbol and time range.
///
/// # Arguments
///
/// * `pool` - The database connection pool.
/// * `symbol` - The trading symbol (e.g., "BTCUSDT").
/// * `interval` - The kline interval (e.g., `KlineInterval::Minutes1`).
/// * `start_time` - The start time for the backfill in milliseconds since the epoch.
/// * `end_time` - An optional end time for the backfill in milliseconds since the epoch.
/// * `limit` - An optional limit on the number of klines to fetch.
///
/// # Returns
///
/// A `Result` containing a tuple with the number of klines backfilled and the end time of the last kline,
/// or an error if the backfill fails.
pub async fn kline_backfill(
    pool: &sqlx::PgPool,
    symbol: &str,
    interval: KlineInterval,
    start_time: u64,
    end_time: Option<u64>,
    limit: Option<u32>,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let raw_data = get_kline_data(symbol, interval, start_time, end_time, limit)
        .await
        .expect("Failed to get kline data");
    let klines = extract_klines_from_string(&raw_data, symbol)
        .expect("Failed to extract klines from string");
    let data_size = klines.len();
    let last_data = klines.last().expect("No kline data found");
    log::info!(
        "Backfilled {} klines for symbol {} from {} to {}",
        data_size,
        symbol,
        DateTime::from_timestamp_millis(start_time as i64)
            .expect("Failed to convert start time to DateTime"),
        last_data.end_time
    );
    let last_end_time = last_data.end_time;

    for kline in klines {
        kline
            .upsert(&pool)
            .await
            .expect("Failed to insert kline data");
    }
    Ok((data_size, last_end_time.timestamp_millis() as usize))
}

/// Continuously backfills kline data for a given symbol until an optional end time is reached.
///
/// This function repeatedly calls `kline_backfill` to fetch and store kline data in batches.
///
/// # Arguments
///
/// * `pool` - The database connection pool.
/// * `symbols` - The trading symbol (e.g., "BTCUSDT").
/// * `interval` - The kline interval (e.g., `KlineInterval::Minutes1`).
/// * `start_time` - The start time for the backfill in milliseconds since the epoch.
/// * `end_time` - An optional end time for the backfill in milliseconds since the epoch. If `None`, it will backfill indefinitely.
/// * `limit` - An optional limit on the number of klines to fetch in each batch.
/// * `delay` - An optional delay in milliseconds between backfill requests. This can be used to avoid hitting API rate limits.
///
/// # Returns
///
/// A `Result` containing the total number of klines backfilled, or an error if the backfill fails.
pub async fn kline_backfill_all(
    pool: &sqlx::PgPool,
    symbols: &str,
    interval: KlineInterval,
    start_time: u64,
    end_time: Option<u64>,
    limit: Option<u32>,
    delay: Option<u64>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut current_time = start_time;
    let mut total_data_size = 0;

    while current_time < end_time.unwrap_or(u64::MAX)
        && current_time <= Utc::now().timestamp_millis() as u64
    {
        let (data_size, last_end_time) =
            kline_backfill(pool, symbols, interval, current_time, None, limit).await?;

        total_data_size += data_size;
        current_time = last_end_time as u64 + 1;
        if let Some(d) = delay {
            tokio::time::sleep(tokio::time::Duration::from_millis(d)).await;
        }
    }

    Ok(total_data_size)
}
