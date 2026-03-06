import React, { useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';
import { fetchDeals } from '../../api/client.ts';
import { LoadingSpinner } from '../common/loading-spinner.tsx';
import { ErrorCard } from '../common/error-card.tsx';

const DealsPage: React.FC = () => {
    const { locationId } = useParams<{ locationId: string }>();
    const parsedLocationId = Number(locationId);

    const retrieveDeals = useCallback(
        () => fetchDeals(parsedLocationId),
        [parsedLocationId],
    );

    const response = useImmediatePromiseState(retrieveDeals);

    if (response.stage === PromiseStage.error) {
        return <ErrorCard message="Unable to load deals." onRetry={response.run} />;
    }

    if (response.value == null) {
        return <LoadingSpinner />;
    }

    return (
        <div className="flex-col">
            <h1>Deals</h1>
            <p>{response.value.deals.length} deals found for week {response.value.week_id}</p>
        </div>
    );
};

export default DealsPage;
