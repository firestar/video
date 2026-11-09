"""Playlists through the real UI: create (real typing), add a video from the
watch page's Save menu (real clicks), verify via API, remove the video from
the playlist page, then delete the playlist (accepting the confirm dialog)."""
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

    if "upload" not in ctx.videos:
        vid = ctx.ensure_ready("studio", title=f"E2E-UI Studio {ctx.run_id}")
        if not vid:
            return
    else:
        vid = ctx.videos["upload"]

    pl_title = f"E2E-UI Playlist {ctx.run_id}"

    # 1. /playlists -> New playlist -> real typing -> Create.
    page.goto(BASE + "/playlists", wait_until="domcontentloaded")
    page.wait_for_selector("h1:has-text('Playlists')", timeout=30000)
    page.get_by_role("button", name="New playlist").click()
    form = page.locator("form")
    form.wait_for(timeout=10000)
    form.get_by_placeholder("Playlist title").type(pl_title, delay=10)
    form.get_by_placeholder("Description (optional)").type("created by ui e2e", delay=10)
    # leave the public checkbox unchecked (private playlist)
    form.get_by_role("button", name="Create").click()

    # 2. The card should appear (a link to /playlists/{id}).
    card = page.locator("a[href^='/playlists/']", has_text=pl_title)
    card.wait_for(timeout=10000)
    ctx.check("playlist card appears on /playlists", card.count() > 0)
    ui.shot(page, "03 playlist created")

    # 3. Find the id via the API.
    s, p = ctx.client.get("/api/playlists", token=owner["token"])
    pid = None
    if s == 200 and isinstance(p, dict):
        for pl in p.get("playlists", []):
            if pl.get("title") == pl_title:
                pid = pl.get("id")
    ctx.check("playlist exists (api)", bool(pid), f"got {s}")
    if not pid:
        return
    ctx.playlists["main"] = pid

    # 4. Watch page: Save menu -> add to the playlist (real clicks).
    page.goto(f"{BASE}/watch/{vid}", wait_until="domcontentloaded")
    page.wait_for_selector("h1", timeout=30000)
    save_btn = page.get_by_role("button", name="Save", exact=True)
    save_btn.click()
    menu = page.locator('div.absolute:has-text("Save to playlist")')
    menu.wait_for(timeout=10000)
    ctx.check("save menu opens with the playlist listed",
              menu.get_by_text(pl_title, exact=True).count() > 0)
    menu.get_by_text(pl_title, exact=True).click()
    # the menu stays open; the item flips to a checkmark once added
    menu.locator("svg.lucide-check").wait_for(timeout=10000)
    ctx.check("checkmark shown for added playlist", True)
    ui.shot(page, "03 video added to playlist")

    # 5. API: videoCount == 1.
    s, pl = ctx.client.get(f"/api/playlists/{pid}", token=owner["token"])
    ctx.check("api: videoCount == 1 after add",
              s == 200 and pl.get("videoCount") == 1, f"got {s}: {pl}")

    # 6. Playlist page: the video is there; remove it via the X button.
    s, v = ctx.client.get(f"/api/videos/{vid}", token=owner["token"])
    vtitle = (v.get("video") or {}).get("title", "") if isinstance(v, dict) else ""
    page.goto(f"{BASE}/playlists/{pid}", wait_until="domcontentloaded")
    page.get_by_text(vtitle, exact=True).wait_for(timeout=10000)
    ctx.check("playlist page shows the video", True)
    page.locator("div.group.relative").first.hover()
    page.locator('button[aria-label="Remove from playlist"]').click()
    page.wait_for_selector("text=No videos in this playlist", timeout=10000)
    s, pl = ctx.client.get(f"/api/playlists/{pid}", token=owner["token"])
    ctx.check("api: videoCount == 0 after remove",
              s == 200 and pl.get("videoCount") == 0, f"got {s}: {pl}")
    ui.shot(page, "03 video removed")

    # 7. Delete the playlist (accept the confirm dialog).
    with page.expect_dialog() as dialog_info:
        page.get_by_role("button", name="Delete").click()
    dialog_info.value.accept()
    page.wait_for_url(f"{BASE}/playlists", timeout=15000)
    s, pl = ctx.client.get(f"/api/playlists/{pid}", token=owner["token"])
    ctx.check("api: playlist deleted (404)", s == 404, f"got {s}: {pl}")
    ui.shot(page, "03 playlist deleted")
