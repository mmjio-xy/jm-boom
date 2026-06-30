use super::*;

pub async fn search_comics(
    keyword: String,
    page: Option<u32>,
    extern_payload: Option<HashMap<String, serde_json::Value>>,
    endpoint: Option<String>,
) -> ApiResult<SearchResultContract> {
    let page = page.unwrap_or(1);
    let keyword = keyword.trim().to_string();
    let extern_payload = normalize_search_extern(extern_payload);
    let order = search_order_from_extern(&extern_payload);
    let endpoint = resolve_api_endpoint(endpoint)?;

    if keyword.is_empty() {
        return Ok(build_search_result(page, 0, Vec::new(), extern_payload));
    }

    let client = build_http_client()?;
    let setting_auth = SettingAuth::current();
    let api_auth = ApiAuth::current();
    let img_host = match request_remote_img_host(&client, &endpoint, &setting_auth).await {
        Ok(img_host) => Some(img_host),
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load remote setting for search covers: {error}"
            ));
            None
        }
    };

    if page == 1 {
        if let Some(comic_id) = direct_search_comic_id(&keyword) {
            match request_comic_detail(&client, &endpoint, &comic_id, &api_auth).await {
                Ok(payload) => {
                    let item = search_payload_from_detail(payload);

                    return Ok(build_search_result(
                        page,
                        1,
                        vec![map_search_comic_item(item, img_host.as_deref())],
                        extern_payload,
                    ));
                }
                Err(error) => {
                    diagnostics::warn(format!(
                        "Failed direct search detail fallback for {comic_id}: {error}"
                    ));
                }
            }
        }
    }

    request_search(
        &client,
        &endpoint,
        &keyword,
        page,
        &order,
        &api_auth,
        img_host.as_deref(),
        extern_payload,
    )
    .await
}

pub(crate) async fn request_search(
    client: &reqwest::Client,
    endpoint: &str,
    keyword: &str,
    page: u32,
    order: &str,
    auth: &ApiAuth,
    img_host: Option<&str>,
    extern_payload: HashMap<String, serde_json::Value>,
) -> ApiResult<SearchResultContract> {
    let mut payload: SearchPayload = request_api_data(
        client,
        endpoint,
        "search",
        &[
            ("page", page.to_string()),
            ("o", order.to_string()),
            ("search_query", keyword.to_string()),
        ],
        auth,
    )
    .await?;

    let items = payload
        .content
        .into_iter()
        .map(|item| map_search_comic_item(item, img_host))
        .collect::<Vec<_>>();

    let redirect_aid = payload.redirect_aid.take();
    let items = if items.is_empty() {
        redirect_aid
            .clone()
            .map(|id| {
                let item = SearchComicPayload {
                    id: id.clone(),
                    name: format!("JM{id}"),
                    ..SearchComicPayload::default()
                };

                vec![map_search_comic_item(item, img_host)]
            })
            .unwrap_or(items)
    } else {
        items
    };

    Ok(build_search_result(
        page,
        payload.total,
        items,
        extern_payload,
    ))
}
