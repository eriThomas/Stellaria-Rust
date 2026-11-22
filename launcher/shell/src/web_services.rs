use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use stellaria_core::lyrics::{lyrics_candidates, parse_lrc, LyricsCache, LyricsResult};
use stellaria_core::weather::{parse_open_meteo_response, LocationInfo, WeatherCache, WeatherSnapshot};

const GEO_URL: &str = "https://get.geojs.io/v1/ip/geo.json";
const OPEN_METEO_URL: &str = "https://api.open-meteo.com/v1/forecast";
const LRCLIB_URL: &str = "https://lrclib.net/api/search";
const MAX_HTTP_BYTES: usize = 512 * 1024;
const MAX_QUERY_CHARS: usize = 160;

#[derive(Default)]
pub struct WebServicesState {
    weather_cache: Mutex<WeatherCache>,
    lyrics_cache: Mutex<LyricsCache>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("stellaria-dashboard/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| "network".to_string())
}

#[derive(Deserialize)]
struct GeoJsResponse {
    city: Option<String>,
    country: Option<String>,
    latitude: Option<String>,
    longitude: Option<String>,
}

async fn resolve_location(
    client: &reqwest::Client,
    state: &WebServicesState,
) -> Result<LocationInfo, String> {
    let now = now_ms();
    if let Ok(cache) = state.weather_cache.lock() {
        if let Some(loc) = cache.fresh_location(now) {
            return Ok(loc);
        }
    }

    let response = client.get(GEO_URL).send().await.map_err(|_| "network".to_string())?;
    if !response.status().is_success() {
        return Err("location-unavailable".to_string());
    }
    let bytes = response.bytes().await.map_err(|_| "network".to_string())?;
    if bytes.len() > MAX_HTTP_BYTES {
        return Err("location-unavailable".to_string());
    }
    let parsed: GeoJsResponse =
        serde_json::from_slice(&bytes).map_err(|_| "location-unavailable".to_string())?;
    let latitude: f64 = parsed
        .latitude
        .as_deref()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| "location-unavailable".to_string())?;
    let longitude: f64 = parsed
        .longitude
        .as_deref()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| "location-unavailable".to_string())?;

    let location = LocationInfo {
        city: parsed.city.unwrap_or_else(|| "—".to_string()),
        country: parsed.country.unwrap_or_default(),
        latitude,
        longitude,
        fetched_at_ms: now,
    };
    if let Ok(mut cache) = state.weather_cache.lock() {
        cache.set_location(location.clone());
    }
    Ok(location)
}

fn encode_query_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

fn build_open_meteo_url(latitude: f64, longitude: f64) -> String {
    format!(
        "{OPEN_METEO_URL}?latitude={latitude}&longitude={longitude}&current=temperature_2m,relative_humidity_2m,apparent_temperature,is_day,weather_code,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset&timezone=auto&forecast_days=7"
    )
}

fn build_lrclib_url(track_name: &str, artist_name: &str) -> String {
    let mut url = format!("{LRCLIB_URL}?track_name={}", encode_query_component(track_name));
    if !artist_name.is_empty() {
        url.push_str("&artist_name=");
        url.push_str(&encode_query_component(artist_name));
    }
    url
}

pub async fn fetch_weather(
    state: &WebServicesState,
    enabled: bool,
    force: bool,
) -> Result<Option<WeatherSnapshot>, String> {
    if !enabled {
        if let Ok(mut cache) = state.weather_cache.lock() {
            cache.clear();
        }
        return Ok(None);
    }

    let now = now_ms();
    if let Ok(cache) = state.weather_cache.lock() {
        if let Some(cached) = cache.fresh_weather(now, force) {
            return Ok(Some(cached));
        }
    }

    let client = http_client()?;
    let location = resolve_location(&client, state).await?;
    let url = build_open_meteo_url(location.latitude, location.longitude);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|_| "network".to_string())?;

    if !response.status().is_success() {
        return Err("weather-unavailable".to_string());
    }
    let bytes = response.bytes().await.map_err(|_| "network".to_string())?;
    if bytes.len() > MAX_HTTP_BYTES {
        return Err("weather-unavailable".to_string());
    }
    let text = String::from_utf8(bytes.to_vec()).map_err(|_| "weather-unavailable".to_string())?;
    let snapshot = parse_open_meteo_response(&text, &location.city, now)?;
    if let Ok(mut cache) = state.weather_cache.lock() {
        cache.set_weather(snapshot.clone());
    }
    Ok(Some(snapshot))
}

#[derive(Deserialize)]
struct LrcLibEntry {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    duration: Option<f64>,
}

pub async fn search_lyrics(
    state: &WebServicesState,
    artist: String,
    title: String,
    duration_secs: Option<f64>,
) -> Result<LyricsResult, String> {
    let artist_trimmed: String = artist.trim().chars().take(MAX_QUERY_CHARS).collect();
    let title_trimmed: String = title.trim().chars().take(MAX_QUERY_CHARS).collect();
    if title_trimmed.is_empty() {
        return Ok(LyricsResult {
            key: String::new(),
            lines: Vec::new(),
        });
    }

    let key = format!("{artist_trimmed}|{title_trimmed}");
    if let Ok(cache) = state.lyrics_cache.lock() {
        if let Some(lines) = cache.get(&key) {
            return Ok(LyricsResult { key, lines });
        }
    }

    let client = http_client()?;
    let candidates = lyrics_candidates(&artist_trimmed, &title_trimmed);
    let mut found_lines = Vec::new();

    for candidate in candidates {
        let url = build_lrclib_url(&candidate.title, &candidate.artist);
        let Ok(response) = client.get(&url).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(bytes) = response.bytes().await else {
            continue;
        };
        if bytes.len() > MAX_HTTP_BYTES {
            continue;
        }
        let Ok(entries) = serde_json::from_slice::<Vec<LrcLibEntry>>(&bytes) else {
            continue;
        };
        let mut synced: Vec<LrcLibEntry> = entries
            .into_iter()
            .filter(|e| e.synced_lyrics.as_deref().is_some_and(|s| !s.is_empty()))
            .collect();
        if synced.is_empty() {
            continue;
        }
        if let Some(target) = duration_secs.filter(|d| *d > 0.0) {
            synced.sort_by(|a, b| {
                let da = (a.duration.unwrap_or(target) - target).abs();
                let db = (b.duration.unwrap_or(target) - target).abs();
                da.total_cmp(&db)
            });
            if let Some(best_dur) = synced[0].duration {
                if (best_dur - target).abs() > 20.0 {
                    continue;
                }
            }
        }
        if let Some(raw_lrc) = synced[0].synced_lyrics.as_deref() {
            found_lines = parse_lrc(raw_lrc);
            if !found_lines.is_empty() {
                break;
            }
        }
    }

    if let Ok(mut cache) = state.lyrics_cache.lock() {
        cache.insert(key.clone(), found_lines.clone());
    }

    Ok(LyricsResult {
        key,
        lines: found_lines,
    })
}
