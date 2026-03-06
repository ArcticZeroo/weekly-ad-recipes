import React from 'react';
import { Link } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchLocations, getCachedLocations } from '../../api/client.ts';
import { LoadingSpinner } from '../common/loading-spinner.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './home-page.module.scss';

const HomePage: React.FC = () => {
    const locationsResponse = useImmediatePromiseState(fetchLocations);
    const locations = locationsResponse.value ?? getCachedLocations();

    if (locationsResponse.stage === PromiseStage.error && locations == null) {
        return <ErrorCard message="Unable to load locations." onRetry={locationsResponse.run} />;
    }

    if (locations == null) {
        return <LoadingSpinner />;
    }

    if (locations.length === 0) {
        return (
            <div className={styles.page}>
                <h1>Weekly Ad Recipes</h1>
                <div className={styles.emptyState}>
                    <p>No store locations configured yet.</p>
                    <p>
                        <Link to="/settings">Go to Settings</Link> to add your local stores.
                    </p>
                </div>
            </div>
        );
    }

    return (
        <div className={`${styles.page} flex-col`}>
            <h1>Weekly Ad Recipes</h1>
            <p>Select a store location to view deals and meal ideas.</p>
            <div className={styles.grid}>
                {locations.map((location) => (
                    <Link
                        key={location.id}
                        to={`/${location.id}/deals`}
                        className={styles.locationCard}
                    >
                        <span className={styles.locationName}>{location.name}</span>
                        {location.address && (
                            <span className={styles.locationMeta}>{location.address}</span>
                        )}
                    </Link>
                ))}
            </div>
        </div>
    );
};

export default HomePage;
