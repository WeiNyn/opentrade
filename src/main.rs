pub mod models;
use binance_spot_connector_rust::{
    market::klines::KlineInterval, market_stream::kline::KlineStream,
    tokio_tungstenite::BinanceWebSocketClient,
};
use env_logger::Builder;
use futures_util::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() {
    Builder::from_default_env()
        .filter(None, log::LevelFilter::Info)
        .init();

    let (mut conn, _) = BinanceWebSocketClient::connect_async_default()
        .await
        .expect("Failed to connect to Binance WebSocket");

    conn.subscribe(vec![
        &KlineStream::new("BTCUSDT", KlineInterval::Minutes1).into(),
    ])
    .await;

    let timer = tokio::time::Instant::now();
    let duration = Duration::new(10, 0); // 10 seconds

    while let Some(message) = conn.as_mut().next().await {
        if timer.elapsed() >= duration {
            log::info!("10 seconds elapsed, closing connection.");
            break;
        }
        match message {
            Ok(message) => {
                let binary_data = message.into_data();
                let data = std::str::from_utf8(&binary_data)
                    .expect("Failed to convert binary data to string");
                log::info!("Received message: {}", data);
            }
            Err(e) => {
                log::error!("Error receiving message: {}", e);
                break;
            }
        }
    }
    conn.close().await.expect("Failed to close connection");
}
