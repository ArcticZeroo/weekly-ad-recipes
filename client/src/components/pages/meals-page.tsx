import React, { useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchMeals } from '../../api/client.ts';
import { LoadingSpinner } from '../common/loading-spinner.tsx';
import { ErrorCard } from '../common/error-card.tsx';

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
        return <LoadingSpinner />;
    }

    return (
        <div className="flex-col">
            <h1>Meal Ideas</h1>
            <p>{response.value.meals.length} meal ideas for week {response.value.week_id}</p>
        </div>
    );
};

export default MealsPage;
