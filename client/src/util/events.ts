type Callback<T> = (value: T) => void;

export class ValueNotifier<T> {
    private readonly _subscribers = new Set<Callback<T>>();
    private _value: T;

    constructor(initialValue: T) {
        this._value = initialValue;
    }

    get value(): T {
        return this._value;
    }

    set value(newValue: T) {
        this._value = newValue;
        this._notifyAll();
    }

    subscribe(callback: Callback<T>): () => void {
        this._subscribers.add(callback);
        return () => {
            this._subscribers.delete(callback);
        };
    }

    private _notifyAll(): void {
        for (const callback of this._subscribers) {
            callback(this._value);
        }
    }
}

interface IRefCountedOptions<T> {
    setup: () => T;
    teardown: (value: T) => void;
}

export class RefCountedValueNotifier<T> extends ValueNotifier<T> {
    private _referenceCount = 0;
    private readonly _options: IRefCountedOptions<T>;

    constructor(initialValue: T, options: IRefCountedOptions<T>) {
        super(initialValue);
        this._options = options;
    }

    subscribe(callback: Callback<T>): () => void {
        this._referenceCount++;
        if (this._referenceCount === 1) {
            this.value = this._options.setup();
        }

        const unsubscribe = super.subscribe(callback);

        return () => {
            unsubscribe();
            this._referenceCount--;
            if (this._referenceCount === 0) {
                this._options.teardown(this.value);
            }
        };
    }
}
