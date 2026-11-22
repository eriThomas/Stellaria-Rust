use serde::{Deserialize, Serialize};

pub const WEATHER_TTL_MS: u64 = 15 * 60 * 1000;
pub const LOCATION_TTL_MS: u64 = 6 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherDescription {
    pub label: &'static str,
    pub icon: &'static str,
}

pub fn describe_weather(code: i32, is_day: bool) -> WeatherDescription {
    let (label, day, night) = match code {
        0 => ("clear_sky", "sunny", "clear_night"),
        1 => ("mainly_clear", "partly_cloudy_day", "partly_cloudy_night"),
        2 => ("partly_cloudy", "partly_cloudy_day", "partly_cloudy_night"),
        3 => ("overcast", "cloud", "cloud"),
        45 => ("fog", "foggy", "foggy"),
        48 => ("rime_fog", "foggy", "foggy"),
        51 => ("light_drizzle", "rainy_light", "rainy_light"),
        53 => ("drizzle", "rainy_light", "rainy_light"),
        55 => ("dense_drizzle", "rainy", "rainy"),
        56 | 57 => ("freezing_drizzle", "weather_mix", "weather_mix"),
        61 => ("light_rain", "rainy_light", "rainy_light"),
        63 => ("rain", "rainy", "rainy"),
        65 => ("heavy_rain", "rainy_heavy", "rainy_heavy"),
        66 | 67 => ("freezing_rain", "weather_mix", "weather_mix"),
        71 => ("light_snow", "weather_snowy", "weather_snowy"),
        73 => ("snow", "weather_snowy", "weather_snowy"),
        75 => ("heavy_snow", "snowing_heavy", "snowing_heavy"),
        77 => ("snow_grains", "weather_snowy", "weather_snowy"),
        80 | 81 => ("showers", "rainy", "rainy"),
        82 => ("heavy_showers", "rainy_heavy", "rainy_heavy"),
        85 => ("snow_showers", "weather_snowy", "weather_snowy"),
        86 => ("heavy_snow_showers", "snowing_heavy", "snowing_heavy"),
        95 => ("thunderstorm", "thunderstorm", "thunderstorm"),
        96 | 99 => ("thunderstorm_hail", "thunderstorm", "thunderstorm"),
        _ => ("unknown_weather", "cloud", "cloud"),
    };
    WeatherDescription {
        label,
        icon: if is_day { day } else { night },
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    pub city: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentWeather {
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: f64,
    pub wind: f64,
    pub code: i32,
    pub is_day: bool,
    pub icon: String,
    pub condition_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyForecast {
    pub date: String,
    pub code: i32,
    pub icon: String,
    pub max: f64,
    pub min: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherSnapshot {
    pub fetched_at_ms: u64,
    pub city: String,
    pub current: CurrentWeather,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
    pub daily: Vec<DailyForecast>,
}

#[derive(Default)]
pub struct WeatherCache {
    location: Option<LocationInfo>,
    weather: Option<WeatherSnapshot>,
}

impl WeatherCache {
    pub fn fresh_location(&self, now_ms: u64) -> Option<LocationInfo> {
        self.location
            .as_ref()
            .filter(|loc| now_ms.saturating_sub(loc.fetched_at_ms) < LOCATION_TTL_MS)
            .cloned()
    }

    pub fn set_location(&mut self, location: LocationInfo) {
        self.location = Some(location);
    }

    pub fn fresh_weather(&self, now_ms: u64, force: bool) -> Option<WeatherSnapshot> {
        if force {
            return None;
        }
        self.weather
            .as_ref()
            .filter(|w| now_ms.saturating_sub(w.fetched_at_ms) < WEATHER_TTL_MS)
            .cloned()
    }

    pub fn set_weather(&mut self, weather: WeatherSnapshot) {
        self.weather = Some(weather);
    }

    pub fn clear(&mut self) {
        self.location = None;
        self.weather = None;
    }
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
    daily: OpenMeteoDaily,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    wind_speed_10m: f64,
    weather_code: i32,
    is_day: i32,
}

#[derive(Deserialize)]
struct OpenMeteoDaily {
    time: Vec<String>,
    weather_code: Vec<i32>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

pub fn parse_open_meteo_response(
    raw_json: &str,
    city: &str,
    now_ms: u64,
) -> Result<WeatherSnapshot, String> {
    let parsed: OpenMeteoResponse =
        serde_json::from_str(raw_json).map_err(|_| "invalid-weather-payload".to_string())?;
    let is_day = parsed.current.is_day == 1;
    let desc = describe_weather(parsed.current.weather_code, is_day);

    let count = parsed
        .daily
        .time
        .len()
        .min(parsed.daily.weather_code.len())
        .min(parsed.daily.temperature_2m_max.len())
        .min(parsed.daily.temperature_2m_min.len())
        .min(7);

    let mut daily = Vec::with_capacity(count);
    for i in 0..count {
        let day_desc = describe_weather(parsed.daily.weather_code[i], true);
        daily.push(DailyForecast {
            date: parsed.daily.time[i].clone(),
            code: parsed.daily.weather_code[i],
            icon: day_desc.icon.to_string(),
            max: parsed.daily.temperature_2m_max[i],
            min: parsed.daily.temperature_2m_min[i],
        });
    }

    Ok(WeatherSnapshot {
        fetched_at_ms: now_ms,
        city: city.to_string(),
        current: CurrentWeather {
            temperature: parsed.current.temperature_2m,
            feels_like: parsed.current.apparent_temperature,
            humidity: parsed.current.relative_humidity_2m,
            wind: parsed.current.wind_speed_10m,
            code: parsed.current.weather_code,
            is_day,
            icon: desc.icon.to_string(),
            condition_key: desc.label.to_string(),
        },
        sunrise: parsed.daily.sunrise.first().cloned(),
        sunset: parsed.daily.sunset.first().cloned(),
        daily,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_day_and_night_weather_icons() {
        assert_eq!(describe_weather(0, true).icon, "sunny");
        assert_eq!(describe_weather(0, false).icon, "clear_night");
        assert_eq!(describe_weather(95, true).icon, "thunderstorm");
        assert_eq!(describe_weather(999, true).label, "unknown_weather");
    }

    #[test]
    fn parses_open_meteo_payload_and_respects_ttl() {
        let sample = r#"{
          "current": {
            "temperature_2m": 24.5,
            "apparent_temperature": 25.1,
            "relative_humidity_2m": 62.0,
            "wind_speed_10m": 12.4,
            "weather_code": 2,
            "is_day": 1
          },
          "daily": {
            "time": ["2026-11-22", "2026-11-23"],
            "weather_code": [2, 61],
            "temperature_2m_max": [27.0, 22.0],
            "temperature_2m_min": [18.0, 16.5],
            "sunrise": ["2026-11-22T05:30", "2026-11-23T05:30"],
            "sunset": ["2026-11-22T18:45", "2026-11-23T18:46"]
          }
        }"#;

        let snap = parse_open_meteo_response(sample, "São Paulo", 1_000).expect("valid payload");
        assert_eq!(snap.city, "São Paulo");
        assert_eq!(snap.current.icon, "partly_cloudy_day");
        assert_eq!(snap.daily.len(), 2);
        assert_eq!(snap.daily[1].icon, "rainy_light");

        let mut cache = WeatherCache::default();
        cache.set_weather(snap);
        assert!(cache.fresh_weather(1_000 + 60_000, false).is_some());
        assert!(cache.fresh_weather(1_000 + 60_000, true).is_none());
        assert!(cache.fresh_weather(1_000 + WEATHER_TTL_MS + 1, false).is_none());
    }
}
