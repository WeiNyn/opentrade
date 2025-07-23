use binance_spot_connector_rust::market::klines::KlineInterval;
use chrono::NaiveDateTime;
use clap::Parser;
use env_logger::Builder;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// Command line arguments for the kline data backfill binary.
///
/// This binary allows backfilling historical kline (candlestick) data from Binance
/// into a PostgreSQL database. It supports various time intervals and flexible
/// time range specifications.
///
/// # Time Range Options
///
/// You can specify the time range in two ways:
/// 1. Using `back_seconds`: Backfill data from N seconds ago to now
/// 2. Using `start_time` and optionally `end_time`: Specify exact time ranges
///
/// # Supported Intervals
///
/// - `1m`, `5m`, `15m`, `30m`: Minute intervals
/// - `1h`, `4h`: Hour intervals
/// - `1d`: Daily interval
///
/// # Examples
///
/// ```bash
/// # Backfill last 24 hours of BTCUSDT 1-minute data
/// cargo run --bin backfill_klines -- --symbol BTCUSDT --back-seconds 86400 --interval 1m
///
/// # Backfill specific date range
/// cargo run --bin backfill_klines -- --symbol ETHUSDT \
///   --start-time "2024-01-01 00:00:00" \
///   --end-time "2024-01-02 00:00:00" \
///   --interval 1h
///
/// # Backfill from specific time to now
/// cargo run --bin backfill_klines -- --symbol ADAUSDT \
///   --start-time "2024-01-01 00:00:00" \
///   --interval 1d
/// ```
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct BackfillKlinesArgs {
    /// The trading pair symbol to backfill (e.g., "BTCUSDT", "ETHUSDT")
    #[arg(short = 's', long)]
    symbol: String,

    /// Number of seconds to backfill from current time backwards.
    /// If provided, this takes precedence over start_time.
    /// Cannot be used together with start_time.
    #[arg(short = 'f', long)]
    back_seconds: Option<i64>,

    /// The start time in format "YYYY-MM-DD HH:MM:SS".
    /// If back_seconds is not provided, this field is required.
    /// All times are treated as UTC.
    #[arg(short = 'S', long)]
    start_time: Option<String>,

    /// The end time in format "YYYY-MM-DD HH:MM:SS".
    /// If not provided, backfill will run until the current time.
    /// All times are treated as UTC.
    #[arg(short = 'E', long)]
    end_time: Option<String>,

    /// The kline interval. Supported values:
    /// - Minutes: "1m", "5m", "15m", "30m"
    /// - Hours: "1h", "4h"
    /// - Days: "1d"
    #[arg(short = 'i', long)]
    interval: String,

    /// PostgreSQL database connection string.
    /// Format: "postgres://username:password@host:port/database"
    #[arg(
        short = 'd',
        long,
        default_value = "postgres://postgres:password@localhost/postgres"
    )]
    db_connection: String,
}

/// Main entry point for the kline backfill binary.
///
/// This binary performs historical kline data backfilling from Binance exchange
/// into a PostgreSQL database. It handles argument parsing, time range validation,
/// database connection setup, and orchestrates the backfill process.
///
/// # Process Flow
///
/// 1. Parse command line arguments
/// 2. Validate and process time range specifications
/// 3. Convert interval string to appropriate enum value
/// 4. Establish database connection
/// 5. Execute backfill operation using the opentrade-core library
/// 6. Report completion statistics
///
/// # Error Handling
///
/// The function will exit with an error message if:
/// - Neither start_time nor back_seconds is provided
/// - Time format parsing fails (must be "YYYY-MM-DD HH:MM:SS")
/// - Unsupported interval is specified
/// - Database connection fails
/// - Backfill operation encounters errors
///
/// # Rate Limiting
///
/// The backfill process includes built-in rate limiting (500ms delay between requests)
/// and batching (1000 klines per request) to comply with Binance API limits.
///
/// # Examples
///
/// ```bash
/// # Backfill last week of BTCUSDT hourly data
/// cargo run --bin backfill_klines -- -s BTCUSDT -f 604800 -i 1h
///
/// # Backfill specific date range for ETHUSDT daily data
/// cargo run --bin backfill_klines -- -s ETHUSDT \
///   -S "2024-01-01 00:00:00" -E "2024-01-31 23:59:59" -i 1d
/// ```
#[tokio::main]
pub async fn main() {
    Builder::from_default_env()
        .filter(None, log::LevelFilter::Info)
        .init();
    let args = BackfillKlinesArgs::parse();

    // If back_seconds is provided, calculate start time
    let start_time = if let Some(seconds) = args.back_seconds {
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::seconds(seconds);
        let start = start.naive_local();
        // Format start time as "YYYY-MM-DD HH:MM:SS"
        Some(start.format("%Y-%m-%d %H:%M:%S").to_string())
    } else {
        args.start_time.clone()
    };

    if start_time.is_none() && args.end_time.is_none() {
        eprintln!("Either --start-time or --end-time must be provided.");
        return;
    }

    let start_time = start_time.unwrap();

    // Here you would implement the logic to backfill klines data
    // For example, you might call a function that fetches the data
    // from an exchange and stores it in a database.

    match args.end_time.clone() {
        Some(end_time) => {
            log::info!(
                "Backfilling klines for symbol: {}, from {} to {}, interval: {}",
                args.symbol,
                &start_time,
                end_time,
                args.interval
            );
        }
        None => {
            log::info!(
                "Backfilling klines for symbol: {}, from {} to now, interval: {}",
                args.symbol,
                &start_time,
                args.interval
            );
        }
    }

    // Placeholder for actual backfill logic
    // backfill_klines(args.symbol, args.start_time, args.end_time, args.interval).await;
    let symbol = args.symbol;
    log::info!("{}", start_time);
    let start_time = NaiveDateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S")
        .expect("Failed to parse start time")
        .and_utc()
        .timestamp_millis() as u64;
    let end_time = args.end_time.map(|end_time| {
        NaiveDateTime::parse_from_str(&end_time, "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse end time")
            .and_utc()
            .timestamp_millis() as u64
    });
    let interval = match args.interval.as_str() {
        "1m" => KlineInterval::Minutes1,
        "5m" => KlineInterval::Minutes5,
        "15m" => KlineInterval::Minutes15,
        "30m" => KlineInterval::Minutes30,
        "1h" => KlineInterval::Hours1,
        "4h" => KlineInterval::Hours4,
        "1d" => KlineInterval::Days1,
        _ => {
            eprintln!("Unsupported interval: {}", args.interval);
            return;
        }
    };
    let limit: Option<u32> = Some(1000); // Limit for the number of klines to fetch
    let delay: Option<u64> = Some(500); // Delay in milliseconds for avoiding rate limits

    let db_connection = args.db_connection;
    let pool = sqlx::PgPool::connect(&db_connection)
        .await
        .expect("Failed to connect to the database");

    

    log::info!(
        "Starting backfill for symbol: {}, interval: {}, start_time: {}, end_time: {:?}, limit: {:?}, delay: {:?}",
        symbol,
        interval,
        start_time,
        end_time,
        limit,
        delay
    );
    let total_backfilled = opentrade_core::ingest::backfill::klines::kline_backfill_all(
        &pool, &symbol, interval, start_time, end_time, limit, delay,
    )
    .await
    .expect("Failed to backfill kline data");

    log::info!("Total backfilled klines: {}", total_backfilled);
}
