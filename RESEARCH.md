# Research: Subtitles (CC) feature — debugging the UI e2e failure

## Context
- Live site: https://video.synload.com (Next.js frontend + axum backend + worker, deployed by kaiad from `firestar/video` main, ~1 min after push; frontend build ~20s, backend ~6.5 min, worker ~7 min).
- Edge: Cloudflare in front. Its WAF 403s a client that hammers one anonymous endpoint (the deploy-wait script polling `GET /api/videos/{id}/subtitles` every 10s got 403 for 30+ min; app's own limiter returns 429). The e2e suites (diverse requests) are not affected.
- Deployed container logs: `ssh_run` on host `video-01` (192.168.1.43, user ops): `docker logs --tail N video-<frontend|backend|worker>-0`.

## Commit timeline (subtitles)
- `685eabc` — full feature: migration 017, api presign purpose=subtitle / register (SRT→VTT) / public list / owner delete; upload-page optional subtitle; studio per-video subtitles panel (hidden `input[type=file]` + "Add .vtt / .srt" button); watch-page CC button + `<track>` + auto-enable; e2e test_16 (API) + e2e-ui/test_08 (UI).
- `99bfd99` — backend routes registered under `/api` (were dead 404s); test_08 added to the UI runner's hardcoded MODULES list.
- `612f23e` — `GET /api/videos/{id}/subtitles` moved to public (optional auth) in `backend/crates/api/src/lib.rs`; test_08 changed to actually `set_input_files` on the hidden input (previously it opened the panel but never uploaded, so it could never pass).

## The bug found (2026-11-10)
UI suite failed at `test_08_subtitles.py:59` waiting for `input[type="file"][accept*="vtt"]` (state=attached). Screenshot `08_studio_panel_open.png` (run dir `.peckboard/ui-e2e/1794285038/`) showed the studio crashed into the error boundary: **"Something went wrong — (G[e.id] ?? []).map is not a function"** (G = minified `subTracks`).

Root cause: `frontend/src/app/studio/page.tsx` `loadSubs` did
`const rows = await apiFetch<Subtitle[]>(`/api/videos/${videoId}/subtitles`)` — treating the body as a bare array, but the backend (`routes.rs` `list_subtitles`, ~line 1245) returns `Json(json!({ "subtitles": items }))`. So `subTracks[v.id]` became the truthy object `{subtitles: []}`, `?? []` didn't apply, and `.map`/`.length` blew up on re-render.
Sequence: click "Subtitles" → panel renders (tracks still undefined → Add button visible) → fetch resolves → `setSubTracks` → re-render throws → whole page replaced by error boundary → hidden input gone → test timeout.
Impact: opening the Subtitles panel in Studio crashed the entire Studio page for ANY video (even with zero tracks). Shipped unnoticed because test_08 was never registered in the UI runner until 99bfd99, and the old test never asserted panel state after the fetch resolved.

Fix (applied, pending deploy): `loadSubs` now reads `body.subtitles` with an `Array.isArray` guard.

## Verified facts
- Deployed studio chunk (content-hashed, e.g. `/_next/static/chunks/0sfph7qlof8hy.js`) contains the hidden input `accept:".vtt,.srt,text/vtt,text/srt"` — the deployed frontend matched the repo; the crash was a runtime data-shape bug, not a stale deploy.
- API suite (e2e/run.py) passes 141/0 incl. all 27 test_16_subtitles checks; it reads `body["subtitles"]` (correct shape) — which is why it couldn't catch the frontend bug.
- Watch page consumes `video.subtitles` from the detail object (bare array, `routes.rs` ~line 436) — correct. Upload page only POSTs register — correct. Studio was the only mis-parsing consumer.
- `apiFetch` (frontend/src/lib/api.ts) returns the parsed JSON body; 204/empty → undefined.

## Test-suite notes
- API runner `e2e/run.py` auto-discovers `test_*.py`; UI runner `e2e-ui/run.py` has a HARDCODED MODULES list — new UI modules must be added there or they silently never run.
- Fixed accounts (no per-run signups): API `e2e-api-{owner,viewer}@synload.local`, UI `e2e-ui-{owner,viewer}@synload.local`.
- UI screenshots land in `.peckboard/ui-e2e/<run_id>/`; viewable by serving the dir over local http (browser tool refuses file://). Port 8765 on this host is taken by another service — use e.g. 8899.
- Ad-hoc helpers at repo root (untracked): `e2e_run_both.py` (API then UI), `e2e_wait_subtitles.py`, `e2e_verify_subtitles.py`, `e2e_make_cc_video.py` (creates a real 5s captioned video + VTT for browser CC verification).
