import React from 'react';
import { Alert, Button } from '@mui/material';

interface IErrorCardProps {
    message: string;
    onRetry?: () => void;
}

export const ErrorCard: React.FC<IErrorCardProps> = ({ message, onRetry }) => {
    return (
        <Alert
            severity="error"
            action={
                onRetry && (
                    <Button color="inherit" size="small" onClick={onRetry}>
                        Retry
                    </Button>
                )
            }
        >
            {message}
        </Alert>
    );
};
