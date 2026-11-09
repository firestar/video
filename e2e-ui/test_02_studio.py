"""Studio: edit a video through the real /studio form — real typing, real
dropdown, real click — then verify via the API. Reverts visibility to public
so later tests can use the video."""
import time

from harness import BASE


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
    title_span = page.locator("span", has_text=old_title).last
    title_span.wait_for(timeout=30000)
    row = title_span.locator("xpath=ancestor::div[contains(@class, 'border')][1]")
    ctx.check("video row visible in Studio", row.count() > 0)

    # 2. Real click on Edit.
    row.locator("button", has_text="Edit").click()
    form = row.locator("form")
    form.wait_for(timeout=10000)

    # 3. Real typing: replace the title (click, select-all, type over it).
    ti = form.locator("input").first
    ti.click()
    page.keyboard.press("Control+a")
    page.keyboard.type(new_title, delay=10)
    form.get_by_placeholder("Description").type(new_desc, delay=10)
    form.get_by_placeholder("Tags (comma-separated)").type(new_tags, delay=10)

    # 4. Real dropdown: visibility -> private.
    form.locator("select").select_option("private")

    # 5. Real click on Save changes.
    form.get_by_role("button", name="Save changes").click()
    page.wait_for_selector("text=Video updated", timeout=30000)
    ctx.check("Studio shows 'Video updated'", True)
    ui.shot(page, "02 studio updated")

    # 6. Verify via the API.
    time.sleep(0.5)
    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    ok = (s == 200 and v.get("title") == new_title
          and v.get("description") == new_desc
          and v.get("visibility") == "private")
    ctx.check("api: title/description/visibility updated", ok,
              f"got title={v.get('title')!r} desc={v.get('description')!r} "
              f"vis={v.get('visibility')!r}")
    tags = v.get("tags") if isinstance(v, dict) else None
    ctx.check("api: tags updated", tags == new_tags.split(", "),
              f"got {tags!r} want {new_tags.split(', ')!r}")

    # 7. Edit again: flip visibility back to public (tests a second edit).
    title_span2 = page.locator("span", has_text=new_title).last
    title_span2.wait_for(timeout=10000)
    row2 = title_span2.locator("xpath=ancestor::div[contains(@class, 'border')][1]")
    row2.locator("button", has_text="Edit").click()
    form2 = row2.locator("form")
    form2.wait_for(timeout=10000)
    form2.locator("select").select_option("public")
    form2.get_by_role("button", name="Save changes").click()
    page.wait_for_selector("text=Video updated", timeout=30000)
    time.sleep(0.5)
    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    ctx.check("api: visibility back to public",
              s == 200 and v.get("visibility") == "public",
              f"got {s} vis={v.get('visibility') if isinstance(v, dict) else v!r}")
