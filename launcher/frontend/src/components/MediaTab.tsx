import { translate, type SupportedLanguage } from '../i18n/messages';
import type { LyricsResult, MediaAction, MediaSnapshot } from '../types/ipc';

interface MediaTabProps {
  lang: SupportedLanguage;
  media: MediaSnapshot | null;
  lyricsEnabled: boolean;
  lyrics: LyricsResult | null;
  onMediaAction: (action: MediaAction) => void;
}

export function MediaTab({
  lang,
  media,
  lyricsEnabled,
  lyrics,
  onMediaAction,
}: MediaTabProps) {
  const current = media?.current ?? null;

  return (
    <section className="media-layout" data-testid="tab-panel-media">
      <div className="media-cover">
        {media?.artworkBase64 ? (
          <img
            src={`data:image/jpeg;base64,${media.artworkBase64}`}
            alt={current?.title || ''}
            className="media-cover"
          />
        ) : (
          <span>♪</span>
        )}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        <article className="card">
          {current ? (
            <>
              <strong style={{ fontSize: 18 }}>{current.title || translate(lang, 'no_media')}</strong>
              <span>{current.artist}</span>
              <small>{current.album}</small>
            </>
          ) : (
            <>
              <strong>{translate(lang, 'nothing_playing')}</strong>
              <small>{translate(lang, 'nothing_playing_hint')}</small>
            </>
          )}
          <div className="controls-row">
            <button
              type="button"
              className="icon-btn"
              disabled={!current}
              onClick={() => onMediaAction('previous')}
            >
              ⏮
            </button>
            <button
              type="button"
              className="icon-btn"
              disabled={!current}
              onClick={() => onMediaAction('togglePlayPause')}
            >
              {current?.isPlaying ? '⏸' : '▶'}
            </button>
            <button
              type="button"
              className="icon-btn"
              disabled={!current}
              onClick={() => onMediaAction('next')}
            >
              ⏭
            </button>
          </div>
        </article>

        {lyricsEnabled && (
          <article className="card">
            <strong>{translate(lang, 'lyrics')}</strong>
            <div className="lyrics-box">
              {!current ? (
                <span>{translate(lang, 'nothing_playing')}</span>
              ) : lyrics === null ? (
                <span>{translate(lang, 'lyrics_loading')}</span>
              ) : lyrics.lines.length === 0 ? (
                <span>{translate(lang, 'lyrics_not_found')}</span>
              ) : (
                lyrics.lines.map((line, index) => (
                  <p key={`${line.timeSecs}-${index}`} style={{ margin: 0 }}>
                    {line.text}
                  </p>
                ))
              )}
            </div>
          </article>
        )}
      </div>
    </section>
  );
}
