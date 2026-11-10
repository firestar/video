"""Age restriction UI: the watch page shows an 18+ gate to a non-adult viewer of
an age-restricted video; confirming the age reveals the player. Also checks the
18+ badge on a video card (via search). Uses the fixed viewer, resetting its age
state (DOB + confirmation) before and after so no new accounts are created.
"""
from harness import BASE


def run(ctx):
    ui = ctx.ui
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    if not owner or not viewer:
        return

    # Deterministic start: clear the viewer's age state so the gate shows.
    ctx.client.patch("/api/me", token=viewer["token"],
                     body={"dateOfBirth": None, "ageConfirmed": False})

    # Upload a real age-restricted video and wait for it to be ready.
    title = f"E2E-UI AgeGate {ctx.run_id}"
    vid = ctx.upload_video("age", title=title, age_restricted=True)
    if not vid:
        return
    status = ctx.wait_ready(vid)
    ctx.check("age video reaches ready", status == "ready", f"status={status}")
    if status != "ready":
        return

    # --- Watch page: the 18+ gate for a non-adult viewer ---
    page = ui.pages.get("viewer")
    if not page:
        page = ui.ui_login("viewer", viewer["email"], viewer["password"])
    page.goto(BASE + f"/watch/{vid}", wait_until="domcontentloaded")
    gate = page.locator("text=This video is for viewers 18 and over")
    gate.wait_for(timeout=30000)
    ctx.check("watch page shows 18+ gate to non-adult", gate.count() > 0)
    ui.shot(page, "10 age gate")

    # Confirm the age through the gate button -> reload -> player appears.
    page.get_by_role("button", name="I am 18 or older").click()
    page.locator("video").wait_for(timeout=45000)
    ctx.check("player visible after age confirm", page.locator("video").count() > 0)
    ui.shot(page, "10 age confirmed player")

    # --- 18+ badge on a video card (owner searches for the video) ---
    opage = ui.pages.get("owner")
    if not opage:
        opage = ui.ui_login("owner", owner["email"], owner["password"])
    opage.goto(BASE + "/search?q=AgeGate", wait_until="domcontentloaded")
    badge = opage.locator("a[href^='/watch/'] >> text=18+")
    badge.first.wait_for(timeout=30000)
    ctx.check("18+ badge shown on video card", badge.count() > 0)
    ui.shot(opage, "10 age badge on card")

    # Leave the fixed viewer clean for the next run.
    ctx.client.patch("/api/me", token=viewer["token"],
                     body={"dateOfBirth": None, "ageConfirmed": False})
