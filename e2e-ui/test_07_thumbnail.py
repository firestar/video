"""Custom video thumbnail: change a ready video's thumbnail through the real
/studio page (genuine file chooser), then verify the LIVE site serves the new
image. The API derives thumbnailUrl from storage (storage.exists + signed GET),
so the definitive check is that the served thumbnail BYTES match the custom
JPEG we uploaded — and that it differs from the worker's auto-generated frame.
"""
import hashlib
import os
import subprocess
import time
import urllib.request

from harness import BASE, FFMPEG, UA, VIDEOS_DIR


def _fetch_bytes(url):
    r = urllib.request.Request(url, method="GET")
    r.add_header("User-Agent", UA)
    with urllib.request.urlopen(r, timeout=60) as resp:
        return resp.status, resp.read()


def _sha(b):
    return hashlib.sha256(b).hexdigest()


def _make_thumbnail(path, color="red", label="CUSTOM"):
    """A distinctive solid-color JPEG with burned-in text, different from the
    auto thumbnail (which is a frame of the video)."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    cmd = [
        FFMPEG, "-y", "-loglevel", "error",
        "-f", "lavfi", "-i", f"color=c={color}:s=640x360:r=24:d=1",
        "-vf", (
            f"drawtext=text='{label}':fontcolor=white:fontsize=64:"
            f"x=(w-text_w)/2:y=(h-text_h)/2"
        ),
        "-frames:v", "1", path,
    ]
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    if p.returncode != 0:
        raise RuntimeError(f"ffmpeg thumbnail failed: {p.stderr[-300:]}")
    return path


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return

    # A ready video (created via the API) so the studio has a ready row to edit.
    vid = ctx.ensure_ready("thumb_test", title=f"E2E-UI Thumb {ctx.run_id}")
    if not vid:
        return

    # Baseline: the auto-generated thumbnail the worker produced.
    s, detail = ctx.client.get(f"/api/videos/{vid}")
    base_url = detail.get("thumbnailUrl") if isinstance(detail, dict) else None
    base_bytes = b""
    if base_url:
        try:
            _, base_bytes = _fetch_bytes(base_url)
        except Exception:
            base_bytes = b""
    base_sha = _sha(base_bytes)
    ctx.check("auto thumbnail exists before change",
              bool(base_url) and len(base_bytes) > 0,
              f"base_url={bool(base_url)} bytes={len(base_bytes)}")

    # A distinctive custom thumbnail (solid red + "CUSTOM").
    thumb_path = os.path.join(VIDEOS_DIR, f"{ctx.run_id}_custom_thumb.jpg")
    _make_thumbnail(thumb_path, color="red", label="CUSTOM")
    with open(thumb_path, "rb") as f:
        custom_sha = _sha(f.read())

    # Drive the real /studio page.
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])
    page.goto(BASE + "/studio", wait_until="domcontentloaded")
    row = page.locator(f'[data-video-id="{vid}"]')
    row.wait_for(state="visible", timeout=30000)
    thumb_btn = row.locator('button:has-text("Thumbnail")')
    thumb_btn.wait_for(state="visible", timeout=30000)
    ui.shot(page, "07 studio before")

    # Click "Thumbnail" (sets the target + opens the file chooser), then pick the file.
    with page.expect_file_chooser() as fc:
        thumb_btn.click()
    time.sleep(0.3)  # let React commit before the change event fires
    fc.value.set_files(thumb_path)

    # Poll the live API until the served thumbnail bytes match our custom file.
    matched = False
    t0 = time.time()
    while time.time() - t0 < 45:
        s, detail = ctx.client.get(f"/api/videos/{vid}")
        url = detail.get("thumbnailUrl") if isinstance(detail, dict) else None
        if url:
            try:
                st, b = _fetch_bytes(url)
                if st == 200 and _sha(b) == custom_sha:
                    matched = True
                    break
            except Exception:
                pass
        time.sleep(2)

    ctx.check("custom thumbnail is served (bytes match upload)", matched,
              f"served sha != custom {custom_sha[:16]}")
    ctx.check("thumbnail actually changed from the auto frame",
              matched and base_sha != custom_sha,
              f"base={base_sha[:16]} custom={custom_sha[:16]}")

    # The studio row should now show the new thumbnail (poll for the UI update).
    ui_ok = False
    t0 = time.time()
    while time.time() - t0 < 15:
        try:
            src = row.locator("img").first.get_attribute("src") or ""
        except Exception:
            src = ""
        if src and src != base_url:
            try:
                st, b = _fetch_bytes(src)
                if st == 200 and _sha(b) == custom_sha:
                    ui_ok = True
                    break
            except Exception:
                pass
        time.sleep(1)
    ui.shot(page, "07 studio after")
    ctx.check("studio row shows the updated thumbnail", ui_ok,
              "row img did not update to the custom image")

    try:
        os.remove(thumb_path)
    except OSError:
        pass
