import React from 'react';

interface IErrorCardProps {
    message: string;
    onRetry?: () => void;
}

export const ErrorCard: React.FC<IErrorCardProps> = ({ message, onRetry }) => {
    return (
        <div className="card error">
            <span>{message}</span>
            {onRetry && (
                <button onClick={onRetry}>Retry</button>
            )}
        </div>
    );
};
