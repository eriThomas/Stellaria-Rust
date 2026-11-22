import { useMemo, useState } from 'react';
import { localeForLanguage, translate, type SupportedLanguage } from '../i18n/messages';
import { ExpressiveShape, Icon } from './ExpressiveShape';

interface CalendarCardProps {
  lang: SupportedLanguage;
  today?: Date;
}

export function CalendarCard({ lang, today: providedToday }: CalendarCardProps) {
  const today = useMemo(() => providedToday ?? new Date(), [providedToday]);
  const [monthCursor, setMonthCursor] = useState(
    () => new Date(today.getFullYear(), today.getMonth(), 1),
  );
  const [slideKey, setSlideKey] = useState(0);
  const [direction, setDirection] = useState(0);
  const locale = localeForLanguage(lang);

  const changeMonth = (delta: number, reset = false) => {
    setMonthCursor(current => {
      const target = reset
        ? new Date(today.getFullYear(), today.getMonth(), 1)
        : new Date(current.getFullYear(), current.getMonth() + delta, 1);
      if (target.getTime() === current.getTime()) return current;
      setDirection(target > current ? 1 : -1);
      setSlideKey(k => k + 1);
      return target;
    });
  };

  const weekdays = useMemo(() => {
    // Monday (2026-11-16) through Sunday (2026-11-22)
    return Array.from({ length: 7 }, (_, i) => {
      const date = new Date(2026, 10, 16 + i);
      return date.toLocaleDateString(locale, { weekday: 'short' });
    });
  }, [locale]);

  const { title, days, isCurrentMonth } = useMemo(() => {
    const year = monthCursor.getFullYear();
    const month = monthCursor.getMonth();
    const firstWeekday = (monthCursor.getDay() + 6) % 7;
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const weeks = Math.ceil((firstWeekday + daysInMonth) / 7);
    const start = new Date(year, month, 1 - firstWeekday);

    const grid = Array.from({ length: weeks * 7 }, (_, index) => {
      const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
      const dayOfWeek = date.getDay();
      return {
        key: `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`,
        day: date.getDate(),
        weekend: dayOfWeek === 0 || dayOfWeek === 6,
        outside: date.getMonth() !== month,
        isToday: date.toDateString() === today.toDateString(),
      };
    });

    return {
      title: monthCursor.toLocaleDateString(locale, { month: 'long', year: 'numeric' }),
      days: grid,
      isCurrentMonth: month === today.getMonth() && year === today.getFullYear(),
    };
  }, [monthCursor, locale, today]);

  return (
    <article
      className="card dash-calendar"
      data-testid="calendar-card"
      style={{ '--stagger': 3 } as React.CSSProperties}
      onWheel={event => {
        event.preventDefault();
        changeMonth(event.deltaY > 0 ? 1 : -1);
      }}
      onAuxClick={event => {
        if (event.button === 1) changeMonth(0, true);
      }}
    >
      <div className="calendar-nav">
        <button
          type="button"
          className="state-layer"
          title={translate(lang, 'prev_month')}
          aria-label={translate(lang, 'prev_month')}
          onClick={() => changeMonth(-1)}
        >
          <Icon name="chevron_left" />
        </button>
        <span
          className={`calendar-title${isCurrentMonth ? '' : ' resettable'}`}
          data-testid="calendar-title"
          role="button"
          tabIndex={0}
          onClick={() => changeMonth(0, true)}
          onKeyDown={e => {
            if (e.key === 'Enter' || e.key === ' ') changeMonth(0, true);
          }}
        >
          {title}
        </span>
        <button
          type="button"
          className="state-layer"
          title={translate(lang, 'next_month')}
          aria-label={translate(lang, 'next_month')}
          onClick={() => changeMonth(1)}
        >
          <Icon name="chevron_right" />
        </button>
      </div>

      <div className="calendar-weekdays">
        {weekdays.map((label, index) => (
          <span key={label + index} className={index >= 5 ? 'weekend' : ''}>
            {label}
          </span>
        ))}
      </div>

      <div
        key={slideKey}
        className={`calendar-days${direction ? ' animating' : ''}`}
        style={{ '--direction': direction } as React.CSSProperties}
      >
        {days.map(entry => (
          <span
            key={entry.key}
            className={[
              entry.weekend ? 'weekend' : '',
              entry.outside && !entry.isToday ? 'outside' : '',
              entry.isToday ? 'today' : '',
            ]
              .filter(Boolean)
              .join(' ')}
          >
            {entry.isToday && <ExpressiveShape name="sunny" />}
            {entry.day}
          </span>
        ))}
      </div>
    </article>
  );
}
