import React, { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { PromiseStage, useDelayedPromiseState } from '@arcticzeroo/react-promise-hook';
import {
    Box,
    Button,
    Card,
    CardActionArea,
    CardContent,
    IconButton,
    Skeleton,
    TextField,
    Tooltip,
    Typography,
} from '@mui/material';
import StarIcon from '@mui/icons-material/Star';
import StarBorderIcon from '@mui/icons-material/StarBorder';
import SearchIcon from '@mui/icons-material/Search';
import { searchLocations, type IFlippStoreMatch } from '../../api/client.ts';
import { useFavorites } from '../../context/favorites-context.tsx';
import { ErrorCard } from '../common/error-card.tsx';

const HomePage: React.FC = () => {
    const [zipCode, setZipCode] = useState('');
    const [searchedZip, setSearchedZip] = useState('');
    const { addFavorite, removeFavorite, isFavorite } = useFavorites();
    const navigate = useNavigate();

    const searchCallback = useCallback(() => searchLocations(zipCode), [zipCode]);
    const searchResponse = useDelayedPromiseState(searchCallback, false);

    const handleSearch = (event: React.FormEvent) => {
        event.preventDefault();
        if (zipCode.trim().length === 0) {
            return;
        }
        searchResponse.run();
        setSearchedZip(zipCode.trim());
    };

    const handleToggleFavorite = (event: React.MouseEvent, chainId: string, zip: string) => {
        event.preventDefault();
        event.stopPropagation();
        if (isFavorite(chainId, zip)) {
            removeFavorite(chainId, zip);
        } else {
            addFavorite(chainId, zip);
        }
    };

    const isMatchFavorited = (match: IFlippStoreMatch): boolean =>
        isFavorite(match.chain_id, searchedZip);

    const renderStoreCard = (
        chainId: string,
        zipCode: string,
        primaryText: string,
        secondaryText?: string,
        isFavorited: boolean = false,
        onToggle?: (event: React.MouseEvent) => void,
    ) => (
        <Card
            key={`${chainId}-${zipCode}`}
            variant="outlined"
            sx={{ position: 'relative', '&:hover': { borderColor: 'primary.main' } }}
        >
            <CardActionArea
                onClick={() => navigate(`/${chainId}/${zipCode}/deals`)}
                sx={{ pr: onToggle ? 6 : undefined }}
            >
                <CardContent>
                    <Typography variant="subtitle1" fontWeight={600}>
                        {primaryText}
                    </Typography>
                    {secondaryText && (
                        <Typography variant="body2" color="text.secondary">
                            {secondaryText}
                        </Typography>
                    )}
                </CardContent>
            </CardActionArea>
            {onToggle && (
                <Tooltip title={isFavorited ? 'Remove from favorites' : 'Add to favorites'}>
                    <IconButton
                        onClick={onToggle}
                        sx={{
                            position: 'absolute',
                            right: 8,
                            top: '50%',
                            transform: 'translateY(-50%)',
                            color: isFavorited ? 'primary.main' : 'text.secondary',
                        }}
                    >
                        {isFavorited ? <StarIcon /> : <StarBorderIcon />}
                    </IconButton>
                </Tooltip>
            )}
        </Card>
    );

    return (
        <Box sx={{ maxWidth: 900, width: '100%', mx: 'auto', display: 'flex', flexDirection: 'column', gap: 3 }}>
            <Typography variant="h4">Weekly Ad Recipes</Typography>

            <Box
                component="form"
                onSubmit={handleSearch}
                sx={{ display: 'flex', gap: 1 }}
            >
                <TextField
                    type="text"
                    placeholder="Enter zip code to find stores"
                    value={zipCode}
                    onChange={(event) => setZipCode(event.target.value)}
                    sx={{ maxWidth: 280 }}
                />
                <Button
                    type="submit"
                    variant="contained"
                    disabled={zipCode.trim().length === 0 || searchResponse.stage === PromiseStage.running}
                    startIcon={<SearchIcon />}
                >
                    Search
                </Button>
            </Box>

            {searchResponse.stage === PromiseStage.error && (
                <ErrorCard message="Unable to search locations." onRetry={searchResponse.run} />
            )}

            {searchResponse.stage === PromiseStage.running && (
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                    <Typography variant="body2" color="text.secondary">
                        Searching for stores...
                    </Typography>
                    <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))', gap: 2 }}>
                        {Array.from({ length: 3 }).map((_, index) => (
                            <Skeleton key={index} variant="rounded" height={72} />
                        ))}
                    </Box>
                </Box>
            )}

            {searchResponse.value != null && searchResponse.value.length > 0 && (
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                    <Typography variant="h6">Stores near {searchedZip}</Typography>
                    <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))', gap: 2 }}>
                        {searchResponse.value.map((match) =>
                            renderStoreCard(
                                match.chain_id,
                                searchedZip,
                                match.chain_name,
                                match.store_name ?? undefined,
                                isMatchFavorited(match),
                                (event) => handleToggleFavorite(event, match.chain_id, searchedZip),
                            ),
                        )}
                    </Box>
                </Box>
            )}

            {searchResponse.value != null && searchResponse.value.length === 0 && (
                <Typography color="text.secondary" textAlign="center">
                    No supported stores found for this zip code.
                </Typography>
            )}

            {searchResponse.value == null && searchResponse.stage !== PromiseStage.running && (
                <Box sx={{ textAlign: 'center', py: 4 }}>
                    <Typography color="text.secondary">
                        Search for stores by zip code to get started.
                    </Typography>
                    <Typography color="text.secondary">
                        Star your favorites for quick access.
                    </Typography>
                </Box>
            )}
        </Box>
    );
};

export default HomePage;
