
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub stream: String,
    pub data: KlinePayloadData,
}

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
    pub fn to_kline_data(&self) -> Result<KlineData> {
        let kline = &self.data.kline;

        fn parse_decimal_string(s: &str) -> Result<BigDecimal> {
            s.parse::<BigDecimal>()
                .context(format!("Failed to parse decimal string: {}", s))
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

pub struct KlineSubscription {
    pub symbol: String,
    pub interval: market::klines::KlineInterval,
}

pub struct KlineStreaming {
    pub symbol: String,
    pub interval: market::klines::KlineInterval,
    pub state: WebSocketState<MaybeTlsStream<TcpStream>>,
    pub callbacks: Vec<Box<dyn MessageHandler<SerdableKlineData>>>,
}

impl KlineStreaming {
    pub async fn new(symbol: &str, interval: market::klines::KlineInterval) -> Result<Self> {
        let (state, _) = BinanceWebSocketClient::connect_async_default().await?;

        Ok(Self {
            symbol: symbol.to_string(),
            interval,
            state,
            callbacks: Vec::new(),
        })
    }

    pub fn add_callback<H: MessageHandler<SerdableKlineData> + 'static>(&mut self, handler: H) {
        self.callbacks.push(Box::new(handler));
    }

    pub async fn subscribe(&mut self) -> Result<()> {
        self.state
            .subscribe(vec![&KlineStream::new(&self.symbol, self.interval).into()])
            .await;
        Ok(())
    }

    pub async fn next(&mut self) -> Result<Option<Result<SerdableKlineData>>> {
        match self.state.as_mut().next().await {
            Some(Ok(message)) => {
                let binary_data = message.into_data();
                let data = std::str::from_utf8(&binary_data)
                    .expect("Failed to convert binary data to string");
                println!("Received Kline message: {}", data);
                let payload = serde_json::from_str::<Payload>(data);
                match payload {
                    Ok(payload) => {
                        let kline_data = payload.to_serializable_kline_data()?;
                        Ok(Some(Ok(kline_data)))
                    }
                    _ => {
                        println!("Failed to parse Kline data: {}", data);
                        Ok(Some(Err(anyhow::Error::msg("Failed to parse Kline data"))))
                    }
                }
            }
            Some(Err(e)) => Ok(Some(Err(anyhow::Error::msg(e.to_string())))),
            None => Ok(None),
        }
    }

    pub async fn listen(&mut self) -> Result<()> {
        while let Some(result) = self.next().await? {
            match result {
                Ok(kline_data) => {
                    for callback in &mut self.callbacks {
                        callback.handle_message(&kline_data).await?;
                    }
                }
                Err(e) => {
                    eprintln!("Error processing Kline data: {}", e);
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait MessageHandler<T: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>> {
    async fn handle_message(&mut self, message: &T) -> Result<()>;
}

struct PrintKlineHandler {
    count: usize,
}

impl PrintKlineHandler {
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
                    println!("Received Kline data: {:?}", kline_data);
                    count += 1;
                    if count >= 10 {
                        break; // Limit the test to 10 messages for performance
                    }
                }
                Err(e) => {
                    count += 1;
                    eprintln!("Error parsing Kline data: {}", e);
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
