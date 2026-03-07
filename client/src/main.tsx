import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { theme } from './theme.ts';
import { FavoritesProvider } from './context/favorites-context.tsx';
import './index.scss';
import { App } from './components/app.tsx';

const root = document.getElementById('root');
if (root == null) {
    throw new Error('Root element not found');
}

createRoot(root).render(
    <StrictMode>
        <BrowserRouter>
            <ThemeProvider theme={theme}>
                <CssBaseline />
                <FavoritesProvider>
                    <App />
                </FavoritesProvider>
            </ThemeProvider>
        </BrowserRouter>
    </StrictMode>,
);
