-- Multi-quality streams: one JSON array of transcode variants per video,
-- written by the worker after the transcode pipeline.
-- Shape: [{"quality":"1080p","width":1920,"height":1080,"objectKey":"videos/<id>_1080p.mp4","sizeBytes":123}]
ALTER TABLE videos ADD COLUMN IF NOT EXISTS streams JSONB;
