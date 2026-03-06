const PREFERRED_LOCATION_KEY = 'preferredLocationId';

export const getPreferredLocationId = (): number | null => {
    const value = localStorage.getItem(PREFERRED_LOCATION_KEY);
    if (value == null) {
        return null;
    }
    const parsed = Number(value);
    return Number.isNaN(parsed) ? null : parsed;
};

export const setPreferredLocationId = (locationId: number | null): void => {
    if (locationId == null) {
        localStorage.removeItem(PREFERRED_LOCATION_KEY);
    } else {
        localStorage.setItem(PREFERRED_LOCATION_KEY, String(locationId));
    }
};
