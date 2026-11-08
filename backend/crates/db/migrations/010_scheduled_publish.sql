-- Scheduled publishing: videos with a future publish_at are transcoded
-- normally but stay hidden (published_at NULL) until the worker's schedule
-- tick flips published_at = publish_at once the time arrives.
ALTER TABLE videos ADD COLUMN IF NOT EXISTS publish_at TIMESTAMPTZ;
