import React, { useCallback, useState } from 'react';
import { PromiseStage, useDelayedPromiseState, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { addLocation, deleteLocation, fetchLocations, searchLocations, type IFlippStoreMatch } from '../../api/client.ts';
import { LoadingSpinner } from '../common/loading-spinner.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './settings-page.module.scss';

const formatDate = (isoString: string): string => {
    const date = new Date(isoString);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
};

const formatDateRange = (from: string | null, to: string | null): string => {
    if (from && to) {
        return `Valid ${formatDate(from)} – ${formatDate(to)}`;
    }
    return '';
};

const SettingsPage: React.FC = () => {
    const [zipCode, setZipCode] = useState('');
    const [addingMatchIndex, setAddingMatchIndex] = useState<number | null>(null);
    const [deletingLocationId, setDeletingLocationId] = useState<number | null>(null);
    const [actionError, setActionError] = useState<string | null>(null);

    const locationsResponse = useImmediatePromiseState(fetchLocations);

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
        setActionError(null);
        searchResponse.run();
    };

    const handleAddLocation = async (match: IFlippStoreMatch, index: number) => {
        setAddingMatchIndex(index);
        setActionError(null);
        try {
            await addLocation({
                chain_id: match.chain_id,
                name: `${match.chain_name} - ${zipCode}`,
                zip_code: zipCode,
                flipp_merchant_id: match.merchant_id ?? undefined,
                flipp_merchant_name: match.merchant_name,
            });
            await locationsResponse.run();
        } catch (error) {
            setActionError(error instanceof Error ? error.message : 'Failed to add location.');
        } finally {
            setAddingMatchIndex(null);
        }
    };

    const handleDeleteLocation = async (locationId: number) => {
        setDeletingLocationId(locationId);
        setActionError(null);
        try {
            await deleteLocation(locationId);
            await locationsResponse.run();
        } catch (error) {
            setActionError(error instanceof Error ? error.message : 'Failed to remove location.');
        } finally {
            setDeletingLocationId(null);
        }
    };

    return (
        <div className={`${styles.page} flex-col`}>
            <h1>Settings</h1>

            {actionError && <ErrorCard message={actionError} />}

            <div className={styles.section}>
                <h2 className={styles.sectionTitle}>Search for Stores</h2>
                <form className={styles.searchForm} onSubmit={handleSearch}>
                    <input
                        type="text"
                        placeholder="Enter zip code"
                        value={zipCode}
                        onChange={(event) => setZipCode(event.target.value)}
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

                {searchResponse.stage === PromiseStage.running && <LoadingSpinner />}

                {searchResponse.value != null && (
                    <div className="flex-col">
                        {searchResponse.value.length === 0 ? (
                            <p className={styles.emptyMessage}>No stores found for this zip code.</p>
                        ) : (
                            searchResponse.value.map((match, index) => {
                                const alreadyAdded = locationsResponse.value?.some(
                                    (location) => location.chain_id === match.chain_id && location.zip_code === zipCode.trim(),
                                ) ?? false;

                                return (
                                    <div key={`${match.chain_id}-${match.flyer_id}`} className={styles.resultCard}>
                                        <div className={styles.resultInfo}>
                                            <span className={styles.resultName}>
                                                {match.chain_name}
                                                {match.store_name && ` — ${match.store_name}`}
                                            </span>
                                            <span className={styles.resultMeta}>
                                                {formatDateRange(match.valid_from, match.valid_to)}
                                            </span>
                                        </div>
                                        {alreadyAdded ? (
                                            <span className={styles.addedLabel}>Added ✓</span>
                                        ) : (
                                            <button
                                                className={styles.addButton}
                                                onClick={() => handleAddLocation(match, index)}
                                                disabled={addingMatchIndex !== null}
                                            >
                                                {addingMatchIndex === index ? 'Adding...' : 'Add'}
                                            </button>
                                        )}
                                    </div>
                                );
                            })
                        )}
                    </div>
                )}
            </div>

            <div className={styles.section}>
                <h2 className={styles.sectionTitle}>Configured Locations</h2>

                {locationsResponse.stage === PromiseStage.error && (
                    <ErrorCard message="Unable to load locations." onRetry={locationsResponse.run} />
                )}

                {locationsResponse.value == null && locationsResponse.stage !== PromiseStage.error && (
                    <LoadingSpinner />
                )}

                {locationsResponse.value != null && (
                    <div className="flex-col">
                        {locationsResponse.value.length === 0 ? (
                            <p className={styles.emptyMessage}>No locations configured yet. Search above to add stores.</p>
                        ) : (
                            locationsResponse.value.map((location) => (
                                <div key={location.id} className={styles.locationCard}>
                                    <div className={styles.locationInfo}>
                                        <span className={styles.locationName}>{location.name}</span>
                                    </div>
                                    <button
                                        className={styles.removeButton}
                                        onClick={() => handleDeleteLocation(location.id)}
                                        disabled={deletingLocationId !== null}
                                    >
                                        {deletingLocationId === location.id ? 'Removing...' : 'Remove'}
                                    </button>
                                </div>
                            ))
                        )}
                    </div>
                )}
            </div>
        </div>
    );
};

export default SettingsPage;
