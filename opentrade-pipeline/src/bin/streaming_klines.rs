use anyhow::Result;
use async_trait::async_trait;
use binance_spot_connector_rust::{market::klines::KlineInterval, market_stream::kline};
use opentrade_core::{
    data_source::websocket::{KlineStreaming, MessageHandler},
    models::{SerdableKlineData, KlineData},
};
use sqlx::{pool, PgPool};

pub struct PrintKlineHandler {
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

pub struct UpsertKlineHandler {
    pool: sqlx::PgPool,
}

impl UpsertKlineHandler {
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
