use super::*;

pub(crate) struct ApiAuth {
    pub(crate) ts: String,
    pub(crate) token: String,
    pub(crate) tokenparam: String,
}

impl ApiAuth {
    pub(crate) fn current() -> Self {
        let ts = current_millis_timestamp();

        Self {
            token: md5_hex(&format!("{ts}{API_VERSION}")),
            tokenparam: format!("{ts},{API_VERSION}"),
            ts,
        }
    }
}

pub(crate) struct SettingAuth {
    pub(crate) ts: String,
    pub(crate) token: String,
    pub(crate) tokenparam: String,
}

impl SettingAuth {
    pub(crate) fn current() -> Self {
        let ts = current_seconds_timestamp();

        Self {
            token: md5_hex(&format!("{ts}{API_SECRET}")),
            tokenparam: format!("{ts},{API_VERSION}"),
            ts,
        }
    }
}
