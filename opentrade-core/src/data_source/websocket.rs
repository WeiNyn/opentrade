
use crate::models::{KlineData, SerdableKlineData};
use anyhow::{Context, Result};
use async_trait::async_trait;
use binance_spot_connector_rust::{
    market,
    market_stream::kline::KlineStream,
    tokio_tungstenite::{BinanceWebSocketClient, WebSocketState},
};
use futures_util::{StreamExt};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::types::BigDecimal;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

/// WebSocket message payload containing Kline stream data.
///
/// This struct represents the top-level message structure received from Binance
/// WebSocket streams. Each message contains metadata about the stream and the
/// actual Kline data payload.
///
/// # Fields
///
/// - `stream`: The stream identifier (e.g., "btcusdt@kline_1m")
/// - `data`: The actual Kline data contained in the message
///
/// # Example
///
/// ```rust
/// use opentrade_core::data_source::websocket::Payload;
/// use serde_json;
///
/// let json = r#"{"stream":"btcusdt@kline_1m","data":{...}}"#;
/// let payload: Payload = serde_json::from_str(json)?;
///
/// println!("Stream: {}", payload.stream);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub stream: String,
    pub data: KlinePayloadData,
}

/// Container for Kline event data within a WebSocket message payload.
///
/// This struct wraps the actual Kline details with metadata about the WebSocket event.
/// It follows the Binance WebSocket API format where Kline data is nested within
/// an event structure that provides context about the message type and timing.
///
/// # Fields
///
/// - `e`: Event type (always "kline" for Kline events)
/// - `E`: Event time (Unix timestamp in milliseconds when the event was generated)
/// - `s`: Symbol (trading pair identifier, e.g., "BTCUSDT")
/// - `k`: The actual Kline data details
///
/// # Example
///
/// ```rust
/// use opentrade_core::data_source::websocket::KlinePayloadData;
/// use serde_json;
///
/// let json = r#"{"e":"kline","E":1640995200000,"s":"BTCUSDT","k":{...}}"#;
/// let kline_payload: KlinePayloadData = serde_json::from_str(json)?;
///
/// assert_eq!(kline_payload.event_type, "kline");
/// assert_eq!(kline_payload.symbol, "BTCUSDT");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlinePayloadData {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "k")]
    pub kline: KlineDetails,
}

/// Detailed Kline (candlestick) data structure from WebSocket streams.
///
/// This struct contains all the specific data points for a single Kline interval
/// as received from Binance WebSocket streams. It includes comprehensive market
/// data such as OHLCV (Open, High, Low, Close, Volume) information along with
/// additional metadata about trades and market activity.
///
/// # Fields
///
/// The field names use single-letter aliases matching Binance API conventions:
/// - `t`: Start time of the Kline interval (Unix timestamp in milliseconds)
/// - `T`: End time of the Kline interval (Unix timestamp in milliseconds)
/// - `s`: Symbol (trading pair, e.g., "BTCUSDT")
/// - `i`: Interval (e.g., "1m", "5m", "1h", "1d")
/// - `f`: First trade ID in this Kline interval
/// - `L`: Last trade ID in this Kline interval
/// - `o`: Opening price (as string to preserve precision)
/// - `c`: Closing price (as string to preserve precision)
/// - `h`: Highest price during the interval (as string)
/// - `l`: Lowest price during the interval (as string)
/// - `v`: Volume of the base asset traded (as string)
/// - `n`: Number of trades during the interval
/// - `x`: Whether this Kline is closed (final) or still updating
/// - `q`: Volume of the quote asset traded (as string)
/// - `V`: Volume of base asset purchased by taker orders (as string)
/// - `Q`: Volume of quote asset purchased by taker orders (as string)
/// - `B`: Unused field (ignored in processing)
///
/// # Example
///
/// ```rust
/// use opentrade_core::data_source::websocket::KlineDetails;
/// use serde_json;
///
/// let json = r#"{
///     "t": 1640995200000,
///     "T": 1640995259999,
///     "s": "BTCUSDT",
///     "i": "1m",
///     "f": 123456,
///     "L": 123500,
///     "o": "50000.00",
///     "c": "50100.00",
///     "h": "50200.00",
///     "l": "49900.00",
///     "v": "10.5",
///     "n": 45,
///     "x": true,
///     "q": "525000.00",
///     "V": "6.2",
///     "Q": "310000.00",
///     "B": "0"
/// }"#;
///
/// let kline: KlineDetails = serde_json::from_str(json)?;
/// assert_eq!(kline.symbol, "BTCUSDT");
/// assert_eq!(kline.interval, "1m");
/// assert!(kline.is_final);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlineDetails {
    #[serde(rename = "t")]
    pub start_time: u64,

    #[serde(rename = "T")]
    pub end_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "f")]
    pub first_trade_id: u64,

    #[serde(rename = "L")]
    pub last_trade_id: u64,

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

    #[serde(rename = "x")]
    pub is_final: bool,

    #[serde(rename = "q")]
    pub quote_volume: String,

    #[serde(rename = "V")]
    pub taker_buy_base_volume: String,

    #[serde(rename = "Q")]
    pub taker_buy_quote_volume: String,

    #[serde(rename = "B")]
    pub ignore: String,
}

impl Payload {
    /// Converts the WebSocket payload into a [`KlineData`] instance for database storage.
    ///
    /// This method transforms the string-based WebSocket data into a strongly-typed
    /// database model with proper decimal precision for financial calculations.
    ///
    /// # Returns
    ///
    /// - `Ok(KlineData)` - Successfully converted Kline data ready for database operations
    /// - `Err(anyhow::Error)` - Conversion failed due to invalid numeric strings
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Any price or volume string cannot be parsed as a BigDecimal
    /// - The error context will include the specific value that failed to parse
    ///
    /// # Example
    ///
    /// ```rust
    /// use opentrade_core::data_source::websocket::Payload;
    /// # use anyhow::Result;
    ///
    /// fn process_websocket_message(payload: Payload) -> Result<()> {
    ///     let kline_data = payload.to_kline_data()?;
    ///     // Now ready for database insertion
    ///     // kline_data.upsert(&pool).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn to_kline_data(&self) -> Result<KlineData> {
        let kline = &self.data.kline;

        fn parse_decimal_string(s: &str) -> Result<BigDecimal> {
            s.parse::<BigDecimal>()
                .context(format!("Failed to parse decimal string: {s}"))
        }

        let quote_volume = parse_decimal_string(&kline.quote_volume)?;

        Ok(KlineData::new(
            &kline.start_time,
            &kline.end_time,
            &kline.symbol,
            &kline.interval,
            kline.first_trade_id as i32,
            kline.last_trade_id as i32,
            parse_decimal_string(&kline.open)?,
            parse_decimal_string(&kline.high)?,
            parse_decimal_string(&kline.low)?,
            parse_decimal_string(&kline.close)?,
            parse_decimal_string(&kline.volume)?,
            Some(kline.trade_count as i32),
            Some(quote_volume),
        ))
    }

    /// Converts the WebSocket payload into a [`SerdableKlineData`] instance for serialization.
    ///
    /// This method transforms the WebSocket data into a serializable format that maintains
    /// the string-based representation suitable for JSON serialization and API responses.
    /// Unlike `to_kline_data()`, this method preserves the original string format without
    /// decimal conversion, making it faster and suitable for pass-through scenarios.
    ///
    /// # Returns
    ///
    /// - `Ok(SerdableKlineData)` - Successfully converted serializable Kline data
    /// - `Err(anyhow::Error)` - Conversion failed (unlikely as no parsing is performed)
    ///
    /// # Example
    ///
    /// ```rust
    /// use opentrade_core::data_source::websocket::Payload;
    /// use serde_json;
    /// # use anyhow::Result;
    ///
    /// fn process_for_api_response(payload: Payload) -> Result<String> {
    ///     let serdable_data = payload.to_serializable_kline_data()?;
    ///     let json = serde_json::to_string(&serdable_data)?;
    ///     Ok(json)
    /// }
    /// ```
    pub fn to_serializable_kline_data(&self) -> Result<SerdableKlineData> {
        let kline = &self.data.kline;

        Ok(SerdableKlineData {
            start_time: kline.start_time,
            end_time: kline.end_time,
            symbol: kline.symbol.clone(),
            interval: kline.interval.clone(),
            first_trade_id: kline.first_trade_id as i32,
            last_trade_id: kline.last_trade_id as i32,
            open: kline.open.clone(),
            high: kline.high.clone(),
            low: kline.low.clone(),
            close: kline.close.clone(),
            volume: kline.volume.clone(),
            trade_count: kline.trade_count,
            quote_volume: kline.quote_volume.clone(),
        })
    }
}

/// Configuration for a Kline data subscription.
///
/// This struct encapsulates the parameters needed to subscribe to a specific
/// Kline (candlestick) data stream from a cryptocurrency exchange. It defines
/// both the trading pair symbol and the time interval for data aggregation.
///
/// # Fields
///
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT", "ETHUSDT")
/// - `interval`: The time interval for Kline aggregation (e.g., 1 minute, 1 hour)
///
/// # Usage
///
/// This struct is typically used internally by the streaming infrastructure
/// to configure WebSocket subscriptions. Most applications will use the
/// higher-level [`KlineStreaming`] interface instead of working with
/// subscriptions directly.
///
/// # Example
///
/// ```rust
/// use opentrade_core::data_source::websocket::KlineSubscription;
/// use binance_spot_connector_rust::market::klines::KlineInterval;
///
/// let subscription = KlineSubscription {
///     symbol: "BTCUSDT".to_string(),
///     interval: KlineInterval::Minutes1,
/// };
/// ```
pub struct KlineSubscription {
    /// The trading pair symbol for the subscription (e.g., "BTCUSDT")
    pub symbol: String,
    /// The time interval for Kline data aggregation
    pub interval: market::klines::KlineInterval,
}

/// High-level WebSocket client for streaming Kline (candlestick) data from Binance.
///
/// `KlineStreaming` provides a convenient interface for establishing WebSocket connections
/// to Binance and receiving real-time Kline data for a specific trading pair and interval.
/// It supports configurable message handlers for processing incoming data and manages
/// the underlying WebSocket connection lifecycle.
///
/// # Features
///
/// - **Automatic Connection Management**: Handles WebSocket connection establishment
/// - **Type-safe Data Processing**: Converts raw messages to structured Kline data
/// - **Configurable Callbacks**: Supports multiple message handlers for flexible processing
/// - **Error Handling**: Provides comprehensive error reporting for connection and parsing issues
/// - **Asynchronous Processing**: Built on Tokio for high-performance async operations
///
/// # Fields
///
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT")
/// - `interval`: The Kline interval for data aggregation
/// - `state`: Internal WebSocket connection state
/// - `callbacks`: Collection of message handlers for processing incoming data
///
/// # Example
///
/// ```rust,no_run
/// use opentrade_core::data_source::websocket::KlineStreaming;
/// use binance_spot_connector_rust::market::klines::KlineInterval;
/// # use anyhow::Result;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // Create a new Kline streaming client
///     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
///
///     // Subscribe to the stream
///     stream.subscribe().await?;
///
///     // Process incoming messages
///     while let Some(result) = stream.next().await? {
///         match result {
///             Ok(kline_data) => {
///                 println!("Received Kline: {:?}", kline_data);
///             }
///             Err(e) => {
///                 eprintln!("Error processing Kline: {}", e);
///             }
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub struct KlineStreaming {
    pub symbol: String,
    pub interval: market::klines::KlineInterval,
    pub state: WebSocketState<MaybeTlsStream<TcpStream>>,
    pub callbacks: Vec<Box<dyn MessageHandler<SerdableKlineData>>>,
}

impl KlineStreaming {
    /// Creates a new [`KlineStreaming`] instance for the specified symbol and interval.
    ///
    /// This constructor establishes a WebSocket connection to Binance and prepares
    /// the client for streaming Kline data. The connection is established but not
    /// yet subscribed to any streams.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT", "ETHUSDT")
    /// * `interval` - The Kline interval for data aggregation (e.g., Minutes1, Hours1)
    ///
    /// # Returns
    ///
    /// - `Ok(KlineStreaming)` - Successfully created streaming client
    /// - `Err(anyhow::Error)` - Failed to establish WebSocket connection
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The WebSocket connection to Binance cannot be established
    /// - Network connectivity issues prevent connection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opentrade_core::data_source::websocket::KlineStreaming;
    /// use binance_spot_connector_rust::market::klines::KlineInterval;
    /// # use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
    ///     println!("WebSocket client created successfully");
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(symbol: &str, interval: market::klines::KlineInterval) -> Result<Self> {
        let (state, _) = BinanceWebSocketClient::connect_async_default().await?;

        Ok(Self {
            symbol: symbol.to_string(),
            interval,
            state,
            callbacks: Vec::new(),
        })
    }

    /// Adds a message handler callback for processing incoming Kline data.
    ///
    /// Message handlers implement the [`MessageHandler`] trait and are called
    /// sequentially for each received Kline message. Multiple handlers can be
    /// registered to perform different processing tasks (e.g., database storage,
    /// logging, real-time analysis).
    ///
    /// # Arguments
    ///
    /// * `handler` - A type implementing [`MessageHandler<SerdableKlineData>`]
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opentrade_core::data_source::websocket::{KlineStreaming, MessageHandler};
    /// use opentrade_core::models::SerdableKlineData;
    /// use binance_spot_connector_rust::market::klines::KlineInterval;
    /// use async_trait::async_trait;
    /// use anyhow::Result;
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait]
    /// impl MessageHandler<SerdableKlineData> for MyHandler {
    ///     async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
    ///         println!("Processing: {}", message.symbol);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
    ///     stream.add_callback(MyHandler);
    ///     Ok(())
    /// }
    /// ```
    pub fn add_callback<H: MessageHandler<SerdableKlineData> + 'static>(&mut self, handler: H) {
        self.callbacks.push(Box::new(handler));
    }

    /// Subscribes to the Kline data stream for the configured symbol and interval.
    ///
    /// This method initiates the WebSocket subscription to receive real-time Kline data
    /// for the trading pair and interval specified during [`KlineStreaming::new()`].
    /// After successful subscription, incoming messages can be processed using [`next()`]
    /// or [`listen()`].
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Subscription was successful and the stream is ready to receive data
    /// - `Err(anyhow::Error)` - Subscription failed due to WebSocket or network issues
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The WebSocket connection is not established or has been closed
    /// - Network connectivity issues prevent the subscription request
    /// - The exchange rejects the subscription (invalid symbol/interval)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opentrade_core::data_source::websocket::KlineStreaming;
    /// use binance_spot_connector_rust::market::klines::KlineInterval;
    /// # use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
    ///
    ///     // Subscribe to the stream
    ///     stream.subscribe().await?;
    ///
    ///     println!("Successfully subscribed to BTCUSDT 1m kline stream");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method must be called before attempting to receive data with [`next()`] or [`listen()`].
    /// Multiple calls to subscribe will replace the previous subscription.
    pub async fn subscribe(&mut self) -> Result<()> {
        self.state
            .subscribe(vec![&KlineStream::new(&self.symbol, self.interval).into()])
            .await;
        Ok(())
    }

    /// Retrieves the next Kline message from the WebSocket stream.
    ///
    /// This method provides low-level access to individual WebSocket messages, allowing
    /// for manual processing and error handling. It's suitable for applications that need
    /// fine-grained control over message processing or custom error handling logic.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Ok(SerdableKlineData)))` - Successfully received and parsed Kline data
    /// - `Ok(Some(Err(anyhow::Error)))` - Received message but parsing failed
    /// - `Ok(None)` - WebSocket stream has ended (connection closed)
    /// - `Err(anyhow::Error)` - Critical error in WebSocket communication
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The WebSocket connection encounters a critical failure
    /// - Binary data cannot be converted to UTF-8 string
    /// - JSON deserialization fails for the WebSocket message structure
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opentrade_core::data_source::websocket::KlineStreaming;
    /// use binance_spot_connector_rust::market::klines::KlineInterval;
    /// # use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
    ///     stream.subscribe().await?;
    ///
    ///     // Process messages individually
    ///     while let Some(result) = stream.next().await? {
    ///         match result {
    ///             Ok(kline_data) => {
    ///                 println!("Received: {} at price {}", kline_data.symbol, kline_data.close);
    ///             }
    ///             Err(e) => {
    ///                 eprintln!("Parse error: {}", e);
    ///                 continue; // Skip invalid messages
    ///             }
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For most applications, consider using [`listen()`] instead, which provides
    /// automatic message processing with registered callbacks and handles errors gracefully.
    pub async fn next(&mut self) -> Result<Option<Result<SerdableKlineData>>> {
        match self.state.as_mut().next().await {
            Some(Ok(message)) => {
                let binary_data = message.into_data();
                let data = std::str::from_utf8(&binary_data)
                    .expect("Failed to convert binary data to string");
                println!("Received Kline message: {data}");
                let payload = serde_json::from_str::<Payload>(data);
                match payload {
                    Ok(payload) => {
                        let kline_data = payload.to_serializable_kline_data()?;
                        Ok(Some(Ok(kline_data)))
                    }
                    _ => {
                        println!("Failed to parse Kline data: {data}");
                        Ok(Some(Err(anyhow::Error::msg("Failed to parse Kline data"))))
                    }
                }
            }
            Some(Err(e)) => Ok(Some(Err(anyhow::Error::msg(e.to_string())))),
            None => Ok(None),
        }
    }

    /// Starts listening for Kline messages and processes them using registered callbacks.
    ///
    /// This method provides a high-level interface for continuous message processing.
    /// It automatically handles the message retrieval loop and delegates processing
    /// to all registered [`MessageHandler`] callbacks. This is the recommended approach
    /// for most applications that need automated data processing.
    ///
    /// # Behavior
    ///
    /// - Continuously calls [`next()`] to retrieve messages
    /// - For each successful message, invokes all registered callbacks in sequence
    /// - Logs parsing errors and continues processing (non-fatal)
    /// - Stops processing if a callback returns an error (fatal)
    /// - Returns when the WebSocket stream ends or a critical error occurs
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Stream ended gracefully (connection closed by server)
    /// - `Err(anyhow::Error)` - Critical error occurred or callback failed
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - A registered callback returns an error during message processing
    /// - Critical WebSocket communication errors occur
    /// - The underlying [`next()`] method encounters unrecoverable errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opentrade_core::data_source::websocket::{KlineStreaming, MessageHandler};
    /// use opentrade_core::models::SerdableKlineData;
    /// use binance_spot_connector_rust::market::klines::KlineInterval;
    /// use async_trait::async_trait;
    /// use anyhow::Result;
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait]
    /// impl MessageHandler<SerdableKlineData> for MyHandler {
    ///     async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
    ///         println!("Processing: {}", message.symbol);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
    ///     stream.add_callback(MyHandler);
    ///     stream.subscribe().await?;
    ///
    ///     // Start continuous processing (blocks until stream ends or error)
    ///     stream.listen().await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Callback Processing
    ///
    /// Callbacks are executed sequentially for each message. If any callback returns
    /// an error, processing stops immediately. For non-critical errors, callbacks
    /// should handle them internally and return `Ok(())` to continue processing.
    ///
    /// # Performance Considerations
    ///
    /// Since callbacks are executed sequentially, avoid long-running operations in
    /// callback implementations. Consider using async channels or background tasks
    /// for time-consuming operations like database writes or external API calls.
    pub async fn listen(&mut self) -> Result<()> {
        while let Some(result) = self.next().await? {
            match result {
                Ok(kline_data) => {
                    for callback in &mut self.callbacks {
                        callback.handle_message(&kline_data).await?;
                    }
                }
                Err(e) => {
                    eprintln!("Error processing Kline data: {e}");
                }
            }
        }
        Ok(())
    }
}

/// Trait for handling incoming WebSocket messages with custom processing logic.
///
/// The `MessageHandler` trait defines a contract for processing incoming messages
/// from WebSocket streams. Implementations can perform various operations such as
/// data storage, real-time analysis, logging, or forwarding to other systems.
///
/// # Type Parameters
///
/// * `T` - The message type that must be sendable, thread-safe, cloneable, and serializable
///
/// # Async Support
///
/// All message handling is asynchronous to support I/O operations like database
/// writes, network calls, or file operations without blocking the WebSocket stream.
///
/// # Error Handling
///
/// Handlers should return `Result<()>` to indicate success or failure. Errors
/// will be propagated up to the streaming client, which can decide how to handle
/// them (e.g., log and continue, or stop processing).
///
/// # Example Implementation
///
/// ```rust
/// use opentrade_core::data_source::websocket::MessageHandler;
/// use opentrade_core::models::SerdableKlineData;
/// use async_trait::async_trait;
/// use anyhow::Result;
///
/// struct DatabaseHandler {
///     // Database connection pool would go here
/// }
///
/// #[async_trait]
/// impl MessageHandler<SerdableKlineData> for DatabaseHandler {
///     async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
///         // Convert to database format
///         let kline_data = opentrade_core::models::KlineData::from(message.clone());
///
///         // Store in database (pseudo-code)
///         // kline_data.upsert(&self.pool).await?;
///
///         println!("Stored Kline data for {}", message.symbol);
///         Ok(())
///     }
/// }
/// ```
///
/// # Multiple Handlers
///
/// Multiple handlers can be registered with a single stream to perform different
/// processing tasks in sequence:
///
/// ```rust,no_run
/// # use opentrade_core::data_source::websocket::KlineStreaming;
/// # use binance_spot_connector_rust::market::klines::KlineInterval;
/// # use anyhow::Result;
/// # async fn example() -> Result<()> {
/// let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
///
/// // Add multiple handlers for different purposes
/// // stream.add_callback(DatabaseHandler::new());
/// // stream.add_callback(LoggingHandler::new());
/// // stream.add_callback(AnalyticsHandler::new());
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait MessageHandler<T: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>> {
    /// Processes an incoming message asynchronously.
    ///
    /// This method is called for each message received from the WebSocket stream.
    /// Implementations should handle the message according to their specific logic
    /// and return `Ok(())` on success or an error on failure.
    ///
    /// # Arguments
    ///
    /// * `message` - A reference to the incoming message
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Message processed successfully
    /// * `Err(anyhow::Error)` - Processing failed with the given error
    ///
    /// # Example
    ///
    /// ```rust
    /// # use opentrade_core::data_source::websocket::MessageHandler;
    /// # use opentrade_core::models::SerdableKlineData;
    /// # use async_trait::async_trait;
    /// # use anyhow::Result;
    /// struct SimpleHandler;
    ///
    /// #[async_trait]
    /// impl MessageHandler<SerdableKlineData> for SimpleHandler {
    ///     async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
    ///         println!("Received Kline for {} at price {}", message.symbol, message.close);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn handle_message(&mut self, message: &T) -> Result<()>;
}

#[allow(dead_code)]
struct PrintKlineHandler {
    count: usize,
}

impl PrintKlineHandler {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[async_trait]
impl MessageHandler<SerdableKlineData> for PrintKlineHandler {
    async fn handle_message(&mut self, message: &SerdableKlineData) -> Result<()> {
        println!("Received Kline data: {:?}", message);
        self.count += 1;
        if self.count >= 10 {
            println!("Processed 10 Kline messages, stopping further processing.");
            return Err(anyhow::Error::msg(
                "Processed 10 Kline messages, stopping further processing.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payload() {
        let json = r#"{"stream":"btcusdt@kline_1m","data":{"e":"kline","E":1751897378015,"s":"BTCUSDT","k":{"t":1751897340000,"T":1751897399999,"s":"BTCUSDT","i":"1m","f":5067431062,"L":5067432892,"o":"108521.04000000","c":"108473.03000000","h":"108521.04000000","l":"108473.02000000","v":"5.21006000","n":1831,"x":false,"q":"565334.99194810","V":"3.03940000","Q":"329823.87289940","B":"0"}}}"#;
        let payload: Payload = serde_json::from_str(json).expect("Failed to parse JSON");
        assert_eq!(payload.stream, "btcusdt@kline_1m");
        assert_eq!(payload.data.symbol, "BTCUSDT");
        assert_eq!(payload.data.kline.interval, "1m");
        assert_eq!(payload.data.kline.open, "108521.04000000");
        assert_eq!(payload.data.kline.close, "108473.03000000");
        assert_eq!(payload.data.kline.high, "108521.04000000");
        assert_eq!(payload.data.kline.low, "108473.02000000");
        assert_eq!(payload.data.kline.volume, "5.21006000");
        assert_eq!(payload.data.kline.quote_volume, "565334.99194810");
    }

    #[tokio::test]
    async fn test_kline_streaming() {
        let mut kline_streaming =
            KlineStreaming::new("BTCUSDT", market::klines::KlineInterval::Minutes1)
                .await
                .expect("Failed to create KlineStreaming instance");

        kline_streaming
            .subscribe()
            .await
            .expect("Failed to subscribe to KlineStreaming");

        let mut count = 0;
        while let Ok(Some(result)) = kline_streaming.next().await {
            match result {
                Ok(kline_data) => {
                    assert_eq!(kline_data.symbol, "BTCUSDT");
                    println!("Received Kline data: {kline_data:?}");
                    count += 1;
                    if count >= 10 {
                        break; // Limit the test to 10 messages for performance
                    }
                }
                Err(e) => {
                    count += 1;
                    eprintln!("Error parsing Kline data: {e}");
                    continue; // Continue to the next message
                }
            }
        }
        assert!(count > 0, "No Kline data received");

        let handler = PrintKlineHandler::new();
        kline_streaming.add_callback(handler);
        kline_streaming
            .listen()
            .await
            .expect("Failed to listen to KlineStreaming");
    }
}
