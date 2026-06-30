use super::*;
use crate::plugin_codec::decode_setting_payload;

pub async fn get_remote_setting(endpoint: Option<String>) -> ApiResult<RemoteSettingResult> {
    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let auth = SettingAuth::current();
    let img_host = request_remote_img_host(&client, &endpoint, &auth).await?;

    Ok(RemoteSettingResult { endpoint, img_host })
}

pub async fn discover_api_endpoints() -> ApiResult<Vec<ApiEndpointProbe>> {
    let client = create_http_client()?;
    let mut candidates = discover_api_endpoint_candidates(&client).await?;

    if candidates.is_empty() {
        candidates.push(DEFAULT_API_ENDPOINT.to_string());
    }

    let auth = SettingAuth::current();
    let mut probes = Vec::with_capacity(candidates.len());

    for endpoint in candidates {
        let started_at = Instant::now();
        let result = request_remote_setting(&client, &endpoint, &auth).await;
        let latency_ms = started_at.elapsed().as_millis() as u64;

        probes.push(match result {
            Ok(setting) => ApiEndpointProbe {
                endpoint,
                available: true,
                latency_ms: Some(latency_ms),
                img_host: Some(setting.img_host),
                error: None,
            },
            Err(error) => ApiEndpointProbe {
                endpoint,
                available: false,
                latency_ms: None,
                img_host: None,
                error: Some(error.to_string()),
            },
        });
    }

    probes.sort_by(|left, right| match (left.available, right.available) {
        (true, true) => left
            .latency_ms
            .unwrap_or(u64::MAX)
            .cmp(&right.latency_ms.unwrap_or(u64::MAX)),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => left.endpoint.cmp(&right.endpoint),
    });

    Ok(probes)
}

pub(crate) async fn request_remote_setting(
    client: &reqwest::Client,
    endpoint: &str,
    auth: &SettingAuth,
) -> ApiResult<RemoteSettingPayload> {
    let request_name = format!("{endpoint}/setting");
    let response = client
        .get(&request_name)
        .header("Tokenparam", &auth.tokenparam)
        .header("Token", &auth.token)
        .query(&[("app_img_shunt", "1"), ("t", auth.ts.as_str())])
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

    let body = response.text().await.map_err(|error| {
        ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
    })?;

    decode_setting_payload::<RemoteSettingPayload>(body.trim(), &auth.ts).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Payload,
            format!(
                "{request_name}: Invalid setting payload: {error}. Body starts with: {}",
                response_preview(&body)
            ),
        )
    })
}

pub(crate) async fn request_remote_img_host(
    client: &reqwest::Client,
    endpoint: &str,
    auth: &SettingAuth,
) -> ApiResult<String> {
    if let Some(img_host) = cached_img_host(endpoint) {
        return Ok(img_host);
    }

    let setting = request_remote_setting(client, endpoint, auth).await?;
    cache_img_host(endpoint, &setting.img_host);

    Ok(setting.img_host)
}

pub(crate) async fn resolve_cached_img_host(
    client: &reqwest::Client,
    endpoint: &str,
) -> ApiResult<String> {
    let auth = SettingAuth::current();

    request_remote_img_host(client, endpoint, &auth).await
}

pub(crate) async fn discover_api_endpoint_candidates(
    client: &reqwest::Client,
) -> ApiResult<Vec<String>> {
    let mut candidates = FALLBACK_API_ENDPOINTS
        .iter()
        .filter_map(|endpoint| normalize_api_endpoint(endpoint).ok())
        .collect::<Vec<_>>();

    match fetch_host_config(client).await {
        Ok(hosts) => {
            candidates.extend(
                hosts
                    .into_iter()
                    .filter_map(|host| normalize_api_endpoint(&host).ok()),
            );
        }
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load JM host config, fallback endpoints only: {error}"
            ));
        }
    }

    let mut unique = Vec::new();
    for endpoint in candidates {
        if !unique.contains(&endpoint) {
            unique.push(endpoint);
        }
    }

    Ok(unique)
}

pub(crate) async fn fetch_host_config(client: &reqwest::Client) -> ApiResult<Vec<String>> {
    let mut last_error = None;

    for url in HOST_CONFIG_URLS {
        match fetch_host_config_from_url(client, url).await {
            Ok(hosts) => return Ok(hosts),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        ApiError::new(ApiErrorKind::Network, "JM host config urls are unavailable")
    }))
}

pub(crate) async fn fetch_host_config_from_url(
    client: &reqwest::Client,
    url: &str,
) -> ApiResult<Vec<String>> {
    let response = client
        .get(url)
        .header("accept", "text/plain,*/*")
        .send()
        .await
        .map_err(|error| ApiError::new(ApiErrorKind::Network, format!("{url}: {error}")))?;

    if !response.status().is_success() {
        return Err(ApiError::new(
            ApiErrorKind::Http,
            format!("{url}: host config returned HTTP {}", response.status()),
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|error| ApiError::new(ApiErrorKind::Network, format!("{url}: {error}")))?;
    let normalized = body
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '='))
        .collect::<String>();
    let key = md5_hex(HOST_CONFIG_AES_SEED);
    let decrypted = decrypt_base64_with_key(&normalized, &key).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Decrypt,
            format!("{url}: failed to decrypt host config: {error}"),
        )
    })?;
    let payload = serde_json::from_str::<HostConfigPayload>(&decrypted).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Payload,
            format!("{url}: invalid host config payload: {error}"),
        )
    })?;

    Ok(payload.server)
}

pub(crate) fn cached_img_host(endpoint: &str) -> Option<String> {
    IMG_HOST_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|cache| cache.get(endpoint).cloned())
}

pub(crate) fn cache_img_host(endpoint: &str, img_host: &str) {
    if let Ok(mut cache) = IMG_HOST_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        cache.insert(endpoint.to_string(), img_host.to_string());
    }
}
