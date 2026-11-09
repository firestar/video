"""Notifications through the real UI: viewer likes + comments (real click /
real typing) on the owner's video; owner opens the bell (real click), sees
both notifications, marks one read by clicking it (navigates to the video),
then marks all read. Verified via the API at the end."""
import re
import time

from harness import BASE

BELL = 'button[aria-label^="Notifications"]'
PANEL = 'div.absolute:has-text("Notifications")'


def _badge(page):
    """The unread-count badge span inside the bell button, or None."""
    b = page.locator(BELL + " span")
    if b.count() == 0:
        return None
    return b.first.inner_text().strip()


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    if not owner or not viewer:
        return

    if "upload" not in ctx.videos:
        vid = ctx.ensure_ready("studio", title=f"E2E-UI Studio {ctx.run_id}")
        if not vid:
            return
    else:
        vid = ctx.videos["upload"]

    # 1. Viewer: log in through the UI (separate context).
    vpage = ui.ui_login("viewer", viewer["email"], viewer["password"])

    # 2. Viewer likes the owner's video (real click).
    vpage.goto(f"{BASE}/watch/{vid}", wait_until="domcontentloaded")
    vpage.wait_for_selector("h1", timeout=30000)
    like_btn = vpage.locator("button:has(svg.lucide-thumbs-up)")
    like_btn.wait_for(timeout=10000)
    like_btn.click()
    vpage.wait_for_function(
        "() => { const b = document.querySelector('button:has(svg.lucide-thumbs-up)'); "
        "return b && b.getAttribute('aria-pressed') === 'true'; }",
        timeout=10000)
    ctx.check("like button reflects liked state (aria-pressed)", True)

    # 3. Viewer posts a comment (real typing + real click).
    body = f"e2e-ui comment {ctx.run_id}"
    vpage.get_by_placeholder("Add a comment\u2026").type(body, delay=10)
    vpage.get_by_role("button", name="Comment", exact=True).click()
    vpage.wait_for_selector(f"text={body}", timeout=15000)
    ctx.check("comment appears in the list", True)
    ui.shot(vpage, "04 viewer liked+commented")

    # 4. Owner: open the bell (real click). Data loads on mount + poll.
    opage = ui.pages.get("owner")
    if not opage:
        opage = ui.ui_login("owner", owner["email"], owner["password"])
    opage.goto(BASE + "/", wait_until="domcontentloaded")
    bell = opage.locator(BELL)
    bell.wait_for(timeout=10000)
    bell.click()
    # wait until the panel shows both notifications (data is async)
    opage.wait_for_selector(f'{PANEL}:has-text("commented on your video")',
                            timeout=20000)
    opage.wait_for_selector(f'{PANEL}:has-text("liked your video")',
                            timeout=5000)
    panel_text = opage.locator(PANEL).inner_text()
    ctx.check("panel shows 'liked your video'",
              "liked your video" in panel_text, f"panel={panel_text[:200]!r}")
    ctx.check("panel shows 'commented on your video'",
              "commented on your video" in panel_text,
              f"panel={panel_text[:200]!r}")
    ctx.check("panel names the viewer",
              viewer["name"] in panel_text, f"panel={panel_text[:200]!r}")
    ctx.check("bell badge shows 2 unread", _badge(opage) == "2",
              f"badge={_badge(opage)!r}")
    ui.shot(opage, "04 owner bell open")

    # 5. Click the comment notification -> marks read + navigates to the video.
    opage.locator("button", has_text="commented on your video").first.click()
    opage.wait_for_url(f"{BASE}/watch/{vid}", timeout=30000)
    ctx.check("clicking notification navigates to the video",
              opage.url == f"{BASE}/watch/{vid}", f"url={opage.url}")

    # 6. Open the bell again: badge should now show 1 (only the like unread).
    bell = opage.locator(BELL)
    bell.wait_for(timeout=10000)
    bell.click()
    opage.wait_for_selector(f'{PANEL}:has-text("liked your video")',
                            timeout=20000)
    ctx.check("badge now shows 1 unread", _badge(opage) == "1",
              f"badge={_badge(opage)!r}")

    # 7. Mark all as read (real click) -> badge disappears.
    opage.get_by_role("button", name="Mark all as read").click()
    time.sleep(1.0)
    badge = opage.locator(BELL + " span",
                          has_text=re.compile(r"^\d+$"))
    ctx.check("badge gone after mark-all-read", badge.count() == 0,
              f"badge count={badge.count()}")
    ui.shot(opage, "04 all read")

    # 8. API: unreadCount == 0.
    s, n = ctx.client.get("/api/notifications", token=owner["token"])
    ctx.check("api: unreadCount == 0",
              s == 200 and n.get("unreadCount") == 0, f"got {s}: {n}")
