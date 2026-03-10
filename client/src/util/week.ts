/**
 * Format a date range from valid_from/valid_to strings into a human-friendly string.
 * Input formats: "2026-03-04T00:00:00-05:00" or "20260304" or "2026-03-04"
 * Output: "Mar 4 – Mar 10"
 */
export const formatDateRange = (validFrom: string | null | undefined, validTo: string | null | undefined): string | null => {
    if (validFrom == null || validTo == null) {
        return null;
    }

    const start = parseFlexibleDate(validFrom);
    const end = parseFlexibleDate(validTo);
    if (start == null || end == null) {
        return null;
    }

    const options: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
    const startString = start.toLocaleDateString('en-US', options);
    const endString = end.toLocaleDateString('en-US', options);
    return `${startString} – ${endString}`;
};

const parseFlexibleDate = (dateString: string): Date | null => {
    // "2026-03-04T00:00:00-05:00" → parse with Date constructor
    if (dateString.includes('T')) {
        const date = new Date(dateString);
        return isNaN(date.getTime()) ? null : date;
    }

    // "20260304" → manual parse
    const compactMatch = /^(\d{4})(\d{2})(\d{2})$/.exec(dateString);
    if (compactMatch != null) {
        return new Date(Number(compactMatch[1]), Number(compactMatch[2]) - 1, Number(compactMatch[3]));
    }

    // "2026-03-04" → parse directly
    const date = new Date(dateString + 'T00:00:00');
    return isNaN(date.getTime()) ? null : date;
};
