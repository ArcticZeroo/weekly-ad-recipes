export const CHAIN_DISPLAY_NAMES: Record<string, string> = {
    'qfc': 'QFC',
    'whole-foods': 'Whole Foods',
};

export const displayChainName = (chainId: string): string =>
    CHAIN_DISPLAY_NAMES[chainId]
    ?? chainId.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
