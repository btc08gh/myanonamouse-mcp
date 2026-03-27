/// mam-count-languages: query MyAnonamouse for the total torrent count per language
/// and print a ranked table to stdout.
///
/// Usage:
///   mam-count-languages --mam-session <mam_id>
///   MAM_SESSION=<mam_id> mam-count-languages
///
/// Each of the 64 MAM language IDs is queried in turn. A small delay is inserted
/// between requests to avoid hammering the site.
use std::time::Duration;

use anyhow::anyhow;
use clap::Parser;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

const BASE_URL: &str = "https://www.myanonamouse.net";

/// All 64 MAM language IDs with their display names.
const LANGUAGES: &[(u32, &str)] = &[
    (1, "English"),
    (2, "Chinese"),
    (3, "Gujarati"),
    (4, "Spanish"),
    (5, "Kannada"),
    (6, "Burmese"),
    (7, "Thai"),
    (8, "Hindi"),
    (9, "Marathi"),
    (10, "Telugu"),
    (11, "Tamil"),
    (12, "Javanese"),
    (13, "Vietnamese"),
    (14, "Punjabi"),
    (15, "Urdu"),
    (16, "Russian"),
    (17, "Afrikaans"),
    (18, "Bulgarian"),
    (19, "Catalan"),
    (20, "Czech"),
    (21, "Danish"),
    (22, "Dutch"),
    (23, "Finnish"),
    (24, "Scottish Gaelic"),
    (25, "Ukrainian"),
    (26, "Greek"),
    (27, "Hebrew"),
    (28, "Hungarian"),
    (29, "Tagalog"),
    (30, "Romanian"),
    (31, "Serbian"),
    (32, "Arabic"),
    (33, "Malay"),
    (34, "Portuguese"),
    (35, "Bengali"),
    (36, "French"),
    (37, "German"),
    (38, "Japanese"),
    (39, "Farsi"),
    (40, "Swedish"),
    (41, "Korean"),
    (42, "Turkish"),
    (43, "Italian"),
    (44, "Cantonese"),
    (45, "Polish"),
    (46, "Latin"),
    (47, "Other"),
    (48, "Norwegian"),
    (49, "Croatian"),
    (50, "Lithuanian"),
    (51, "Bosnian"),
    (52, "Brazilian Portuguese"),
    (53, "Indonesian"),
    (54, "Slovenian"),
    (55, "Castilian Spanish"),
    (56, "Irish"),
    (57, "Manx"),
    (58, "Malayalam"),
    (59, "Ancient Greek"),
    (60, "Sanskrit"),
    (61, "Estonian"),
    (62, "Latvian"),
    (63, "Icelandic"),
    (64, "Albanian"),
];

#[derive(Parser)]
#[command(
    name = "mam-count-languages",
    about = "Count MAM torrents per language and print a ranked table"
)]
struct Cli {
    /// MyAnonamouse session cookie value (mam_id)
    #[arg(long, env = "MAM_SESSION")]
    mam_session: String,

    /// Delay between requests in milliseconds (default: 500)
    #[arg(long, default_value_t = 500)]
    delay_ms: u64,

    /// Only show languages with at least this many torrents
    #[arg(long, default_value_t = 0)]
    min_count: u64,
}

#[derive(Deserialize)]
struct SearchResponse {
    found: Option<u64>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Build client with mam_id cookie
    let mut headers = HeaderMap::new();
    let cookie = HeaderValue::from_str(&format!("mam_id={}", cli.mam_session))
        .map_err(|_| anyhow!("Invalid mam_session value"))?;
    headers.insert(header::COOKIE, cookie);
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .default_headers(headers)
        .build()?;

    let delay = Duration::from_millis(cli.delay_ms);
    let total_langs = LANGUAGES.len();

    eprintln!("Querying {} languages ({}ms delay between requests)...", total_langs, cli.delay_ms);

    let mut results: Vec<(u32, &str, u64)> = Vec::with_capacity(total_langs);

    for (i, &(id, name)) in LANGUAGES.iter().enumerate() {
        eprint!("  [{:>2}/{total_langs}] {name:<22} ", i + 1);

        let body = serde_json::json!({
            "tor": {
                "text": "",
                "srchIn": ["title"],
                "searchType": "all",
                "searchIn": "torrents",
                "main_cat": [],
                "cat": [],
                "browse_lang": [id],
                "startNumber": "0",
                "perpage": 1,
            }
        });

        let resp = client
            .post(format!("{BASE_URL}/tor/js/loadSearchJSONbasic.php"))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        let count = if !status.is_success() {
            eprintln!("HTTP {status}");
            return Err(anyhow!("Request failed for language {id} ({name}): HTTP {status}: {text}"));
        } else if let Ok(parsed) = serde_json::from_str::<SearchResponse>(&text) {
            if let Some(err) = &parsed.error {
                if err.contains("Nothing returned") {
                    0
                } else {
                    eprintln!("error: {err}");
                    return Err(anyhow!("API error for language {id} ({name}): {err}"));
                }
            } else {
                parsed.found.unwrap_or(0)
            }
        } else {
            eprintln!("parse error");
            return Err(anyhow!("Failed to parse response for language {id} ({name}): {text}"));
        };

        eprintln!("{count}");
        results.push((id, name, count));

        if i + 1 < total_langs {
            tokio::time::sleep(delay).await;
        }
    }

    // Sort by count descending, then name ascending for ties
    results.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)));

    let total: u64 = results.iter().map(|(_, _, c)| c).sum();

    println!();
    println!("{:<5}  {:<24}  {:>8}  {:>6}", "Rank", "Language", "Torrents", "Share");
    println!("{}", "-".repeat(52));

    let mut rank = 0u32;
    for (id, name, count) in &results {
        if *count < cli.min_count {
            continue;
        }
        rank += 1;
        let share = if total > 0 {
            (*count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{rank:<5}  {name:<24}  {:>8}  {:>5.1}%   (id={id})",
            count,
            share,
        );
    }

    println!("{}", "-".repeat(52));
    println!("{:<5}  {:<24}  {:>8}", "", "Total", total);

    Ok(())
}
