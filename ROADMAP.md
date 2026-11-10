# Roadmap

Feature roadmap for the video platform. Work top to bottom; check items off as
they ship live and are e2e-verified.

- [x] **Custom video thumbnails** — worker auto-generates a thumbnail; owner
  can replace it from /studio (presigned upload, overwrites the auto frame).
  Live + e2e-ui `test_07_thumbnail` (7/7). Commits `4dd21b3` (feature),
  `6fec3be` (test fix: fetch the signed URL, not the 403-ing public URL).
- [x] **Subtitles / captions** — upload CC tracks (VTT/SRT) per video from
  the upload page and /studio; watch player shows a captions (CC) menu with
  language selection and renders the active track. Live + e2e `test_16_subtitles`
  (API 141) and e2e-ui `test_08_subtitles` (UI 70). Commits `685eabc` (feature),
  `99bfd99` (route prefix + UI test registration), `a9cc3bb` (studio crash fix),
  `ff6eb46` (UI test fixes).
- [x] **Embed** — per-video embeddable player: `/embed/{videoId}` is a
  chrome-free autoplay-muted page (public API, no login; quality + CC menus,
  `?t=`/`?autoplay=`, 'Video not available' for missing/private) and the watch
  page offers an Embed dialog with a copyable iframe snippet. CSP: `frame-ancestors *`
  on `/embed/`, `'self'` elsewhere. Live + e2e `test_17_embed` (API 156) and
  e2e-ui `test_09_embed` (UI 81, incl. a true cross-origin iframe via a local
  host page). Commits `d203d70` (feature), `ef9f653` (embed textarea aria-label).
- [ ] **Age restriction** — mark a video as 18+; age-gate on the watch page
  (and in lists) for logged-out / underage viewers.
- [ ] **Live streaming** — RTMP ingest, live watch page, live chat.

## Conventions

- Backend: Rust/Axum, SQLx/Postgres, migrations in `backend/crates/db/migrations/`.
- Storage: Garage (S3), bucket `media`; private bucket — serve via presigned
  GET URLs (the anonymous public URL 403s; the frontend always uses the
  signed URL the API returns).
- Deploy: push to `main` → Kaiad builds all 3 services and deploys to
  agent `video-01` (see `kaiad.yaml`).
- Verify: API suite `python3 e2e/run.py` (156 checks) and real-input UI suite
  `python3 e2e-ui/run.py` (81 checks) against the live site. Fixed test
  accounts are reused; never create new accounts on the live site.
