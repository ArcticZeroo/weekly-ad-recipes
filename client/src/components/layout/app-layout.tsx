import React, { useState } from 'react';
import { Box, Drawer, Toolbar } from '@mui/material';
import { NavBar } from '../nav/nav-bar.tsx';
import { FavoritesSidebar } from './favorites-sidebar.tsx';

const SIDEBAR_WIDTH = 260;

interface IAppLayoutProps {
    children: React.ReactNode;
}

export const AppLayout: React.FC<IAppLayoutProps> = ({ children }) => {
    const [mobileOpen, setMobileOpen] = useState(false);

    const handleToggleSidebar = () => setMobileOpen((prev) => !prev);
    const handleClose = () => setMobileOpen(false);

    const sidebarContent = (
        <>
            <Toolbar />
            <FavoritesSidebar onNavigate={handleClose} />
        </>
    );

    return (
        <Box sx={{ display: 'flex', minHeight: '100vh' }}>
            <NavBar onToggleSidebar={handleToggleSidebar} />

            <Box
                component="nav"
                sx={{ width: { md: SIDEBAR_WIDTH }, flexShrink: { md: 0 } }}
            >
                <Drawer
                    variant="temporary"
                    open={mobileOpen}
                    onClose={handleClose}
                    ModalProps={{ keepMounted: true }}
                    sx={{
                        display: { xs: 'block', md: 'none' },
                        '& .MuiDrawer-paper': { width: SIDEBAR_WIDTH, boxSizing: 'border-box' },
                    }}
                >
                    {sidebarContent}
                </Drawer>

                <Drawer
                    variant="permanent"
                    sx={{
                        display: { xs: 'none', md: 'block' },
                        '& .MuiDrawer-paper': { width: SIDEBAR_WIDTH, boxSizing: 'border-box' },
                    }}
                    open
                >
                    {sidebarContent}
                </Drawer>
            </Box>

            <Box
                component="main"
                sx={{
                    flexGrow: 1,
                    display: 'flex',
                    flexDirection: 'column',
                    minWidth: 0,
                    width: { md: `calc(100% - ${SIDEBAR_WIDTH}px)` },
                }}
            >
                <Toolbar />
                <Box sx={{ flex: 1, p: { xs: 2, md: 3 } }}>
                    {children}
                </Box>
            </Box>
        </Box>
    );
};
