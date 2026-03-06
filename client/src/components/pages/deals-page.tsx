import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchDeals, refreshDeals } from '../../api/client.ts';
import type { Deal } from '../../models/generated/Deal.ts';
import type { DealsResponse } from '../../models/generated/DealsResponse.ts';
import { formatWeekId } from '../../util/week.ts';
import { LoadingCard } from '../common/loading-card.tsx';
import { Skeleton } from '../common/skeleton.tsx';
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
    const { chain, zip } = useParams<{ chain: string; zip: string }>();
    const [isRefreshing, setIsRefreshing] = useState(false);
    const [refreshedData, setRefreshedData] = useState<DealsResponse | null>(null);
    const [refreshError, setRefreshError] = useState<string | null>(null);
    const [activeCategory, setActiveCategory] = useState<string | null>(null);
    const sectionRefs = useRef<Map<string, HTMLDivElement>>(new Map());
    const tabBarRef = useRef<HTMLDivElement>(null);
    const isScrollingFromClick = useRef(false);
    const [loadingElapsed, setLoadingElapsed] = useState(false);

    const retrieveDeals = useCallback(
        () => fetchDeals(chain!, zip!),
        [chain, zip],
    );

    const response = useImmediatePromiseState(retrieveDeals);
    const dealsData = refreshedData ?? response.value;

    useEffect(() => {
        if (dealsData != null) {
            setLoadingElapsed(false);
            return;
        }

        const timer = setTimeout(() => setLoadingElapsed(true), 3000);
        return () => clearTimeout(timer);
    }, [dealsData]);

    const groupedDeals = useMemo(() => {
        if (dealsData == null) {
            return new Map<string, Deal[]>();
        }
        return groupDealsByCategory(dealsData.deals);
    }, [dealsData]);

    const categories = useMemo(() => Array.from(groupedDeals.keys()), [groupedDeals]);

    // Set initial active category
    useEffect(() => {
        if (categories.length > 0 && activeCategory == null) {
            setActiveCategory(categories[0] ?? null);
        }
    }, [categories, activeCategory]);

    // Intersection observer to track which category is in view
    useEffect(() => {
        const observer = new IntersectionObserver(
            (entries) => {
                if (isScrollingFromClick.current) {
                    return;
                }
                for (const entry of entries) {
                    if (entry.isIntersecting) {
                        const category = entry.target.getAttribute('data-category');
                        if (category) {
                            setActiveCategory(category);
                        }
                    }
                }
            },
            {
                rootMargin: '-120px 0px -60% 0px',
                threshold: 0,
            },
        );

        for (const element of sectionRefs.current.values()) {
            observer.observe(element);
        }

        return () => observer.disconnect();
    }, [categories]);

    const handleTabClick = (category: string) => {
        setActiveCategory(category);
        const element = sectionRefs.current.get(category);
        if (element) {
            isScrollingFromClick.current = true;
            const tabBarHeight = tabBarRef.current?.offsetHeight ?? 0;
            const elementTop = element.getBoundingClientRect().top + window.scrollY;
            // Account for sticky nav + sticky tab bar
            window.scrollTo({ top: elementTop - tabBarHeight - 80, behavior: 'smooth' });
            setTimeout(() => {
                isScrollingFromClick.current = false;
            }, 800);
        }
    };

    const handleRefresh = async () => {
        setIsRefreshing(true);
        setRefreshError(null);
        try {
            const freshData = await refreshDeals(chain!, zip!);
            setRefreshedData(freshData);
            setActiveCategory(null);
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
        return (
            <div className={`${styles.page} flex-col`}>
                <div className={styles.header}>
                    <div className="flex-col">
                        <Skeleton height="2rem" width="6rem" />
                        <Skeleton height="0.9rem" width="12rem" />
                    </div>
                </div>
                <div className={styles.tabBar}>
                    {Array.from({ length: 5 }).map((_, index) => (
                        <Skeleton key={index} height="2rem" width="5rem" borderRadius="0" />
                    ))}
                </div>
                <div className={styles.categorySection}>
                    <Skeleton height="1.1rem" width="10rem" />
                    <div className={styles.dealsGrid}>
                        {Array.from({ length: 8 }).map((_, index) => (
                            <div key={index} className={styles.dealCard}>
                                <Skeleton height="120px" width="120px" borderRadius="8px" />
                                <Skeleton height="1rem" width="85%" />
                                <Skeleton height="0.8rem" width="50%" />
                                <Skeleton height="0.9rem" width="40%" />
                            </div>
                        ))}
                    </div>
                </div>
                <LoadingCard
                    message="Loading deals..."
                    subMessage={loadingElapsed ? 'Scanning weekly ad — this may take 15-30 seconds' : undefined}
                />
            </div>
        );
    }

    const { deals, week_id: weekId } = dealsData;

    return (
        <div className={`${styles.page} flex-col`}>
            <div className={styles.header}>
                <div className="flex-col">
                    <h1>Deals</h1>
                    <span className={styles.meta}>
                        {deals.length} deals · {formatWeekId(weekId)}
                    </span>
                </div>
                {isRefreshing && (
                    <span className={styles.meta}>Refreshing deals...</span>
                )}
                <div className={styles.headerActions}>
                    <button onClick={handleRefresh} disabled={isRefreshing}>
                        {isRefreshing ? 'Refreshing...' : 'Refresh'}
                    </button>
                    <Link to={`/${chain}/${zip}/meals`}>
                        <button className={styles.viewMealsButton}>View Meals</button>
                    </Link>
                </div>
            </div>

            {refreshError && <ErrorCard message={refreshError} />}

            {deals.length === 0 ? (
                <p className={styles.meta}>No deals found for this location this week.</p>
            ) : (
                <>
                    <div ref={tabBarRef} className={styles.tabBar}>
                        {categories.map((category) => (
                            <button
                                key={category}
                                className={`${styles.tab} ${activeCategory === category ? styles.tabActive : ''}`}
                                onClick={() => handleTabClick(category)}
                            >
                                {capitalizeCategory(category)}
                            </button>
                        ))}
                    </div>

                    {Array.from(groupedDeals.entries()).map(([category, categoryDeals]) => (
                        <div
                            key={category}
                            className={styles.categorySection}
                            data-category={category}
                            ref={(element) => {
                                if (element) {
                                    sectionRefs.current.set(category, element);
                                } else {
                                    sectionRefs.current.delete(category);
                                }
                            }}
                        >
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
                    ))}
                </>
            )}
        </div>
    );
};

export default DealsPage;
