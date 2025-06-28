use binance_spot_connector_rust::{
    hyper::{BinanceHttpClient, Error},
    market::{self, klines::KlineInterval},
};
use serde::de::Error as SerdeDeError;
use serde_json::Value;
use sqlx::types::BigDecimal;

use crate::models::KlineData;

pub async fn get_kline_data(
    symbol: &str,
    interval: KlineInterval,
    start_time: u64,
    end_time: u64,
) -> Result<String, Error> {
    let client = BinanceHttpClient::default();
    let request = market::klines(symbol, interval)
        .start_time(start_time)
        .end_time(end_time);
    let response = client.send(request).await?;
    let data = response.into_body_str().await?;
    Ok(data)
}

pub fn parse_decimal_string(
    value: &Value,
) -> Result<BigDecimal, serde_json::Error> {
    value.as_str()
        .ok_or_else(|| serde_json::Error::custom("Invalid decimal string"))
        .and_then(|s| s.parse::<BigDecimal>().map_err(|_| serde_json::Error::custom("Invalid decimal format")))
}


pub fn parse_kline_data(
    kline: Value,
    symbol: &str,
) -> Result<KlineData, serde_json::Error> {
    match kline.is_array() {
        true => {
            let array = kline.as_array().unwrap();
            let open_time = array.get(0)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| serde_json::Error::custom("Missing or invalid open time"))?;
            let open_price = parse_decimal_string(
                array.get(1)
                    .ok_or_else(|| serde_json::Error::custom("Missing open price"))?
            )?;
            let high_price = parse_decimal_string(
                array.get(2)
                    .ok_or_else(|| serde_json::Error::custom("Missing high price"))?
            )?;
            let low_price = parse_decimal_string(
                array.get(3)
                    .ok_or_else(|| serde_json::Error::custom("Missing low price"))?
            )?;
            let close_price = parse_decimal_string(
                array.get(4)
                    .ok_or_else(|| serde_json::Error::custom("Missing close price"))?
            )?;
            let volume = parse_decimal_string(
                array.get(5)
                    .ok_or_else(|| serde_json::Error::custom("Missing volume"))?
            )?;
            let close_time = array.get(6)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| serde_json::Error::custom("Missing or invalid close time"))?;
            let quote_volume = parse_decimal_string(
                array.get(7)
                    .ok_or_else(|| serde_json::Error::custom("Missing or invalid quote volume"))?
            )?;
            let number_of_trades = array.get(8)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| serde_json::Error::custom("Missing or invalid number of trades"))?;
            let _taker_buy_base_volume = parse_decimal_string(
                array.get(9)
                    .ok_or_else(|| serde_json::Error::custom("Missing or invalid taker buy base volume"))?
            )?;
            let _taker_buy_quote_volume = parse_decimal_string(
                array.get(10)
                    .ok_or_else(|| serde_json::Error::custom("Missing or invalid taker buy quote volume"))?
            )?;
            Ok(KlineData::new(
                &open_time,
                &close_time,
                symbol,
                "1m",
                0 as i32,
                0 as i32,
                open_price,
                high_price,
                low_price,
                close_price,
                volume,
                Some(number_of_trades as i32),
                Some(quote_volume),
            ))
            }
        false => {
            return Err(serde_json::Error::custom("Expected kline data to be an array"));
        }
    }

}

pub fn extract_klines_from_string(
    klines_data: &str,
    symbol: &str,
) -> Result<Vec<KlineData>, serde_json::Error> {
    let data: Value = serde_json::from_str(klines_data)?;

    match data.is_array() {
        true => {
            // Process the array
            let mut klines = Vec::new();
            for item in data.as_array().unwrap() {
                let kline = parse_kline_data(item.clone(), symbol)?;
                klines.push(kline);
            }
            Ok(klines)
        },
        false => {
            return Err(serde_json::Error::custom("Expected klines data is an array"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_parse_decimal_string_success() {
        let value = json!("123.45");
        let result = parse_decimal_string(&value).unwrap();
        assert_eq!(result, BigDecimal::from_str("123.45").unwrap());
    }

    #[test]
    fn test_parse_decimal_string_invalid_string() {
        let value = json!(123);
        let result = parse_decimal_string(&value);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_string_invalid_format() {
        let value = json!("not-a-decimal");
        let result = parse_decimal_string(&value);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_kline_data_success() {
        let kline_value = json!([
            1499040000000i64,
            "0.01634790",
            "0.80000000",
            "0.01575800",
            "0.01577100",
            "148976.11427815",
            1499644799999i64,
            "2434.19055334",
            308,
            "1756.87402397",
            "28.46694368",
            "0"
        ]);
        let result = parse_kline_data(kline_value, "BTCUSDT");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_kline_data_not_an_array() {
        let kline_value = json!({"a": "b"});
        let result = parse_kline_data(kline_value, "BTCUSDT");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Expected kline data to be an array");
    }

    #[test]
    fn test_parse_kline_data_missing_data() {
        let kline_value = json!([
            1499040000000i64,
            "0.01634790"
        ]);
        let result = parse_kline_data(kline_value, "BTCUSDT");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_klines_from_string_success() {
        let klines_string = r#"[
            [
                1499040000000,
                "0.01634790",
                "0.80000000",
                "0.01575800",
                "0.01577100",
                "148976.11427815",
                1499644799999,
                "2434.19055334",
                308,
                "1756.87402397",
                "28.46694368",
                "0"
            ]
        ]"#;
        let result = extract_klines_from_string(klines_string, "BTCUSDT");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_extract_klines_from_string_invalid_json() {
        let klines_string = r#"[
            [
                1499040000000,
                "0.01634790",
        "#;
        let result = extract_klines_from_string(klines_string, "BTCUSDT");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_klines_from_string_not_an_array() {
        let klines_string = r#"{"a": "b"}"#;
        let result = extract_klines_from_string(klines_string, "BTCUSDT");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Expected klines data is an array");
    }

    #[tokio::test]
    async fn test_get_data_e2e() {
        let result = get_kline_data("BTCUSDT", KlineInterval::Minutes1, 1751073120000, 1751103239999).await.unwrap();
        let klines = extract_klines_from_string(&result, "BTCUSDT").unwrap();
        println!("Klines: {:?}", klines);
        assert!(!klines.is_empty());
    }
}
