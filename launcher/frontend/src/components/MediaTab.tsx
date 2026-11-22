import { useMemo } from 'react';
import { translate, type SupportedLanguage } from '../i18n/messages';
import type { LyricsResult, MediaAction, MediaSnapshot } from '../types/ipc';
import { ExpressiveShape, Icon } from './ExpressiveShape';

interface MediaTabProps {
  lang: SupportedLanguage;
  media: MediaSnapshot | null;
  lyricsEnabled: boolean;
  lyrics: LyricsResult | null;
  onMediaAction: (action: MediaAction) => void;
}

function formatTime(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}

export function MediaTab({
  lang,
  media,
  lyricsEnabled,
  lyrics,
  onMediaAction,
}: MediaTabProps) {
  const current = media?.current ?? null;
  const playing = Boolean(current?.isPlaying);
  const sessions = media?.sessions ?? [];

  const wavePath = useMemo(() => {
    let path = 'M-40 12';
    for (let x = -40; x <= 440; x += 20) {
      path += `Q${x + 10} ${(x / 20) % 2 === 0 ? 6 : 18} ${x + 20} 12`;
    }
    return path;
  }, []);

  const fraction =
    current && current.durationSecs > 0
      ? Math.min(1, Math.max(0, current.positionSecs / current.durationSecs))
      : playing
        ? 0.52
        : 0;
  const x = Math.max(0, Math.min(400, fraction * 400));

  return (
    <section
      className={`page media${current ? '' : ' empty'}${playing ? ' playing' : ''}`}
      data-page="media"
      data-testid="tab-panel-media"
    >
      <div className="media-visual">
        <div className="media-visual-ring">
          <ExpressiveShape name="cookie9" />
        </div>
        <div className="cover large">
          {media?.artworkBase64 ? (
            <img src={`data:image/jpeg;base64,${media.artworkBase64}`} alt={current?.title || ''} />
          ) : (
            <Icon name="art_track" filled />
          )}
        </div>
      </div>

      <div className="media-main">
        <div className="media-empty">
          <div className="media-empty-shape">
            <ExpressiveShape name="clamShell" />
            <Icon name="queue_music" />
          </div>
          <span className="media-empty-title">{translate(lang, 'nothing_playing')}</span>
          <span className="media-empty-text">{translate(lang, 'nothing_playing_hint')}</span>
        </div>

        <div className="media-details">
          <span className="media-title">{current?.title ?? ''}</span>
          <span className="media-artist">{current?.artist || translate(lang, 'unknown_artist')}</span>
          <span className="media-album">{current?.album || translate(lang, 'unknown_album')}</span>
          <div className="media-progress">
            <span className="media-time">{formatTime(current?.positionSecs ?? 0)}</span>
            <svg className="wavy" viewBox="0 0 400 24" preserveAspectRatio="none">
              <defs>
                <clipPath id="wave-clip">
                  <rect x="0" y="0" width={Math.max(0, x - 6)} height="24" />
                </clipPath>
              </defs>
              <g clipPath="url(#wave-clip)">
                <g className="wave-group">
                  <path className="wave" d={wavePath} />
                </g>
              </g>
              <line className="rest" x1={Math.min(400, x + 6)} y1="12" x2="400" y2="12" />
              <rect className="handle" x={Math.max(0, x - 2)} y="2" width="4" height="20" rx="2" />
            </svg>
            <span className="media-time">
              {current && current.durationSecs > 0 ? formatTime(current.durationSecs) : '--:--'}
            </span>
          </div>
          <div className="button-row large">
            <button type="button" className="icon-button state-layer small" disabled>
              <Icon name="shuffle" filled />
            </button>
            <button
              type="button"
              className="icon-button state-layer"
              disabled={!current}
              onClick={() => onMediaAction('previous')}
            >
              <Icon name="skip_previous" filled />
            </button>
            <button
              type="button"
              className={`icon-button state-layer fill${playing ? ' checked' : ''}`}
              disabled={!current}
              onClick={() => onMediaAction('togglePlayPause')}
            >
              <Icon name={playing ? 'pause' : 'play_arrow'} filled />
            </button>
            <button
              type="button"
              className="icon-button state-layer"
              disabled={!current}
              onClick={() => onMediaAction('next')}
            >
              <Icon name="skip_next" filled />
            </button>
            <button type="button" className="icon-button state-layer small" disabled>
              <Icon name="repeat" filled />
            </button>
          </div>
        </div>

        <div className="media-players">
          <span className="media-players-title">{translate(lang, 'players')}</span>
          <div className="media-player-list scroll">
            {sessions.length > 0 ? (
              sessions.map(session => (
                <button
                  key={session.sessionId}
                  type="button"
                  className={`player state-layer${session.sessionId === current?.sessionId ? ' current' : ''}`}
                >
                  <Icon name={session.isPlaying ? 'graphic_eq' : 'music_note'} filled />
                  <span className="player-text">
                    <span>{session.title || translate(lang, 'no_media')}</span>
                    <span>{session.artist || session.sessionId}</span>
                  </span>
                </button>
              ))
            ) : (
              <span className="player-text" style={{ padding: 4, opacity: 0.7 }}>
                <span>{translate(lang, 'no_players')}</span>
              </span>
            )}
          </div>

          {lyricsEnabled && (
            <div className="media-lyrics-section">
              <span className="media-players-title">{translate(lang, 'lyrics')}</span>
              <div className={`media-lyrics${lyrics && lyrics.lines.length > 0 ? ' synced' : ''}`}>
                {!current ? (
                  <span className="lyrics-empty">{translate(lang, 'nothing_playing')}</span>
                ) : lyrics === null ? (
                  <span className="lyrics-empty">{translate(lang, 'lyrics_loading')}</span>
                ) : lyrics.lines.length === 0 ? (
                  <span className="lyrics-empty">{translate(lang, 'lyrics_not_found')}</span>
                ) : (
                  lyrics.lines.map((line, index) => (
                    <p key={`${line.timeSecs}-${index}`} className="lyric-line">
                      {line.text}
                    </p>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
