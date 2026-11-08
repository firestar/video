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
- **Backend compile breakage from `fac5f59`** (discovered while working on quality selector):
  - The studio commit never compiled the backend; it shipped with 6 syntax errors in `routes.rs`:
    1. `ApiError} {` instead of `ApiError>` (update_video signature)
    2. `State<AppState),` instead of `State<AppState>,` (delete_video signature)
    3. missing `)` in delete_video's `json!` response
    4. `|r)` instead of `|r|` in `me` handler
    5. missing closing `}` on `me` function
    6. `exists().then(|| ... .await)` — invalid: `.then` closure can't be async, and `exists()` returns `Result<bool>` not `bool` (3 sites: get_video stream, get_video thumbnail, delete_video)
  - Also `Uuid::from_str` used without importing `std::str::FromStr`
  - **Consequence: the live API has been running a stale image since before `fac5f59`** — the studio backend routes (PATCH/DELETE video, `/api/me` channelId) were never actually live
  - All fixed; `cargo build --workspace` + `cargo test --workspace` (38 tests) green
- **Video quality selector (backend)**:
  - Migration `005_streams.sql`: `ALTER TABLE videos ADD COLUMN streams JSONB`
  - `domain::StreamVariant` + `Video.streams: Vec<StreamVariant>`
  - Worker: multi-quality ladder — 720p (source ≤720p) or 1080/720/480/360 (source >720p); one ffmpeg pass per variant, keys `videos/{id}_{quality}.mp4`; stores `streams` JSONB with quality/width/height/sizeBytes/objectKey
  - `finalize` now also sets `object_key` to the top-quality variant (fixes latent bug: `streamUrl` was signing the raw source file, not the transcode)
  - `get_video` returns `streams: [{quality, width, height, sizeBytes, url}]` with signed URLs
  - Verified end-to-end locally with `smoke-test-quality.sh` (docker redis+postgres+s3mock, real worker, 720p source → 3 variants)
 - **Video quality selector (frontend)**:
   - `types.ts`: `StreamVariant` interface, `Video.streams?`
   - Watch page: quality menu button (shows current quality) → dropdown of available qualities; switching swaps `src` and restores `currentTime`; falls back to single `streamUrl` when no `streams` (old videos)
   - **Shipped live** (commit `b845ccd`, deployed 06:37–06:39 UTC): live E2E verified — upload 720p source → 3 variants (1280×720/854×480/640×360) in ~3s, signed stream URLs, `object_key` = top quality
   - Root cause of the 06:04 queue failure: stale `sourceKey`/`outputPrefix` payload format from before the multi-quality rework; worker + smoke script on disk were already on `videoId`/`objectKey`
 - **Comments fix**: `POST /api/videos/{id}/comments` was registered on the PUBLIC router but its handler requires the `JwtClaims` extension that only `auth_middleware` injects on protected routes → every comment creation returned 500. Moved the route to the protected router (commit `8a13a27`, deployed 07:35 UTC). Live-verified: POST → 201, comment appears in `GET /api/comments/{videoId}` with author joined
  - **Notifications (shipped live, commit `a60b8df`, deployed 07:55–07:57 UTC)**:
    - Migration `006_notifications.sql`: notifications table (user, channel, video, type, read_at) + per-user indexes
    - db crate `notifications.rs`: `notify_new_uploads` (fan-out to subscribers, excludes owner), `list_for_user` (joined channel name/avatar + video title/thumbnail), `unread_count`, `mark_read`, `mark_all_read`
    - Worker: after a video finalizes `ready`, fan out `new_upload` notifications (best-effort — never fails a finished transcode)
    - API (protected): `GET /api/notifications`, `POST /api/notifications/read-all`, `POST /api/notifications/{id}/read`
    - Frontend: `NotificationsBell.tsx` in Navbar — bell with unread badge, 30s polling, dropdown (channel avatar + video title + relative time), click marks read + opens video, mark-all-as-read
    - Live E2E verified (`e2e-live-notifications.sh`): subscribe → upload → subscriber gets 1 notification, owner gets none, mark-read → unreadCount 0; dropdown screenshot-verified in browser
  - **Channel avatar + banner upload (in flight)**:
    - `presign_upload` accepts `purpose`: `avatar` (≤5MB, image only, `avatars/<uuid>.jpg`), `banner` (≤10MB, image only, `banners/<uuid>.jpg`), default `video`
    - `users::update_avatar` db fn; `ChannelUpdateBody.avatar_url`; `PATCH /api/channels/{id}` sets the user's avatar
    - `GET /api/channels/{id}` + PATCH response now include `channel.avatarUrl` (was missing — frontend read it but backend never sent it)
    - Frontend: `ChannelEditor.tsx` (owner-only "Edit channel" on the channel page): name, description, avatar + banner file pickers with previews; presign → PUT → PATCH with public `/media/` URL; `mediaUrl()` helper in `lib/api.ts`
  
  ## In flight
  - [ ] Channel avatar + banner upload: build, test, commit, push, live-verify (presign purpose + PATCH + channel page render)
  - [ ] Add playlist / watch-next queue
 ## In flight
 - [ ] Notifications: wait for Kaiad build of `a60b8df` (backend + worker + frontend), then live E2E: subscribe → upload → notification appears → mark read → unreadCount 0
 - [ ] Add playlist / watch-next queue

## Key Files
- `backend/crates/api/src/routes.rs` — all API handlers
- `backend/crates/storage/src/lib.rs` — `signed_get` and `public_url`
- `backend/crates/api/src/state.rs` — `AppState` with `storage` field
- `backend/crates/worker/src/jobs.rs` — multi-quality transcode pipeline, thumbnail, key format `videos/{id}_{quality}.mp4`, notification fan-out after ready
- `backend/crates/db/src/notifications.rs` — notification queries (fan-out, list, unread, mark read)
- `frontend/src/components/NotificationsBell.tsx` — navbar bell + dropdown
- `frontend/src/components/ChannelEditor.tsx` — owner channel editor (name/description/avatar/banner)
- `e2e-live-notifications.sh` — live notifications E2E (gitignored like all *.sh)
- `backend/crates/db/src/videos.rs` — `finalize` takes `streams` + updates `object_key`
- `backend/crates/db/migrations/005_streams.sql` — `streams JSONB` column
- `frontend/src/app/watch/[videoId]/page.tsx` — quality menu
- `smoke-test-quality.sh` — local multi-quality integration test (gitignored like all *.sh)
- `docker-compose.yml` — service definitions
- `kaiad.yaml` — Kaiad pipeline config

## Notes
- `streamUrl` is presigned in `get_video`; `object_key` now points at the top-quality transcode (was the raw source)
- `thumbnailUrl` uses `signed_get` in `videos_with_channels`
- Kaiad build should go green after push to main
- Local test env quirks (this host): `minio/*` Docker Hub org is egress-blocked → use `adobe/s3mock` (bucket must be ≥3 chars, create via `PUT /{bucket}` after startup); `pg:16` → `postgres:16`
