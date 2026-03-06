import { Route, Routes } from 'react-router-dom';
import { NavBar } from './nav/nav-bar.tsx';
import { SelectedLocationContext, selectedLocationNotifier } from '../context/location.ts';
import HomePage from './pages/home-page.tsx';
import DealsPage from './pages/deals-page.tsx';
import MealsPage from './pages/meals-page.tsx';
import SettingsPage from './pages/settings-page.tsx';

export const App: React.FC = () => {
    return (
        <SelectedLocationContext.Provider value={selectedLocationNotifier}>
            <NavBar />
            <main className="flex-col flex-grow" style={{ padding: 'var(--default-padding)' }}>
                <Routes>
                    <Route path="/" element={<HomePage />} />
                    <Route path="/:locationId/deals" element={<DealsPage />} />
                    <Route path="/:locationId/meals" element={<MealsPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </SelectedLocationContext.Provider>
    );
};
