const FAVORITE_LOCATIONS_KEY = 'favoriteLocationIds';

export const getFavoriteLocationIds = (): Set<number> => {
    const value = localStorage.getItem(FAVORITE_LOCATIONS_KEY);
    if (value == null) {
        return new Set();
    }
    try {
        const parsed: unknown = JSON.parse(value);
        if (Array.isArray(parsed)) {
            return new Set(parsed.filter((item): item is number => typeof item === 'number'));
        }
    } catch {
        // ignore
    }
    return new Set();
};

export const saveFavoriteLocationIds = (ids: Set<number>): void => {
    localStorage.setItem(FAVORITE_LOCATIONS_KEY, JSON.stringify([...ids]));
};

export const addFavoriteLocation = (locationId: number): Set<number> => {
    const favorites = getFavoriteLocationIds();
    favorites.add(locationId);
    saveFavoriteLocationIds(favorites);
    return favorites;
};

export const removeFavoriteLocation = (locationId: number): Set<number> => {
    const favorites = getFavoriteLocationIds();
    favorites.delete(locationId);
    saveFavoriteLocationIds(favorites);
    return favorites;
};
