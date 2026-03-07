import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { AppLayout } from './layout/app-layout.tsx';
import HomePage from './pages/home-page.tsx';
import DealsPage from './pages/deals-page.tsx';
import MealsPage from './pages/meals-page.tsx';

export const App: React.FC = () => {
    return (
        <AppLayout>
            <Routes>
                <Route path="/" element={<HomePage />} />
                <Route path="/:chain/:zip/deals" element={<DealsPage />} />
                <Route path="/:chain/:zip/meals" element={<MealsPage />} />
            </Routes>
        </AppLayout>
    );
};
