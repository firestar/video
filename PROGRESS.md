# Progress

## Done
- **Thumbnail URL fix (list responses)**: `backend/crates/api/src/routes.rs`
  - Added `storage` parameter to `videos_with_channels` helper
  - All callers updated: `list_videos`, `get_related_videos`, `search`, `home_feed` (2 spots)
  - `thumbnailUrl` is now a presigned GET URL (24h TTL), matching `streamUrl` pattern
  - Commit: `75a3674` — "Sign thumbnail URLs in list responses"
- **Single video endpoint (`get_video`)**: sign `thumbnailUrl` in `GET /api/videos/{id}`
- **Feed endpoint**: sign `thumbnailUrl` in `GET /api/home/feed`
- **Search endpoint**: sign `thumbnailUrl` in `GET /api/search`
- **Channel endpoint**: sign `thumbnailUrl` in `GET /api/channels/{id}`
- **History endpoint**: sign `thumbnailUrl` in `GET /api/history`
- **Studio feature**: edit/delete own videos (`PATCH`/`DELETE /api/videos/{id}`), list all channel videos (`GET /api/videos?channelId=...`), `/api/me` returns `channelId`; frontend `/studio` page for channel management
- **Home feed**: home page now uses `/api/home/feed` with reason badges (trending/subscription/related)
- **Watch history**: history page shows video title, thumbnail, channel, view count; supports clear-all
- **Studio page build fix**: resolved 4 syntax errors in `/studio` (missing `)` in `deleteVideo`, missing `}` in 3 `onChange` handlers, missing `}` in the `publishedAt` span) — frontend now builds clean (verified with `tsc` + `next build`)

## Next
- [ ] Verify live deployment after push (Kaiad builds on each commit)
- [ ] Add a video player quality/bitrate selector
- [ ] Add playlist / watch-next queue

## Key Files
- `backend/crates/api/src/routes.rs` — all API handlers
- `backend/crates/storage/src/lib.rs` — `signed_get` and `public_url`
- `backend/crates/api/src/state.rs` — `AppState` with `storage` field
- `backend/crates/worker/src/jobs.rs` — thumbnail generation, key format `thumbnails/{id}.jpg`
- `docker-compose.yml` — service definitions
- `kaiad.yaml` — Kaiad pipeline config

## Notes
- `streamUrl` is already presigned in `get_video` (line ~255-273)
- `thumbnailUrl` was using `public_url` (unsigned) — now uses `signed_get` in `videos_with_channels`
- Kaiad build should go green after push to main
