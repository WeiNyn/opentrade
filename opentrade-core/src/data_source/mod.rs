//! # Data Source Module
//!
//! This module provides abstractions and implementations for connecting to various
//! cryptocurrency exchange data sources. It supports both REST API and WebSocket
//! connections for retrieving market data in different modes.
//!
//! ## Submodules
//!
//! - [`rest`] - RESTful HTTP API client implementations for fetching historical data
//! - [`websocket`] - Real-time WebSocket streaming implementations for live market data
//!
//! ## Usage Patterns
//!
//! ### REST API Data Fetching
//! Use REST APIs for:
//! - Historical data backfill operations
//! - One-time data queries
//! - Batch processing scenarios
//! - Rate-limited data retrieval
//!
//! ### WebSocket Streaming
//! Use WebSocket connections for:
//! - Real-time market data streaming
//! - Live trading applications
//! - Continuous monitoring systems
//! - Low-latency data requirements
//!
//! ## Architecture
//!
//! The data source module follows a modular design where each connection type
//! (REST/WebSocket) is implemented in its own submodule with standardized
//! interfaces for data retrieval and processing.

pub mod rest;
pub mod websocket;