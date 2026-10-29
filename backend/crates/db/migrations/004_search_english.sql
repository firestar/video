-- Switch full-text search to the 'english' config so it matches the
-- plainto_tsquery('english', ...) used by db::videos::search (stemming + stopwords).
-- 002 must stay untouched: it is already applied on prod (sqlx checksum).
DROP INDEX IF EXISTS idx_videos_search;

ALTER TABLE videos DROP COLUMN IF EXISTS search_vector;

ALTER TABLE videos ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(description, '')), 'B')
    ) STORED;

CREATE INDEX idx_videos_search ON videos USING GIN (search_vector);
