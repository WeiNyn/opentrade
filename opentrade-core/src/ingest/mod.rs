//! # Data Ingestion Module
//!
//! This module provides comprehensive data ingestion capabilities for cryptocurrency
//! market data. It handles both real-time streaming and historical data backfill
//! operations with robust error handling and data validation.
//!
//! ## Core Functionality
//!
//! The ingestion module is designed to:
//! - Process large volumes of market data efficiently
//! - Handle data deduplication and conflict resolution
//! - Provide configurable ingestion strategies
//! - Support multiple data sources and formats
//! - Ensure data integrity and consistency
//!
//! ## Submodules
//!
//! - [`backfill`] - Historical data backfill operations and batch processing
//!
//! ## Usage Patterns
//!
//! ### Historical Data Backfill
//! ```rust,no_run
//! // Example of using backfill functionality
//! use opentrade_core::ingest::backfill;
//!
//! // Backfill operations for historical data
//! // Implementation details available in the backfill submodule
//! ```
//!
//! ## Architecture
//!
//! The ingestion system follows a pipeline architecture where data flows through
//! various stages of validation, transformation, and storage. Each stage can be
//! configured independently to meet specific requirements.

pub mod backfill;