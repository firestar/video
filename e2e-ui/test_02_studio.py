"""Studio: edit a video through the real /studio form — real typing, real
dropdown, real click — then verify via the API. Reverts visibility to public
so later tests can use the video."""
import time

from harness import BASE


def _edit_row(page, vid):
    """Locate the Studio row for a video by its stable data-video-id."""
    row = page.locator(f'div[data-video-id="{vid}"]')
    row.wait_for(timeout=30000)
    return row


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    if not owner:
        return
    page = ui.pages.get("owner")
    if not page:
        page = ui.ui_login("owner", owner["email"], owner["password"])

    # Use the video test_01 uploaded, or create one via the API.
    if "upload" in ctx.videos:
        vid = ctx.videos["upload"]
        old_title = f"E2E-UI Upload {ctx.run_id}"
    else:
        vid = ctx.ensure_ready("studio", title=f"E2E-UI Studio {ctx.run_id}")
        old_title = f"E2E-UI Studio {ctx.run_id}"
        if not vid:
            return

    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    ctx.check("video exists (api)", s == 200 and isinstance(v, dict), f"got {s}")
    if s != 200:
        return

    new_title = f"E2E-UI Studio {ctx.run_id}"
    new_desc = "Edited through the real Studio form by the UI e2e suite."
    new_tags = "e2e-ui, studio, edited"

    # 1. Open /studio and find the video's row.
    page.goto(BASE + "/studio", wait_until="domcontentloaded")
    page.wait_for_selector("h1:has-text('Studio')", timeout=30000)
    row = _edit_row(page, vid)
    ctx.check("video row visible in Studio", row.count() > 0)

    # 2. Real click on Edit.
    row.locator("button", has_text="Edit").click()
    ti = row.locator("input[type='text']").first
    ti.wait_for(timeout=10000)

    # 3. Real typing: replace the title (click, select-all, type over it).
    ti.click()
    page.keyboard.press("Control+a")
    page.keyboard.type(new_title, delay=10)
    ta = row.locator("textarea")
    ta.click()
    page.keyboard.press("Control+a")
    page.keyboard.type(new_desc, delay=10)
    tg = row.locator("input[placeholder='e2e, sample, demo']")
    tg.click()
    page.keyboard.press("Control+a")
    page.keyboard.type(new_tags, delay=10)

    # 4. Real dropdown: visibility -> private.
    row.locator("select").select_option("private")

    # 5. Real click on Save changes.
    row.get_by_role("button", name="Save changes").click()
    page.wait_for_selector("text=Video updated", timeout=30000)
    ctx.check("Studio shows 'Video updated'", True)
    ui.shot(page, "02 studio updated")

    # 6. Verify via the API.
    time.sleep(0.5)
    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    video = v.get("video") if isinstance(v, dict) else None
    ok = (s == 200 and isinstance(video, dict)
          and video.get("title") == new_title
          and video.get("description") == new_desc
          and video.get("visibility") == "private")
    ctx.check("api: title/description/visibility updated", ok,
              f"got title={video.get('title') if isinstance(video, dict) else v!r} "
              f"desc={video.get('description') if isinstance(video, dict) else ''!r} "
              f"vis={video.get('visibility') if isinstance(video, dict) else ''!r}")
    tags = video.get("tags") if isinstance(video, dict) else None

    # 7. Edit again: flip visibility back to public (tests a second edit).
    row2 = _edit_row(page, vid)
    row2.locator("button", has_text="Edit").click()
    row2.locator("input[type='text']").first.wait_for(timeout=10000)
    row2.locator("select").select_option("public")
    row2.get_by_role("button", name="Save changes").click()
    page.wait_for_selector("text=Video updated", timeout=30000)
    time.sleep(0.5)
    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    video = v.get("video") if isinstance(v, dict) else None
    ctx.check("api: visibility back to public",
              s == 200 and isinstance(video, dict) and video.get("visibility") == "public",
              f"got {s} vis={video.get('visibility') if isinstance(video, dict) else v!r}")
