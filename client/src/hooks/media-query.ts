import { useCallback, useSyncExternalStore } from 'react';

const MOBILE_BREAKPOINT_PX = 800;

export const DeviceType = {
    Mobile: 'mobile',
    Desktop: 'desktop',
} as const;

export type DeviceType = (typeof DeviceType)[keyof typeof DeviceType];

const mediaQuery = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT_PX}px)`);

const subscribeToDeviceType = (callback: () => void): () => void => {
    mediaQuery.addEventListener('change', callback);
    return () => mediaQuery.removeEventListener('change', callback);
};

const getDeviceType = (): DeviceType => {
    return mediaQuery.matches ? DeviceType.Mobile : DeviceType.Desktop;
};

export const useDeviceType = (): DeviceType => {
    const subscribe = useCallback(
        (callback: () => void) => subscribeToDeviceType(callback),
        [],
    );

    return useSyncExternalStore(subscribe, getDeviceType);
};
