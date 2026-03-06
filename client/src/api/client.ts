import type { DealsResponse } from '../models/generated/DealsResponse.ts';
import type { MealsResponse } from '../models/generated/MealsResponse.ts';
import type { StoreChain } from '../models/generated/StoreChain.ts';
import type { StoreLocation } from '../models/generated/StoreLocation.ts';

export interface IFlippStoreMatch {
    chain_id: string;
    chain_name: string;
    flyer_id: number | null;
    merchant_id: number | null;
    merchant_name: string;
    store_name: string | null;
    valid_from: string | null;
    valid_to: string | null;
}

const fetchJson = async <T>(url: string, options?: RequestInit): Promise<T> => {
    const response = await fetch(url, options);

    if (!response.ok) {
        const text = await response.text().catch(() => response.statusText);
        throw new Error(`Request failed (${response.status}): ${text}`);
    }

    return response.json() as Promise<T>;
};

// Store chains
export const fetchChains = async (): Promise<StoreChain[]> => {
    return fetchJson<StoreChain[]>('/api/chains');
};

// Search for stores by zip (lightweight, no DB writes)
export const searchLocations = async (zipCode: string): Promise<IFlippStoreMatch[]> => {
    return fetchJson<IFlippStoreMatch[]>(`/api/locations/search?zip=${encodeURIComponent(zipCode)}`);
};

// Resolve a store match into a stable StoreLocation with an ID (creates in DB if needed)
export const resolveLocation = async (match: IFlippStoreMatch, zipCode: string): Promise<StoreLocation> => {
    return fetchJson<StoreLocation>('/api/locations/resolve', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            chain_id: match.chain_id,
            chain_name: match.chain_name,
            zip_code: zipCode,
            flipp_merchant_id: match.merchant_id,
            flipp_merchant_name: match.merchant_name,
        }),
    });
};

// Deals
export const fetchDeals = async (locationId: number): Promise<DealsResponse> => {
    return fetchJson<DealsResponse>(`/api/deals/${locationId}`);
};

export const refreshDeals = async (locationId: number): Promise<DealsResponse> => {
    return fetchJson<DealsResponse>(`/api/deals/${locationId}/refresh`, {
        method: 'POST',
    });
};

// Meals
export const fetchMeals = async (locationId: number): Promise<MealsResponse> => {
    return fetchJson<MealsResponse>(`/api/meals/${locationId}`);
};
