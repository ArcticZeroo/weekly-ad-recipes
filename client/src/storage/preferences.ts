const FAVORITE_LOCATIONS_KEY = 'favoriteLocations';

export interface IFavoriteLocation {
    chainId: string;
    zipCode: string;
}

export const getFavoriteLocations = (): IFavoriteLocation[] => {
    const value = localStorage.getItem(FAVORITE_LOCATIONS_KEY);
    if (value == null) {
        return [];
    }
    try {
        const parsed: unknown = JSON.parse(value);
        if (Array.isArray(parsed)) {
            return parsed.filter(
                (item): item is IFavoriteLocation =>
                    typeof item === 'object' &&
                    item != null &&
                    typeof (item as Record<string, unknown>).chainId === 'string' &&
                    typeof (item as Record<string, unknown>).zipCode === 'string',
            );
        }
    } catch {
        // ignore
    }
    return [];
};

const saveFavoriteLocations = (favorites: IFavoriteLocation[]): void => {
    localStorage.setItem(FAVORITE_LOCATIONS_KEY, JSON.stringify(favorites));
};

export const addFavoriteLocation = (chainId: string, zipCode: string): IFavoriteLocation[] => {
    const favorites = getFavoriteLocations();
    if (!favorites.some((favorite) => favorite.chainId === chainId && favorite.zipCode === zipCode)) {
        favorites.push({ chainId, zipCode });
        saveFavoriteLocations(favorites);
    }
    return favorites;
};

export const removeFavoriteLocation = (chainId: string, zipCode: string): IFavoriteLocation[] => {
    const favorites = getFavoriteLocations().filter(
        (favorite) => !(favorite.chainId === chainId && favorite.zipCode === zipCode),
    );
    saveFavoriteLocations(favorites);
    return favorites;
};
