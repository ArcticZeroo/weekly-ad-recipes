export const CHAIN_DISPLAY_NAMES: Record<string, string> = {
    'qfc': 'QFC',
    'safeway': 'Safeway',
    'fred-meyer': 'Fred Meyer',
    'whole-foods': 'Whole Foods',
};

export const displayChainName = (chainId: string): string =>
    CHAIN_DISPLAY_NAMES[chainId] ?? chainId;
