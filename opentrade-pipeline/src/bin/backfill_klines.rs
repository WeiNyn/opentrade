use binance_spot_connector_rust::market::klines::KlineInterval;
use chrono::NaiveDateTime;
use clap::Parser;
use env_logger::Builder;

// Backfill Klines Data
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct BackfillKlinesArgs {
    /// The symbol to backfill
    #[arg(short = 's', long)]
    symbol: String,

    #[arg(short = 'f', long)]
    back_seconds: Option<i64>,

    /// The start time in format "YYYY-MM-DD HH:MM:SS"
    #[arg(short = 'S', long)]
    start_time: Option<String>,

    /// The end time in format "YYYY-MM-DD HH:MM:SS", optional
    #[arg(short = 'E', long)]
    end_time: Option<String>,

    /// The interval for the klines (e.g., "1m", "5m", "1h", "1d")
    #[arg(short = 'i', long)]
    interval: String,

    /// The database connection string
    #[arg(
        short = 'd',
        long,
        default_value = "postgres://postgres:password@localhost/postgres"
    )]
    db_connection: String,
}

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
