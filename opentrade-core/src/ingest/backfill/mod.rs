//! # Historical Data Backfill Module
//!
//! This module provides functionality for efficiently backfilling historical market data
//! from cryptocurrency exchanges. It handles large-scale data retrieval, processing,
//! and storage operations with built-in error handling and rate limiting.
//!
//! ## Features
//!
//! - **Batch Processing**: Efficient handling of large historical data sets
//! - **Rate Limiting**: Respects exchange API rate limits to prevent throttling
//! - **Data Validation**: Ensures data integrity and consistency during backfill
//! - **Conflict Resolution**: Handles duplicate data with upsert operations
//! - **Progress Tracking**: Monitors backfill progress and provides status updates
//! - **Resumable Operations**: Supports interruption and resumption of backfill jobs
//!
//! ## Submodules
//!
//! - [`klines`] - Kline (candlestick) data backfill operations and utilities
//!
//! ## Usage Patterns
//!
//! ### Basic Backfill Operation
//! ```rust,no_run
//! use opentrade_core::ingest::backfill::klines;
//! use chrono::{DateTime, Utc};
//! use sqlx::PgPool;
//!
//! async fn backfill_historical_data(
//!     pool: &PgPool,
//!     symbol: &str,
//!     start_time: DateTime<Utc>,
//!     end_time: DateTime<Utc>
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Implementation details available in klines submodule
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The backfill module follows a worker-based architecture where each data type
//! (klines, trades, etc.) has its own specialized processor that can operate
//! independently or in coordination with other processors.

pub mod klines;