use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerInfo},
    schemars,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MamServer {
    client: Arc<reqwest::Client>,
    tool_router: ToolRouter<Self>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<TorrentResult>,
    total: u64,
    found: u64,
}

#[derive(Deserialize)]
struct TorrentResult {
    id: u64,
    title: String,
    catname: Option<String>,
    size: Option<String>,
    author_info: Option<Value>,
    narrator_info: Option<Value>,
    series_info: Option<Value>,
    tags: Option<Value>,
    seeders: Option<u64>,
    leechers: Option<u64>,
    free: Option<u64>,
    vip: Option<u64>,
    added: Option<String>,
    dl: Option<String>,
}

#[derive(Deserialize)]
struct UserDataResponse {
    username: Option<String>,
    uid: Option<u64>,
    classname: Option<String>,
    downloaded: Option<String>,
    uploaded: Option<String>,
    ratio: Option<f64>,
    seedbonus: Option<u64>,
    wedges: Option<u64>,
    country_name: Option<String>,
    notifs: Option<Value>,
}

#[derive(Deserialize)]
struct BonusEntry {
    timestamp: f64,
    amount: Value,
    #[serde(rename = "type")]
    bonus_type: String,
    tid: Option<Value>,
    title: Option<String>,
    other_userid: Option<Value>,
    other_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Parameter types
// ---------------------------------------------------------------------------

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Search query text
    query: String,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Sort order. Valid values: titleAsc, titleDesc, sizeAsc, sizeDesc, seedersAsc, seedersDesc,
    /// leechersAsc, leechersDesc, snatchedAsc, snatchedDesc, dateAsc, dateDesc, random.
    /// Defaults to relevance.
    #[serde(default)]
    sort: Option<String>,
    /// Filter by main category IDs. Valid values: 13 (AudioBooks), 14 (E-Books),
    /// 15 (Musicology), 16 (Radio). Omit to search all categories.
    #[serde(default)]
    main_cat: Option<Vec<u32>>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct UserDataParams {
    /// User ID to fetch data for. Omit to fetch data for the authenticated user.
    user_id: Option<u64>,
    /// Include unread notifications in the response
    #[serde(default)]
    include_notifications: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct BonusHistoryParams {
    /// Bonus types to filter by. Valid values: giftPoints, giftWedge, wedgePF, wedgeGFL,
    /// torrentThanks, millionaires. Omit for all types.
    bonus_types: Option<Vec<String>>,
    /// Fetch history for another user by their user ID
    other_user_id: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct NoParams {}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl MamServer {
    /// Search for torrents on MyAnonamouse. Returns a formatted list of matching torrents
    /// including title, authors, narrators, series, size, category, and seeder/leecher counts.
    /// Supports filtering by main category and sorting by seeders, size, date, and other fields.
    #[tool]
    async fn search_torrents(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<String, String> {
        let limit = p.limit.unwrap_or(20).min(100);
        let sort_type = p.sort.as_deref().unwrap_or("default");
        let main_cat: Vec<u32> = p.main_cat.unwrap_or_default();
        let body = serde_json::json!({
            "tor": {
                "text": p.query,
                "srchIn": ["title", "author", "narrator"],
                "searchType": "all",
                "searchIn": "torrents",
                "main_cat": main_cat,
                "browseFlagsHideVsShow": "0",
                "startDate": "",
                "endDate": "",
                "hash": "",
                "sortType": sort_type,
                "startNumber": "0",
                "perpage": limit,
            },
            "dlLink": "true",
            "thumbnail": "false",
        });

        let resp = self
            .client
            .post(format!("{}/tor/js/loadSearchJSONbasic.php", crate::mam::BASE_URL))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::mam::enrich_error(status.as_u16(), &text));
        }

        let body = resp.text().await.map_err(|e| format!("Failed to read search response: {e}"))?;
        let parsed: SearchResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse search response: {e}\nBody: {body}"))?;

        Ok(Self::format_search_response(parsed, &p.query))
    }

    /// Fetch profile data for the authenticated user or another user by ID.
    /// Returns username, class, upload/download stats, ratio, and optionally notifications.
    #[tool]
    async fn get_user_data(
        &self,
        Parameters(p): Parameters<UserDataParams>,
    ) -> Result<String, String> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(id) = p.user_id {
            query.push(("id", id.to_string()));
        }
        if p.include_notifications.unwrap_or(false) {
            query.push(("notif", "true".to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/jsonLoad.php", crate::mam::BASE_URL))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::mam::enrich_error(status.as_u16(), &text));
        }

        let parsed: UserDataResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse user data response: {e}"))?;

        Ok(Self::format_user_data(parsed))
    }

    /// Fetch bonus point transaction history for the authenticated user.
    /// Returns a list of transactions with timestamp, type, amount, and associated torrent or user.
    #[tool]
    async fn get_user_bonus_history(
        &self,
        Parameters(p): Parameters<BonusHistoryParams>,
    ) -> Result<String, String> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(types) = &p.bonus_types {
            query.push(("type", types.join(",")));
        }
        if let Some(uid) = p.other_user_id {
            query.push(("other_userid", uid.to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/json/userBonusHistory.php", crate::mam::BASE_URL))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::mam::enrich_error(status.as_u16(), &text));
        }

        let entries: Vec<BonusEntry> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse bonus history response: {e}"))?;

        Ok(Self::format_bonus_history(entries))
    }

    /// Get the current IP address and ASN information as seen by MyAnonamouse.
    #[tool]
    async fn get_ip_info(&self, Parameters(_): Parameters<NoParams>) -> Result<String, String> {
        crate::mam::get_ip_info(&self.client)
            .await
            .map(|info| {
                format!(
                    "IP:           {}\nASN:          {}\nOrganization: {}",
                    info.ip, info.asn_string(), info.as_org
                )
            })
            .map_err(|e| e.to_string())
    }

    /// Register or refresh the current IP as a dynamic seedbox IP on MyAnonamouse.
    /// Rate limited to once per hour by MyAnonamouse.
    #[tool]
    async fn update_seedbox_ip(
        &self,
        Parameters(_): Parameters<NoParams>,
    ) -> Result<String, String> {
        let resp = self
            .client
            .get(format!("{}/json/dynamicSeedbox.php", crate::mam::BASE_URL))
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::mam::enrich_error(status.as_u16(), &text));
        }

        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            let success = v.get("Success").and_then(|s| s.as_bool()).unwrap_or(false);
            let msg = v
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("(no message)");
            let ip = v.get("ip").and_then(|i| i.as_str()).unwrap_or("");
            let asn = v.get("ASN").and_then(|a| a.as_str()).unwrap_or_default();

            if !success {
                return Err(format!(
                    "{msg}\n[Hint: This endpoint is rate-limited to once per hour.]"
                ));
            }

            let mut out = msg.to_string();
            if !ip.is_empty() {
                out.push_str(&format!("\nRegistered IP: {ip}"));
            }
            if !asn.is_empty() {
                out.push_str(&format!("\nASN: {asn}"));
            }
            return Ok(out);
        }

        Ok(text)
    }
}

// ---------------------------------------------------------------------------
// Constructor and helpers
// ---------------------------------------------------------------------------

impl MamServer {
    pub fn new(client: Arc<reqwest::Client>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    // --- Response formatters ---

    fn format_search_response(resp: SearchResponse, query: &str) -> String {
        if resp.data.is_empty() {
            return format!("No results found for \"{}\".", query);
        }

        let mut out = format!(
            "Showing {} of {} result(s) for \"{}\"\n",
            resp.total, resp.found, query
        );

        for (i, t) in resp.data.iter().enumerate() {
            out.push_str(&format!("\n{}. {}\n", i + 1, t.title));

            if let Some(cat) = &t.catname {
                out.push_str(&format!("   Category:  {cat}\n"));
            }
            if let Some(size) = &t.size {
                out.push_str(&format!("   Size:      {size}\n"));
            }

            let authors = t.author_info.as_ref()
                .and_then(|v| v.as_str())
                .map(Self::parse_name_map)
                .unwrap_or_default();
            if !authors.is_empty() {
                out.push_str(&format!("   Authors:   {}\n", authors.join(", ")));
            }

            let narrators = t.narrator_info.as_ref()
                .and_then(|v| v.as_str())
                .map(Self::parse_name_map)
                .unwrap_or_default();
            if !narrators.is_empty() {
                out.push_str(&format!("   Narrators: {}\n", narrators.join(", ")));
            }

            let series = t.series_info.as_ref()
                .and_then(|v| v.as_str())
                .map(Self::parse_series_map)
                .unwrap_or_default();
            if !series.is_empty() {
                out.push_str(&format!("   Series:    {}\n", series.join(", ")));
            }

            if let Some(tags) = t.tags.as_ref().and_then(|v| v.as_str()) {
                if !tags.is_empty() {
                    out.push_str(&format!("   Tags:      {tags}\n"));
                }
            }

            if t.seeders.is_some() || t.leechers.is_some() {
                out.push_str(&format!(
                    "   S/L:       {}/{}\n",
                    t.seeders.map_or("-".to_string(), |n| n.to_string()),
                    t.leechers.map_or("-".to_string(), |n| n.to_string()),
                ));
            }

            let is_free = t.free.unwrap_or(0) == 1;
            let is_vip = t.vip.unwrap_or(0) == 1;
            if is_free || is_vip {
                let flags: Vec<&str> = [is_free.then_some("Free"), is_vip.then_some("VIP")]
                    .into_iter()
                    .flatten()
                    .collect();
                out.push_str(&format!("   Flags:     {}\n", flags.join(", ")));
            }

            if let Some(added) = &t.added {
                out.push_str(&format!("   Added:     {added}\n"));
            }

            out.push_str(&format!("   ID:        {}\n", t.id));

            if let Some(dl) = &t.dl {
                if !dl.is_empty() {
                    out.push_str(&format!("   DL key:    {dl}\n"));
                }
            }
        }

        out
    }

    fn format_user_data(resp: UserDataResponse) -> String {
        let mut out = String::new();

        if let Some(name) = &resp.username {
            out.push_str(&format!("Username:    {name}\n"));
        }
        if let Some(uid) = resp.uid {
            out.push_str(&format!("User ID:     {uid}\n"));
        }
        if let Some(class) = &resp.classname {
            out.push_str(&format!("Class:       {class}\n"));
        }
        if let Some(country) = &resp.country_name {
            out.push_str(&format!("Country:     {country}\n"));
        }
        if let Some(up) = &resp.uploaded {
            out.push_str(&format!("Uploaded:    {up}\n"));
        }
        if let Some(down) = &resp.downloaded {
            out.push_str(&format!("Downloaded:  {down}\n"));
        }
        if let Some(ratio) = resp.ratio {
            out.push_str(&format!("Ratio:       {ratio:.2}\n"));
        }
        if let Some(bonus) = resp.seedbonus {
            out.push_str(&format!("Seed bonus:  {bonus}\n"));
        }
        if let Some(wedges) = resp.wedges {
            out.push_str(&format!("Wedges:      {wedges}\n"));
        }

        if let Some(notifs) = &resp.notifs {
            let count = match notifs {
                Value::Array(arr) => arr.len(),
                Value::Object(map) => map.len(),
                _ => 0,
            };
            if count > 0 {
                out.push_str(&format!("Notifications: {count} unread\n"));
            }
        }

        if out.is_empty() {
            out.push_str("(no data returned)");
        }

        out
    }

    fn format_bonus_history(entries: Vec<BonusEntry>) -> String {
        if entries.is_empty() {
            return "No bonus history found.".to_string();
        }

        let mut out = format!("{} transaction(s):\n", entries.len());

        for entry in &entries {
            // timestamp is unix seconds (with microseconds as fractional part)
            let secs = entry.timestamp as i64;
            let amount = Self::value_as_i64(&entry.amount);
            let sign = if amount >= 0 { "+" } else { "" };

            out.push_str(&format!(
                "\n  [{ts}] {sign}{amount} — {btype}",
                ts = secs,
                sign = sign,
                amount = amount,
                btype = entry.bonus_type,
            ));

            match (&entry.title, &entry.tid) {
                (Some(title), _) if !title.is_empty() => {
                    out.push_str(&format!("\n    Torrent: {title}"));
                }
                (_, Some(tid)) => {
                    out.push_str(&format!("\n    Torrent ID: {}", Self::value_as_str(tid)));
                }
                _ => {}
            }
            match (&entry.other_name, &entry.other_userid) {
                (Some(name), _) if !name.is_empty() => {
                    out.push_str(&format!("\n    User: {name}"));
                }
                (_, Some(uid)) => {
                    out.push_str(&format!("\n    User ID: {}", Self::value_as_str(uid)));
                }
                _ => {}
            }
            out.push('\n');
        }

        out
    }

    // --- Utility helpers ---

    fn value_as_str(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        }
    }

    fn value_as_i64(v: &Value) -> i64 {
        match v {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }

    /// Parse a JSON-encoded map of `{ "id": "name" }` into a sorted list of names.
    fn parse_name_map(json_str: &str) -> Vec<String> {
        let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(json_str) else {
            return vec![];
        };
        let mut names: Vec<String> = map
            .values()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        names.sort();
        names
    }

    /// Parse a JSON-encoded map of `{ "id": ["series name", "pos_str", pos_float] }` into display strings.
    /// pos_float of -1.0 means no position specified.
    fn parse_series_map(json_str: &str) -> Vec<String> {
        let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(json_str) else {
            return vec![];
        };
        let mut series: Vec<String> = map
            .values()
            .filter_map(|v| {
                let arr = v.as_array()?;
                let name = arr.first()?.as_str()?;
                // pos_str is element 1, pos_float is element 2 (-1.0 = unspecified)
                let pos_str = arr.get(1).and_then(|p| p.as_str()).unwrap_or("");
                let pos_float = arr.get(2).and_then(|p| p.as_f64()).unwrap_or(-1.0);
                let pos = if !pos_str.is_empty() {
                    pos_str.to_string()
                } else if pos_float >= 0.0 {
                    pos_float.to_string()
                } else {
                    String::new()
                };
                if pos.is_empty() {
                    Some(name.to_string())
                } else {
                    Some(format!("{name} [{pos}]"))
                }
            })
            .collect();
        series.sort();
        series
    }
}

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for MamServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default().with_server_info(Implementation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
    }
}
