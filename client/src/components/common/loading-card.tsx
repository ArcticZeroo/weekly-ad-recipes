import React from 'react';
import { Box, CircularProgress, Typography } from '@mui/material';

interface ILoadingCardProps {
    message: string;
    subMessage?: string;
}

export const LoadingCard: React.FC<ILoadingCardProps> = ({ message, subMessage }) => {
    return (
        <Box
            sx={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 2,
                p: 4,
                bgcolor: 'background.paper',
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: 3,
                textAlign: 'center',
            }}
        >
            <CircularProgress size={32} />
            <Typography fontWeight={500}>{message}</Typography>
            {subMessage && (
                <Typography variant="body2" color="text.secondary">
                    {subMessage}
                </Typography>
            )}
        </Box>
    );
};
