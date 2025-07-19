use anyhow::Result;
use async_trait::async_trait;
use binance_spot_connector_rust::market::klines::KlineInterval;
use opentrade_core::{
    data_source::websocket::{KlineStreaming, MessageHandler},
    models::{KlineData, SerdableKlineData},
};
use sqlx::PgPool;

/// A message handler that prints incoming kline data to the console.
///
/// This handler implements the [`MessageHandler`] trait to process streaming
/// kline data from the Binance WebSocket API. It provides a simple logging
/// mechanism that prints each received kline message and tracks the total
/// number of messages processed.
///
/// # Purpose
///
/// - Debug and monitoring of incoming kline data streams
/// - Verification that the WebSocket connection is receiving data
/// - Basic statistics tracking for message throughput
///
/// # Behavior
///
/// - Logs each individual kline message at INFO level
/// - Prints a summary message every 10 processed messages
/// - Maintains an internal counter of processed messages
///
/// # Example Usage
///
/// ```rust
/// use opentrade_core::data_source::websocket::KlineStreaming;
/// use binance_spot_connector_rust::market::klines::KlineInterval;
///
/// // Create a new print handler
/// let print_handler = PrintKlineHandler { count: 0 };
///
/// // Add to a kline streaming instance
/// let mut kline_streaming = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
/// kline_streaming.add_callback(print_handler);
/// ```
pub struct PrintKlineHandler {
    /// Counter tracking the number of kline messages processed
    count: usize,
}

#[async_trait]
impl MessageHandler<SerdableKlineData> for PrintKlineHandler {
    async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
        log::info!("Received Kline data: {:?}", message);
        self.count += 1;
        if self.count % 10 == 0 {
            log::info!("Processed {} Kline messages", self.count);
        }
        Ok(())
    }
}

/// A message handler that persists incoming kline data to a PostgreSQL database.
///
/// This handler implements the [`MessageHandler`] trait to process streaming
/// kline data and store it in a database using upsert operations. It converts
/// the serializable kline data format to the internal [`KlineData`] model
/// and persists it to the configured database.
///
/// # Purpose
///
/// - Real-time persistence of streaming kline data
/// - Data deduplication through upsert operations
/// - Integration with the opentrade data storage layer
///
/// # Database Operations
///
/// - Converts [`SerdableKlineData`] to [`KlineData`] model
/// - Performs upsert operations to handle duplicate data gracefully
/// - Logs successful database operations for monitoring
///
/// # Error Handling
///
/// Database errors are handled by panicking with an error message.
/// In production, consider implementing more robust error handling
/// with retry logic and graceful degradation.
///
/// # Example Usage
///
/// ```rust
/// use sqlx::PgPool;
/// use opentrade_core::data_source::websocket::KlineStreaming;
/// use binance_spot_connector_rust::market::klines::KlineInterval;
///
/// // Create database connection
/// let pool = PgPool::connect("postgres://user:pass@localhost/db").await?;
///
/// // Create upsert handler
/// let upsert_handler = UpsertKlineHandler::new(pool);
///
/// // Add to streaming instance
/// let mut kline_streaming = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
/// kline_streaming.add_callback(upsert_handler);
/// ```
pub struct UpsertKlineHandler {
    /// Database connection pool for executing upsert operations
    pool: sqlx::PgPool,
}

impl UpsertKlineHandler {
    /// Creates a new [`UpsertKlineHandler`] with the provided database connection pool.
    ///
    /// # Parameters
    ///
    /// * `pool` - A PostgreSQL connection pool ([`sqlx::PgPool`]) that will be used
    ///   for executing database upsert operations. The pool should be properly
    ///   configured and tested for connectivity before being passed to this constructor.
    ///
    /// # Returns
    ///
    /// Returns a new instance of [`UpsertKlineHandler`] ready to process kline messages.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sqlx::PgPool;
    ///
    /// // Establish database connection
    /// let pool = PgPool::connect("postgres://user:password@localhost/trading_db").await?;
    ///
    /// // Create the handler
    /// let handler = UpsertKlineHandler::new(pool);
    /// ```
    ///
    /// # Database Requirements
    ///
    /// The database connection pool must have access to the kline data tables
    /// as defined in the opentrade schema. Ensure that:
    /// - The database connection is active and valid
    /// - The required tables exist (typically created via migrations)
    /// - The connection user has INSERT/UPDATE permissions
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessageHandler<SerdableKlineData> for UpsertKlineHandler {
    async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
        log::info!("Upserting Kline data: {:?}", message);
        let kline_data = KlineData::from(message.clone());
        kline_data
            .upsert(&self.pool)
            .await
            .expect("Failed to upsert kline data");
        log::info!("Kline data upserted successfully");
        println!("Kline data upserted: {:?}", kline_data);
        Ok(())
    }
}

/// Main entry point for the real-time kline data streaming binary.
///
/// This binary establishes a WebSocket connection to Binance to stream live
/// kline (candlestick) data for a specific trading pair and processes the
/// incoming data using multiple message handlers.
///
/// # Process Flow
///
/// 1. Create a [`KlineStreaming`] instance for BTCUSDT with 1-minute intervals
/// 2. Add a [`PrintKlineHandler`] for console logging and monitoring
/// 3. Establish a PostgreSQL database connection
/// 4. Add an [`UpsertKlineHandler`] for database persistence
/// 5. Subscribe to the WebSocket stream
/// 6. Begin listening for incoming messages indefinitely
///
/// # Message Handlers
///
/// The binary uses two message handlers:
/// - **PrintKlineHandler**: Logs each message and provides throughput statistics
/// - **UpsertKlineHandler**: Persists kline data to the PostgreSQL database
///
/// # Configuration
///
/// Currently uses hardcoded values:
/// - **Symbol**: BTCUSDT (Bitcoin/Tether trading pair)
/// - **Interval**: 1 minute
/// - **Database**: Local PostgreSQL with default credentials
///
/// # Error Handling
///
/// The application will panic and exit if:
/// - WebSocket connection to Binance fails
/// - Database connection cannot be established
/// - Subscription to kline stream fails
/// - Critical errors occur during message processing
///
/// # Usage
///
/// ```bash
/// # Start the streaming kline data processor
/// cargo run --bin streaming_klines
/// ```
///
/// # Monitoring
///
/// The application provides:
/// - Console output for each processed message
/// - Periodic statistics (every 10 messages)
/// - Database persistence confirmation logs
/// - Error messages for any failures
///
/// # Production Considerations
///
/// For production deployment, consider:
/// - Making symbol and interval configurable via CLI arguments
/// - Using environment variables for database configuration
/// - Implementing graceful shutdown handling
/// - Adding reconnection logic for WebSocket failures
/// - Implementing more robust error handling and recovery
#[tokio::main]
async fn main() {
    let mut kline_streaming = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1)
        .await
        .unwrap();
    kline_streaming.add_callback(PrintKlineHandler { count: 0 });

    let pool = PgPool::connect("postgres://postgres:password@localhost/postgres")
        .await
        .expect("Failed to connect to database");
    kline_streaming.add_callback(UpsertKlineHandler::new(pool));

    kline_streaming
        .subscribe()
        .await
        .expect("Failed to subscribe to Kline data");

    kline_streaming
        .listen()
        .await
        .expect("Failed to listen for Kline data");
}
