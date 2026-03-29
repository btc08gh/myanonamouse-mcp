use std::collections::HashSet;
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
struct TorrentDetail {
    id: u64,
    title: String,
    catname: Option<String>,
    lang_code: Option<String>,
    size: Option<String>,
    numfiles: Option<u64>,
    filetype: Option<String>,
    author_info: Option<Value>,
    narrator_info: Option<Value>,
    series_info: Option<Value>,
    tags: Option<Value>,
    description: Option<String>,
    isbn: Option<Value>, // API returns integer or string
    mediainfo: Option<String>,
    seeders: Option<u64>,
    leechers: Option<u64>,
    times_completed: Option<u64>,
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
// Genre / language / sort lookup tables
// ---------------------------------------------------------------------------

/// Normalize a lookup key: lowercase, replace non-alphanumeric with space, collapse whitespace.
fn normalize_lookup(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

const AUDIOBOOK_GENRES: &[(&str, u32)] = &[
    ("action adventure", 39), ("action", 39), ("adventure", 39),
    ("art", 49),
    ("biographical", 50), ("biography", 50), ("biographies", 50),
    ("business", 83),
    ("computer internet", 51), ("computer", 51), ("internet", 51),
    ("crafts", 97),
    ("crime thriller", 40), ("crime", 40), ("thriller", 40),
    ("fantasy", 41),
    ("food", 106), ("cooking", 106), ("culinary", 106),
    ("general fiction", 42), ("fiction", 42),
    ("general non fiction", 52), ("non fiction", 52), ("nonfiction", 52),
    ("historical fiction", 98),
    ("history", 54), ("historical", 54),
    ("home garden", 55), ("home", 55), ("garden", 55),
    ("horror", 43),
    ("humor", 99), ("humour", 99), ("comedy", 99),
    ("instructional", 84),
    ("juvenile", 44), ("children", 44), ("kids", 44),
    ("language", 56), ("languages", 56), ("linguistics", 56),
    ("literary classics", 45), ("classics", 45), ("classic literature", 45),
    ("math science tech", 57), ("math", 57), ("mathematics", 57), ("science", 57), ("technology", 57),
    ("medical", 85), ("medicine", 85), ("health", 85),
    ("mystery", 87), ("mysteries", 87), ("detective", 87),
    ("nature", 119),
    ("philosophy", 88), ("philosophical", 88),
    ("pol soc relig", 58), ("politics", 58), ("political", 58), ("social", 58), ("religion", 58), ("religious", 58), ("sociology", 58),
    ("recreation", 59), ("sports", 59), ("leisure", 59),
    ("romance", 46),
    ("science fiction", 47), ("sci fi", 47), ("scifi", 47), ("sf", 47),
    ("self help", 53), ("self improvement", 53), ("personal development", 53),
    ("travel adventure", 89), ("travel", 89),
    ("true crime", 100),
    ("urban fantasy", 108),
    ("western", 48),
    ("young adult", 111), ("ya", 111), ("teen", 111),
];

const EBOOK_GENRES: &[(&str, u32)] = &[
    ("action adventure", 60), ("action", 60), ("adventure", 60),
    ("art", 71),
    ("biographical", 72), ("biography", 72), ("biographies", 72),
    ("business", 90),
    ("comics graphic novels", 61), ("comics", 61), ("graphic novels", 61), ("manga", 61),
    ("computer internet", 73), ("computer", 73), ("internet", 73),
    ("crafts", 101),
    ("crime thriller", 62), ("crime", 62), ("thriller", 62),
    ("fantasy", 63),
    ("food", 107), ("cooking", 107), ("culinary", 107),
    ("general fiction", 64), ("fiction", 64),
    ("general non fiction", 74), ("non fiction", 74), ("nonfiction", 74),
    ("historical fiction", 102),
    ("history", 76), ("historical", 76),
    ("home garden", 77), ("home", 77), ("garden", 77),
    ("horror", 65),
    ("humor", 103), ("humour", 103), ("comedy", 103),
    ("illusion magic", 115), ("illusion", 115), ("magic", 115), ("mentalism", 115),
    ("instructional", 91),
    ("juvenile", 66), ("children", 66), ("kids", 66),
    ("language", 78), ("languages", 78), ("linguistics", 78),
    ("literary classics", 67), ("classics", 67), ("classic literature", 67),
    ("magazines newspapers", 79), ("magazines", 79), ("newspapers", 79), ("periodicals", 79),
    ("math science tech", 80), ("math", 80), ("mathematics", 80), ("science", 80), ("technology", 80),
    ("medical", 92), ("medicine", 92), ("health", 92),
    ("mixed collections", 118), ("collections", 118), ("anthology", 118),
    ("mystery", 94), ("mysteries", 94), ("detective", 94),
    ("nature", 120),
    ("philosophy", 95), ("philosophical", 95),
    ("pol soc relig", 81), ("politics", 81), ("political", 81), ("social", 81), ("religion", 81), ("religious", 81), ("sociology", 81),
    ("recreation", 82), ("sports", 82), ("leisure", 82),
    ("romance", 68),
    ("science fiction", 69), ("sci fi", 69), ("scifi", 69), ("sf", 69),
    ("self help", 75), ("self improvement", 75), ("personal development", 75),
    ("travel adventure", 96), ("travel", 96),
    ("true crime", 104),
    ("urban fantasy", 109),
    ("western", 70),
    ("young adult", 112), ("ya", 112), ("teen", 112),
];

const MUSIC_GENRES: &[(&str, u32)] = &[
    ("art", 49),
    ("guitar bass tabs", 19), ("guitar", 19), ("bass tabs", 19), ("tabs", 19),
    ("individual sheet", 20), ("sheet music", 20),
    ("individual sheet mp3", 24),
    ("instructional media", 22), ("instructional", 22),
    ("lick library ltp jam", 113), ("lick library", 113), ("ltp", 113),
    ("lick library techniques", 114),
    ("music complete editions", 17), ("complete editions", 17),
    ("music book", 26),
    ("music book mp3", 27),
    ("sheet collection", 30),
    ("sheet collection mp3", 31),
    ("instructional book with video", 126), ("instructional book", 126),
];

const RADIO_GENRES: &[(&str, u32)] = &[
    ("comedy", 127),
    ("factual documentary", 128), ("factual", 128), ("documentary", 128),
    ("drama", 130),
    ("reading", 132), ("readings", 132), ("spoken word", 132),
];

const LANGUAGES: &[(&str, u32)] = &[
    ("english", 1), ("en", 1),
    ("chinese", 2), ("mandarin", 2), ("zh", 2),
    ("gujarati", 3),
    ("spanish", 4), ("es", 4), ("espanol", 4),
    ("kannada", 5),
    ("burmese", 6), ("myanmar", 6),
    ("thai", 7), ("th", 7),
    ("hindi", 8), ("hi", 8),
    ("marathi", 9),
    ("telugu", 10),
    ("tamil", 11),
    ("javanese", 12),
    ("vietnamese", 13), ("vi", 13),
    ("punjabi", 14),
    ("urdu", 15),
    ("russian", 16), ("ru", 16),
    ("afrikaans", 17),
    ("bulgarian", 18), ("bg", 18),
    ("catalan", 19), ("ca", 19),
    ("czech", 20), ("cs", 20),
    ("danish", 21), ("da", 21),
    ("dutch", 22), ("nl", 22),
    ("finnish", 23), ("fi", 23),
    ("scottish gaelic", 24), ("gaelic", 24),
    ("ukrainian", 25), ("uk", 25),
    ("greek", 26), ("el", 26),
    ("hebrew", 27), ("he", 27),
    ("hungarian", 28), ("hu", 28),
    ("tagalog", 29), ("filipino", 29),
    ("romanian", 30), ("ro", 30),
    ("serbian", 31), ("sr", 31),
    ("arabic", 32), ("ar", 32),
    ("malay", 33), ("ms", 33),
    ("portuguese", 34), ("pt", 34),
    ("bengali", 35), ("bn", 35),
    ("french", 36), ("fr", 36), ("francais", 36),
    ("german", 37), ("de", 37), ("deutsch", 37),
    ("japanese", 38), ("ja", 38),
    ("farsi", 39), ("persian", 39), ("fa", 39),
    ("swedish", 40), ("sv", 40),
    ("korean", 41), ("ko", 41),
    ("turkish", 42), ("tr", 42),
    ("italian", 43), ("it", 43),
    ("cantonese", 44),
    ("polish", 45), ("pl", 45),
    ("latin", 46), ("la", 46),
    ("other", 47),
    ("norwegian", 48), ("no", 48), ("norsk", 48),
    ("croatian", 49), ("hr", 49),
    ("lithuanian", 50), ("lt", 50),
    ("bosnian", 51), ("bs", 51),
    ("brazilian portuguese", 52), ("pt br", 52), ("brazilian", 52), ("brazil", 52),
    ("indonesian", 53), ("id", 53),
    ("slovenian", 54), ("sl", 54), ("slovene", 54),
    ("castilian spanish", 55), ("castilian", 55), ("castellano", 55),
    ("irish", 56), ("ga", 56), ("irish gaelic", 56),
    ("manx", 57),
    ("malayalam", 58),
    ("ancient greek", 59), ("greek ancient", 59),
    ("sanskrit", 60),
    ("estonian", 61), ("et", 61),
    ("latvian", 62), ("lv", 62),
    ("icelandic", 63), ("is", 63),
    ("albanian", 64), ("sq", 64),
];

/// Map a slice of genre name strings to cat IDs using the given table.
/// Returns Err with a helpful message listing valid names if any name is unrecognized.
fn lookup_genres(
    names: &[String],
    table: &[(&str, u32)],
    valid_list: &str,
) -> Result<Vec<u32>, String> {
    let mut ids: Vec<u32> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for name in names {
        let key = normalize_lookup(name);
        if let Some(&(_, id)) = table.iter().find(|(k, _)| *k == key.as_str()) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        } else {
            unknown.push(name.clone());
        }
    }
    if unknown.is_empty() {
        Ok(ids)
    } else {
        Err(format!(
            "Unrecognized genre(s): {}. Valid genres: {}",
            unknown.join(", "),
            valid_list,
        ))
    }
}

/// Map a slice of language name strings to language IDs.
/// Returns Err with a helpful message if any name is unrecognized.
fn map_languages(names: &[String]) -> Result<Vec<u32>, String> {
    let mut ids: Vec<u32> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for name in names {
        let key = normalize_lookup(name);
        if let Some(&(_, id)) = LANGUAGES.iter().find(|(k, _)| *k == key.as_str()) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        } else {
            unknown.push(name.clone());
        }
    }
    if unknown.is_empty() {
        Ok(ids)
    } else {
        Err(format!(
            "Unrecognized language(s): {}. Most common languages on MAM: English, German, \
             French, Spanish, Italian. Many more are supported — pass any standard language \
             name (e.g. \"Dutch\", \"Japanese\", \"Portuguese\") or ISO 639-1 code (e.g. \"de\", \"fr\").",
            unknown.join(", "),
        ))
    }
}

/// Map a natural-language or raw-API sort string to the MAM API sort value.
/// Accepts both human-readable forms ("newest", "most seeders") and raw API
/// strings ("dateDesc", "seedersDesc") for backward compatibility.
fn parse_sort(s: &str) -> Result<&'static str, String> {
    match normalize_lookup(s).as_str() {
        // Natural language + raw API aliases
        "newest" | "date desc" | "date added descending" | "datedesc" => Ok("dateDesc"),
        "oldest" | "date asc" | "date added ascending" | "dateasc" => Ok("dateAsc"),
        "most seeders" | "seeders desc" | "seeders descending" | "seedersdesc" => Ok("seedersDesc"),
        "fewest seeders" | "seeders asc" | "seeders ascending" | "seedersasc" => Ok("seedersAsc"),
        "most leechers" | "leechers desc" | "leechersdesc" => Ok("leechersDesc"),
        "fewest leechers" | "leechers asc" | "leechersasc" => Ok("leechersAsc"),
        "title a z" | "title ascending" | "title asc" | "titleasc" => Ok("titleAsc"),
        "title z a" | "title descending" | "title desc" | "titledesc" => Ok("titleDesc"),
        "largest" | "size desc" | "size descending" | "sizedesc" => Ok("sizeDesc"),
        "smallest" | "size asc" | "size ascending" | "sizeasc" => Ok("sizeAsc"),
        "most snatched" | "snatched desc" | "snatcheddesc" => Ok("snatchedDesc"),
        "least snatched" | "snatched asc" | "snatched ascending" | "snatchedasc" => Ok("snatchedAsc"),
        "most files" | "files desc" | "filesdesc" => Ok("fileDesc"),
        "fewest files" | "files asc" | "filesasc" => Ok("fileAsc"),
        "category a z" | "category asc" | "categoryasc" => Ok("categoryAsc"),
        "category z a" | "category desc" | "categorydesc" => Ok("categoryDesc"),
        "random" => Ok("random"),
        "relevance" | "default" | "" => Ok("default"),
        _ => Err(format!(
            "Unknown sort \"{s}\". Valid values: newest, oldest, most seeders, fewest seeders, \
             most leechers, fewest leechers, title a-z, title z-a, largest, smallest, \
             most snatched, least snatched, most files, fewest files, \
             category a-z, category z-a, random, relevance."
        )),
    }
}

// ---------------------------------------------------------------------------
// Parameter types
// ---------------------------------------------------------------------------

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchAudiobooksParams {
    /// Search query — matches title, author, narrator, and series name
    query: String,
    /// Genre filter (case-insensitive). Valid genres: Action/Adventure, Art, Biographical,
    /// Business, Computer/Internet, Crafts, Crime/Thriller, Fantasy, Food, General Fiction,
    /// General Non-Fiction, Historical Fiction, History, Home/Garden, Horror, Humor,
    /// Instructional, Juvenile, Language, Literary Classics, Math/Science/Tech, Medical,
    /// Mystery, Nature, Philosophy, Pol/Soc/Relig, Recreation, Romance, Science Fiction,
    /// Self-Help, Travel/Adventure, True Crime, Urban Fantasy, Western, Young Adult.
    /// Multiple genres are OR-combined.
    #[serde(default)]
    genre: Option<Vec<String>>,
    /// Language filter. Top languages on MAM: "English", "German", "French", "Spanish", "Italian".
    /// Many more are supported — pass any standard language name or ISO 639-1 code (e.g. "de", "fr").
    /// Multiple languages are OR-combined.
    #[serde(default)]
    language: Option<Vec<String>>,
    /// Sort order (case-insensitive). Valid values: newest, oldest, most seeders, fewest seeders,
    /// title a-z, title z-a, largest, smallest, most snatched, most leechers, random,
    /// relevance (default).
    #[serde(default)]
    sort: Option<String>,
    /// Filter by torrent type. Valid values: all (default, includes dead torrents),
    /// active (1+ seeders), inactive (0 seeders), fl (freeleech), fl-VIP (freeleech or VIP),
    /// VIP, nVIP (not VIP).
    #[serde(default)]
    search_type: Option<String>,
    /// Minimum number of seeders (inclusive). Use 1 to exclude dead torrents.
    #[serde(default)]
    min_seeders: Option<i32>,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Result offset for pagination (default: 0)
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchEbooksParams {
    /// Search query — matches title, author, and series name
    query: String,
    /// Genre filter (case-insensitive). Valid genres: Action/Adventure, Art, Biographical,
    /// Business, Comics/Graphic Novels, Computer/Internet, Crafts, Crime/Thriller, Fantasy,
    /// Food, General Fiction, General Non-Fiction, Historical Fiction, History, Home/Garden,
    /// Horror, Humor, Illusion/Magic, Instructional, Juvenile, Language, Literary Classics,
    /// Magazines/Newspapers, Math/Science/Tech, Medical, Mixed Collections, Mystery, Nature,
    /// Philosophy, Pol/Soc/Relig, Recreation, Romance, Science Fiction, Self-Help,
    /// Travel/Adventure, True Crime, Urban Fantasy, Western, Young Adult.
    /// Multiple genres are OR-combined.
    #[serde(default)]
    genre: Option<Vec<String>>,
    /// Language filter. Top languages on MAM: "English", "German", "French", "Spanish", "Italian".
    /// Many more are supported — pass any standard language name or ISO 639-1 code (e.g. "de", "fr").
    /// Multiple languages are OR-combined.
    #[serde(default)]
    language: Option<Vec<String>>,
    /// Sort order (case-insensitive). Valid values: newest, oldest, most seeders, fewest seeders,
    /// title a-z, title z-a, largest, smallest, most snatched, most leechers, random,
    /// relevance (default).
    #[serde(default)]
    sort: Option<String>,
    /// Filter by torrent type. Valid values: all (default, includes dead torrents),
    /// active (1+ seeders), inactive (0 seeders), fl (freeleech), fl-VIP (freeleech or VIP),
    /// VIP, nVIP (not VIP).
    #[serde(default)]
    search_type: Option<String>,
    /// Minimum number of seeders (inclusive). Use 1 to exclude dead torrents.
    #[serde(default)]
    min_seeders: Option<i32>,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Result offset for pagination (default: 0)
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchMusicParams {
    /// Search query — matches title and author/composer name
    query: String,
    /// Genre filter (case-insensitive). Valid genres: Art, Guitar/Bass Tabs, Individual Sheet,
    /// Individual Sheet MP3, Instructional Media, Lick Library LTP/Jam,
    /// Lick Library Techniques, Music Complete Editions, Music Book, Music Book MP3,
    /// Sheet Collection, Sheet Collection MP3, Instructional Book with Video.
    /// Multiple genres are OR-combined.
    #[serde(default)]
    genre: Option<Vec<String>>,
    /// Language filter. Top languages on MAM: "English", "German", "French", "Spanish", "Italian".
    /// Many more are supported — pass any standard language name or ISO 639-1 code (e.g. "de", "fr").
    /// Multiple languages are OR-combined.
    #[serde(default)]
    language: Option<Vec<String>>,
    /// Sort order (case-insensitive). Valid values: newest, oldest, most seeders, fewest seeders,
    /// title a-z, title z-a, largest, smallest, most snatched, most leechers, random,
    /// relevance (default).
    #[serde(default)]
    sort: Option<String>,
    /// Filter by torrent type. Valid values: all (default, includes dead torrents),
    /// active (1+ seeders), inactive (0 seeders), fl (freeleech), fl-VIP (freeleech or VIP),
    /// VIP, nVIP (not VIP).
    #[serde(default)]
    search_type: Option<String>,
    /// Minimum number of seeders (inclusive). Use 1 to exclude dead torrents.
    #[serde(default)]
    min_seeders: Option<i32>,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Result offset for pagination (default: 0)
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchRadioParams {
    /// Search query — matches title and series name
    query: String,
    /// Genre filter (case-insensitive). Valid genres: Comedy, Factual/Documentary, Drama,
    /// Reading. Multiple genres are OR-combined.
    #[serde(default)]
    genre: Option<Vec<String>>,
    /// Language filter. Top languages on MAM: "English", "German", "French", "Spanish", "Italian".
    /// Many more are supported — pass any standard language name or ISO 639-1 code (e.g. "de", "fr").
    /// Multiple languages are OR-combined.
    #[serde(default)]
    language: Option<Vec<String>>,
    /// Sort order (case-insensitive). Valid values: newest, oldest, most seeders, fewest seeders,
    /// title a-z, title z-a, largest, smallest, most snatched, most leechers, random,
    /// relevance (default).
    #[serde(default)]
    sort: Option<String>,
    /// Filter by torrent type. Valid values: all (default, includes dead torrents),
    /// active (1+ seeders), inactive (0 seeders), fl (freeleech), fl-VIP (freeleech or VIP),
    /// VIP, nVIP (not VIP).
    #[serde(default)]
    search_type: Option<String>,
    /// Minimum number of seeders (inclusive). Use 1 to exclude dead torrents.
    #[serde(default)]
    min_seeders: Option<i32>,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Result offset for pagination (default: 0)
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Search query text
    query: String,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default)]
    limit: Option<u32>,
    /// Result offset for pagination (default: 0)
    #[serde(default)]
    offset: Option<u32>,
    /// Sort order. Accepts natural language ("newest", "most seeders", "title a-z") or
    /// raw API strings ("dateDesc", "seedersDesc"). Defaults to relevance.
    /// Full list: newest, oldest, most seeders, fewest seeders, most leechers, fewest leechers,
    /// title a-z, title z-a, largest, smallest, most snatched, least snatched,
    /// most files, fewest files, category a-z, category z-a, random, relevance.
    #[serde(default)]
    sort: Option<String>,
    /// Filter by main category ID. Valid values: 13 (AudioBooks), 14 (E-Books),
    /// 15 (Musicology), 16 (Radio). Omit to search all categories.
    /// Use `cat` instead when you know the specific genre.
    #[serde(default)]
    main_cat: Option<Vec<u32>>,
    /// Filter by torrent type. Valid values:
    /// "all" (default, includes dead torrents), "active" (1+ seeders), "inactive" (0 seeders),
    /// "fl" (freeleech), "fl-VIP" (freeleech or VIP), "VIP", "nVIP" (not VIP).
    #[serde(default)]
    search_type: Option<String>,
    /// Language filter. Top languages on MAM: "English", "German", "French", "Spanish", "Italian".
    /// Many more are supported — pass any standard language name or ISO 639-1 code (e.g. "de", "fr").
    /// Multiple languages are OR-combined. Omit to search all languages.
    #[serde(default)]
    lang: Option<Vec<String>>,
    /// Minimum number of seeders (inclusive). Use 1 to exclude dead torrents.
    #[serde(default)]
    min_seeders: Option<i32>,
    /// Filter by subcategory ID. Use list_categories to get the full table.
    /// Audiobooks: 39 (Action/Adventure), 49 (Art), 50 (Biographical), 83 (Business),
    /// 51 (Computer/Internet), 97 (Crafts), 40 (Crime/Thriller), 41 (Fantasy), 106 (Food),
    /// 42 (General Fiction), 52 (General Non-Fiction), 98 (Historical Fiction), 54 (History),
    /// 55 (Home/Garden), 43 (Horror), 99 (Humor), 84 (Instructional), 44 (Juvenile),
    /// 56 (Language), 45 (Literary Classics), 57 (Math/Science/Tech), 85 (Medical),
    /// 87 (Mystery), 119 (Nature), 88 (Philosophy), 58 (Pol/Soc/Relig), 59 (Recreation),
    /// 46 (Romance), 47 (Science Fiction), 53 (Self-Help), 89 (Travel/Adventure),
    /// 100 (True Crime), 108 (Urban Fantasy), 48 (Western), 111 (Young Adult).
    /// Ebooks: 60-112 range — call list_categories for the full table.
    /// Musicology: 17-31 range. Radio: 127-132 range.
    #[serde(default)]
    cat: Option<Vec<u32>>,
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
    ///
    /// Tips for best results:
    /// - Use `genre` to narrow by genre name (e.g. "Fantasy", "Mystery").
    /// - Use `language` to filter by spoken language (e.g. "English", "French").
    /// - Use `search_type: "active"` to exclude dead torrents (no seeders).
    /// - Use `search_type: "fl"` to find freeleech torrents (do not count against download ratio,
    ///   though seeding to site requirements is still required).
    /// - Use `sort: "most seeders"` to surface the best-seeded results first.
    /// - Use `offset` with `limit` to page through results.
    #[tool]
    async fn search_audiobooks(
        &self,
        Parameters(p): Parameters<SearchAudiobooksParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => lookup_genres(
                genres,
                AUDIOBOOK_GENRES,
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
            Some(langs) if !langs.is_empty() => map_languages(langs)?,
            _ => vec![],
        };
        let sort = parse_sort(p.sort.as_deref().unwrap_or(""))?;
        self.do_search(
            &p.query,
            vec![13],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
        )
        .await
    }

    /// Search for ebooks on MyAnonamouse (MAM).
    /// Returns matching torrents with title, authors, series, size, seeders, and download URL.
    ///
    /// Tips for best results:
    /// - Use `genre` to narrow by genre name (e.g. "Fantasy", "Science Fiction", "Comics").
    /// - Use `language` to filter by language (e.g. "English", "French").
    /// - Use `search_type: "active"` to exclude dead torrents (no seeders).
    /// - Use `search_type: "fl"` to find freeleech torrents (do not count against download ratio,
    ///   though seeding to site requirements is still required).
    /// - Use `sort: "most seeders"` to surface the best-seeded results first.
    /// - Use `offset` with `limit` to page through results.
    #[tool]
    async fn search_ebooks(
        &self,
        Parameters(p): Parameters<SearchEbooksParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => lookup_genres(
                genres,
                EBOOK_GENRES,
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
            Some(langs) if !langs.is_empty() => map_languages(langs)?,
            _ => vec![],
        };
        let sort = parse_sort(p.sort.as_deref().unwrap_or(""))?;
        self.do_search(
            &p.query,
            vec![14],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
        )
        .await
    }

    /// Search for musicology content on MyAnonamouse (MAM) — sheet music, instructional
    /// media, guitar tabs, music books, and similar resources.
    /// Returns matching torrents with title, size, seeders, and download URL.
    ///
    /// Tips for best results:
    /// - Use `genre` to narrow by type (e.g. "Guitar/Bass Tabs", "Sheet Collection", "Music Book").
    /// - Use `search_type: "active"` to exclude dead torrents.
    /// - Use `sort: "newest"` to find recently added content.
    #[tool]
    async fn search_music(
        &self,
        Parameters(p): Parameters<SearchMusicParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => lookup_genres(
                genres,
                MUSIC_GENRES,
                "Art, Guitar/Bass Tabs, Individual Sheet, Individual Sheet MP3, \
                 Instructional Media, Lick Library LTP/Jam, Lick Library Techniques, \
                 Music Complete Editions, Music Book, Music Book MP3, Sheet Collection, \
                 Sheet Collection MP3, Instructional Book with Video",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => map_languages(langs)?,
            _ => vec![],
        };
        let sort = parse_sort(p.sort.as_deref().unwrap_or(""))?;
        self.do_search(
            &p.query,
            vec![15],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
        )
        .await
    }

    /// Search for radio content on MyAnonamouse (MAM) — BBC Radio, podcasts, dramatisations,
    /// comedy recordings, readings, and similar audio programmes.
    /// Returns matching torrents with title, size, seeders, and download URL.
    ///
    /// Tips for best results:
    /// - Use `genre` to narrow by type: Comedy, Factual/Documentary, Drama, Reading.
    /// - Use `search_type: "active"` to exclude dead torrents.
    /// - Use `sort: "newest"` to find recently added content.
    #[tool]
    async fn search_radio(
        &self,
        Parameters(p): Parameters<SearchRadioParams>,
    ) -> Result<String, String> {
        let cat = match p.genre.as_deref() {
            Some(genres) if !genres.is_empty() => lookup_genres(
                genres,
                RADIO_GENRES,
                "Comedy, Factual/Documentary, Drama, Reading",
            )?,
            _ => vec![],
        };
        let lang = match p.language.as_deref() {
            Some(langs) if !langs.is_empty() => map_languages(langs)?,
            _ => vec![],
        };
        let sort = parse_sort(p.sort.as_deref().unwrap_or(""))?;
        self.do_search(
            &p.query,
            vec![16],
            cat,
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
        )
        .await
    }

    /// Return the full category and subcategory table for MyAnonamouse.
    /// Use this to look up numeric IDs for the `main_cat` and `cat` parameters of search_torrents.
    /// The per-category search tools (search_audiobooks, search_ebooks, search_music,
    /// search_radio) accept genre names directly and do not require these IDs.
    #[tool]
    async fn list_categories(
        &self,
        Parameters(_): Parameters<NoParams>,
    ) -> Result<String, String> {
        Ok(Self::format_categories())
    }

    /// Search for torrents on MyAnonamouse (MAM) across all categories with full parameter
    /// control. Prefer search_audiobooks, search_ebooks, search_music, or search_radio for
    /// typical searches — this tool is for cross-category queries or advanced filtering.
    ///
    /// Tips for best results:
    /// - Use `cat` with numeric subcategory IDs (call list_categories to get the full table).
    /// - Use `main_cat` to restrict to a top-level category without specifying a subcategory.
    /// - Use `search_type: "active"` to exclude dead torrents (no seeders).
    /// - Use `search_type: "fl"` or `"fl-VIP"` to find freeleech torrents.
    /// - Use `sort: "most seeders"` or `"newest"` to control ordering.
    /// - Search matches title, author, narrator, and series name by default.
    #[tool]
    async fn search_torrents(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<String, String> {
        let sort = parse_sort(p.sort.as_deref().unwrap_or(""))?;
        let lang = match p.lang.as_deref() {
            Some(names) if !names.is_empty() => map_languages(names)?,
            _ => vec![],
        };
        self.do_search(
            &p.query,
            p.main_cat.unwrap_or_default(),
            p.cat.unwrap_or_default(),
            lang,
            sort,
            p.search_type.as_deref().unwrap_or("all"),
            p.min_seeders,
            p.limit.unwrap_or(20).min(100),
            p.offset.unwrap_or(0),
        )
        .await
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

    /// Fetch full details for a single torrent by its ID or info-hash.
    /// Returns all available fields: title, category, language, authors, narrators, series,
    /// tags, description, ISBN, media info, file count, size, seeders, leechers, flags,
    /// date added, and download URL.
    /// Use this after finding a torrent ID from a search to get complete information.
    #[tool]
    async fn get_torrent_details(
        &self,
        Parameters(p): Parameters<GetTorrentDetailsParams>,
    ) -> Result<String, String> {
        if p.id.is_none() && p.hash.is_none() {
            return Err("Provide either `id` or `hash`.".to_string());
        }

        let mut tor = serde_json::json!({
            "searchType": "all",
            "searchIn": "torrents",
            "startNumber": "0",
            "perpage": 1,
        });
        if let Some(id) = p.id {
            tor["id"] = serde_json::json!(id);
        }
        if let Some(hash) = &p.hash {
            tor["hash"] = serde_json::json!(hash);
        }

        let body = serde_json::json!({
            "tor": tor,
            "dlLink": "true",
            "description": "",
            "isbn": "",
            "mediaInfo": "",
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

        let body = resp.text().await.map_err(|e| format!("Failed to read response: {e}"))?;

        if let Ok(v) = serde_json::from_str::<Value>(&body) {
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
            Some(t) => Ok(Self::format_torrent_detail(t)),
        }
    }

    /// Get the current IP address and ASN information as seen by MyAnonamouse.
    #[tool]
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
// Registry and constructor
// ---------------------------------------------------------------------------

/// Registry of all tools: (name, group, enabled_by_default).
/// Used by --list-tools and to build the enabled set at startup.
pub const TOOL_REGISTRY: &[(&str, &str, bool)] = &[
    ("search_audiobooks",      "default", true),
    ("search_ebooks",          "default", true),
    ("search_music",           "default", true),
    ("search_radio",           "default", true),
    ("get_torrent_details",    "default", true),
    ("get_ip_info",            "default", true),
    ("search_torrents",        "power",   false),
    ("list_categories",        "power",   false),
    ("get_user_data",          "user",    false),
    ("get_user_bonus_history", "user",    false),
    ("update_seedbox_ip",      "seedbox", false),
];

/// All tool names known to MamServer.
pub const ALL_TOOL_NAMES: &[&str] = &[
    "search_audiobooks",
    "search_ebooks",
    "search_music",
    "search_radio",
    "get_torrent_details",
    "get_ip_info",
    "search_torrents",
    "list_categories",
    "get_user_data",
    "get_user_bonus_history",
    "update_seedbox_ip",
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
        }
    }

    // --- Shared search execution ---

    async fn do_search(
        &self,
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
            "perpage": limit,
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

        Ok(Self::format_search_response(parsed, query))
    }

    // --- Response formatters ---

    fn format_search_response(resp: SearchResponse, query: &str) -> String {
        if resp.data.is_empty() {
            return format!("No results found for \"{query}\".");
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

            let authors = t
                .author_info
                .as_ref()
                .and_then(|v| v.as_str())
                .map(Self::parse_name_map)
                .unwrap_or_default();
            if !authors.is_empty() {
                out.push_str(&format!("   Authors:   {}\n", authors.join(", ")));
            }

            let narrators = t
                .narrator_info
                .as_ref()
                .and_then(|v| v.as_str())
                .map(Self::parse_name_map)
                .unwrap_or_default();
            if !narrators.is_empty() {
                out.push_str(&format!("   Narrators: {}\n", narrators.join(", ")));
            }

            let series = t
                .series_info
                .as_ref()
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
                    out.push_str(&format!(
                        "   DL URL:    {}/tor/download.php/{dl}\n",
                        crate::mam::BASE_URL
                    ));
                }
            }
        }

        out
    }

    fn format_torrent_detail(t: TorrentDetail) -> String {
        let mut out = String::new();

        out.push_str(&format!("Title:       {}\n", t.title));
        out.push_str(&format!("ID:          {}\n", t.id));

        if let Some(cat) = &t.catname {
            out.push_str(&format!("Category:    {cat}\n"));
        }
        if let Some(lang) = &t.lang_code {
            out.push_str(&format!("Language:    {lang}\n"));
        }
        if let Some(size) = &t.size {
            out.push_str(&format!("Size:        {size}\n"));
        }
        if let Some(n) = t.numfiles {
            out.push_str(&format!("Files:       {n}\n"));
        }
        if let Some(ft) = &t.filetype {
            if !ft.is_empty() {
                out.push_str(&format!("File type:   {ft}\n"));
            }
        }

        let authors = t
            .author_info
            .as_ref()
            .and_then(|v| v.as_str())
            .map(Self::parse_name_map)
            .unwrap_or_default();
        if !authors.is_empty() {
            out.push_str(&format!("Authors:     {}\n", authors.join(", ")));
        }

        let narrators = t
            .narrator_info
            .as_ref()
            .and_then(|v| v.as_str())
            .map(Self::parse_name_map)
            .unwrap_or_default();
        if !narrators.is_empty() {
            out.push_str(&format!("Narrators:   {}\n", narrators.join(", ")));
        }

        let series = t
            .series_info
            .as_ref()
            .and_then(|v| v.as_str())
            .map(Self::parse_series_map)
            .unwrap_or_default();
        if !series.is_empty() {
            out.push_str(&format!("Series:      {}\n", series.join(", ")));
        }

        if let Some(tags) = t.tags.as_ref().and_then(|v| v.as_str()) {
            if !tags.is_empty() {
                out.push_str(&format!("Tags:        {tags}\n"));
            }
        }
        if let Some(isbn) = &t.isbn {
            let s = Self::value_as_str(isbn);
            if !s.is_empty() && s != "0" {
                out.push_str(&format!("ISBN:        {s}\n"));
            }
        }

        out.push_str(&format!(
            "Seeders:     {}\nLeechers:    {}\nSnatched:    {}\n",
            t.seeders.unwrap_or(0),
            t.leechers.unwrap_or(0),
            t.times_completed.unwrap_or(0),
        ));

        let is_free = t.free.unwrap_or(0) == 1;
        let is_vip = t.vip.unwrap_or(0) == 1;
        if is_free || is_vip {
            let flags: Vec<&str> = [is_free.then_some("Free"), is_vip.then_some("VIP")]
                .into_iter()
                .flatten()
                .collect();
            out.push_str(&format!("Flags:       {}\n", flags.join(", ")));
        }

        if let Some(added) = &t.added {
            out.push_str(&format!("Added:       {added}\n"));
        }

        if let Some(dl) = &t.dl {
            if !dl.is_empty() {
                out.push_str(&format!(
                    "DL URL:      {}/tor/download.php/{dl}\n",
                    crate::mam::BASE_URL
                ));
            }
        }

        if let Some(desc) = &t.description {
            if !desc.is_empty() {
                let md = htmd::convert(desc).unwrap_or_else(|_| desc.clone());
                out.push_str(&format!("\nDescription:\n{md}\n"));
            }
        }
        if let Some(mi) = &t.mediainfo {
            if !mi.is_empty() {
                out.push_str(&format!("\nMedia Info:\n{mi}\n"));
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
            let secs = entry.timestamp as i64;
            let ts = chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| secs.to_string());
            let amount = Self::value_as_i64(&entry.amount);
            let sign = if amount >= 0 { "+" } else { "" };

            out.push_str(&format!(
                "\n  [{ts}] {sign}{amount} — {btype}",
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

    fn format_categories() -> String {
        let mut out = String::new();
        out.push_str("MAIN CATEGORIES  (use with search_torrents main_cat=[])\n");
        out.push_str("  13  AudioBooks\n");
        out.push_str("  14  E-Books\n");
        out.push_str("  15  Musicology\n");
        out.push_str("  16  Radio\n");

        out.push_str("\nAUDIOBOOK SUBCATEGORIES  (use with search_torrents cat=[])\n");
        for (name, id) in [
            ("Action/Adventure", 39u32), ("Art", 49), ("Biographical", 50), ("Business", 83),
            ("Computer/Internet", 51), ("Crafts", 97), ("Crime/Thriller", 40), ("Fantasy", 41),
            ("Food", 106), ("General Fiction", 42), ("General Non-Fiction", 52),
            ("Historical Fiction", 98), ("History", 54), ("Home/Garden", 55), ("Horror", 43),
            ("Humor", 99), ("Instructional", 84), ("Juvenile", 44), ("Language", 56),
            ("Literary Classics", 45), ("Math/Science/Tech", 57), ("Medical", 85),
            ("Mystery", 87), ("Nature", 119), ("Philosophy", 88), ("Pol/Soc/Relig", 58),
            ("Recreation", 59), ("Romance", 46), ("Science Fiction", 47), ("Self-Help", 53),
            ("Travel/Adventure", 89), ("True Crime", 100), ("Urban Fantasy", 108),
            ("Western", 48), ("Young Adult", 111),
        ] {
            out.push_str(&format!("  {id:>4}  {name}\n"));
        }

        out.push_str("\nEBOOK SUBCATEGORIES  (use with search_torrents cat=[])\n");
        for (name, id) in [
            ("Action/Adventure", 60u32), ("Art", 71), ("Biographical", 72), ("Business", 90),
            ("Comics/Graphic Novels", 61), ("Computer/Internet", 73), ("Crafts", 101),
            ("Crime/Thriller", 62), ("Fantasy", 63), ("Food", 107), ("General Fiction", 64),
            ("General Non-Fiction", 74), ("Historical Fiction", 102), ("History", 76),
            ("Home/Garden", 77), ("Horror", 65), ("Humor", 103), ("Illusion/Magic", 115),
            ("Instructional", 91), ("Juvenile", 66), ("Language", 78), ("Literary Classics", 67),
            ("Magazines/Newspapers", 79), ("Math/Science/Tech", 80), ("Medical", 92),
            ("Mixed Collections", 118), ("Mystery", 94), ("Nature", 120), ("Philosophy", 95),
            ("Pol/Soc/Relig", 81), ("Recreation", 82), ("Romance", 68), ("Science Fiction", 69),
            ("Self-Help", 75), ("Travel/Adventure", 96), ("True Crime", 104),
            ("Urban Fantasy", 109), ("Western", 70), ("Young Adult", 112),
        ] {
            out.push_str(&format!("  {id:>4}  {name}\n"));
        }

        out.push_str("\nMUSICOLOGY SUBCATEGORIES  (use with search_torrents cat=[])\n");
        for (name, id) in [
            ("Art", 49u32), ("Guitar/Bass Tabs", 19), ("Individual Sheet", 20),
            ("Individual Sheet MP3", 24), ("Instructional Media", 22),
            ("Lick Library LTP/Jam", 113), ("Lick Library Techniques", 114),
            ("Music Complete Editions", 17), ("Music Book", 26), ("Music Book MP3", 27),
            ("Sheet Collection", 30), ("Sheet Collection MP3", 31),
            ("Instructional Book with Video", 126),
        ] {
            out.push_str(&format!("  {id:>4}  {name}\n"));
        }

        out.push_str("\nRADIO SUBCATEGORIES  (use with search_torrents cat=[])\n");
        for (name, id) in [
            ("Comedy", 127u32), ("Factual/Documentary", 128), ("Drama", 130), ("Reading", 132),
        ] {
            out.push_str(&format!("  {id:>4}  {name}\n"));
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
