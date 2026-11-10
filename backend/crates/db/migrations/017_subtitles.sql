-- Subtitles: per-video caption tracks (one row per language).
-- object_key always points at a WebVTT object (SRT uploads are converted
-- server-side at registration).
CREATE TABLE IF NOT EXISTS subtitles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    video_id UUID NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    language TEXT NOT NULL,
    format TEXT NOT NULL DEFAULT 'vtt',
    object_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (video_id, language)
);
CREATE INDEX IF NOT EXISTS idx_subtitles_video ON subtitles(video_id);
