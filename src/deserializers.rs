use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub fn d_action() -> Option<String> {
    Some("stream".to_string())
}

pub fn d_blank() -> Option<String> {
    None
}

pub fn d_zero() -> Option<u64> {
    None
}

pub fn de_optional_string_or_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Ok(
        match Value::deserialize(deserializer)? {
            Value::String(s) => {
                let s = s.trim();
                if s.is_empty() {
                    return Ok(None);
                }
                if let Ok(number) = s.parse::<u64>() {
                    Some(number)
                } else if let Ok(number) = s.parse::<f64>() {
                    Some(number.floor() as u64)
                } else {
                    return Ok(None);
                }
            }
            Value::Number(num) => {
                if let Some(number) = num.as_u64() {
                    Some(number)
                } else if let Some(number) = num.as_f64() {
                    Some(number.floor() as u64)
                } else {
                    return Ok(None);
                }
            }
            _ => Some(0)
        }
    )
}
