import type { DealsResponse } from '../models/generated/DealsResponse.ts';
import type { MealsResponse } from '../models/generated/MealsResponse.ts';
import type { StoreChain } from '../models/generated/StoreChain.ts';

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

// Deals
export const fetchDeals = async (chain: string, zip: string): Promise<DealsResponse> => {
    return fetchJson<DealsResponse>(`/api/deals/${encodeURIComponent(chain)}/${encodeURIComponent(zip)}`);
};

export const refreshDeals = async (chain: string, zip: string): Promise<DealsResponse> => {
    return fetchJson<DealsResponse>(`/api/deals/${encodeURIComponent(chain)}/${encodeURIComponent(zip)}/refresh`, {
        method: 'POST',
    });
};

// Meals
export const fetchMeals = async (chain: string, zip: string): Promise<MealsResponse> => {
    return fetchJson<MealsResponse>(`/api/meals/${encodeURIComponent(chain)}/${encodeURIComponent(zip)}`);
};
