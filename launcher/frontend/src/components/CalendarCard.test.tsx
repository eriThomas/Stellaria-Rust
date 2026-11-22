import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CalendarCard } from './CalendarCard';

describe('CalendarCard', () => {
  it('navigates between months and resets to today when clicking title', () => {
    const fixedToday = new Date(2026, 10, 22); // Nov 22, 2026
    render(<CalendarCard lang="en" today={fixedToday} />);

    const title = screen.getByTestId('calendar-title');
    expect(title.textContent?.toLowerCase()).toContain('november');
    expect(title.textContent).toContain('2026');

    fireEvent.click(screen.getByRole('button', { name: 'Next month' }));
    expect(title.textContent?.toLowerCase()).toContain('december');

    fireEvent.click(title);
    expect(title.textContent?.toLowerCase()).toContain('november');
  });
});
