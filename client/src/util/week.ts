/**
 * Convert an ISO week string like "2026-W10" into the Monday–Sunday date range,
 * then format it as a human-friendly string like "Mar 2 – Mar 8".
 */
export const formatWeekId = (weekId: string): string => {
    const range = weekIdToDateRange(weekId);
    if (range == null) {
        return weekId;
    }
    const [start, end] = range;
    const options: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
    const startStr = start.toLocaleDateString('en-US', options);
    const endStr = end.toLocaleDateString('en-US', options);
    return `${startStr} – ${endStr}`;
};

/**
 * Parse "YYYY-Www" into [Monday, Sunday] Date objects, or null if invalid.
 */
const weekIdToDateRange = (weekId: string): [Date, Date] | null => {
    const match = /^(\d{4})-W(\d{2})$/.exec(weekId);
    if (match == null) {
        return null;
    }
    const year = Number(match[1]);
    const week = Number(match[2]);

    // ISO 8601: week 1 contains January 4th.
    // Find Monday of week 1, then offset to the target week.
    const jan4 = new Date(year, 0, 4);
    const dayOfWeek = jan4.getDay() || 7; // convert Sunday=0 to 7
    const monday = new Date(jan4);
    monday.setDate(jan4.getDate() - (dayOfWeek - 1) + (week - 1) * 7);

    const sunday = new Date(monday);
    sunday.setDate(monday.getDate() + 6);

    return [monday, sunday];
};
