import { createContext } from 'react';
import { ValueNotifier } from '../util/events.ts';
import { getPreferredLocationId } from '../storage/preferences.ts';

export const selectedLocationNotifier = new ValueNotifier<number | null>(
    getPreferredLocationId(),
);

export const SelectedLocationContext = createContext<ValueNotifier<number | null>>(
    selectedLocationNotifier,
);
