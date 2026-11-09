-- Fix: Postgres treats NULLs as distinct in UNIQUE constraints, so the
-- (user_id, video_id, comment_id, kind) constraint never deduped video
-- likes/dislikes (comment_id IS NULL) and ON CONFLICT DO NOTHING never
-- fired — double-likes inflated the counters. Rebuild the constraint with
-- NULLS NOT DISTINCT (PG15+) and repair any rows the bug already created.

-- 1. Drop duplicate vote rows (keep the oldest per user/video/comment/kind).
DELETE FROM likes a
USING likes b
WHERE a.id < b.id
  AND a.user_id = b.user_id
  AND a.video_id = b.video_id
  AND a.kind = b.kind
  AND a.comment_id IS NOT DISTINCT FROM b.comment_id;

-- 2. Rebuild counters from the actual rows.
UPDATE videos v SET
  like_count = (SELECT COUNT(*) FROM likes l
                WHERE l.video_id = v.id AND l.comment_id IS NULL AND l.kind = 'like'),
  dislike_count = (SELECT COUNT(*) FROM likes l
                   WHERE l.video_id = v.id AND l.comment_id IS NULL AND l.kind = 'dislike');

UPDATE comments c SET
  like_count = (SELECT COUNT(*) FROM likes l
                WHERE l.comment_id = c.id AND l.kind = 'like');

-- 3. Re-add the constraint so NULL comment_ids conflict correctly.
ALTER TABLE likes DROP CONSTRAINT uq_likes;
ALTER TABLE likes ADD CONSTRAINT uq_likes UNIQUE NULLS NOT DISTINCT (user_id, video_id, comment_id, kind);
