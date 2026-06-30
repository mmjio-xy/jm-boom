use super::*;

pub async fn get_comic_detail(
    comic_id: String,
    endpoint: Option<String>,
) -> ApiResult<ComicDetailResult> {
    let comic_id = comic_id.trim().to_string();

    if comic_id.is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Comic detail needs a comic_id",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let setting_auth = SettingAuth::current();
    let api_auth = ApiAuth::current();
    let img_host_future = request_remote_img_host(&client, &endpoint, &setting_auth);
    let payload_future = request_comic_detail(&client, &endpoint, &comic_id, &api_auth);
    let (img_host_result, payload_result) = tokio::join!(img_host_future, payload_future);
    let img_host = match img_host_result {
        Ok(img_host) => Some(img_host),
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load remote setting for detail images: {error}"
            ));
            None
        }
    };

    Ok(ComicDetailResult {
        endpoint,
        comic: map_comic_detail(payload_result?, img_host.as_deref()),
    })
}

pub async fn toggle_comic_favorite(
    comic_id: String,
    current_favorite: bool,
    endpoint: Option<String>,
) -> ApiResult<FavoriteToggleResult> {
    let comic_id = comic_id.trim().to_string();

    if comic_id.is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Favorite toggle needs a comic_id",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let auth = ApiAuth::current();
    let request_name = format!("{endpoint}/favorite");
    let response = client
        .post(&request_name)
        .with_jm_headers(&request_name, &auth, true)?
        .form(&[("aid", comic_id.as_str())])
        .send()
        .await
        .map_err(|error| {
            ApiError::new(ApiErrorKind::Network, format!("{request_name}: {error}"))
        })?;

    let _: serde_json::Value = decode_api_response(response, &request_name, &auth).await?;

    Ok(FavoriteToggleResult {
        endpoint,
        favorited: !current_favorite,
    })
}

pub async fn get_favorite_comics(
    page: Option<u32>,
    folder_id: Option<String>,
    order: Option<String>,
    endpoint: Option<String>,
) -> ApiResult<FavoriteListResult> {
    let page = page.unwrap_or(1).max(1);
    let folder_id = folder_id.unwrap_or_default();
    let folder_id = folder_id.trim().to_string();
    let order = order.unwrap_or_else(|| "mr".to_string()).trim().to_string();
    let order = if order.is_empty() {
        "mr".to_string()
    } else {
        order
    };
    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let setting_auth = SettingAuth::current();
    let api_auth = ApiAuth::current();
    let img_host_future = request_remote_img_host(&client, &endpoint, &setting_auth);
    let payload_future =
        request_favorite_comics(&client, &endpoint, page, &folder_id, &order, &api_auth);
    let (img_host_result, payload_result) = tokio::join!(img_host_future, payload_future);
    let img_host = match img_host_result {
        Ok(img_host) => Some(img_host),
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load remote setting for favorite covers: {error}"
            ));
            None
        }
    };
    let payload = payload_result?;
    let total = payload.total;
    let folders = payload
        .folder_list
        .into_iter()
        .filter(|folder| !folder.id.trim().is_empty())
        .map(|folder| FavoriteFolder {
            id: folder.id,
            name: folder.name,
        })
        .collect();
    let items = payload
        .list
        .into_iter()
        .filter(|item| !item.id.trim().is_empty())
        .map(|item| map_favorite_comic(item, img_host.as_deref()))
        .collect::<Vec<_>>();
    let has_more = if total > 0 {
        page.saturating_mul(20) < total
    } else {
        items.len() >= 20
    };

    Ok(FavoriteListResult {
        endpoint,
        page,
        total,
        has_more,
        folders,
        items,
    })
}

pub async fn get_comic_comments(
    comic_id: String,
    page: Option<u32>,
    endpoint: Option<String>,
) -> ApiResult<ComicCommentsResult> {
    let comic_id = comic_id.trim().to_string();
    let page = page.unwrap_or(1);

    if comic_id.is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Comic comments need a comic_id",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let setting_auth = SettingAuth::current();
    let api_auth = ApiAuth::current();
    let img_host_future = request_remote_img_host(&client, &endpoint, &setting_auth);
    let payload_future = request_comic_comments(&client, &endpoint, &comic_id, page, &api_auth);
    let (img_host_result, payload_result) = tokio::join!(img_host_future, payload_future);
    let img_host = match img_host_result {
        Ok(img_host) => Some(img_host),
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load remote setting for comment avatars: {error}"
            ));
            None
        }
    };
    let payload = payload_result?;

    Ok(ComicCommentsResult {
        endpoint,
        page,
        total: payload.total,
        comments: payload
            .list
            .into_iter()
            .map(|comment| map_comment(comment, img_host.as_deref()))
            .collect(),
    })
}

pub(crate) async fn request_comic_detail(
    client: &reqwest::Client,
    endpoint: &str,
    comic_id: &str,
    auth: &ApiAuth,
) -> ApiResult<ComicDetailPayload> {
    let request_name = format!("{endpoint}/album");
    let value: serde_json::Value = request_api_data(
        client,
        endpoint,
        "album",
        &[("id", comic_id.to_string())],
        auth,
    )
    .await?;

    if value
        .as_object()
        .map(|object| object.is_empty() || !object.contains_key("name"))
        .unwrap_or(false)
    {
        return Err(ApiError::new(
            ApiErrorKind::Payload,
            format!("{request_name}: 当前条目可能是小说或书库内容，暂不支持漫画详情阅读"),
        ));
    }

    serde_json::from_value(value).map_err(|error| {
        ApiError::new(
            ApiErrorKind::Payload,
            format!("{request_name}: Invalid payload: {error}"),
        )
    })
}

pub(crate) async fn request_comic_comments(
    client: &reqwest::Client,
    endpoint: &str,
    comic_id: &str,
    page: u32,
    auth: &ApiAuth,
) -> ApiResult<CommentListPayload> {
    request_api_data(
        client,
        endpoint,
        "forum",
        &[
            ("page", page.to_string()),
            ("aid", comic_id.to_string()),
            ("mode", "manhua".to_string()),
        ],
        auth,
    )
    .await
}

pub(crate) async fn request_favorite_comics(
    client: &reqwest::Client,
    endpoint: &str,
    page: u32,
    folder_id: &str,
    order: &str,
    auth: &ApiAuth,
) -> ApiResult<FavoriteListPayload> {
    request_api_data(
        client,
        endpoint,
        "favorite",
        &[
            ("page", page.to_string()),
            ("folder_id", folder_id.to_string()),
            ("o", order.to_string()),
        ],
        auth,
    )
    .await
}
