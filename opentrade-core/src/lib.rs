//! # OpenTrade Core Library
//!
//! `opentrade-core` is a comprehensive Rust library for cryptocurrency trading data management
//! and real-time market data ingestion. It provides robust infrastructure for collecting,
//! processing, and storing financial market data from various sources.
//!
//! ## Features
//!
//! - **Real-time WebSocket Streaming**: Live market data ingestion with configurable callbacks
//! - **Historical Data Backfill**: Efficient batch processing of historical market data
//! - **Database Integration**: PostgreSQL support with optimized data models and CRUD operations
//! - **Flexible Data Sources**: Modular architecture supporting multiple exchange APIs
//! - **Type-safe Models**: Strongly-typed data structures for financial instruments and market data
//!
//! ## Core Modules
//!
//! - [`models`] - Core data structures for market data (Klines, trades, etc.)
//! - [`data_source`] - Data source implementations for REST and WebSocket APIs
//! - [`ingest`] - Data ingestion pipelines for real-time and historical data processing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use opentrade_core::models::KlineData;
//! use opentrade_core::data_source::websocket::KlineStreaming;
//! use binance_spot_connector_rust::market::klines::KlineInterval;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a WebSocket connection for real-time Kline data
//!     let mut stream = KlineStreaming::new("BTCUSDT", KlineInterval::Minutes1).await?;
//!
//!     // Subscribe to the stream
//!     stream.subscribe().await?;
//!
//!     // Process incoming data
//!     while let Some(result) = stream.next().await? {
//!         match result {
//!             Ok(kline_data) => println!("Received: {:?}", kline_data),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Database Support
//!
//! The library includes built-in PostgreSQL support with optimized schema and operations:
//!
//! ```rust,no_run
//! use opentrade_core::models::KlineData;
//! use sqlx::PgPool;
//!
//! async fn store_kline_data(pool: &PgPool, kline: KlineData) -> Result<(), sqlx::Error> {
//!     // Upsert operation with conflict resolution
//!     kline.upsert(pool).await?;
//!     Ok(())
//! }
//! ```

pub mod models;
pub mod data_source;
pub mod ingest;