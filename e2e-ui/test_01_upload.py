"""Upload through the real /upload page: genuine file input, real typing,
real click, redirect to /watch, worker transcode to ready, video plays."""
import time

from harness import BASE


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return

    # 1. Log in through the UI with real typing (also tests the login page).
    page = ui.ui_login("owner", owner["email"], owner["password"])
    ctx.check("ui login lands on home", page.url.rstrip("/") == BASE,
              f"url={page.url}")
    ui.shot(page, "01 after login")

    # 2. Generate a real video whose FILENAME is the title we want — the
    #    upload page auto-fills the title from the filename.
    title = f"E2E-UI Upload {ctx.run_id}"
    path = ctx.gen_video("upload", title, filename=f"{title}.mp4")

    # 3. Go to /upload and drop the file in through the real file input.
    page.goto(BASE + "/upload", wait_until="domcontentloaded")
    page.set_input_files('input[type="file"]', path)
    time.sleep(0.5)  # let pickFile's state settle

    # 4. Title auto-filled from the filename?
    title_input = page.get_by_placeholder("Title (required)")
    auto = title_input.input_value()
    ctx.check("title auto-filled from filename", auto == title,
              f"auto={auto!r} want={title!r}")

    # 5. Real typing into description + tags.
    page.get_by_placeholder("Description").type(
        "Uploaded through the real upload page by the UI e2e suite.", delay=10)
    page.get_by_placeholder("Tags (comma-separated)").type("e2e-ui, upload", delay=10)

    # 6. Real click on Upload.
    page.get_by_role("button", name="Upload", exact=True).click()

    # 7. Redirect to /watch/{id} (upload can take a moment: presign + PUT + create).
    page.wait_for_url(f"{BASE}/watch/**", timeout=120000)
    vid = page.url.split("/watch/")[1].split("?")[0]
    ctx.videos["upload"] = vid
    ctx.check("redirected to /watch/{id}", bool(vid), f"url={page.url}")

    # 8. The worker must genuinely transcode it to ready.
    status = ctx.wait_ready(vid)
    ctx.check("worker transcodes video to ready", status == "ready",
              f"status={status}")
    if status != "ready":
        return

    # 9. Reload — the page still shows the "not ready" placeholder from load.
    page.reload(wait_until="domcontentloaded")
    ctx.check("watch page shows the title",
              page.locator("h1", has_text=title).count() > 0,
              "h1 with title not found")

    # 10. The video must actually play: wait for playback, then prove
    #     currentTime advances.
    try:
        page.wait_for_function(
            "() => { const v = document.querySelector('video');"
            " return v && !v.paused && v.currentTime > 0; }",
            timeout=30000)
    except Exception as e:
        ctx.check("video starts playing", False, f"timeout: {e}")
        return
    t1 = page.evaluate("() => document.querySelector('video').currentTime")
    time.sleep(1.5)
    t2 = page.evaluate("() => document.querySelector('video').currentTime")
    ctx.check("video is playing (currentTime advancing)", t2 > t1,
              f"t1={t1} t2={t2}")
    ui.shot(page, "01 watch page playing")
