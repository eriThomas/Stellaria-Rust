import { localeForLanguage, translate, type SupportedLanguage } from '../i18n/messages';
import type { WeatherSnapshot } from '../types/ipc';
import { Icon } from './ExpressiveShape';

interface WeatherTabProps {
  lang: SupportedLanguage;
  weatherEnabled: boolean;
  weather: WeatherSnapshot | null;
  weatherError: string | null;
  onRefresh: () => void;
}

export function WeatherTab({
  lang,
  weatherEnabled,
  weather,
  weatherError,
  onRefresh,
}: WeatherTabProps) {
  const locale = localeForLanguage(lang);
  const dateText = new Date().toLocaleDateString(locale, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
  const formattedDate = dateText.charAt(0).toUpperCase() + dateText.slice(1);

  return (
    <section className="page weather" data-page="weather" data-testid="tab-panel-weather">
      <header className="weather-header">
        <div>
          <span className="weather-city">
            {!weatherEnabled
              ? translate(lang, 'weather_disabled')
              : weatherError
                ? translate(lang, 'weather_unavailable')
                : weather?.city ?? translate(lang, 'loading')}
          </span>
          <span className="weather-date">{formattedDate}</span>
        </div>
        <div className="weather-stats">
          <div className="weather-stat" onClick={onRefresh} style={{ cursor: 'pointer' }}>
            <Icon name="wb_twilight" />
            <div>
              <span className="stat-label">{translate(lang, 'sunrise')}</span>
              <span className="stat-value">{weather?.sunrise?.slice(11, 16) ?? '--:--'}</span>
            </div>
          </div>
          <div className="weather-stat">
            <Icon name="bedtime" />
            <div>
              <span className="stat-label">{translate(lang, 'sunset')}</span>
              <span className="stat-value">{weather?.sunset?.slice(11, 16) ?? '--:--'}</span>
            </div>
          </div>
        </div>
      </header>

      <div className="card weather-hero" style={{ '--stagger': 0 } as React.CSSProperties}>
        <Icon name={weather?.current.icon ?? 'cloud'} className="weather-hero-icon" />
        <div className="weather-hero-text">
          <span className="weather-hero-temp">
            {weather ? `${Math.round(weather.current.temperature)}°C` : '--°C'}
          </span>
          <span className="weather-hero-label">
            {weather ? translate(lang, weather.current.conditionKey) : '—'}
          </span>
        </div>
      </div>

      <div className="weather-details">
        <div className="card detail" style={{ '--stagger': 1 } as React.CSSProperties}>
          <Icon name="water_drop" className="secondary" />
          <div>
            <span className="detail-label">{translate(lang, 'humidity')}</span>
            <span className="detail-value">
              {weather ? `${Math.round(weather.current.humidity)}%` : '--'}
            </span>
          </div>
        </div>
        <div className="card detail" style={{ '--stagger': 2 } as React.CSSProperties}>
          <Icon name="thermostat" className="primary" />
          <div>
            <span className="detail-label">{translate(lang, 'feels_like')}</span>
            <span className="detail-value">
              {weather ? `${Math.round(weather.current.feelsLike)}°C` : '--'}
            </span>
          </div>
        </div>
        <div className="card detail" style={{ '--stagger': 3 } as React.CSSProperties}>
          <Icon name="air" className="tertiary" />
          <div>
            <span className="detail-label">{translate(lang, 'wind')}</span>
            <span className="detail-value">
              {weather ? `${Math.round(weather.current.wind)} km/h` : '--'}
            </span>
          </div>
        </div>
      </div>

      <span className="forecast-title">{translate(lang, 'forecast_7d')}</span>
      <div className="forecast">
        {(weather?.daily ?? []).map((day, index) => {
          const date = new Date(`${day.date}T12:00:00`);
          return (
            <div key={day.date} className="card forecast-day" style={{ '--stagger': 4 + index } as React.CSSProperties}>
              <span className="forecast-name">
                {index === 0
                  ? translate(lang, 'today')
                  : date.toLocaleDateString(locale, { weekday: 'short' })}
              </span>
              <span className="forecast-date">
                {date.toLocaleDateString(locale, { day: 'numeric', month: 'short' })}
              </span>
              <Icon name={day.icon} />
              <span className="forecast-range">
                {Math.round(day.min)}° / {Math.round(day.max)}°
              </span>
            </div>
          );
        })}
      </div>
    </section>
  );
}
