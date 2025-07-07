use core::num;
use std::f32::consts::E;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::data_source::rest::parse_decimal_string;
use binance_spot_connector_rust::{
    market,
    market_stream::{kline::KlineStream, klines},
    tokio_tungstenite::{BinanceWebSocketClient, WebSocketState},
};
use futures_util::{Stream, StreamExt};
use serde::de::Error as SerdeDeError;
use serde_json::{
    self,
    Value::{self, Array, Number as JsonNumber, Object, String as JsonString},
};
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

use crate::models::KlineData;

pub struct KlineSubscription {
    pub symbol: String,
    pub interval: market::klines::KlineInterval,
}

pub struct KlineStreaming {
    pub symbol: String,
    pub interval: market::klines::KlineInterval,
    pub state: WebSocketState<MaybeTlsStream<TcpStream>>,
}

impl KlineStreaming {
    pub async fn new(
        symbol: &str,
        interval: market::klines::KlineInterval,
    ) -> Result<Self, anyhow::Error> {
        let (state, _) = BinanceWebSocketClient::connect_async_default().await?;

        Ok(Self {
            symbol: symbol.to_string(),
            interval,
            state,
        })
    }

    pub async fn subscribe(&mut self) -> Result<(), anyhow::Error> {
        self.state
            .subscribe(vec![&KlineStream::new(&self.symbol, self.interval).into()])
            .await;
        Ok(())
    }

    async fn next(
        &mut self,
    ) -> Result<Option<Result<KlineData, serde_json::Error>>, serde_json::Error> {
        match self.state.as_mut().next().await {
            Some(Ok(message)) => {
                let binary_data = message.into_data();
                let data = std::str::from_utf8(&binary_data)
                    .expect("Failed to convert binary data to string");
                println!("Received message: {}", data);
                match parse_kline_message(data.to_string()) {
                    Ok(kline_data) => Ok(Some(Ok(kline_data))),
                    // Err(e) => Ok(Some(Err(e))),
                    _ => {
                        return Ok(Some(Err(serde_json::Error::custom(
                            "Failed to parse Kline data",
                        ))));
                    }
                }
            }
            Some(Err(e)) => Ok(Some(Err(serde_json::Error::custom(e.to_string())))),
            None => Ok(None),
        }
    }
}

impl Stream for KlineStreaming {
    type Item = Result<KlineData, serde_json::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self).poll_next(cx)
    }
}

fn parse_kline_message(message: String) -> Result<KlineData, serde_json::Error> {
    let payload: Value = serde_json::from_str(&message)?;
    let data =
        match payload.is_object() {
            true => payload.as_object().unwrap().get("data").ok_or_else(|| {
                serde_json::Error::custom("Missing 'data' field in Kline message")
            })?,
            false => return Err(serde_json::Error::custom("Invalid Kline data format")),
        };

    if data.is_object() {
        match data.get("s") {
            Some(JsonString(symbol)) => {
                if let Some(Object(kline)) = data.get("k") {
                    let open_time = kline
                        .get("t")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| serde_json::Error::custom("Missing or invalid open time"))?;
                    let close_time = kline.get("T").and_then(|v| v.as_u64()).ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid close time")
                    })?;
                    let high = parse_decimal_string(kline.get("h").ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid high price")
                    })?)?;
                    let low = parse_decimal_string(kline.get("l").ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid low price")
                    })?)?;
                    let open = parse_decimal_string(kline.get("o").ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid open price")
                    })?)?;
                    let close = parse_decimal_string(kline.get("c").ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid close price")
                    })?)?;
                    let volume =
                        parse_decimal_string(kline.get("v").ok_or_else(|| {
                            serde_json::Error::custom("Missing or invalid volume")
                        })?)?;
                    let quote_volume = parse_decimal_string(kline.get("q").ok_or_else(|| {
                        serde_json::Error::custom("Missing or invalid quote volume")
                    })?)?;
                    let number_of_trades =
                        kline.get("n").and_then(|v| v.as_u64()).ok_or_else(|| {
                            serde_json::Error::custom("Missing or invalid number of trades")
                        })?;
                    let kline_data = KlineData::new(
                        &open_time,
                        &close_time,
                        &symbol,
                        "1m",
                        0 as i32,
                        0 as i32,
                        open,
                        high,
                        low,
                        close,
                        volume,
                        Some(number_of_trades as i32),
                        Some(quote_volume),
                    );
                    return Ok(kline_data);
                }
            }
            _ => {
                return Err(serde_json::Error::custom("Missing or invalid symbol"));
            }
        }
    }
    Err(serde_json::Error::custom("Invalid Kline data format"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KlineData;

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
                    println!(
                        "Received Kline data: {:?}",
                        kline_data
                    );
                    count += 1;
                    if count >= 10 {
                        break; // Limit the test to 10 messages for performance
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing Kline data: {}", e);
                    continue; // Continue to the next message
                }
            }
        }
        assert!(count > 0, "No Kline data received");
    }
}
