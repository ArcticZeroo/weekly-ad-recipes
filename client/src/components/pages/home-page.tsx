import React, { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { PromiseStage, useDelayedPromiseState } from '@arcticzeroo/react-promise-hook';
import { resolveLocation, searchLocations, type IFlippStoreMatch } from '../../api/client.ts';
import {
    addFavoriteLocation,
    getFavoriteLocationIds,
    removeFavoriteLocation,
} from '../../storage/preferences.ts';
import { Skeleton } from '../common/skeleton.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './home-page.module.scss';

interface IFavoriteEntry {
    locationId: number;
    chainName: string;
    zipCode: string;
}

const FAVORITES_DATA_KEY = 'favoriteLocationsData';

const loadFavoriteEntries = (): IFavoriteEntry[] => {
    try {
        const raw = localStorage.getItem(FAVORITES_DATA_KEY);
        if (raw == null) {
            return [];
        }
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed)) {
            return parsed as IFavoriteEntry[];
        }
    } catch {
        // ignore
    }
    return [];
};

const saveFavoriteEntries = (entries: IFavoriteEntry[]): void => {
    localStorage.setItem(FAVORITES_DATA_KEY, JSON.stringify(entries));
};

const HomePage: React.FC = () => {
    const [zipCode, setZipCode] = useState('');
    const [favoriteIds, setFavoriteIds] = useState<Set<number>>(getFavoriteLocationIds);
    const [favoriteEntries, setFavoriteEntries] = useState<IFavoriteEntry[]>(loadFavoriteEntries);
    const [searchedZip, setSearchedZip] = useState('');
    const [resolvingChain, setResolvingChain] = useState<string | null>(null);
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

    const handleResolveAndNavigate = async (match: IFlippStoreMatch) => {
        setResolvingChain(match.chain_id);
        try {
            const location = await resolveLocation(match, searchedZip);
            navigate(`/${location.id}/deals`);
        } catch {
            // fall through, user can retry
        } finally {
            setResolvingChain(null);
        }
    };

    const handleToggleFavorite = async (match: IFlippStoreMatch) => {
        const existing = favoriteEntries.find(
            (entry) => entry.chainName === match.chain_name && entry.zipCode === searchedZip,
        );

        if (existing) {
            const updatedIds = removeFavoriteLocation(existing.locationId);
            setFavoriteIds(new Set(updatedIds));
            const updatedEntries = favoriteEntries.filter(
                (entry) => entry.locationId !== existing.locationId,
            );
            setFavoriteEntries(updatedEntries);
            saveFavoriteEntries(updatedEntries);
        } else {
            setResolvingChain(match.chain_id);
            try {
                const location = await resolveLocation(match, searchedZip);
                const updatedIds = addFavoriteLocation(location.id);
                setFavoriteIds(new Set(updatedIds));
                const entry: IFavoriteEntry = {
                    locationId: location.id,
                    chainName: match.chain_name,
                    zipCode: searchedZip,
                };
                const updatedEntries = [...favoriteEntries, entry];
                setFavoriteEntries(updatedEntries);
                saveFavoriteEntries(updatedEntries);
            } catch {
                // resolve failed
            } finally {
                setResolvingChain(null);
            }
        }
    };

    const handleRemoveFavorite = (entry: IFavoriteEntry) => {
        const updatedIds = removeFavoriteLocation(entry.locationId);
        setFavoriteIds(new Set(updatedIds));
        const updatedEntries = favoriteEntries.filter(
            (existing) => existing.locationId !== entry.locationId,
        );
        setFavoriteEntries(updatedEntries);
        saveFavoriteEntries(updatedEntries);
    };

    const isMatchFavorited = (match: IFlippStoreMatch): boolean => {
        return favoriteEntries.some(
            (entry) => entry.chainName === match.chain_name && entry.zipCode === searchedZip,
        );
    };

    const activeFavorites = favoriteEntries.filter((entry) => favoriteIds.has(entry.locationId));

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
                                    onClick={() => handleResolveAndNavigate(match)}
                                    disabled={resolvingChain !== null}
                                >
                                    <span className={styles.locationName}>
                                        {resolvingChain === match.chain_id ? 'Loading...' : match.chain_name}
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

            {activeFavorites.length > 0 && (
                <div className={styles.section}>
                    <h2 className={styles.sectionTitle}>Your Stores</h2>
                    <div className={styles.grid}>
                        {activeFavorites.map((entry) => (
                            <div key={entry.locationId} className={styles.favoriteCard}>
                                <button
                                    className={styles.resultLink}
                                    onClick={() => navigate(`/${entry.locationId}/deals`)}
                                >
                                    <span className={styles.locationName}>
                                        {entry.chainName} - {entry.zipCode}
                                    </span>
                                </button>
                                <button
                                    className={`${styles.favoriteButton} ${styles.favorited}`}
                                    onClick={() => handleRemoveFavorite(entry)}
                                    title="Remove from favorites"
                                >
                                    ★
                                </button>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {activeFavorites.length === 0 && searchResponse.value == null && (
                <div className={styles.emptyState}>
                    <p>Search for stores by zip code to get started.</p>
                    <p>Star your favorites for quick access.</p>
                </div>
            )}
        </div>
    );
};

export default HomePage;
