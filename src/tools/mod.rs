// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::sync::Arc;

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo, SetLevelRequestParams},
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MamServer {
    client: Arc<reqwest::Client>,
    tool_router: ToolRouter<Self>,
    enabled_tools: HashSet<String>,
}

// ---------------------------------------------------------------------------
// Parameter types
// ---------------------------------------------------------------------------

/// Strips the `"default"` key from a generated JSON Schema.
/// Applied via `#[schemars(transform = remove_null_default)]` on fields that need `#[serde(default)]`
/// for correct deserialization but should not advertise `"default": null` to LLMs.
fn remove_null_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

/// Deserializes either a single string or an array of strings into `Option<Vec<String>>`.
/// LLMs sometimes pass a bare string even when the schema says array.
mod string_or_vec {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(String),
            Many(Vec<String>),
        }
        let opt = Option::<OneOrMany>::deserialize(deserializer)?;
        Ok(opt.map(|v| match v {
            OneOrMany::One(s) => vec![s],
            OneOrMany::Many(v) => v,
        }))
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchAudiobooksParams {
    /// Search query — matches title, author, narrator, and series name
    query: String,
    /// Genre name (e.g. Fantasy, Mystery). Invalid values return an error listing all valid options.
    /// Multiple genres are OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    genre: Option<Vec<String>>,
    /// Language name or ISO 639-1 code (e.g. "French", "de"). OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    language: Option<Vec<String>>,
    /// Sort order: newest, oldest, most seeders, title a-z, relevance (default).
    sort: Option<String>,
    /// Torrent filter: all (default), active (1+ seeders), inactive, fl (freeleech), fl-VIP, VIP, nVIP.
    search_type: Option<String>,
    /// Minimum seeders (1 excludes dead torrents).
    min_seeders: Option<i32>,
    /// Max results (default 20, max 100).
    limit: Option<u32>,
    /// Pagination offset (default 0).
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchEbooksParams {
    /// Search query — matches title, author, and series name
    query: String,
    /// Genre name (e.g. Fantasy, Science Fiction, Comics). Invalid values return an error listing all valid options.
    /// Multiple genres are OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    genre: Option<Vec<String>>,
    /// Language name or ISO 639-1 code (e.g. "French", "de"). OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    language: Option<Vec<String>>,
    /// Sort order: newest, oldest, most seeders, title a-z, relevance (default).
    sort: Option<String>,
    /// Torrent filter: all (default), active (1+ seeders), inactive, fl (freeleech), fl-VIP, VIP, nVIP.
    search_type: Option<String>,
    /// Minimum seeders (1 excludes dead torrents).
    min_seeders: Option<i32>,
    /// Max results (default 20, max 100).
    limit: Option<u32>,
    /// Pagination offset (default 0).
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchMusicParams {
    /// Search query — matches title and author/composer name
    query: String,
    /// Genre name (e.g. Guitar/Bass Tabs, Sheet Collection, Music Book). Invalid values return an error listing all valid options.
    /// Multiple genres are OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    genre: Option<Vec<String>>,
    /// Language name or ISO 639-1 code (e.g. "French", "de"). OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    language: Option<Vec<String>>,
    /// Sort order: newest, oldest, most seeders, title a-z, relevance (default).
    sort: Option<String>,
    /// Torrent filter: all (default), active (1+ seeders), inactive, fl (freeleech), fl-VIP, VIP, nVIP.
    search_type: Option<String>,
    /// Minimum seeders (1 excludes dead torrents).
    min_seeders: Option<i32>,
    /// Max results (default 20, max 100).
    limit: Option<u32>,
    /// Pagination offset (default 0).
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchRadioParams {
    /// Search query — matches title and series name
    query: String,
    /// Genre name (e.g. Comedy, Drama, Reading). Invalid values return an error listing all valid options.
    /// Multiple genres are OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    genre: Option<Vec<String>>,
    /// Language name or ISO 639-1 code (e.g. "French", "de"). OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    language: Option<Vec<String>>,
    /// Sort order: newest, oldest, most seeders, title a-z, relevance (default).
    sort: Option<String>,
    /// Torrent filter: all (default), active (1+ seeders), inactive, fl (freeleech), fl-VIP, VIP, nVIP.
    search_type: Option<String>,
    /// Minimum seeders (1 excludes dead torrents).
    min_seeders: Option<i32>,
    /// Max results (default 20, max 100).
    limit: Option<u32>,
    /// Pagination offset (default 0).
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Search query text
    query: String,
    /// Max results (default 20, max 100).
    limit: Option<u32>,
    /// Pagination offset (default 0).
    offset: Option<u32>,
    /// Sort order: newest, oldest, most seeders, title a-z, relevance (default).
    sort: Option<String>,
    /// Main category ID: 13 (AudioBooks), 14 (E-Books), 15 (Musicology), 16 (Radio). Omit for all.
    main_cat: Option<Vec<u32>>,
    /// Torrent filter: all (default), active (1+ seeders), inactive, fl (freeleech), fl-VIP, VIP, nVIP.
    search_type: Option<String>,
    /// Language name or ISO 639-1 code (e.g. "French", "de"). OR-combined.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    lang: Option<Vec<String>>,
    /// Minimum seeders (1 excludes dead torrents).
    min_seeders: Option<i32>,
    /// Subcategory ID. Call list_categories for the full table.
    cat: Option<Vec<u32>>,
    /// Fields to search in. Valid values: title, author, description, tags, series, narrator, filenames, fileTypes.
    /// Defaults to title, author, series, narrator.
    #[serde(default, deserialize_with = "string_or_vec::deserialize")]
    #[schemars(transform = remove_null_default)]
    srch_in: Option<Vec<String>>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct GetTop10Params {
    /// Time period for Top 10: day, week, month, year, all (default).
    period: Option<String>,
    /// Main category ID: 13 (AudioBooks), 14 (E-Books), 15 (Musicology), 16 (Radio). Omit for all.
    main_cat: Option<Vec<u32>>,
    /// Subcategory ID. Call list_categories for the full table.
    cat: Option<Vec<u32>>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct UserDataParams {
    /// User ID to fetch data for. Omit to fetch data for the authenticated user.
    user_id: Option<u64>,
    /// Include unread notifications in the response
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
struct GetTorrentDetailsParams {
    /// Torrent ID to look up. Provide either this or `hash`, not both.
    id: Option<u64>,
    /// Torrent info-hash (hex string) to look up. Provide either this or `id`, not both.
    hash: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct NoParams {}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl MamServer {
    /// Search for audiobooks on MyAnonamouse (MAM).
    /// Returns matching torrents with title, authors, narrators, series, size, seeders,
    /// and download URL.
    #[tool(name = "mam_search_audiobooks", title = "Search Audiobooks", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn search_audiobooks(
        &self,
        Parameters(p): Parameters<SearchAudiobooksParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => crate::mam::lookup::lookup_genres(
                genres,
                crate::mam::lookup::AUDIOBOOK_GENRES,
                "Action/Adventure, Art, Biographical, Business, Computer/Internet, Crafts, \
                 Crime/Thriller, Fantasy, Food, General Fiction, General Non-Fiction, \
                 Historical Fiction, History, Home/Garden, Horror, Humor, Instructional, \
                 Juvenile, Language, Literary Classics, Math/Science/Tech, Medical, Mystery, \
                 Nature, Philosophy, Pol/Soc/Relig, Recreation, Romance, Science Fiction, \
                 Self-Help, Travel/Adventure, True Crime, Urban Fantasy, Western, Young Adult",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => crate::mam::lookup::map_languages(langs)?,
            _ => vec![],
        };
        let sort = crate::mam::lookup::parse_sort(p.sort.as_deref().unwrap_or(""))?;
        crate::mam::api::do_search(
            &self.client,
            &p.query,
            vec![13],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
            None,
        )
        .await
    }

    /// Search for ebooks on MyAnonamouse (MAM).
    /// Returns matching torrents with title, authors, series, size, seeders, and download URL.
    #[tool(name = "mam_search_ebooks", title = "Search E-Books", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn search_ebooks(
        &self,
        Parameters(p): Parameters<SearchEbooksParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => crate::mam::lookup::lookup_genres(
                genres,
                crate::mam::lookup::EBOOK_GENRES,
                "Action/Adventure, Art, Biographical, Business, Comics/Graphic Novels, \
                 Computer/Internet, Crafts, Crime/Thriller, Fantasy, Food, General Fiction, \
                 General Non-Fiction, Historical Fiction, History, Home/Garden, Horror, Humor, \
                 Illusion/Magic, Instructional, Juvenile, Language, Literary Classics, \
                 Magazines/Newspapers, Math/Science/Tech, Medical, Mixed Collections, Mystery, \
                 Nature, Philosophy, Pol/Soc/Relig, Recreation, Romance, Science Fiction, \
                 Self-Help, Travel/Adventure, True Crime, Urban Fantasy, Western, Young Adult",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => crate::mam::lookup::map_languages(langs)?,
            _ => vec![],
        };
        let sort = crate::mam::lookup::parse_sort(p.sort.as_deref().unwrap_or(""))?;
        crate::mam::api::do_search(
            &self.client,
            &p.query,
            vec![14],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
            None,
        )
        .await
    }

    /// Search for musicology content on MyAnonamouse (MAM) — sheet music, instructional
    /// media, guitar tabs, music books, and similar resources.
    /// Returns matching torrents with title, size, seeders, and download URL.
    #[tool(name = "mam_search_music", title = "Search Music", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn search_music(
        &self,
        Parameters(p): Parameters<SearchMusicParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => crate::mam::lookup::lookup_genres(
                genres,
                crate::mam::lookup::MUSIC_GENRES,
                "Art, Guitar/Bass Tabs, Individual Sheet, Individual Sheet MP3, \
                 Instructional Media, Lick Library LTP/Jam, Lick Library Techniques, \
                 Music Complete Editions, Music Book, Music Book MP3, Sheet Collection, \
                 Sheet Collection MP3, Instructional Book with Video",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => crate::mam::lookup::map_languages(langs)?,
            _ => vec![],
        };
        let sort = crate::mam::lookup::parse_sort(p.sort.as_deref().unwrap_or(""))?;
        crate::mam::api::do_search(
            &self.client,
            &p.query,
            vec![15],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
            None,
        )
        .await
    }

    /// Search for radio content on MyAnonamouse (MAM) — BBC Radio, podcasts, dramatisations,
    /// comedy recordings, readings, and similar audio programmes.
    /// Returns matching torrents with title, size, seeders, and download URL.
    #[tool(name = "mam_search_radio", title = "Search Radio", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn search_radio(
        &self,
        Parameters(p): Parameters<SearchRadioParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => crate::mam::lookup::lookup_genres(
                genres,
                crate::mam::lookup::RADIO_GENRES,
                "Comedy, Factual/Documentary, Drama, Reading",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => crate::mam::lookup::map_languages(langs)?,
            _ => vec![],
        };
        let sort = crate::mam::lookup::parse_sort(p.sort.as_deref().unwrap_or(""))?;
        crate::mam::api::do_search(
            &self.client,
            &p.query,
            vec![16],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
            None,
        )
        .await
    }

    /// Return the full category and subcategory table for MyAnonamouse.
    /// Use this to look up numeric IDs for the `main_cat` and `cat` parameters of search_torrents.
    /// The per-category search tools (search_audiobooks, search_ebooks, search_music,
    /// search_radio) accept genre names directly and do not require these IDs.
    #[tool(name = "mam_list_categories", title = "List Categories", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn list_categories(
        &self,
        Parameters(_): Parameters<NoParams>,
    ) -> Result<String, String> {
        Ok(crate::mam::format::format_categories())
    }

    /// Search for torrents on MyAnonamouse (MAM) across all categories with full parameter
    /// control. Prefer search_audiobooks, search_ebooks, search_music, or search_radio for
    /// typical searches — this tool is for cross-category queries or advanced filtering.
    #[tool(name = "mam_search_torrents", title = "Search Torrents", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn search_torrents(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<String, String> {
        let sort = crate::mam::lookup::parse_sort(p.sort.as_deref().unwrap_or(""))?;
        let lang = match p.lang.as_deref() {
            Some(names) if !names.is_empty() => crate::mam::lookup::map_languages(names)?,
            _ => vec![],
        };
        crate::mam::api::do_search(
            &self.client,
            &p.query,
            p.main_cat.unwrap_or_default(),
            p.cat.unwrap_or_default(),
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
            p.srch_in,
        )
        .await
    }

    /// Fetch the top 10 most-snatched torrents on MyAnonamouse (MAM).
    /// Can be filtered by category and time period (day, week, month, year, all).
    #[tool(name = "mam_get_top_10", title = "Get Top 10", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn get_top_10(&self, Parameters(p): Parameters<GetTop10Params>) -> Result<String, String> {
        crate::mam::api::get_top_10(
            &self.client,
            p.main_cat.unwrap_or_default(),
            p.cat.unwrap_or_default(),
            p.period.as_deref(),
        )
        .await
    }

    /// Fetch profile data for the authenticated user or another user by ID.
    /// Returns username, class, upload/download stats, ratio, and optionally notifications.
    #[tool(name = "mam_get_user_data", title = "Get User Data", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn get_user_data(
        &self,
        Parameters(p): Parameters<UserDataParams>,
    ) -> Result<String, String> {
        crate::mam::api::get_user_data(
            &self.client,
            p.user_id,
            p.include_notifications.unwrap_or(false),
        )
        .await
    }

    /// Fetch bonus point transaction history for the authenticated user.
    /// Returns a list of transactions with timestamp, type, amount, and associated torrent or user.
    #[tool(name = "mam_get_user_bonus_history", title = "Get Bonus History", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn get_user_bonus_history(
        &self,
        Parameters(p): Parameters<BonusHistoryParams>,
    ) -> Result<String, String> {
        crate::mam::api::get_user_bonus_history(&self.client, p.bonus_types, p.other_user_id)
            .await
    }

    /// Fetch full details for a single torrent by its ID or info-hash.
    /// Returns all available fields: title, category, language, authors, narrators, series,
    /// tags, description, ISBN, media info, file count, size, seeders, leechers, flags,
    /// date added, and download URL.
    /// Use this after finding a torrent ID from a search to get complete information.
    #[tool(name = "mam_get_torrent_details", title = "Get Torrent Details", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn get_torrent_details(
        &self,
        Parameters(p): Parameters<GetTorrentDetailsParams>,
    ) -> Result<String, String> {
        if p.id.is_none() && p.hash.is_none() {
            return Err("Provide either `id` or `hash`.".to_string());
        }
        crate::mam::api::get_torrent_details(&self.client, p.id, p.hash).await
    }

    /// Get the current IP address and ASN information as seen by MyAnonamouse.
    #[tool(name = "mam_get_ip_info", title = "Get IP Info", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true))]
    async fn get_ip_info(&self, Parameters(_): Parameters<NoParams>) -> Result<String, String> {
        crate::mam::get_ip_info(&self.client)
            .await
            .map(|info| {
                format!(
                    "IP:           {}\nASN:          {}\nOrganization: {}",
                    info.ip,
                    info.asn_string(),
                    info.as_org
                )
            })
            .map_err(|e| e.to_string())
    }

    /// Register or refresh the current IP as a dynamic seedbox IP on MyAnonamouse.
    /// Rate limited to once per hour by MyAnonamouse.
    #[tool(name = "mam_update_seedbox_ip", title = "Update Seedbox IP", annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true))]
    async fn update_seedbox_ip(
        &self,
        Parameters(_): Parameters<NoParams>,
    ) -> Result<String, String> {
        crate::mam::api::update_seedbox_ip(&self.client).await
    }
}

// ---------------------------------------------------------------------------
// Registry and constructor
// ---------------------------------------------------------------------------

/// Registry of all tools: (name, group, enabled_by_default).
/// Used by --list-tools and to build the enabled set at startup.
pub const TOOL_REGISTRY: &[(&str, &str, bool)] = &[
    ("mam_search_audiobooks",      "default", true),
    ("mam_search_ebooks",          "default", true),
    ("mam_search_music",           "default", true),
    ("mam_search_radio",           "default", true),
    ("mam_get_torrent_details",    "default", true),
    ("mam_get_top_10",             "default", true),
    ("mam_get_ip_info",            "seedbox", false),
    ("mam_search_torrents",        "power",   false),
    ("mam_list_categories",        "power",   false),
    ("mam_get_user_data",          "user",    false),
    ("mam_get_user_bonus_history", "user",    false),
    ("mam_update_seedbox_ip",      "seedbox", false),
];

/// All tool names known to MamServer.
pub const ALL_TOOL_NAMES: &[&str] = &[
    "mam_search_audiobooks",
    "mam_search_ebooks",
    "mam_search_music",
    "mam_search_radio",
    "mam_get_torrent_details",
    "mam_get_top_10",
    "mam_get_ip_info",
    "mam_search_torrents",
    "mam_list_categories",
    "mam_get_user_data",
    "mam_get_user_bonus_history",
    "mam_update_seedbox_ip",
];

impl MamServer {
    pub fn new(client: Arc<reqwest::Client>, enabled_tools: HashSet<String>) -> Self {
        let mut router = Self::tool_router();
        for name in ALL_TOOL_NAMES {
            if !enabled_tools.contains(*name) {
                router.remove_route(name);
            }
        }
        Self {
            client,
            tool_router: router,
            enabled_tools,
        }
    }
}

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for MamServer {
    fn get_info(&self) -> ServerInfo {
        let mut tool_names: Vec<&str> = TOOL_REGISTRY
            .iter()
            .filter(|(name, _, _)| self.enabled_tools.contains(*name))
            .map(|(name, _, _)| *name)
            .collect();
        tool_names.sort();
        let instructions = format!("Available tools: {}", tool_names.join(", "));
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_logging()
                .build(),
        )
        .with_server_info(Implementation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(instructions)
    }

    async fn set_level(
        &self,
        _request: SetLevelRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        Ok(())
    }
}
