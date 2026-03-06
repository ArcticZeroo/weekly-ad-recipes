CREATE TABLE store_locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    name TEXT NOT NULL,
    address TEXT,
    zip_code TEXT NOT NULL,
    flipp_merchant_id INTEGER,
    flipp_merchant_name TEXT,
    weekly_ad_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE deals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    location_id INTEGER NOT NULL REFERENCES store_locations(id),
    week_id TEXT NOT NULL,
    item_name TEXT NOT NULL,
    brand TEXT,
    deal_description TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'uncategorized',
    image_url TEXT,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_deals_loc_week ON deals(location_id, week_id);

CREATE TABLE meal_ideas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    location_id INTEGER NOT NULL REFERENCES store_locations(id),
    week_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    on_sale_ingredients TEXT NOT NULL,
    additional_ingredients TEXT NOT NULL,
    estimated_savings TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_meals_loc_week ON meal_ideas(location_id, week_id);
