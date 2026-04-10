// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

/// Normalize a lookup key: lowercase, replace non-alphanumeric with space, collapse whitespace.
pub(crate) fn normalize_lookup(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) const AUDIOBOOK_GENRES: &[(&str, u32)] = &[
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

pub(crate) const EBOOK_GENRES: &[(&str, u32)] = &[
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

pub(crate) const MUSIC_GENRES: &[(&str, u32)] = &[
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

pub(crate) const RADIO_GENRES: &[(&str, u32)] = &[
    ("comedy", 127),
    ("factual documentary", 128), ("factual", 128), ("documentary", 128),
    ("drama", 130),
    ("reading", 132), ("readings", 132), ("spoken word", 132),
];

pub(crate) const LANGUAGES: &[(&str, u32)] = &[
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
pub(crate) fn lookup_genres(
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
pub(crate) fn map_languages(names: &[String]) -> Result<Vec<u32>, String> {
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
pub(crate) fn parse_sort(s: &str) -> Result<&'static str, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_lookup() {
        assert_eq!(normalize_lookup("Action-Adventure"), "action adventure");
        assert_eq!(normalize_lookup("Sci-Fi"), "sci fi");
        assert_eq!(normalize_lookup("  Classics  "), "classics");
        assert_eq!(normalize_lookup("Title (A-Z)"), "title a z");
    }

    #[test]
    fn test_parse_sort() {
        assert_eq!(parse_sort("newest").unwrap(), "dateDesc");
        assert_eq!(parse_sort("most seeders").unwrap(), "seedersDesc");
        assert_eq!(parse_sort("title a-z").unwrap(), "titleAsc");
        assert_eq!(parse_sort("most snatched").unwrap(), "snatchedDesc");
        assert_eq!(parse_sort("").unwrap(), "default");
        assert!(parse_sort("invalid").is_err());
    }
}
