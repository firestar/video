"""Embed through the real UI: the embeddable player plays in a logged-out
browser with no site chrome, plays inside a CROSS-ORIGIN iframe (a local
page hosting the iframe — the real third-party use case), the watch page
offers a copyable iframe snippet, and private videos are gated.
"""
import functools
import http.server
import os
import threading
import time

from harness import BASE, VIDEOS_DIR


def _wait_played(video, timeout=30):
    """Poll until the <video> has actually played (currentTime > 0).

    The generated clips are only ~3 s, so a finished (paused) video still
    counts: currentTime only advances by playing. Returns (played, state).
    """
    t0 = time.time()
    state = None
    while time.time() - t0 < timeout:
        state = video.evaluate(
            "v => ({paused: v.paused, currentTime: v.currentTime, "
            "readyState: v.readyState, error: v.error ? v.error.code : null})"
        )
        if state and state["currentTime"] > 0 and state["readyState"] >= 2:
            return True, state
        time.sleep(0.5)
    return False, state


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return

    vid = ctx.ensure_ready("embed_ui", title=f"E2E-UI Embed {ctx.run_id}")
    if not vid:
        return

    # --- 1. Logged-out browser opens /embed/{id} directly: plays, no chrome. ---
    anon = ui.pages.get("anon")
    if not anon:
        anon = ui.new_context("anon")
    anon.goto(BASE + f"/embed/{vid}", wait_until="domcontentloaded")
    video = anon.locator("video").first
    video.wait_for(state="visible", timeout=30000)
    played, state = _wait_played(video)
    ui.shot(anon, "09 embed page playing")
    ctx.check("embed page plays (logged out)", played, f"state={state}")
    ctx.check("embed page has no site navbar",
              anon.locator("header.sticky").count() == 0
              and anon.get_by_text("YT Clone").count() == 0,
              "site chrome leaked into the embed")
    hdr = anon.locator("header").first
    ctx.check("embed header shows the title",
              "E2E-UI Embed" in hdr.inner_text(),
              f"header={hdr.inner_text()!r}")

    # --- 2. Cross-origin iframe: a local http:// page hosts the iframe. ---
    os.makedirs(VIDEOS_DIR, exist_ok=True)
    host_html = os.path.join(VIDEOS_DIR, f"{ctx.run_id}_embed_host.html")
    with open(host_html, "w") as f:
        f.write(
            "<!doctype html><html><body style='margin:0;background:#111'>"
            f"<iframe id='f' src='{BASE}/embed/{vid}' width='640' height='360' "
            "allow='autoplay; fullscreen; picture-in-picture' allowfullscreen></iframe>"
            "</body></html>"
        )
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=VIDEOS_DIR)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    port = server.server_address[1]
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        anon.goto(f"http://127.0.0.1:{port}/{os.path.basename(host_html)}",
                  wait_until="domcontentloaded")
        fvideo = anon.frame_locator("#f").locator("video")
        fvideo.wait_for(state="visible", timeout=30000)
        played, state = _wait_played(fvideo)
        ui.shot(anon, "09 cross-origin iframe playing")
        ctx.check("cross-origin iframe plays the video", played, f"state={state}")
    finally:
        server.shutdown()
        try:
            os.remove(host_html)
        except OSError:
            pass

    # --- 3. Watch page: the Embed button offers a copyable iframe snippet. ---
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])
    try:
        ui.contexts["owner"].grant_permissions(
            ["clipboard-read", "clipboard-write"], origin=BASE)
    except Exception:
        pass
    page.goto(BASE + f"/watch/{vid}", wait_until="domcontentloaded")
    embed_btn = page.get_by_role("button", name="Embed video")
    embed_btn.wait_for(state="visible", timeout=30000)
    embed_btn.click()
    ta = page.get_by_role("textbox", name="Embed code")
    ta.wait_for(state="visible", timeout=5000)
    snippet = ta.input_value()
    ctx.check("embed snippet is an iframe for this video",
              "<iframe" in snippet and f"/embed/{vid}" in snippet,
              f"snippet={snippet!r}")
    ui.shot(page, "09 embed dialog open")
    page.get_by_role("button", name="Copy embed code").click()
    copied = False
    t0 = time.time()
    while time.time() - t0 < 5:
        if page.get_by_role("button", name="Copied!").count() > 0:
            copied = True
            break
        time.sleep(0.3)
    ctx.check("copy confirms with 'Copied!'", copied, "no confirmation shown")
    ui.shot(page, "09 embed dialog copied")

    # --- 4. Private video: the embed gates it (no player, no leak). ---
    s, ps = ctx.client.post("/api/uploads/presign", token=owner["token"], body={
        "file_name": "e2e-embed-private.mp4", "content_type": "video/mp4", "size": 1000})
    priv = None
    if s == 200 and isinstance(ps, dict):
        s, v = ctx.client.post("/api/videos", token=owner["token"], body={
            "title": f"E2E-UI Embed Private {ctx.run_id}",
            "description": "e2e-ui embed private",
            "tags": ["e2e-ui", "embed_private"],
            "object_key": ps["object_key"],
            "visibility": "private",
        })
        if s in (200, 201) and isinstance(v, dict) and v.get("video", {}).get("id"):
            priv = v["video"]["id"]
            ctx.videos["embed_private"] = priv  # ctx.cleanup() deletes it
    ctx.check("create private video (embed gate)", bool(priv), f"got {s}")
    if priv:
        anon.goto(BASE + f"/embed/{priv}", wait_until="domcontentloaded")
        gated = False
        t0 = time.time()
        while time.time() - t0 < 15:
            if anon.get_by_text("Video not available").count() > 0:
                gated = True
                break
            time.sleep(0.5)
        ui.shot(anon, "09 embed private gated")
        ctx.check("private video shows 'Video not available' in embed", gated,
                  f"body={anon.inner_text('body')[:200]!r}")
        ctx.check("private embed renders no player",
                  anon.locator("video").count() == 0,
                  "player rendered for a private video")
