import { Route, Routes } from 'react-router-dom';
import { NavBar } from './nav/nav-bar.tsx';
import HomePage from './pages/home-page.tsx';
import DealsPage from './pages/deals-page.tsx';
import MealsPage from './pages/meals-page.tsx';

export const App: React.FC = () => {
    return (
        <>
            <NavBar />
            <main className="flex-col flex-grow" style={{ padding: 'var(--default-padding)' }}>
                <Routes>
                    <Route path="/" element={<HomePage />} />
                    <Route path="/:chain/:zip/deals" element={<DealsPage />} />
                    <Route path="/:chain/:zip/meals" element={<MealsPage />} />
                </Routes>
            </main>
        </>
    );
};
