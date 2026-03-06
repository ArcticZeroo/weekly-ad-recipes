export const classNames = (...names: Array<string | false | null | undefined>): string => {
    return names.filter(Boolean).join(' ');
};
