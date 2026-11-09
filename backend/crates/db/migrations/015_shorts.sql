-- Shorts: a short-form vertical video (user-flagged at upload).
ALTER TABLE videos ADD COLUMN is_short BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX idx_videos_is_short ON videos (is_short) WHERE is_short;
