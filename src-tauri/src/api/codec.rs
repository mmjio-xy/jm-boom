use super::*;

pub(crate) async fn request_api_data<T>(
    client: &reqwest::Client,
    endpoint: &str,
    path: &str,
    query: &[(&str, String)],
    auth: &ApiAuth,
) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    let request_name = format!("{endpoint}/{path}");
    let url = format!("{endpoint}/{path}");
    let query = query
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();

    let response = client
        .get(url)
        .with_jm_headers(&request_name, auth, true)?
        .query(&query)
        .send()
        .await
        .map_err(|error| {
            ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(ApiError::new(
            ApiErrorKind::Http,
            format!("{request_name}: API returned HTTP {}", response.status()),
        ));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().await.map_err(|error| {
        ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
    })?;
    let body = body.trim();

    if body.is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::Empty,
            format!("{request_name}: API returned an empty response"),
        ));
    }

    let envelope: ApiResponse<serde_json::Value> = serde_json::from_str(body).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Decode,
            format!(
                "{request_name}: Invalid API response ({content_type}): {error}. Body starts with: {}",
                response_preview(body)
            ),
        )
    })?;

    if envelope.code != 200 {
        return Err(ApiError::new(
            ApiErrorKind::Api,
            envelope
                .error_msg
                .map(|message| format!("{request_name}: {message}"))
                .unwrap_or_else(|| format!("{request_name}: API returned code {}", envelope.code)),
        ));
    }

    let data = envelope.data.ok_or_else(|| {
        ApiError::new(
            ApiErrorKind::MissingData,
            format!("{request_name}: API response did not include data"),
        )
    })?;

    match data {
        serde_json::Value::String(encrypted) => {
            let decrypted = decrypt_data(&encrypted, &auth.ts).map_err(|error| {
                ApiError::new(ApiErrorKind::Decrypt, format!("{request_name}: {error}"))
            })?;
            serde_json::from_str(&decrypted).map_err(|error| {
                ApiError::new(
                    ApiErrorKind::Payload,
                    format!(
                        "{request_name}: Invalid payload: {error}. Payload starts with: {}",
                        response_preview(&decrypted)
                    ),
                )
            })
        }
        value => serde_json::from_value(value).map_err(|error| {
            ApiError::new(
                ApiErrorKind::Payload,
                format!("{request_name}: Invalid payload: {error}"),
            )
        }),
    }
}

pub(crate) async fn request_api_form_data<T>(
    client: &reqwest::Client,
    endpoint: &str,
    path: &str,
    fields: Vec<(String, String)>,
    auth: &ApiAuth,
) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    request_api_form_data_with_jwt(client, endpoint, path, fields, auth, true).await
}

pub(crate) async fn request_api_form_data_with_jwt<T>(
    client: &reqwest::Client,
    endpoint: &str,
    path: &str,
    fields: Vec<(String, String)>,
    auth: &ApiAuth,
    use_jwt: bool,
) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    let request_name = format!("{endpoint}/{path}");
    let url = format!("{endpoint}/{path}");

    let response = client
        .post(url)
        .with_jm_headers(&request_name, auth, use_jwt)?
        .form(&fields)
        .send()
        .await
        .map_err(|error| {
            ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
        })?;

    decode_api_response(response, &request_name, auth).await
}

pub(crate) async fn decode_api_response<T>(
    response: reqwest::Response,
    request_name: &str,
    auth: &ApiAuth,
) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    let status = response.status();

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().await.map_err(|error| {
        ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
    })?;
    let body = body.trim();

    if body.is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::Empty,
            format!("{request_name}: API returned an empty response"),
        ));
    }

    let envelope: ApiResponse<serde_json::Value> = serde_json::from_str(body).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Decode,
            format!(
                "{request_name}: Invalid API response ({content_type}): {error}. Body starts with: {}",
                response_preview(body)
            ),
        )
    })?;

    if !status.is_success() {
        return Err(ApiError::new(
            ApiErrorKind::Api,
            envelope
                .error_msg
                .map(|message| format!("{request_name}: {message}"))
                .unwrap_or_else(|| format!("{request_name}: API returned HTTP {status}")),
        ));
    }

    if envelope.code != 200 {
        return Err(ApiError::new(
            ApiErrorKind::Api,
            envelope
                .error_msg
                .map(|message| format!("{request_name}: {message}"))
                .unwrap_or_else(|| format!("{request_name}: API returned code {}", envelope.code)),
        ));
    }

    let data = envelope.data.ok_or_else(|| {
        ApiError::new(
            ApiErrorKind::MissingData,
            format!("{request_name}: API response did not include data"),
        )
    })?;

    match data {
        serde_json::Value::String(encrypted) => {
            let decrypted = decrypt_data(&encrypted, &auth.ts).map_err(|error| {
                ApiError::new(ApiErrorKind::Decrypt, format!("{request_name}: {error}"))
            })?;
            serde_json::from_str(&decrypted).map_err(|error| {
                ApiError::new(
                    ApiErrorKind::Payload,
                    format!(
                        "{request_name}: Invalid payload: {error}. Payload starts with: {}",
                        response_preview(&decrypted)
                    ),
                )
            })
        }
        value => serde_json::from_value(value).map_err(|error| {
            ApiError::new(
                ApiErrorKind::Payload,
                format!("{request_name}: Invalid payload: {error}"),
            )
        }),
    }
}

pub(crate) fn decrypt_data(data: &str, ts: &str) -> Result<String, String> {
    let key = md5_hex(&format!("{ts}{API_SECRET}"));
    decrypt_base64_with_key(data, &key)
}

pub(crate) fn decrypt_base64_with_key(data: &str, key: &str) -> Result<String, String> {
    let encrypted = BASE64_STANDARD
        .decode(data)
        .map_err(|error| format!("Invalid encrypted data: {error}"))?;
    let decrypted = Aes256EcbDec::new_from_slice(key.as_bytes())
        .map_err(|error| format!("Invalid AES key: {error}"))?
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted)
        .map_err(|error| format!("Failed to decrypt response: {error}"))?;

    String::from_utf8(decrypted).map_err(|error| format!("Invalid decrypted text: {error}"))
}

pub(crate) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(crate) fn current_millis_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

pub(crate) fn current_seconds_timestamp() -> String {
    current_timestamp().to_string()
}

pub(crate) fn android_user_agent() -> &'static str {
    "Mozilla/5.0 (Linux; Android 13; jm-boom Build/TQ1A.230305.002; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/120.0.6099.230 Mobile Safari/537.36"
}

pub(crate) fn request_url_host(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .filter(|host| !host.is_empty())
}

pub(crate) fn md5_hex(input: &str) -> String {
    format!("{:x}", md5::compute(input))
}
