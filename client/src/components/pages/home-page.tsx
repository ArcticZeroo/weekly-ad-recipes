import React, { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { PromiseStage, useDelayedPromiseState } from '@arcticzeroo/react-promise-hook';
import { searchLocations, type IFlippStoreMatch } from '../../api/client.ts';
import {
    addFavoriteLocation,
    getFavoriteLocations,
    removeFavoriteLocation,
    type IFavoriteLocation,
} from '../../storage/preferences.ts';
import { Skeleton } from '../common/skeleton.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './home-page.module.scss';

const HomePage: React.FC = () => {
    const [zipCode, setZipCode] = useState('');
    const [favorites, setFavorites] = useState<IFavoriteLocation[]>(getFavoriteLocations);
    const [searchedZip, setSearchedZip] = useState('');
    const navigate = useNavigate();

    const searchCallback = useCallback(
        () => searchLocations(zipCode),
        [zipCode],
    );
    const searchResponse = useDelayedPromiseState(searchCallback);

    const handleSearch = (event: React.FormEvent) => {
        event.preventDefault();
        if (zipCode.trim().length === 0) {
            return;
        }
        searchResponse.run();
        setSearchedZip(zipCode.trim());
    };

    const handleNavigate = (chainId: string, zip: string) => {
        navigate(`/${chainId}/${zip}/deals`);
    };

    const handleToggleFavorite = (match: IFlippStoreMatch) => {
        const isFavorited = favorites.some(
            (favorite) => favorite.chainId === match.chain_id && favorite.zipCode === searchedZip,
        );

        if (isFavorited) {
            const updated = removeFavoriteLocation(match.chain_id, searchedZip);
            setFavorites(updated);
        } else {
            const updated = addFavoriteLocation(match.chain_id, searchedZip);
            setFavorites(updated);
        }
    };

    const handleRemoveFavorite = (favorite: IFavoriteLocation) => {
        const updated = removeFavoriteLocation(favorite.chainId, favorite.zipCode);
        setFavorites(updated);
    };

    const isMatchFavorited = (match: IFlippStoreMatch): boolean => {
        return favorites.some(
            (favorite) => favorite.chainId === match.chain_id && favorite.zipCode === searchedZip,
        );
    };

    return (
        <div className={`${styles.page} flex-col`}>
            <h1>Weekly Ad Recipes</h1>

            <form className={styles.searchForm} onSubmit={handleSearch}>
                <input
                    type="text"
                    placeholder="Enter zip code to find stores"
                    value={zipCode}
                    onChange={(event) => setZipCode(event.target.value)}
                    className={styles.searchInput}
                />
                <button
                    type="submit"
                    disabled={zipCode.trim().length === 0 || searchResponse.stage === PromiseStage.running}
                >
                    Search
                </button>
            </form>

            {searchResponse.stage === PromiseStage.error && (
                <ErrorCard message="Unable to search locations." onRetry={searchResponse.run} />
            )}

            {searchResponse.stage === PromiseStage.running && (
                <div className={styles.section}>
                    <span className={styles.searchingMessage}>Searching for stores...</span>
                    <div className={styles.grid}>
                        {Array.from({ length: 3 }).map((_, index) => (
                            <Skeleton key={index} height="3.5rem" borderRadius="12px" />
                        ))}
                    </div>
                </div>
            )}

            {searchResponse.value != null && searchResponse.value.length > 0 && (
                <div className={styles.section}>
                    <h2 className={styles.sectionTitle}>Stores near {searchedZip}</h2>
                    <div className={styles.grid}>
                        {searchResponse.value.map((match) => (
                            <div key={match.chain_id} className={styles.resultCard}>
                                <button
                                    className={styles.resultLink}
                                    onClick={() => handleNavigate(match.chain_id, searchedZip)}
                                >
                                    <span className={styles.locationName}>
                                        {match.chain_name}
                                    </span>
                                </button>
                                <button
                                    className={`${styles.favoriteButton} ${isMatchFavorited(match) ? styles.favorited : ''}`}
                                    onClick={() => handleToggleFavorite(match)}
                                    title={isMatchFavorited(match) ? 'Remove from favorites' : 'Add to favorites'}
                                >
                                    {isMatchFavorited(match) ? '★' : '☆'}
                                </button>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {searchResponse.value != null && searchResponse.value.length === 0 && (
                <p className={styles.emptyMessage}>No supported stores found for this zip code.</p>
            )}

            {favorites.length > 0 && (
                <div className={styles.section}>
                    <h2 className={styles.sectionTitle}>Your Stores</h2>
                    <div className={styles.grid}>
                        {favorites.map((favorite) => (
                            <div key={`${favorite.chainId}-${favorite.zipCode}`} className={styles.favoriteCard}>
                                <button
                                    className={styles.resultLink}
                                    onClick={() => handleNavigate(favorite.chainId, favorite.zipCode)}
                                >
                                    <span className={styles.locationName}>
                                        {favorite.chainId} - {favorite.zipCode}
                                    </span>
                                </button>
                                <button
                                    className={`${styles.favoriteButton} ${styles.favorited}`}
                                    onClick={() => handleRemoveFavorite(favorite)}
                                    title="Remove from favorites"
                                >
                                    ★
                                </button>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {favorites.length === 0 && searchResponse.value == null && (
                <div className={styles.emptyState}>
                    <p>Search for stores by zip code to get started.</p>
                    <p>Star your favorites for quick access.</p>
                </div>
            )}
        </div>
    );
};

export default HomePage;
