import { useMemo, useState } from 'react';
import { localeForLanguage, translate, type SupportedLanguage } from '../i18n/messages';

interface CalendarCardProps {
  lang: SupportedLanguage;
  today?: Date;
}

export function CalendarCard({ lang, today: providedToday }: CalendarCardProps) {
  const today = useMemo(() => providedToday ?? new Date(), [providedToday]);
  const [monthCursor, setMonthCursor] = useState(
    () => new Date(today.getFullYear(), today.getMonth(), 1),
  );
  const locale = localeForLanguage(lang);

  const changeMonth = (delta: number, reset = false) => {
    setMonthCursor(current =>
      reset
        ? new Date(today.getFullYear(), today.getMonth(), 1)
        : new Date(current.getFullYear(), current.getMonth() + delta, 1),
    );
  };

  const { title, days, isCurrentMonth } = useMemo(() => {
    const year = monthCursor.getFullYear();
    const month = monthCursor.getMonth();
    const firstWeekday = (monthCursor.getDay() + 6) % 7;
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const weeks = Math.ceil((firstWeekday + daysInMonth) / 7);
    const start = new Date(year, month, 1 - firstWeekday);

    const grid = Array.from({ length: weeks * 7 }, (_, index) => {
      const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
      return {
        key: `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`,
        day: date.getDate(),
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
      className="card"
      data-testid="calendar-card"
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
          className="icon-btn"
          aria-label={translate(lang, 'prev_month')}
          onClick={() => changeMonth(-1)}
        >
          ‹
        </button>
        <button
          type="button"
          className="icon-btn"
          onClick={() => changeMonth(0, true)}
          data-testid="calendar-title"
          aria-current={isCurrentMonth ? 'date' : undefined}
        >
          {title}
        </button>
        <button
          type="button"
          className="icon-btn"
          aria-label={translate(lang, 'next_month')}
          onClick={() => changeMonth(1)}
        >
          ›
        </button>
      </div>
      <div className="calendar-grid">
        {days.map(entry => (
          <span
            key={entry.key}
            className={`calendar-day${entry.outside ? ' outside' : ''}${entry.isToday ? ' today' : ''}`}
          >
            {entry.day}
          </span>
        ))}
      </div>
    </article>
  );
}
