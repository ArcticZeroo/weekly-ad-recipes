import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { NavBar } from './nav/nav-bar.tsx';
import { SelectedLocationContext, selectedLocationNotifier } from '../context/location.ts';

const HomePage = React.lazy(() => import('./pages/home-page.tsx'));
const DealsPage = React.lazy(() => import('./pages/deals-page.tsx'));
const MealsPage = React.lazy(() => import('./pages/meals-page.tsx'));
const SettingsPage = React.lazy(() => import('./pages/settings-page.tsx'));

export const App: React.FC = () => {
    return (
        <SelectedLocationContext.Provider value={selectedLocationNotifier}>
            <NavBar />
            <main className="flex-col flex-grow" style={{ padding: 'var(--default-padding)' }}>
                <React.Suspense fallback={<div className="flex flex-center flex-grow">Loading...</div>}>
                    <Routes>
                        <Route path="/" element={<HomePage />} />
                        <Route path="/:locationId/deals" element={<DealsPage />} />
                        <Route path="/:locationId/meals" element={<MealsPage />} />
                        <Route path="/settings" element={<SettingsPage />} />
                    </Routes>
                </React.Suspense>
            </main>
        </SelectedLocationContext.Provider>
    );
};
