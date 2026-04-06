// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use super::{enrich_error, BASE_URL};

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
) -> Result<String, String> {
    let mut tor = serde_json::json!({
        "text": query,
        "srchIn": ["title", "author", "narrator", "series"],
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
                    return Ok(r#"{"data":[],"total":0,"found":0}"#.to_string());
                }
                return Err(format!("Search error: {msg}"));
            }
        }
    }

    Ok(text)
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

    resp.text().await.map_err(|e| format!("Failed to read user data response: {e}"))
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

    resp.text().await.map_err(|e| format!("Failed to read bonus history response: {e}"))
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

    let text = resp.text().await.map_err(|e| format!("Failed to read response: {e}"))?;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if v.get("data").is_none() {
            if let Some(msg) = v.get("error").and_then(|e| e.as_str()) {
                if msg.contains("Nothing returned") {
                    return Ok(r#"{"data":[],"total":0,"found":0}"#.to_string());
                }
                return Err(format!("Lookup error: {msg}"));
            }
        }
    }

    Ok(text)
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
        if !success {
            let msg = v
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("(no message)");
            return Err(format!(
                "{msg}\n[Hint: This endpoint is rate-limited to once per hour.]"
            ));
        }
    }

    Ok(text)
}
