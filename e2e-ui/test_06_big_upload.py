"""Big upload: push a >100 MB video through the real /upload page.

This is the regression test for the S3 multipart path. Files larger than
50 MB are uploaded in 50 MB parts (each its own PUT) so no single request
exceeds Cloudflare's 100 MB body cap — a single presigned PUT of this size
would 413 at the edge. The test proves the multipart path is actually taken
(by watching the network) and that the result genuinely transcodes to ready
and plays:

  multipart/init -> N x 50 MB part PUTs -> multipart/complete ->
  worker transcode to ready -> video plays (currentTime advancing).
"""
import os
import subprocess
import time

from harness import BASE, FFMPEG, VIDEOS_DIR

# Cloudflare's single-request body cap. The generated file must exceed this
# (in bytes) for the test to be meaningful — under it, a single PUT would
# have worked and the multipart path wouldn't be proven.
CLOUDFLARE_CAP = 100 * 1024 * 1024  # 100 MiB


def make_big_video(path, seconds=95, bitrate="10M"):
    """Generate a real H.264 1080p mp4 of roughly (bitrate * seconds / 8)
    bytes using animated testsrc2 content. 10 Mbit/s x 95 s ~= 119 MB,
    comfortably over Cloudflare's 100 MB cap. No drawtext: the file only
    needs to be large and transcodable, so we keep the encode simple."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    cmd = [
        FFMPEG, "-y", "-loglevel", "error",
        "-f", "lavfi", "-i", "testsrc2=size=1920x1080:rate=30",
        "-t", str(seconds),
        "-c:v", "libx264", "-preset", "veryfast", "-b:v", bitrate,
        "-pix_fmt", "yuv420p",
        path,
    ]
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    if p.returncode != 0:
        raise RuntimeError(f"ffmpeg big video failed: {p.stderr[-500:]}")
    return path


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return

    # 1. Generate a >100 MB video and register it so cleanup removes the
    #    local file (it is ~120 MB and regenerated every run).
    title = f"E2E-UI BigUpload {ctx.run_id}"
    path = os.path.join(VIDEOS_DIR, f"{title}.mp4")
    make_big_video(path)
    ctx._video_paths["big_upload"] = path
    size = os.path.getsize(path)
    ctx.check("big video exceeds Cloudflare's 100 MB cap",
              size > CLOUDFLARE_CAP,
              f"size={size} bytes ({size / 1024 / 1024:.1f} MiB)")
    if size <= CLOUDFLARE_CAP:
        # Not large enough to prove the multipart path; stop here.
        return

    # 2. Reuse the logged-in owner page (test_01 logged in), else log in.
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])

    # 3. Track every request so we can prove the multipart path was taken.
    api_calls = []

    def _on_request(req):
        api_calls.append((req.method, req.url))

    page.on("request", _on_request)

    # 4. Go to /upload and drop the big file in through the real file input.
    page.goto(BASE + "/upload", wait_until="domcontentloaded")
    page.set_input_files('input[type="file"]', path)
    time.sleep(0.5)  # let pickFile's state settle

    # 5. Title auto-filled from the filename?
    auto = page.get_by_placeholder("Title (required)").input_value()
    ctx.check("big upload: title auto-filled from filename", auto == title,
              f"auto={auto!r} want={title!r}")

    # 6. Real click on Upload. The browser runs the multipart path:
    #    init -> N x 50 MB part PUTs -> complete -> create -> redirect.
    page.get_by_role("main").get_by_role("button", name="Upload", exact=True).click()

    # 7. Redirect to /watch/{id}. A ~120 MB multipart upload takes a while.
    page.wait_for_url(f"{BASE}/watch/**", timeout=300000)
    vid = page.url.split("/watch/")[1].split("?")[0]
    ctx.videos["big_upload"] = vid
    ctx.check("big upload redirected to /watch/{id}", bool(vid), f"url={page.url}")

    # 8. Prove the multipart path was actually used (not a single PUT).
    init_calls = [u for m, u in api_calls if "/api/uploads/multipart/init" in u]
    part_puts = [u for m, u in api_calls if m == "PUT" and "/media/" in u]
    ctx.check("used multipart/init", len(init_calls) >= 1,
              f"init_calls={len(init_calls)}")
    # ~120 MB / 50 MB parts = 3 parts.
    ctx.check("uploaded in multiple 50 MB parts", len(part_puts) >= 3,
              f"part_puts={len(part_puts)} urls={part_puts[:4]}")

    # 9. The worker must genuinely transcode it to ready.
    status = ctx.wait_ready(vid, timeout=300)
    ctx.check("worker transcodes big video to ready", status == "ready",
              f"status={status}")
    if status != "ready":
        return

    # 10. The video must actually play: wait for playback, then prove
    #     currentTime advances.
    try:
        page.wait_for_function(
            "() => { const v = document.querySelector('video');"
            " return v && !v.paused && v.currentTime > 0; }",
            timeout=30000)
    except Exception as e:
        ctx.check("big video starts playing", False, f"timeout: {e}")
        return
    t1 = page.evaluate("() => document.querySelector('video').currentTime")
    time.sleep(1.5)
    t2 = page.evaluate("() => document.querySelector('video').currentTime")
    ctx.check("big video is playing (currentTime advancing)", t2 > t1,
              f"t1={t1} t2={t2}")
    ui.shot(page, "06 big upload playing")
