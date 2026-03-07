import React from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
    Box,
    Divider,
    IconButton,
    List,
    ListItemButton,
    ListItemText,
    Tooltip,
    Typography,
} from '@mui/material';
import HomeIcon from '@mui/icons-material/Home';
import StarIcon from '@mui/icons-material/Star';
import StarBorderIcon from '@mui/icons-material/StarBorder';
import { useFavorites } from '../../context/favorites-context.tsx';
import { displayChainName } from '../../util/chains.ts';

interface IFavoritesSidebarProps {
    onNavigate?: () => void;
}

export const FavoritesSidebar: React.FC<IFavoritesSidebarProps> = ({ onNavigate }) => {
    const { favorites, removeFavorite } = useFavorites();
    const { chain: activeChain, zip: activeZip } = useParams<{ chain?: string; zip?: string }>();
    const navigate = useNavigate();

    const handleLocationClick = (chainId: string, zipCode: string) => {
        navigate(`/${chainId}/${zipCode}/deals`);
        onNavigate?.();
    };

    const handleRemoveFavorite = (event: React.MouseEvent, chainId: string, zipCode: string) => {
        event.stopPropagation();
        removeFavorite(chainId, zipCode);
    };

    return (
        <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
            <Box sx={{ px: 2, py: 1.5 }}>
                <Typography
                    variant="overline"
                    color="text.secondary"
                    sx={{ lineHeight: 1, letterSpacing: '0.1em' }}
                >
                    My Stores
                </Typography>
            </Box>
            <Divider />
            <List sx={{ flex: 1, overflow: 'auto', px: 1, py: 0.5 }}>
                <ListItemButton
                    onClick={() => {
                        navigate('/');
                        onNavigate?.();
                    }}
                    sx={{ mb: 0.5 }}
                >
                    <HomeIcon sx={{ mr: 1.5, fontSize: '1.1rem', color: 'text.secondary' }} />
                    <ListItemText
                        primary="Home"
                        primaryTypographyProps={{ fontSize: '0.875rem' }}
                    />
                </ListItemButton>

                {favorites.length > 0 && <Divider sx={{ my: 0.5 }} />}

                {favorites.map((favorite) => {
                    const isActive =
                        favorite.chainId === activeChain && favorite.zipCode === activeZip;
                    return (
                        <ListItemButton
                            key={`${favorite.chainId}-${favorite.zipCode}`}
                            selected={isActive}
                            onClick={() => handleLocationClick(favorite.chainId, favorite.zipCode)}
                            sx={{ mb: 0.5, pr: 0.5 }}
                        >
                            <ListItemText
                                primary={displayChainName(favorite.chainId)}
                                secondary={favorite.zipCode}
                                primaryTypographyProps={{
                                    fontSize: '0.875rem',
                                    fontWeight: isActive ? 600 : 400,
                                }}
                                secondaryTypographyProps={{ fontSize: '0.75rem' }}
                            />
                            <Tooltip title="Remove from favorites">
                                <IconButton
                                    size="small"
                                    onClick={(event) =>
                                        handleRemoveFavorite(event, favorite.chainId, favorite.zipCode)
                                    }
                                    sx={{
                                        color: 'primary.main',
                                        opacity: 0.7,
                                        '&:hover': { opacity: 1 },
                                    }}
                                >
                                    <StarIcon fontSize="small" />
                                </IconButton>
                            </Tooltip>
                        </ListItemButton>
                    );
                })}

                {favorites.length === 0 && (
                    <Box sx={{ px: 1, py: 3, textAlign: 'center' }}>
                        <StarBorderIcon sx={{ fontSize: '2rem', color: 'text.secondary', mb: 1 }} />
                        <Typography variant="caption" color="text.secondary" display="block">
                            Star locations on the home page to add favorites here
                        </Typography>
                    </Box>
                )}
            </List>
        </Box>
    );
};
