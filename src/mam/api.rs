// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use serde::Deserialize;

use super::{enrich_error, BASE_URL};
use super::format::{
    format_bonus_history, format_search_response, format_torrent_detail, format_user_data,
};
use super::types::{BonusEntry, SearchResponse, TorrentDetail, UserDataResponse};

pub(crate) async fn do_search(
    client: &reqwest::Client,
    query: &str,
    main_cat: Vec<u32>,
    cat: Vec<u32>,
    lang: Vec<u32>,
    sort_type: &str,
    search_type: &str,
    min_seeders: Option<i32>,
    limit: u32,
    offset: u32,
    srch_in: Option<Vec<String>>,
) -> Result<String, String> {
    let srch_in_val = if let Some(fields) = srch_in {
        serde_json::json!(fields)
    } else {
        serde_json::json!(["title", "author", "narrator", "series"])
    };

    let mut tor = serde_json::json!({
        "text": query,
        "srchIn": srch_in_val,
        "searchType": search_type,
        "searchIn": "torrents",
        "main_cat": main_cat,
        "cat": cat,
        "browseFlagsHideVsShow": "0",
        "startDate": "",
        "endDate": "",
        "hash": "",
        "sortType": sort_type,
        "startNumber": offset,
    });
    // Omit browse_lang when empty — sending [] breaks the MAM search engine
    if !lang.is_empty() {
        tor["browse_lang"] = serde_json::json!(lang);
    }
    if let Some(min) = min_seeders {
        tor["minSeeders"] = serde_json::json!(min);
    }
    let body = serde_json::json!({
        "tor": tor,
        "perpage": limit,
        "dlLink": "true",
        "thumbnail": "false",
    });

    let resp = client
        .post(format!("{BASE_URL}/tor/js/loadSearchJSONbasic.php"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_error(status.as_u16(), &text));
    }

    let text = resp.text().await.map_err(|e| format!("Failed to read search response: {e}"))?;

    // MAM returns {"error":"Nothing returned, out of 0"} for empty result sets
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if v.get("data").is_none() {
            if let Some(msg) = v.get("error").and_then(|e| e.as_str()) {
                if msg.contains("Nothing returned") {
                    return Ok(format!("No results found for \"{query}\"."));
                }
                return Err(format!("Search error: {msg}"));
            }
        }
    }

    let parsed: SearchResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse search response: {e}\nBody: {text}"))?;

    Ok(format_search_response(parsed, query))
}

pub(crate) async fn get_user_data(
    client: &reqwest::Client,
    user_id: Option<u64>,
    include_notifications: bool,
) -> Result<String, String> {
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(id) = user_id {
        query.push(("id", id.to_string()));
    }
    if include_notifications {
        query.push(("notif", "true".to_string()));
    }

    let resp = client
        .get(format!("{BASE_URL}/jsonLoad.php"))
        .query(&query)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_error(status.as_u16(), &text));
    }

    let parsed: UserDataResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse user data response: {e}"))?;

    Ok(format_user_data(parsed))
}

pub(crate) async fn get_user_bonus_history(
    client: &reqwest::Client,
    bonus_types: Option<Vec<String>>,
    other_user_id: Option<u64>,
) -> Result<String, String> {
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(types) = &bonus_types {
        query.push(("type", types.join(",")));
    }
    if let Some(uid) = other_user_id {
        query.push(("other_userid", uid.to_string()));
    }

    let resp = client
        .get(format!("{BASE_URL}/json/userBonusHistory.php"))
        .query(&query)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_error(status.as_u16(), &text));
    }

    let entries: Vec<BonusEntry> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse bonus history response: {e}"))?;

    Ok(format_bonus_history(entries))
}

pub(crate) async fn get_torrent_details(
    client: &reqwest::Client,
    id: Option<u64>,
    hash: Option<String>,
) -> Result<String, String> {
    let mut tor = serde_json::json!({
        "searchType": "all",
        "searchIn": "torrents",
        "startNumber": "0",
        "perpage": 1,
    });
    if let Some(id) = id {
        tor["id"] = serde_json::json!(id);
    }
    if let Some(hash) = &hash {
        tor["hash"] = serde_json::json!(hash);
    }

    let body = serde_json::json!({
        "tor": tor,
        "dlLink": "true",
        "description": "",
        "isbn": "",
        "mediaInfo": "",
    });

    let resp = client
        .post(format!("{BASE_URL}/tor/js/loadSearchJSONbasic.php"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_error(status.as_u16(), &text));
    }

    let body = resp.text().await.map_err(|e| format!("Failed to read response: {e}"))?;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        if v.get("data").is_none() {
            if let Some(msg) = v.get("error").and_then(|e| e.as_str()) {
                if msg.contains("Nothing returned") {
                    return Ok("No torrent found.".to_string());
                }
                return Err(format!("Lookup error: {msg}"));
            }
        }
    }

    #[derive(Deserialize)]
    struct DetailResponse {
        data: Vec<TorrentDetail>,
    }
    let parsed: DetailResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {e}\nBody: {body}"))?;

    match parsed.data.into_iter().next() {
        None => Ok("No torrent found.".to_string()),
        Some(t) => Ok(format_torrent_detail(t)),
    }
}

pub(crate) async fn get_top_10(
    client: &reqwest::Client,
    main_cat: Vec<u32>,
    cat: Vec<u32>,
    period: Option<&str>,
) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp();
    let start_date = match period {
        Some("day") => (now - 86400).to_string(),
        Some("week") => (now - 7 * 86400).to_string(),
        Some("month") => (now - 30 * 86400).to_string(),
        Some("year") => (now - 365 * 86400).to_string(),
        _ => "".to_string(),
    };
    let end_date = if start_date.is_empty() {
        "".to_string()
    } else {
        now.to_string()
    };

    let mut tor = serde_json::json!({
        "text": "",
        "searchType": "all",
        "searchIn": "torrents",
        "main_cat": main_cat,
        "cat": cat,
        "sortType": "snatchedDesc",
        "startNumber": 0,
        "startDate": start_date,
        "endDate": end_date,
    });

    if cat.is_empty() && main_cat.is_empty() {
        tor["cat"] = serde_json::json!([0]);
    }

    let body = serde_json::json!({
        "tor": tor,
        "perpage": 10,
        "dlLink": "true",
        "bookmarks": "true",
    });

    let resp = client
        .post(format!("{BASE_URL}/tor/js/loadSearchJSONbasic.php"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_error(status.as_u16(), &text));
    }

    let text = resp.text().await.map_err(|e| format!("Failed to read response: {e}"))?;
    let parsed: SearchResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse response: {e}\nBody: {text}"))?;

    let period_label = match period {
        Some("day") => "past day",
        Some("week") => "past week",
        Some("month") => "past month",
        Some("year") => "past year",
        _ => "all time",
    };
    Ok(format_search_response(parsed, &format!("Top 10 ({period_label})")))
}

pub(crate) async fn update_seedbox_ip(client: &reqwest::Client) -> Result<String, String> {
    let resp = client
        .get(format!("{BASE_URL}/json/dynamicSeedbox.php"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(enrich_error(status.as_u16(), &text));
    }

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
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
