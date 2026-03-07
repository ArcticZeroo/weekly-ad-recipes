import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { Box, Button, Card, CardContent, Skeleton, Tab, Tabs, Typography } from '@mui/material';
import RefreshIcon from '@mui/icons-material/Refresh';
import RestaurantMenuIcon from '@mui/icons-material/RestaurantMenu';
import { fetchDeals, refreshDeals } from '../../api/client.ts';
import type { Deal } from '../../models/generated/Deal.ts';
import type { DealsResponse } from '../../models/generated/DealsResponse.ts';
import { currentWeekRange, formatWeekId } from '../../util/week.ts';
import { LoadingCard } from '../common/loading-card.tsx';
import { Skeleton as SkeletonWrapper } from '../common/skeleton.tsx';
import { ErrorCard } from '../common/error-card.tsx';

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

    useEffect(() => {
        if (categories.length > 0 && activeCategory == null) {
            setActiveCategory(categories[0] ?? null);
        }
    }, [categories, activeCategory]);

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
                rootMargin: '-128px 0px -60% 0px',
                threshold: 0,
            },
        );

        for (const element of sectionRefs.current.values()) {
            observer.observe(element);
        }

        return () => observer.disconnect();
    }, [categories]);

    const handleTabChange = (_event: React.SyntheticEvent, category: string) => {
        setActiveCategory(category);
        const element = sectionRefs.current.get(category);
        if (element) {
            isScrollingFromClick.current = true;
            const tabBarHeight = tabBarRef.current?.offsetHeight ?? 0;
            const elementTop = element.getBoundingClientRect().top + window.scrollY;
            window.scrollTo({ top: elementTop - tabBarHeight - 64, behavior: 'smooth' });
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
            <Box sx={{ maxWidth: 1100, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 2 }}>
                    <Box>
                        <Typography variant="h4">Deals</Typography>
                        <Typography variant="body2" color="text.secondary">{currentWeekRange()}</Typography>
                    </Box>
                    <Box sx={{ display: 'flex', gap: 1 }}>
                        <Button variant="outlined" disabled startIcon={<RefreshIcon />}>Refresh</Button>
                        <Button variant="contained" disabled startIcon={<RestaurantMenuIcon />}>View Meals</Button>
                    </Box>
                </Box>
                <Box sx={{ display: 'flex', gap: 2, borderBottom: '1px solid', borderColor: 'divider', pb: 0.5 }}>
                    {Array.from({ length: 6 }).map((_, index) => (
                        <SkeletonWrapper key={index} height="2rem" width={`${4 + Math.random() * 3}rem`} borderRadius="0" />
                    ))}
                </Box>
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                    <Skeleton variant="text" width={120} height={24} />
                    <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: 2 }}>
                        {Array.from({ length: 8 }).map((_, index) => (
                            <Card key={index}>
                                <CardContent sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                                    <Skeleton variant="rectangular" height={120} width={120} sx={{ alignSelf: 'center', borderRadius: 2 }} />
                                    <Skeleton variant="text" width="85%" />
                                    <Skeleton variant="text" width="50%" />
                                    <Skeleton variant="text" width="40%" />
                                </CardContent>
                            </Card>
                        ))}
                    </Box>
                </Box>
                <LoadingCard
                    message="Loading deals..."
                    subMessage={loadingElapsed ? 'Scanning weekly ad — this may take 15-30 seconds' : undefined}
                />
            </Box>
        );
    }

    const { deals, week_id: weekId } = dealsData;

    return (
        <Box sx={{ maxWidth: 1100, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 2 }}>
                <Box>
                    <Typography variant="h4">Deals</Typography>
                    <Typography variant="body2" color="text.secondary">
                        {deals.length} deals · {formatWeekId(weekId)}
                    </Typography>
                </Box>
                {isRefreshing && (
                    <Typography variant="body2" color="text.secondary">Refreshing deals...</Typography>
                )}
                <Box sx={{ display: 'flex', gap: 1 }}>
                    <Button
                        variant="outlined"
                        onClick={handleRefresh}
                        disabled={isRefreshing}
                        startIcon={<RefreshIcon />}
                    >
                        {isRefreshing ? 'Refreshing...' : 'Refresh'}
                    </Button>
                    <Button
                        variant="contained"
                        component={Link}
                        to={`/${chain}/${zip}/meals`}
                        startIcon={<RestaurantMenuIcon />}
                    >
                        View Meals
                    </Button>
                </Box>
            </Box>

            {refreshError && <ErrorCard message={refreshError} />}

            {deals.length === 0 ? (
                <Typography color="text.secondary">No deals found for this location this week.</Typography>
            ) : (
                <>
                    <Box
                        ref={tabBarRef}
                        sx={{
                            position: 'sticky',
                            top: { xs: 56, sm: 64 },
                            zIndex: 10,
                            bgcolor: 'background.default',
                            borderBottom: '1px solid',
                            borderColor: 'divider',
                        }}
                    >
                        <Tabs
                            value={activeCategory ?? false}
                            onChange={handleTabChange}
                            variant="scrollable"
                            scrollButtons="auto"
                        >
                            {categories.map((category) => (
                                <Tab
                                    key={category}
                                    label={capitalizeCategory(category)}
                                    value={category}
                                />
                            ))}
                        </Tabs>
                    </Box>

                    {Array.from(groupedDeals.entries()).map(([category, categoryDeals]) => (
                        <Box
                            key={category}
                            data-category={category}
                            ref={(element: HTMLDivElement | null) => {
                                if (element) {
                                    sectionRefs.current.set(category, element);
                                } else {
                                    sectionRefs.current.delete(category);
                                }
                            }}
                            sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}
                        >
                            <Typography
                                variant="h6"
                                sx={{ pb: 1, borderBottom: '1px solid', borderColor: 'divider' }}
                            >
                                {capitalizeCategory(category)} ({categoryDeals.length})
                            </Typography>
                            <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: 2 }}>
                                {categoryDeals.map((deal) => (
                                    <Card key={deal.id}>
                                        <CardContent sx={{ display: 'flex', flexDirection: 'column', gap: 0.75 }}>
                                            {deal.image_url && (
                                                <Box
                                                    component="img"
                                                    src={deal.image_url}
                                                    alt={deal.item_name}
                                                    sx={{
                                                        width: 120,
                                                        height: 120,
                                                        objectFit: 'contain',
                                                        borderRadius: 2,
                                                        alignSelf: 'center',
                                                        bgcolor: '#1e1e36',
                                                    }}
                                                />
                                            )}
                                            <Typography variant="subtitle2" fontWeight={600}>
                                                {deal.item_name}
                                            </Typography>
                                            {deal.brand && (
                                                <Typography variant="caption" color="text.secondary">
                                                    {deal.brand}
                                                </Typography>
                                            )}
                                            <Typography variant="body2">
                                                {deal.deal_description}
                                            </Typography>
                                        </CardContent>
                                    </Card>
                                ))}
                            </Box>
                        </Box>
                    ))}
                </>
            )}
        </Box>
    );
};

export default DealsPage;
