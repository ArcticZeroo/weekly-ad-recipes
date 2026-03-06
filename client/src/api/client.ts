import type { DealsResponse } from '../models/generated/DealsResponse.ts';
import type { MealsResponse } from '../models/generated/MealsResponse.ts';
import type { StoreChain } from '../models/generated/StoreChain.ts';
import type { StoreLocation } from '../models/generated/StoreLocation.ts';

export interface IFlippStoreMatch {
    chain_id: string;
    chain_name: string;
    flyer_id: number;
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

// Store locations — cached in memory to avoid re-fetch on every navigation
let locationsCache: StoreLocation[] | null = null;

export const fetchLocations = async (): Promise<StoreLocation[]> => {
    const fresh = await fetchJson<StoreLocation[]>('/api/locations');
    locationsCache = fresh;
    return fresh;
};

export const getCachedLocations = (): StoreLocation[] | null => locationsCache;

export const invalidateLocationsCache = (): void => {
    locationsCache = null;
};

export interface IAddLocationRequest {
    chain_id: string;
    name: string;
    address?: string;
    zip_code: string;
    flipp_merchant_id?: number;
    flipp_merchant_name?: string;
    weekly_ad_url?: string;
}

export const addLocation = async (location: IAddLocationRequest): Promise<StoreLocation> => {
    const result = await fetchJson<StoreLocation>('/api/locations', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(location),
    });
    invalidateLocationsCache();
    return result;
};

export const deleteLocation = async (locationId: number): Promise<void> => {
    const response = await fetch(`/api/locations/${locationId}`, {
        method: 'DELETE',
    });

    if (!response.ok) {
        const text = await response.text().catch(() => response.statusText);
        throw new Error(`Request failed (${response.status}): ${text}`);
    }

    invalidateLocationsCache();
};

export const searchLocations = async (zipCode: string): Promise<IFlippStoreMatch[]> => {
    return fetchJson<IFlippStoreMatch[]>(`/api/locations/search?zip=${encodeURIComponent(zipCode)}`);
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
