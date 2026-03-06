import { useContext, useSyncExternalStore } from 'react';
import type { ValueNotifier } from '../util/events.ts';

export const useValueNotifier = <T>(notifier: ValueNotifier<T>): T => {
    return useSyncExternalStore(
        (callback) => notifier.subscribe(callback),
        () => notifier.value,
    );
};

export const useValueNotifierContext = <T>(context: React.Context<ValueNotifier<T>>): T => {
    const notifier = useContext(context);
    return useValueNotifier(notifier);
};
