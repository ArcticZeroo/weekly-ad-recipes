import React from 'react';
import styles from './skeleton.module.scss';

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
        <div
            className={styles.skeleton}
            style={{ height, width, borderRadius }}
        />
    );
};
