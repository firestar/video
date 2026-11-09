-- Support editing a comment and pinning one comment per video (by the owner).
ALTER TABLE comments ADD COLUMN updated_at TIMESTAMPTZ NULL;
ALTER TABLE comments ADD COLUMN pinned_at TIMESTAMPTZ NULL;
CREATE INDEX IF NOT EXISTS idx_comments_pinned ON comments(video_id, pinned_at) WHERE pinned_at IS NOT NULL;
