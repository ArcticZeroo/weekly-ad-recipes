import { createTheme } from '@mui/material/styles';

export const theme = createTheme({
    palette: {
        mode: 'dark',
        primary: {
            main: '#4a7dff',
            light: '#6090ff',
        },
        error: {
            main: '#ff4d4d',
        },
        success: {
            main: '#4dff88',
        },
        background: {
            default: '#1a1a2e',
            paper: '#2d2d4a',
        },
        text: {
            primary: '#e0e0e8',
            secondary: '#9090a8',
        },
        divider: '#3a3a5a',
    },
    shape: {
        borderRadius: 12,
    },
    typography: {
        fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        h4: { fontWeight: 700 },
        h5: { fontWeight: 600 },
        h6: { fontWeight: 600 },
    },
    components: {
        MuiCard: {
            styleOverrides: {
                root: {
                    backgroundImage: 'none',
                    backgroundColor: '#2d2d4a',
                    border: '1px solid #3a3a5a',
                },
            },
        },
        MuiButton: {
            defaultProps: {
                disableElevation: true,
            },
            styleOverrides: {
                root: {
                    textTransform: 'none',
                    fontWeight: 500,
                },
            },
        },
        MuiDrawer: {
            styleOverrides: {
                paper: {
                    backgroundColor: '#25253e',
                    borderRight: '1px solid #3a3a5a',
                },
            },
        },
        MuiAppBar: {
            defaultProps: {
                elevation: 0,
            },
            styleOverrides: {
                root: {
                    backgroundColor: '#25253e',
                    borderBottom: '1px solid #3a3a5a',
                },
            },
        },
        MuiTab: {
            styleOverrides: {
                root: {
                    textTransform: 'none',
                    minHeight: 48,
                },
            },
        },
        MuiChip: {
            styleOverrides: {
                root: {
                    borderRadius: '6px',
                },
            },
        },
        MuiListItemButton: {
            styleOverrides: {
                root: {
                    borderRadius: '8px',
                    '&.Mui-selected': {
                        backgroundColor: 'rgba(74, 125, 255, 0.15)',
                        '&:hover': {
                            backgroundColor: 'rgba(74, 125, 255, 0.25)',
                        },
                    },
                },
            },
        },
        MuiTextField: {
            defaultProps: {
                variant: 'outlined',
                size: 'small',
            },
        },
        MuiCssBaseline: {
            styleOverrides: {
                body: {
                    scrollbarColor: '#3a3a5a #1a1a2e',
                    '&::-webkit-scrollbar': {
                        width: '8px',
                    },
                    '&::-webkit-scrollbar-track': {
                        background: '#1a1a2e',
                    },
                    '&::-webkit-scrollbar-thumb': {
                        background: '#3a3a5a',
                        borderRadius: '4px',
                    },
                },
            },
        },
    },
});
