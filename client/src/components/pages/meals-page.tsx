import React, { useCallback } from 'react';
import { Link, useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchMeals } from '../../api/client.ts';
import { formatWeekId } from '../../util/week.ts';
import { LoadingCard } from '../common/loading-card.tsx';
import { Skeleton } from '../common/skeleton.tsx';
import { ErrorCard } from '../common/error-card.tsx';
import styles from './meals-page.module.scss';

const MealsPage: React.FC = () => {
    const { locationId } = useParams<{ locationId: string }>();
    const parsedLocationId = Number(locationId);

    const retrieveMeals = useCallback(
        () => fetchMeals(parsedLocationId),
        [parsedLocationId],
    );

    const response = useImmediatePromiseState(retrieveMeals);

    if (response.stage === PromiseStage.error) {
        return <ErrorCard message="Unable to load meal ideas." onRetry={response.run} />;
    }

    if (response.value == null) {
        return (
            <div className={`${styles.page} flex-col`}>
                <div className={styles.header}>
                    <div className="flex-col">
                        <h1>Meal Ideas</h1>
                        <Skeleton height="0.9rem" width="14rem" />
                    </div>
                </div>
                <div className={styles.mealsGrid}>
                    {Array.from({ length: 4 }).map((_, index) => (
                        <div key={index} className={styles.mealCard}>
                            <Skeleton height="1.2rem" width="65%" />
                            <Skeleton height="2.5rem" />
                            <div className={styles.ingredientsSection}>
                                <Skeleton height="0.75rem" width="3rem" />
                                <div className={styles.ingredientsList}>
                                    <Skeleton height="1.5rem" width="5rem" borderRadius="6px" />
                                    <Skeleton height="1.5rem" width="4rem" borderRadius="6px" />
                                    <Skeleton height="1.5rem" width="6rem" borderRadius="6px" />
                                </div>
                            </div>
                            <div className={styles.ingredientsSection}>
                                <Skeleton height="0.75rem" width="4.5rem" />
                                <div className={styles.ingredientsList}>
                                    <Skeleton height="1.5rem" width="3.5rem" borderRadius="6px" />
                                    <Skeleton height="1.5rem" width="4.5rem" borderRadius="6px" />
                                </div>
                            </div>
                            <Skeleton height="0.85rem" width="7rem" />
                        </div>
                    ))}
                </div>
                <LoadingCard message="Generating meal ideas from this week's deals..." />
            </div>
        );
    }

    const { meals, week_id: weekId, cached } = response.value;

    if (meals.length === 0) {
        return (
            <div className={`${styles.page} flex-col`}>
                <h1>Meal Ideas</h1>
                <div className={styles.emptyState}>
                    <p>No meal ideas available yet.</p>
                    <p>
                        <Link to={`/${parsedLocationId}/deals`}>View deals first</Link> — meal ideas
                        are generated from your current weekly ad deals.
                    </p>
                </div>
            </div>
        );
    }

    return (
        <div className={`${styles.page} flex-col`}>
            <div className={styles.header}>
                <div className="flex-col">
                    <h1>Meal Ideas</h1>
                    <span className={styles.meta}>
                        {meals.length} meal ideas · {formatWeekId(weekId)}
                        {cached && ' · cached'}
                    </span>
                </div>
                <Link to={`/${parsedLocationId}/deals`}>
                    <button>Back to Deals</button>
                </Link>
            </div>

            <div className={styles.mealsGrid}>
                {meals.map((meal) => (
                    <div key={meal.id} className={styles.mealCard}>
                        <h3 className={styles.mealName}>{meal.name}</h3>
                        <p className={styles.mealDescription}>{meal.description}</p>

                        {meal.on_sale_ingredients.length > 0 && (
                            <div className={styles.ingredientsSection}>
                                <span className={styles.ingredientsLabel}>On Sale</span>
                                <div className={styles.ingredientsList}>
                                    {meal.on_sale_ingredients.map((ingredient) => (
                                        <span
                                            key={ingredient}
                                            className={`${styles.ingredientTag} ${styles.onSaleTag}`}
                                        >
                                            {ingredient}
                                        </span>
                                    ))}
                                </div>
                            </div>
                        )}

                        {meal.additional_ingredients.length > 0 && (
                            <div className={styles.ingredientsSection}>
                                <span className={styles.ingredientsLabel}>Also Needed</span>
                                <div className={styles.ingredientsList}>
                                    {meal.additional_ingredients.map((ingredient) => (
                                        <span key={ingredient} className={styles.ingredientTag}>
                                            {ingredient}
                                        </span>
                                    ))}
                                </div>
                            </div>
                        )}

                        <span className={styles.savings}>
                            Estimated savings: {meal.estimated_savings}
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default MealsPage;
