import React from 'react';
import styles from './loading-card.module.scss';

interface ILoadingCardProps {
    message: string;
    subMessage?: string;
}

export const LoadingCard: React.FC<ILoadingCardProps> = ({ message, subMessage }) => {
    return (
        <div className={styles.loadingCard}>
            <div className={styles.spinner} />
            <span className={styles.message}>{message}</span>
            {subMessage && <span className={styles.subMessage}>{subMessage}</span>}
        </div>
    );
};
