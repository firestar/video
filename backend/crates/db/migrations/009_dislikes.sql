-- Dislikes: a `kind` on likes (like/dislike are mutually exclusive on videos;
-- comments stay like-only) plus a dislike counter on videos.
ALTER TABLE likes ADD COLUMN kind TEXT NOT NULL DEFAULT 'like';
ALTER TABLE likes DROP CONSTRAINT likes_user_id_video_id_comment_id_key;
ALTER TABLE likes ADD CONSTRAINT uq_likes UNIQUE (user_id, video_id, comment_id, kind);
ALTER TABLE videos ADD COLUMN dislike_count BIGINT NOT NULL DEFAULT 0;
