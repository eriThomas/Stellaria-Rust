import { localeForLanguage, translate, type SupportedLanguage } from '../i18n/messages';
import type { WeatherSnapshot } from '../types/ipc';

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

  if (!weatherEnabled) {
    return (
      <section className="card" data-testid="tab-panel-weather">
        <strong>{translate(lang, 'weather_disabled')}</strong>
      </section>
    );
  }

  if (weatherError) {
    return (
      <section className="card" data-testid="tab-panel-weather">
        <strong>{translate(lang, 'weather_unavailable')}</strong>
        <button type="button" className="icon-btn" onClick={onRefresh}>
          ↻
        </button>
      </section>
    );
  }

  if (!weather) {
    return (
      <section className="card" data-testid="tab-panel-weather">
        <span>{translate(lang, 'loading')}</span>
      </section>
    );
  }

  return (
    <section data-testid="tab-panel-weather">
      <header className="weather-header">
        <div>
          <strong style={{ fontSize: 18 }}>{weather.city}</strong>
          <div>
            <small>
              {new Date().toLocaleDateString(locale, {
                weekday: 'long',
                day: 'numeric',
                month: 'long',
              })}
            </small>
          </div>
        </div>
        <div className="controls-row">
          <small>
            {translate(lang, 'sunrise')}: {weather.sunrise?.slice(11, 16) ?? '--:--'}
          </small>
          <small>
            {translate(lang, 'sunset')}: {weather.sunset?.slice(11, 16) ?? '--:--'}
          </small>
          <button type="button" className="icon-btn" onClick={onRefresh}>
            ↻
          </button>
        </div>
      </header>

      <article className="card" style={{ flexDirection: 'row', alignItems: 'center', gap: 16 }}>
        <span style={{ fontSize: 36, fontWeight: 700 }}>{Math.round(weather.current.temperature)}°C</span>
        <span>{translate(lang, weather.current.conditionKey)}</span>
      </article>

      <div className="weather-details-grid">
        <article className="card">
          <small>{translate(lang, 'humidity')}</small>
          <strong>{Math.round(weather.current.humidity)}%</strong>
        </article>
        <article className="card">
          <small>{translate(lang, 'feels_like')}</small>
          <strong>{Math.round(weather.current.feelsLike)}°C</strong>
        </article>
        <article className="card">
          <small>{translate(lang, 'wind')}</small>
          <strong>{Math.round(weather.current.wind)} km/h</strong>
        </article>
      </div>

      <small>{translate(lang, 'forecast_7d')}</small>
      <div className="forecast-row" style={{ marginTop: 6 }}>
        {weather.daily.map((day, index) => {
          const date = new Date(`${day.date}T12:00:00`);
          return (
            <article key={day.date} className="card" style={{ alignItems: 'center', padding: 8 }}>
              <small>
                {index === 0
                  ? translate(lang, 'today')
                  : date.toLocaleDateString(locale, { weekday: 'short' })}
              </small>
              <strong>
                {Math.round(day.min)}° / {Math.round(day.max)}°
              </strong>
            </article>
          );
        })}
      </div>
    </section>
  );
}
