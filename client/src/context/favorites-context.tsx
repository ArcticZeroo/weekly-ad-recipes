import React, { createContext, useCallback, useContext, useState } from 'react';
import {
    addFavoriteLocation,
    getFavoriteLocations,
    removeFavoriteLocation,
    type IFavoriteLocation,
} from '../storage/preferences.ts';

interface IFavoritesContext {
    favorites: IFavoriteLocation[];
    addFavorite: (chainId: string, zipCode: string) => void;
    removeFavorite: (chainId: string, zipCode: string) => void;
    isFavorite: (chainId: string, zipCode: string) => boolean;
}

const FavoritesContext = createContext<IFavoritesContext | null>(null);

export const FavoritesProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [favorites, setFavorites] = useState<IFavoriteLocation[]>(getFavoriteLocations);

    const addFavorite = useCallback((chainId: string, zipCode: string) => {
        setFavorites(addFavoriteLocation(chainId, zipCode));
    }, []);

    const removeFavorite = useCallback((chainId: string, zipCode: string) => {
        setFavorites(removeFavoriteLocation(chainId, zipCode));
    }, []);

    const isFavorite = useCallback(
        (chainId: string, zipCode: string): boolean =>
            favorites.some((favorite) => favorite.chainId === chainId && favorite.zipCode === zipCode),
        [favorites],
    );

    return (
        <FavoritesContext.Provider value={{ favorites, addFavorite, removeFavorite, isFavorite }}>
            {children}
        </FavoritesContext.Provider>
    );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useFavorites = (): IFavoritesContext => {
    const context = useContext(FavoritesContext);
    if (context == null) {
        throw new Error('useFavorites must be used within a FavoritesProvider');
    }
    return context;
};
