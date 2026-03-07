import React from 'react';
import { Skeleton as MuiSkeleton } from '@mui/material';

interface ISkeletonProps {
    height?: string;
    width?: string;
    borderRadius?: string;
}

export const Skeleton: React.FC<ISkeletonProps> = ({
    height = '1rem',
    width = '100%',
    borderRadius,
}) => {
    return (
        <MuiSkeleton
            variant="rectangular"
            height={height}
            width={width}
            animation="wave"
            sx={{ borderRadius: borderRadius ?? 1 }}
        />
    );
};
