DROP INDEX IF EXISTS idx_unique_location;
CREATE UNIQUE INDEX idx_unique_merchant ON store_locations(flipp_merchant_id) WHERE flipp_merchant_id IS NOT NULL;
