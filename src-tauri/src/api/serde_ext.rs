use serde::Deserialize;

pub(crate) fn u32_from_string_or_number<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| serde::de::Error::custom("expected a valid u32 number")),
        serde_json::Value::String(value) => value
            .parse::<u32>()
            .map_err(|error| serde::de::Error::custom(format!("expected a u32 string: {error}"))),
        _ => Err(serde::de::Error::custom("expected a u32 number or string")),
    }
}

pub(crate) fn u32_from_string_or_number_or_empty<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| serde::de::Error::custom("expected a valid u32 number")),
        serde_json::Value::String(value) => {
            let value = value.trim();

            if value.is_empty() {
                return Ok(0);
            }

            value.parse::<u32>().map_err(|error| {
                serde::de::Error::custom(format!("expected a u32 string: {error}"))
            })
        }
        serde_json::Value::Null => Ok(0),
        _ => Err(serde::de::Error::custom(
            "expected a u32 number, string, or empty value",
        )),
    }
}

pub(crate) fn f32_from_percent_string_or_number_or_empty<'de, D>(
    deserializer: D,
) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Number(number) => number
            .as_f64()
            .map(|value| value as f32)
            .ok_or_else(|| serde::de::Error::custom("expected a valid f32 number")),
        serde_json::Value::String(value) => {
            let value = value.trim().trim_end_matches('%');

            if value.is_empty() {
                return Ok(0.0);
            }

            value.parse::<f32>().map_err(|error| {
                serde::de::Error::custom(format!("expected an f32 string: {error}"))
            })
        }
        serde_json::Value::Null => Ok(0.0),
        _ => Err(serde::de::Error::custom(
            "expected a f32 number, percent string, or empty value",
        )),
    }
}

pub(crate) fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        _ => Err(serde::de::Error::custom("expected a string or number")),
    }
}

pub(crate) fn string_or_number_or_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        _ => Err(serde::de::Error::custom(
            "expected a string, number, or empty value",
        )),
    }
}

pub(crate) fn optional_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::String(value) => {
            if value.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        serde_json::Value::Number(value) => Ok(Some(value.to_string())),
        serde_json::Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom(
            "expected an optional string or number",
        )),
    }
}

pub(crate) fn lossy_string_from_scalar<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        _ => Err(serde::de::Error::custom("expected a scalar value")),
    }
}

pub(crate) fn optional_lossy_string_from_scalar<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::String(value) => {
            if value.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        serde_json::Value::Number(value) => Ok(Some(value.to_string())),
        serde_json::Value::Bool(value) => Ok(Some(value.to_string())),
        serde_json::Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom(
            "expected an optional scalar value",
        )),
    }
}

pub(crate) fn lossy_string_vec_from_array_or_scalar<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Array(items) => {
            let values = items
                .into_iter()
                .filter_map(|item| match item {
                    serde_json::Value::String(value) => Some(value),
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    serde_json::Value::Bool(value) => Some(value.to_string()),
                    _ => None,
                })
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>();

            Ok(values)
        }
        serde_json::Value::String(value) => {
            if value.trim().is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![value])
            }
        }
        serde_json::Value::Number(value) => Ok(vec![value.to_string()]),
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(serde::de::Error::custom(
            "expected a string array, scalar, or empty value",
        )),
    }
}

pub(crate) fn bool_or_int_string_or_empty<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Bool(value) => Ok(value),
        serde_json::Value::Number(value) => Ok(value.as_u64().unwrap_or_default() != 0),
        serde_json::Value::String(value) => {
            let value = value.trim().to_ascii_lowercase();

            Ok(matches!(value.as_str(), "1" | "true" | "yes" | "ok"))
        }
        serde_json::Value::Null => Ok(false),
        _ => Err(serde::de::Error::custom(
            "expected a bool, int, string, or empty value",
        )),
    }
}

pub(crate) fn optional_i64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("expected a valid i64 number")),
        serde_json::Value::String(value) => {
            if value.trim().is_empty() {
                return Ok(None);
            }

            value.parse::<i64>().map(Some).map_err(|error| {
                serde::de::Error::custom(format!("expected an i64 string: {error}"))
            })
        }
        _ => Err(serde::de::Error::custom("expected an i64 number or string")),
    }
}
