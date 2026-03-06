import { createContext } from 'react';
import { ValueNotifier } from '../util/events.ts';

export const selectedLocationNotifier = new ValueNotifier<number | null>(null);

export const SelectedLocationContext = createContext<ValueNotifier<number | null>>(
    selectedLocationNotifier,
);
