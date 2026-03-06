# Weekly Ad Recipes

## Project Overview

A web app that downloads weekly ads from local grocery stores (QFC, Safeway, Whole Foods, Fred Meyer), uses AI to extract/categorize deals, and suggests meal ideas from current sales.

**Tech Stack**: Rust + Axum backend, React 19 + TypeScript + Vite frontend, SQLite for caching, Claude API for AI.

## Code Change Guidance

Always make a checklist with your plan before making changes.

If you find important learnings during development, consider documenting them in this file for future reference.

### General Guidance

- Avoid duplicating code wherever possible. Look for opportunities to share code
- Try to avoid huge multi-purpose functions where possible. Move out individual logical parts into helpers so that the big functions are more readable
- No obvious self-evident comments

### JavaScript/TypeScript Guidance

- No `any`
- No `as` casts unless absolutely necessary
- `const` to declare functions instead of the `function` keyword
- `{ type }` instead of `{ type: type }` in objects
- No abbreviated variable or function names. Always use full words:
  - `index` not `idx` (but `i` is acceptable for loop indices)
  - `button` not `btn`
  - `event` not `e` or `evt`
  - `value` not `val`
  - `element` not `el` or `elem`
  - `result` not `res`
  - `error` not `err`
  - `response` not `resp` or `res`
  - `callback` not `cb`
  - `parameter` not `param`

### TypeScript Patterns

- All interface names start with `I` (e.g., `IMyComponentProps`, `IDealCardProps`)
- Use `React.FC<IProps>` for typed components
- Strict mode: `noUnusedLocals`, `noUnusedParameters`, `noUncheckedIndexedAccess`

### React Patterns

```tsx
interface IMyComponentProps {
    prop1: string;
    prop2?: boolean;
}

export const MyComponent: React.FC<IMyComponentProps> = ({ prop1, prop2 = false }) => {
    return <div />;
};
```

### Promise Handling

Use `@arcticzeroo/react-promise-hook` (`useImmediatePromiseState`, `useDelayedPromiseState`).

**You MUST handle all three states**: loading, error, and success. Never skip loading or error handling.

```tsx
import { PromiseStage, useImmediatePromiseState } from '@arcticzeroo/react-promise-hook';

const response = useImmediatePromiseState(fetchData);

if (response.stage === PromiseStage.error) {
    return (
        <div className="card error">
            <span>Unable to load data!</span>
            <button onClick={response.run}>Retry</button>
        </div>
    );
}

if (response.value == null) {
    return <LoadingSpinner />;
}

return <MyDataView data={response.value} />;
```

### CSS Utility Classes

Use the utility classes defined in `index.scss` instead of writing `display: flex` in component CSS files:

| Class | Effect |
|---|---|
| `flex` | `display: flex; align-items: center; gap: var(--default-padding);` |
| `flex-col` | `display: flex; flex-direction: column; gap: var(--default-padding);` |
| `flex-center` | `align-items: center; justify-content: center;` |
| `flex-wrap` | `flex-wrap: wrap;` |
| `flex-between` | `justify-content: space-between;` |
| `flex-grow` | `flex-grow: 1;` |

Combine: `className="flex flex-center flex-wrap"`

### CSS Colors & Measurements

- All colors must come from CSS custom properties in `index.scss`. Never hardcode hex colors.
- Use `var(--default-padding)` for responsive spacing (adapts to breakpoints)
- Use `var(--constant-padding)` for fixed `0.5rem` spacing
- Never hardcode `rem`/`px` values for standard spacing

### Mobile Responsiveness

- Use the `useDeviceType` hook (800px breakpoint) in code
- Use `@media (max-width: 800px)` in SCSS

### Rust Guidance

- Use `thiserror` for error types
- Implement `IntoResponse` on `AppError` for Axum
- Use `sqlx` with compile-time checked queries where practical
- Use `serde` for all serialization
- Use `ts-rs` with `#[derive(TS)]` and `#[ts(export)]` on all API response types

## Build & Development

### Server (Rust)
```bash
cd server
cargo build          # build
cargo test           # run tests + generate TypeScript bindings
cargo run            # start server on port 3001
```

### Client (React)
```bash
cd client
npm install          # install dependencies
npm run dev          # dev server on port 5173, proxies /api to localhost:3001
npm run build        # production build to dist/
npm run lint         # ESLint
```

### Environment Variables
Create `server/.env`:
```
ANTHROPIC_API_KEY=sk-ant-...
DATABASE_URL=sqlite:data.db
```

## Type Sharing

Rust structs with `#[derive(TS)]` auto-generate TypeScript interfaces in `client/src/models/generated/` when running `cargo test`. Generated files are committed.
