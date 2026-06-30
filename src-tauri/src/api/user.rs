use super::*;

pub async fn login(
    username: String,
    password: String,
    endpoint: Option<String>,
) -> ApiResult<LoginResult> {
    let username = username.trim().to_string();

    if username.is_empty() || password.trim().is_empty() {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Login needs both username and password",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    clear_session();
    let client = build_http_client()?;
    let setting_auth = SettingAuth::current();
    let login_auth = ApiAuth::current();
    let img_host_future = request_remote_img_host(&client, &endpoint, &setting_auth);
    let payload_future = request_login(&client, &endpoint, &username, &password, &login_auth);
    let (img_host_result, payload_result) = tokio::join!(img_host_future, payload_future);
    let payload = payload_result?;
    set_jwt_token(payload.jwttoken.as_deref())?;
    let img_host = match img_host_result {
        Ok(img_host) => Some(img_host),
        Err(error) => {
            diagnostics::warn(format!(
                "Failed to load remote setting for user avatar: {error}"
            ));
            None
        }
    };

    Ok(LoginResult {
        endpoint,
        user: map_login_user(payload, img_host.as_deref()),
    })
}

pub async fn get_sign_in_data(
    user_id: u32,
    endpoint: Option<String>,
) -> ApiResult<SignInDataResult> {
    if user_id == 0 {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Sign-in data needs a user_id",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let auth = ApiAuth::current();
    let payload = request_sign_in_data(&client, &endpoint, user_id, &auth).await?;

    Ok(SignInDataResult {
        endpoint,
        daily_id: payload.daily_id,
        three_days_coin: payload.three_days_coin,
        three_days_exp: payload.three_days_exp,
        seven_days_coin: payload.seven_days_coin,
        seven_days_exp: payload.seven_days_exp,
        event_name: payload.event_name,
        current_progress: payload.current_progress,
        background_pc: payload.background_pc,
        background_phone: payload.background_phone,
        records: map_sign_in_records(payload.record),
    })
}

pub async fn sign_in(
    user_id: u32,
    daily_id: u32,
    endpoint: Option<String>,
) -> ApiResult<SignInResult> {
    if user_id == 0 || daily_id == 0 {
        return Err(ApiError::new(
            ApiErrorKind::MissingData,
            "Sign-in needs both user_id and daily_id",
        ));
    }

    let endpoint = resolve_api_endpoint(endpoint)?;
    let client = build_http_client()?;
    let auth = ApiAuth::current();
    let payload = request_sign_in(&client, &endpoint, user_id, daily_id, &auth).await?;

    Ok(SignInResult {
        endpoint,
        message: payload.msg,
    })
}

pub(crate) async fn request_login(
    client: &reqwest::Client,
    endpoint: &str,
    username: &str,
    password: &str,
    auth: &ApiAuth,
) -> ApiResult<LoginPayload> {
    request_api_form_data_with_jwt(
        client,
        endpoint,
        "login",
        vec![
            ("username".to_string(), username.to_string()),
            ("password".to_string(), password.to_string()),
        ],
        auth,
        false,
    )
    .await
}

pub(crate) async fn request_sign_in_data(
    client: &reqwest::Client,
    endpoint: &str,
    user_id: u32,
    auth: &ApiAuth,
) -> ApiResult<SignInDataPayload> {
    request_api_data(
        client,
        endpoint,
        "daily",
        &[("user_id", user_id.to_string())],
        auth,
    )
    .await
}

pub(crate) async fn request_sign_in(
    client: &reqwest::Client,
    endpoint: &str,
    user_id: u32,
    daily_id: u32,
    auth: &ApiAuth,
) -> ApiResult<SignInPayload> {
    request_api_form_data(
        client,
        endpoint,
        "daily_chk",
        vec![
            ("user_id".to_string(), user_id.to_string()),
            ("daily_id".to_string(), daily_id.to_string()),
        ],
        auth,
    )
    .await
}
