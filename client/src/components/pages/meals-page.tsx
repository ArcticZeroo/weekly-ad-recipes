import React, { useCallback } from 'react';
import { Link, useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { Box, Button, Card, CardContent, Chip, Skeleton, Typography } from '@mui/material';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import { fetchMeals } from '../../api/client.ts';
import { currentWeekRange, formatWeekId } from '../../util/week.ts';
import { LoadingCard } from '../common/loading-card.tsx';
import { ErrorCard } from '../common/error-card.tsx';

const MealsPage: React.FC = () => {
    const { chain, zip } = useParams<{ chain: string; zip: string }>();

    const retrieveMeals = useCallback(
        () => fetchMeals(chain!, zip!),
        [chain, zip],
    );

    const response = useImmediatePromiseState(retrieveMeals);

    if (response.stage === PromiseStage.error) {
        return <ErrorCard message="Unable to load meal ideas." onRetry={response.run} />;
    }

    if (response.value == null) {
        return (
            <Box sx={{ maxWidth: 1100, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 2 }}>
                    <Box>
                        <Typography variant="h4">Meal Ideas</Typography>
                        <Typography variant="body2" color="text.secondary">{currentWeekRange()}</Typography>
                    </Box>
                    <Button
                        variant="outlined"
                        component={Link}
                        to={`/${chain}/${zip}/deals`}
                        startIcon={<ArrowBackIcon />}
                    >
                        Back to Deals
                    </Button>
                </Box>
                <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: 2 }}>
                    {Array.from({ length: 4 }).map((_, index) => (
                        <Card key={index}>
                            <CardContent sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
                                <Skeleton variant="text" width="65%" height={28} />
                                <Skeleton variant="rectangular" height={48} />
                                <Box>
                                    <Skeleton variant="text" width={60} height={16} />
                                    <Box sx={{ display: 'flex', gap: 0.75, mt: 0.5, flexWrap: 'wrap' }}>
                                        <Skeleton variant="rounded" height={24} width={80} />
                                        <Skeleton variant="rounded" height={24} width={64} />
                                        <Skeleton variant="rounded" height={24} width={96} />
                                    </Box>
                                </Box>
                                <Box>
                                    <Skeleton variant="text" width={80} height={16} />
                                    <Box sx={{ display: 'flex', gap: 0.75, mt: 0.5, flexWrap: 'wrap' }}>
                                        <Skeleton variant="rounded" height={24} width={56} />
                                        <Skeleton variant="rounded" height={24} width={72} />
                                    </Box>
                                </Box>
                                <Skeleton variant="text" width={112} height={20} />
                            </CardContent>
                        </Card>
                    ))}
                </Box>
                <LoadingCard message="Generating meal ideas from this week's deals..." />
            </Box>
        );
    }

    const { meals, week_id: weekId, cached } = response.value;

    if (meals.length === 0) {
        return (
            <Box sx={{ maxWidth: 1100, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
                <Typography variant="h4">Meal Ideas</Typography>
                <Box sx={{ textAlign: 'center', py: 4 }}>
                    <Typography color="text.secondary">No meal ideas available yet.</Typography>
                    <Typography color="text.secondary" sx={{ mt: 1 }}>
                        <Link to={`/${chain}/${zip}/deals`}>View deals first</Link> — meal ideas
                        are generated from your current weekly ad deals.
                    </Typography>
                </Box>
            </Box>
        );
    }

    return (
        <Box sx={{ maxWidth: 1100, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 2 }}>
                <Box>
                    <Typography variant="h4">Meal Ideas</Typography>
                    <Typography variant="body2" color="text.secondary">
                        {meals.length} meal ideas · {formatWeekId(weekId)}
                        {cached && ' · cached'}
                    </Typography>
                </Box>
                <Button
                    variant="outlined"
                    component={Link}
                    to={`/${chain}/${zip}/deals`}
                    startIcon={<ArrowBackIcon />}
                >
                    Back to Deals
                </Button>
            </Box>

            <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: 2 }}>
                {meals.map((meal) => (
                    <Card key={meal.id}>
                        <CardContent sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
                            <Typography variant="h6">{meal.name}</Typography>
                            <Typography variant="body2" color="text.secondary" sx={{ lineHeight: 1.6 }}>
                                {meal.description}
                            </Typography>

                            {meal.on_sale_ingredients.length > 0 && (
                                <Box>
                                    <Typography
                                        variant="caption"
                                        fontWeight={600}
                                        sx={{ textTransform: 'uppercase', letterSpacing: '0.05em', color: 'text.secondary' }}
                                    >
                                        On Sale
                                    </Typography>
                                    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75, mt: 0.5 }}>
                                        {meal.on_sale_ingredients.map((ingredient) => (
                                            <Chip
                                                key={ingredient}
                                                label={ingredient}
                                                size="small"
                                                color="success"
                                                variant="outlined"
                                            />
                                        ))}
                                    </Box>
                                </Box>
                            )}

                            {meal.additional_ingredients.length > 0 && (
                                <Box>
                                    <Typography
                                        variant="caption"
                                        fontWeight={600}
                                        sx={{ textTransform: 'uppercase', letterSpacing: '0.05em', color: 'text.secondary' }}
                                    >
                                        Also Needed
                                    </Typography>
                                    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75, mt: 0.5 }}>
                                        {meal.additional_ingredients.map((ingredient) => (
                                            <Chip
                                                key={ingredient}
                                                label={ingredient}
                                                size="small"
                                                variant="outlined"
                                            />
                                        ))}
                                    </Box>
                                </Box>
                            )}

                            <Typography variant="body2" fontWeight={600} color="success.main">
                                Estimated savings: {meal.estimated_savings}
                            </Typography>
                        </CardContent>
                    </Card>
                ))}
            </Box>
        </Box>
    );
};

export default MealsPage;
