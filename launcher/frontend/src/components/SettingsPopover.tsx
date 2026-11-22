import { translate, type SupportedLanguage } from '../i18n/messages';
import type { DashboardSettings, DashboardSettingsPatch } from '../types/ipc';

interface SettingsPopoverProps {
  lang: SupportedLanguage;
  settings: DashboardSettings;
  onUpdate: (patch: DashboardSettingsPatch) => void;
}

export function SettingsPopover({ lang, settings, onUpdate }: SettingsPopoverProps) {
  return (
    <aside className="settings-popover" data-testid="settings-popover">
      <strong>{translate(lang, 'settings')}</strong>

      <label className="setting-row">
        <span>{translate(lang, 'setting_language')}</span>
        <select
          value={settings.language}
          onChange={e => onUpdate({ language: e.target.value })}
          aria-label={translate(lang, 'setting_language')}
        >
          <option value="auto">Auto</option>
          <option value="en">English</option>
          <option value="pt-BR">Português (Brasil)</option>
          <option value="fr">Français</option>
          <option value="es">Español</option>
          <option value="de">Deutsch</option>
        </select>
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_theme_mode')}</span>
        <select
          value={settings.themeMode}
          onChange={e => onUpdate({ themeMode: e.target.value as 'dark' | 'light' })}
          aria-label={translate(lang, 'setting_theme_mode')}
        >
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_source_color')}</span>
        <input
          type="color"
          value={settings.sourceColor}
          onChange={e => onUpdate({ sourceColor: e.target.value })}
          aria-label={translate(lang, 'setting_source_color')}
        />
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_clock_seconds')}</span>
        <input
          type="checkbox"
          checked={settings.clockSeconds}
          onChange={e => onUpdate({ clockSeconds: e.target.checked })}
        />
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_weather')}</span>
        <input
          type="checkbox"
          checked={settings.weatherEnabled}
          onChange={e => onUpdate({ weatherEnabled: e.target.checked })}
        />
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_lyrics')}</span>
        <input
          type="checkbox"
          checked={settings.lyricsEnabled}
          onChange={e => onUpdate({ lyricsEnabled: e.target.checked })}
        />
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_reduced_motion')}</span>
        <input
          type="checkbox"
          checked={settings.reducedMotion}
          onChange={e => onUpdate({ reducedMotion: e.target.checked })}
        />
      </label>

      <label className="setting-row">
        <span>{translate(lang, 'setting_game_mode')}</span>
        <input
          type="checkbox"
          checked={settings.gameMode}
          onChange={e => onUpdate({ gameMode: e.target.checked })}
        />
      </label>
    </aside>
  );
}
