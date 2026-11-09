# synload API e2e suite

A rerunnable, self-contained end-to-end test suite for the synload backend.
It exercises every feature through the **public HTTP API** against a live
backend. The shared owner/viewer users are fixed accounts (registered once,
then logged in on every run); each test creates its own videos, so the suite
is rerunnable without accumulating a fresh account per run.

## Run

```bash
# against the production backend (default)
python3 e2e/run.py

# against any backend
E2E_BASE=https://staging.synload.com python3 e2e/run.py
```

Exits `0` only if there were no failures. A module that crashes is recorded as
a failure but does not stop the rest of the suite.

## Optional: ready-video checks

Most features work on a video of any status, so the suite is fully functional
with just API access. A few endpoints (`GET /api/videos`, `/api/home/feed`,
`/api/shorts`, related) filter on `status='ready'`. The suite leans on the
site's existing ready content for those.

To additionally assert that *a video the suite itself created* appears in the
public list / shorts feed, set `E2E_SQL_CMD` — a shell command that reads a SQL
statement on stdin and runs it against the app database. On the production
deployment (Postgres container `video-postgres` on agent `video-01`):

```bash
E2E_SQL_CMD='ssh -i ~/.ssh/<key> ops@192.168.1.43 "docker exec -i video-postgres psql -U postgres -d video -tA"' \
  python3 e2e/run.py
```

Without it, those specific assertions are reported as `SKIP` (not failures).

## What it covers

| Module | Feature |
|--------|---------|
| `test_01_auth` | register, login, `/api/me`, duplicate email, weak password, bad credentials |
| `test_02_account` | update profile (name/email/bio), change password, re-login |
| `test_03_videos` | create, get, public list, update, delete, visibility, related, channel-videos |
| `test_04_comments` | create, threaded list, reply, edit, delete, like, pin/unpin |
| `test_05_likes` | like/unlike, dislike/undislike, `hasLiked`/`hasDisliked` |
| `test_06_subscriptions` | subscribe, list, unsubscribe, own-channel 400, 404 |
| `test_07_watch_later` | add, list, remove, `inWatchLater` |
| `test_08_history` | record position, list, record view, clear |
| `test_09_playlists` | create, list, get, add/remove item, update, delete, non-owner 404 |
| `test_10_search` | search videos + channels |
| `test_11_notifications` | `new_comment` + `new_like`, mark read, read-all, `unreadCount` |
| `test_12_reports` | report video, own 400, report comment, own comment 400, invalid type |
| `test_13_shorts` | upload `isShort` flag, `/api/shorts` feed shape, ready short appears (with `E2E_SQL_CMD`) |
| `test_14_analytics` | channel analytics shape, owner-only |
| `test_15_feed` | home feed shape, reasons map |

## Notes

- **Why videos end up `failed`:** the suite creates videos with a presigned
  object key but never PUTs the bytes to the bucket — the upload endpoint sits
  behind Cloudflare bot protection, which rejects non-browser clients. The
  worker then marks the video `failed`. Every feature endpoint works on a video
  of any status, so this is harmless. (A browser-based upload — e.g. the
  Playwright suite in `frontend/e2e` — exercises the real transcode path.)
- The shared owner/viewer are fixed accounts (`e2e-api-{owner,viewer}@synload.local`),
  registered once and reused; a start-of-run reset clears their notifications
  and any leftover subscription. A few dedicated tests (`test_01_auth`,
  `test_02_account`) still register throwaway users, since they test
  registration and destructive profile/password changes directly. Each run uses
  a unique `run_id` (epoch ms) so those throwaway users never collide. Created
  videos are deleted at the end (best-effort); users and channels have no
  delete endpoint and are left in place.
- No third-party dependencies — stdlib only (`urllib`, `json`, `subprocess`).
