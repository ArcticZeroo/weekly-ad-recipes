import React from 'react';
import { AppBar, IconButton, Toolbar, Typography } from '@mui/material';
import MenuIcon from '@mui/icons-material/Menu';
import { Link } from 'react-router-dom';

interface INavBarProps {
    onToggleSidebar: () => void;
}

export const NavBar: React.FC<INavBarProps> = ({ onToggleSidebar }) => {
    return (
        <AppBar
            position="fixed"
            sx={{ zIndex: (muiTheme) => muiTheme.zIndex.drawer + 1 }}
        >
            <Toolbar>
                <IconButton
                    color="inherit"
                    aria-label="open navigation"
                    edge="start"
                    onClick={onToggleSidebar}
                    sx={{ mr: 1, display: { md: 'none' } }}
                >
                    <MenuIcon />
                </IconButton>
                <Typography
                    variant="h6"
                    component={Link}
                    to="/"
                    sx={{
                        color: 'text.primary',
                        textDecoration: 'none',
                        fontWeight: 600,
                        '&:hover': { color: 'primary.main' },
                    }}
                >
                    Weekly Ad Recipes
                </Typography>
            </Toolbar>
        </AppBar>
    );
};
