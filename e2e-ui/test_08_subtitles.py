"""Subtitles through the real UI: upload a .vtt from /studio (genuine file
chooser), verify the track appears in the studio panel, then open the watch
page and verify the CC button + <track> element actually render the captions
(auto-enabled, switchable to Off).
"""
import os
import time

from harness import BASE, VIDEOS_DIR


VTT_EN = (
    "WEBVTT\n\n"
    "00:00:00.500 --> 00:00:02.500\n"
    "E2E UI captions\n"
)


def _list_subs(ctx, vid):
    s, body = ctx.client.get(f"/api/videos/{vid}/subtitles")
    if s == 200 and isinstance(body, dict) and isinstance(body.get("subtitles"), list):
        return body["subtitles"]
    return None


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return

    vid = ctx.ensure_ready("sub_ui", title=f"E2E-UI Subtitles {ctx.run_id}")
    if not vid:
        return

    vtt_path = os.path.join(VIDEOS_DIR, f"{ctx.run_id}_subs.vtt")
    os.makedirs(VIDEOS_DIR, exist_ok=True)
    with open(vtt_path, "w") as f:
        f.write(VTT_EN)

    # --- Studio: open the panel, upload via the real file chooser. ---
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])
    page.goto(BASE + "/studio", wait_until="domcontentloaded")
    row = page.locator(f'[data-video-id="{vid}"]')
    row.wait_for(state="visible", timeout=30000)
    sub_btn = row.locator('button:has-text("Subtitles")')
    sub_btn.wait_for(state="visible", timeout=30000)
    sub_btn.click()
    add_btn = row.locator('button:has-text("Add .vtt / .srt")')
    add_btn.wait_for(state="visible", timeout=10000)
    ui.shot(page, "08 studio panel open")
    # Genuinely upload the VTT: the panel's "Add .vtt / .srt" button just
    # clicks a hidden <input type=file>; set the file on it directly (the
    # language select defaults to "en"). The browser then presigns, PUTs and
    # registers the track.
    sub_input = page.locator('input[type="file"][accept*="vtt"]')
    sub_input.wait_for(state="attached", timeout=10000)
    sub_input.set_input_files(vtt_path)
    # Give the browser a beat to start the presign/PUT/register before polling.
    time.sleep(3)
    # Poll the API until the track is registered.
    tracks = None
    t0 = time.time()
    while time.time() - t0 < 45:
        tracks = _list_subs(ctx, vid)
        if tracks and any(t.get("language") == "en" for t in tracks):
            break
        time.sleep(2)
    ctx.check("subtitle track registered (en)",
              bool(tracks) and any(t.get("language") == "en" for t in (tracks or [])),
              f"tracks={tracks}")

    # The studio panel should now list the track with a Remove button.
    ui_ok = False
    t0 = time.time()
    while time.time() - t0 < 15:
        try:
            ui_ok = row.locator('span:text-is("en")').count() > 0 and \
                row.locator('button:has-text("Remove")').count() > 0
        except Exception:
            ui_ok = False
        if ui_ok:
            break
        time.sleep(1)
    ui.shot(page, "08 studio track listed")
    ctx.check("studio panel lists the en track with Remove", ui_ok,
              "track row not visible in the panel")

    # --- Watch page: the CC button appears and auto-enables the track. ---
    page.goto(BASE + f"/watch/{vid}", wait_until="domcontentloaded")
    video = page.locator("video").first
    video.wait_for(state="visible", timeout=30000)
    cc_btn = page.locator('button[title="Subtitles (c)"]')
    cc_btn.wait_for(state="visible", timeout=30000)
    ui.shot(page, "08 watch CC on")

    # Auto-enable: the first track should be 'showing' (poll until tracks load).
    showing = False
    t0 = time.time()
    while time.time() - t0 < 20:
        state = video.evaluate(
            "v => Array.from(v.textTracks).map(t => ({lang: t.language, mode: t.mode}))"
        )
        if any(t["lang"] == "en" and t["mode"] == "showing" for t in state):
            showing = True
            break
        time.sleep(1)
    ctx.check("CC auto-enabled: en track is showing", showing, f"tracks={state}")
    # The button label reflects the active language.
    ctx.check("CC button shows the active language",
              cc_btn.inner_text().strip() == "en",
              f"label={cc_btn.inner_text()!r}")

    # Open the menu and switch to Off.
    cc_btn.click()
    off_btn = page.get_by_role("button", name="Off", exact=True)
    off_btn.wait_for(state="visible", timeout=5000)
    off_btn.click()
    off_btn.click()
    time.sleep(1)
    state = video.evaluate(
        "v => Array.from(v.textTracks).map(t => ({lang: t.language, mode: t.mode}))"
    )
    ctx.check("switching to Off hides the track",
              all(t["mode"] != "showing" for t in state), f"tracks={state}")
    ctx.check("CC button back to 'CC' label",
              cc_btn.inner_text().strip() == "CC",
              f"label={cc_btn.inner_text()!r}")
    ui.shot(page, "08 watch CC off")

    try:
        os.remove(vtt_path)
    except OSError:
        pass
