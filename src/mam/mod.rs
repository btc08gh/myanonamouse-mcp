use anyhow::anyhow;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

pub const BASE_URL: &str = "https://www.myanonamouse.net";

/// Build a reqwest Client pre-configured with the mam_id session cookie and a browser User-Agent.
pub fn build_client(mam_session: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    let cookie = HeaderValue::from_str(&format!("mam_id={mam_session}"))
        .map_err(|_| anyhow!("Invalid mam_session value — contains non-ASCII characters"))?;
    headers.insert(header::COOKIE, cookie);

    reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {e}"))
}

/// Response from `/json/jsonIp.php`
#[derive(Debug, Deserialize)]
pub struct IpInfo {
    pub ip: String,
    #[serde(rename = "ASN")]
    pub asn: serde_json::Value,
    #[serde(rename = "AS")]
    pub as_org: String,
}

impl IpInfo {
    pub fn asn_string(&self) -> String {
        match &self.asn {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }
    }
}

/// Fetch current IP info — used by `--test-connection` and the `get_ip_info` tool.
pub async fn get_ip_info(client: &reqwest::Client) -> anyhow::Result<IpInfo> {
    let resp = client
        .get(format!("{BASE_URL}/json/jsonIp.php"))
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!(enrich_error(status.as_u16(), &body)));
    }

    serde_json::from_str::<IpInfo>(&body)
        .map_err(|e| anyhow!("Failed to parse IP info response: {e}\nBody: {body}"))
}

/// Produce a human-readable error string for MAM HTTP errors, with LLM hints where useful.
pub fn enrich_error(status: u16, body: &str) -> String {
    let hint = match status {
        401 | 403 => Some(
            "The mam_id session cookie is invalid or expired. \
             Ask the user to go to Preferences → Security on MyAnonamouse and provide the updated mam_id value.",
        ),
        429 => Some("Rate limited by MyAnonamouse. Wait before retrying."),
        503 => Some("MyAnonamouse is temporarily unavailable. Try again later."),
        _ => None,
    };
    match hint {
        Some(h) => format!("HTTP {status}: {body}\n[Hint: {h}]"),
        None => format!("HTTP {status}: {body}"),
    }
}
