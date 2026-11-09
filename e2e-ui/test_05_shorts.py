"""Shorts: upload 3 vertical shorts through the real /upload page (Short
checkbox), wait for the worker to make them ready, then drive the /shorts
feed with GENUINE input:
  - phone 375x812: CDP Input.dispatchTouchEvent swipe (touchStart/Move/End)
  - desktop 1280x720: CDP Input.dispatchMouseEvent mouseWheel
Each must scroll exactly one card and flip the active/playing short.
"""
import time

from harness import BASE
from ui import shorts_state, wait_playing


def _upload_short(ctx, ui, page, n, color):
    """Upload one short through the real /upload page. Returns the video id."""
    title = f"E2E-UI Short {n} {ctx.run_id}"
    path = ctx.gen_video(f"short{n}", title, vertical=True, color=color,
                         filename=f"{title}.mp4")
    page.goto(BASE + "/upload", wait_until="domcontentloaded")
    page.set_input_files('input[type="file"]', path)
    time.sleep(0.5)
    # tick the Short checkbox (real click)
    page.get_by_label("Short").check()
    page.get_by_role("button", name="Upload", exact=True).click()
    page.wait_for_url(f"{BASE}/watch/**", timeout=120000)
    vid = page.url.split("/watch/")[1].split("?")[0]
    ctx.videos[f"short{n}"] = vid
    ctx.check(f"short {n} uploaded via UI", bool(vid), f"url={page.url}")
    return vid


def _assert_flip(ctx, label, before, after, playing_after):
    ch = before["clientH"]
    ctx.check(f"{label}: feed rendered 3+ cards",
              before["count"] >= 3, f"count={before['count']}")
    ctx.check(f"{label}: started at card 0",
              before["scrollTop"] == 0 and before["activeByScroll"] == 0,
              f"scrollTop={before['scrollTop']} active={before['activeByScroll']}")
    ctx.check(f"{label}: card 0 was playing",
              before["per"][0]["paused"] is False,
              f"per0={before['per'][0]}")
    ctx.check(f"{label}: scrolled exactly one card",
              after["scrollTop"] == ch and after["activeByScroll"] == 1,
              f"scrollTop={after['scrollTop']} clientH={ch} "
              f"active={after['activeByScroll']}")
    ctx.check(f"{label}: card 0 now paused",
              after["per"][0]["paused"] is True, f"per0={after['per'][0]}")
    ctx.check(f"{label}: card 1 now playing",
              playing_after is not None,
              f"per1={after['per'][1] if after['count'] > 1 else None}")


def _drive_and_check(ctx, ui, page, label):
    """Wait for card 0 to play, capture state, swipe/wheel one card, assert."""
    page.wait_for_selector('[data-testid="short-card"]', timeout=30000)
    page.wait_for_function(
        "() => document.querySelectorAll('[data-testid=\"short-card\"]')"
        ".length >= 3", timeout=30000)
    wait_playing(page, 0)
    before = shorts_state(page)
    ui.shot(page, f"05 {label} before")

    box = page.locator('[data-testid="shorts-feed"]').bounding_box()
    ch = before["clientH"]
    cx = box["x"] + box["width"] / 2
    cy = box["y"] + box["height"] / 2
    if label.startswith("PHONE"):
        ui.real_swipe(page, cx, cy, ch)
    else:
        ui.real_wheel(page, cx, cy, ch)
    # wait for the snap to settle
    t0 = time.time()
    while time.time() - t0 < 10:
        if shorts_state(page)["scrollTop"] == ch:
            break
        time.sleep(0.3)
    time.sleep(1.0)
    after = shorts_state(page)
    playing1 = wait_playing(page, 1, timeout=15)
    ui.shot(page, f"05 {label} after")
    _assert_flip(ctx, label, before, after, playing1)


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])

    # 1. Upload 3 vertical shorts through the real upload page.
    vids = []
    for n, color in ((1, "red"), (2, "green"), (3, "blue")):
        v = _upload_short(ctx, ui, page, n, color)
        if not v:
            return
        vids.append(v)

    # 2. Wait for the worker to make all three ready (the feed is ready-only).
    all_ready = True
    for n, v in enumerate(vids, 1):
        status = ctx.wait_ready(v)
        ok = status == "ready"
        all_ready = all_ready and ok
        ctx.check(f"short {n} reaches ready", ok, f"status={status}")
    if not all_ready:
        return

    # 3. PHONE: genuine touch swipe.
    ppage = ui.new_context("phone", mobile=True)
    ppage.goto(BASE + "/shorts", wait_until="domcontentloaded")
    _drive_and_check(ctx, ui, ppage, "PHONE swipe")

    # 4. DESKTOP: genuine mouse wheel.
    dpage = ui.new_context("desktop2")
    dpage.goto(BASE + "/shorts", wait_until="domcontentloaded")
    _drive_and_check(ctx, ui, dpage, "DESKTOP wheel")
