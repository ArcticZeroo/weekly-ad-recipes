import React, { useCallback, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchDeals, refreshDeals } from '../../api/client.ts';
import type { Deal } from '../../models/generated/Deal.ts';
import type { DealsResponse } from '../../models/generated/DealsResponse.ts';
import { LoadingSpinner } from '../common/loading-spinner.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './deals-page.module.scss';

const capitalizeCategory = (category: string): string => {
    return category
        .split(/[\s_-]+/)
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(' ');
};

const groupDealsByCategory = (deals: Deal[]): Map<string, Deal[]> => {
    const groups = new Map<string, Deal[]>();
    for (const deal of deals) {
        const category = deal.category || 'Other';
        const existing = groups.get(category);
        if (existing) {
            existing.push(deal);
        } else {
            groups.set(category, [deal]);
        }
    }
    return groups;
};

const DealsPage: React.FC = () => {
    const { locationId } = useParams<{ locationId: string }>();
    const parsedLocationId = Number(locationId);
    const [isRefreshing, setIsRefreshing] = useState(false);
    const [refreshedData, setRefreshedData] = useState<DealsResponse | null>(null);
    const [refreshError, setRefreshError] = useState<string | null>(null);

    const retrieveDeals = useCallback(
        () => fetchDeals(parsedLocationId),
        [parsedLocationId],
    );

    const response = useImmediatePromiseState(retrieveDeals);
    const dealsData = refreshedData ?? response.value;

    const groupedDeals = useMemo(() => {
        if (dealsData == null) {
            return new Map<string, Deal[]>();
        }
        return groupDealsByCategory(dealsData.deals);
    }, [dealsData]);

    const handleRefresh = async () => {
        setIsRefreshing(true);
        setRefreshError(null);
        try {
            const freshData = await refreshDeals(parsedLocationId);
            setRefreshedData(freshData);
        } catch (error) {
            setRefreshError(error instanceof Error ? error.message : 'Failed to refresh deals.');
        } finally {
            setIsRefreshing(false);
        }
    };

    if (response.stage === PromiseStage.error) {
        return <ErrorCard message="Unable to load deals." onRetry={response.run} />;
    }

    if (dealsData == null) {
        return <LoadingSpinner />;
    }

    const { deals, week_id: weekId } = dealsData;

    return (
        <div className={`${styles.page} flex-col`}>
            <div className={styles.header}>
                <div className="flex-col">
                    <h1>Deals</h1>
                    <span className={styles.meta}>
                        {deals.length} deals for week {weekId}
                    </span>
                </div>
                <div className={styles.headerActions}>
                    <button onClick={handleRefresh} disabled={isRefreshing}>
                        {isRefreshing ? 'Refreshing...' : 'Refresh'}
                    </button>
                    <Link to={`/${parsedLocationId}/meals`}>
                        <button className={styles.viewMealsButton}>View Meals</button>
                    </Link>
                </div>
            </div>

            {refreshError && <ErrorCard message={refreshError} />}

            {deals.length === 0 ? (
                <p className={styles.meta}>No deals found for this location this week.</p>
            ) : (
                Array.from(groupedDeals.entries()).map(([category, categoryDeals]) => (
                    <div key={category} className={styles.categorySection}>
                        <h2 className={styles.categoryTitle}>
                            {capitalizeCategory(category)} ({categoryDeals.length})
                        </h2>
                        <div className={styles.dealsGrid}>
                            {categoryDeals.map((deal) => (
                                <div key={deal.id} className={styles.dealCard}>
                                    {deal.image_url && (
                                        <img
                                            src={deal.image_url}
                                            alt={deal.item_name}
                                            className={styles.dealImage}
                                        />
                                    )}
                                    <span className={styles.dealName}>{deal.item_name}</span>
                                    {deal.brand && (
                                        <span className={styles.dealBrand}>{deal.brand}</span>
                                    )}
                                    <span className={styles.dealDescription}>{deal.deal_description}</span>
                                </div>
                            ))}
                        </div>
                    </div>
                ))
            )}
        </div>
    );
};

export default DealsPage;
