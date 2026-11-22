use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

pub const MAX_LYRICS_CACHE: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub time_secs: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsCandidate {
    pub artist: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub key: String,
    pub lines: Vec<LyricLine>,
}

const DECORATION_KEYWORDS: [&str; 13] = [
    "official",
    "officiel",
    "video",
    "vidéo",
    "clip",
    "audio",
    "lyric",
    "paroles",
    "visualiz",
    "visualis",
    "remaster",
    "live",
    "explicit",
];

fn strip_bracketed_decorations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let open = chars[i];
        if open == '(' || open == '[' {
            let close = if open == '(' { ')' } else { ']' };
            if let Some(rel_end) = chars[i + 1..].iter().position(|&c| c == close) {
                let inner: String = chars[i + 1..i + 1 + rel_end].iter().collect();
                let lower = inner.to_ascii_lowercase();
                if DECORATION_KEYWORDS.iter().any(|kw| lower.contains(kw))
                    || lower == "hd"
                    || lower == "4k"
                {
                    i += rel_end + 2;
                    continue;
                }
            }
        }
        out.push(open);
        i += 1;
    }
    out
}

fn strip_featuring(input: &str) -> &str {
    let lower = input.to_ascii_lowercase();
    for marker in [" feat. ", " feat ", " ft. ", " ft ", " featuring "] {
        if let Some(pos) = lower.find(marker) {
            return &input[..pos];
        }
    }
    input
}

fn normalize_spaces(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn lyrics_candidates(artist_raw: &str, title_raw: &str) -> Vec<LyricsCandidate> {
    let mut artist = artist_raw.trim().to_string();
    for suffix in [" - Topic", " - topic", "VEVO", "vevo"] {
        if let Some(stripped) = artist.strip_suffix(suffix) {
            artist = stripped.trim().to_string();
        }
    }

    let stripped = strip_bracketed_decorations(title_raw);
    let title = normalize_spaces(strip_featuring(&stripped));

    let mut candidates = Vec::new();
    for sep in [" - ", " – ", " — "] {
        if let Some((left, right)) = title.split_once(sep) {
            let l = left.trim();
            let r = right.trim();
            if !l.is_empty() && !r.is_empty() {
                candidates.push(LyricsCandidate {
                    artist: l.to_string(),
                    title: r.to_string(),
                });
                break;
            }
        }
    }
    if !title.is_empty() {
        candidates.push(LyricsCandidate { artist, title });
    }
    candidates
}

fn parse_timestamp(tag: &str) -> Option<f64> {
    let (mins, secs) = tag.split_once(':')?;
    let m: f64 = mins.parse().ok()?;
    let s: f64 = secs.parse().ok()?;
    if m.is_finite() && s.is_finite() && m >= 0.0 && s >= 0.0 {
        Some(m * 60.0 + s)
    } else {
        None
    }
}

pub fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in raw.lines() {
        let mut stamps = Vec::new();
        let mut rest = line;
        while let Some(start) = rest.find('[') {
            if let Some(end) = rest[start + 1..].find(']') {
                let tag = &rest[start + 1..start + 1 + end];
                if let Some(ts) = parse_timestamp(tag) {
                    stamps.push(ts);
                }
                rest = &rest[start + 1 + end + 1..];
            } else {
                break;
            }
        }
        if stamps.is_empty() {
            continue;
        }
        let text = rest.trim();
        let display = if text.is_empty() { "♪" } else { text };
        for ts in stamps {
            lines.push(LyricLine {
                time_secs: ts,
                text: display.to_string(),
            });
        }
    }
    lines.sort_by(|a, b| a.time_secs.total_cmp(&b.time_secs));
    lines
}

#[derive(Default)]
pub struct LyricsCache {
    entries: VecDeque<(String, Vec<LyricLine>)>,
}

impl LyricsCache {
    pub fn get(&self, key: &str) -> Option<Vec<LyricLine>> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    pub fn insert(&mut self, key: String, lines: Vec<LyricLine>) {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == &key) {
            self.entries.remove(pos);
        }
        self.entries.push_back((key, lines));
        while self.entries.len() > MAX_LYRICS_CACHE {
            self.entries.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_artist_and_title_decorations() {
        let candidates = lyrics_candidates(
            "Daft Punk - Topic",
            "Daft Punk - Instant Crush (Official Video) feat. Julian Casablancas",
        );
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].artist, "Daft Punk");
        assert_eq!(candidates[0].title, "Instant Crush");
        assert_eq!(candidates[1].artist, "Daft Punk");
    }

    #[test]
    fn parses_synced_lrc_with_multiple_stamps() {
        let lrc = "[00:12.50][00:40.00]Chorus line\n[00:05.00]Intro\n[00:20.00]\n";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].text, "Intro");
        assert!((parsed[0].time_secs - 5.0).abs() < 0.001);
        assert_eq!(parsed[1].text, "Chorus line");
        assert_eq!(parsed[2].text, "♪");
        assert!((parsed[3].time_secs - 40.0).abs() < 0.001);
    }

    #[test]
    fn bounds_cache_at_forty_entries() {
        let mut cache = LyricsCache::default();
        for i in 0..55 {
            cache.insert(format!("song-{i}"), vec![]);
        }
        assert_eq!(cache.len(), MAX_LYRICS_CACHE);
        assert!(cache.get("song-0").is_none());
        assert!(cache.get("song-54").is_some());
    }
}
