CREATE TABLE wfm_stores (
    store_id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    city TEXT,
    state TEXT,
    zip_code TEXT,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE wfm_store_lookups (
    zip_code TEXT PRIMARY KEY,
    store_id TEXT NOT NULL REFERENCES wfm_stores(store_id),
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
