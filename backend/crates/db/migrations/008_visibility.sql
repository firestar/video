-- Video visibility: public (default), unlisted (viewable by URL, hidden from
-- listings/search), private (owner only).
ALTER TABLE videos
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public'
    CHECK (visibility IN ('public', 'unlisted', 'private'));

CREATE INDEX IF NOT EXISTS idx_videos_visibility ON videos(visibility);
