"""Watch later: add, list, remove, inWatchLater flag."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # add to watch later
    s, body = c.post("/api/watch-later", token=viewer["token"], body={"videoId": vid})
    ctx.check("add to watch later -> 204", s == 204, f"got {s}: {body}")

    # list watch later includes the video
    s, wl = c.get("/api/watch-later", token=viewer["token"])
    ids = [v.get("id") for v in wl.get("videos", [])] if isinstance(wl, dict) else []
    ctx.check("watch later list includes video", s == 200 and vid in ids, f"got {s}, ids={ids}")

    # get video shows inWatchLater=true
    s, v = c.get(f"/api/videos/{vid}", token=viewer["token"])
    ctx.check("inWatchLater=true after add", s == 200 and v.get("inWatchLater") is True, f"got {s}: {v.get('inWatchLater')}")

    # remove from watch later
    s, body = c.delete(f"/api/watch-later/{vid}", token=viewer["token"])
    ctx.check("remove from watch later -> 204", s == 204, f"got {s}: {body}")
    s, wl = c.get("/api/watch-later", token=viewer["token"])
    ids = [v.get("id") for v in wl.get("videos", [])] if isinstance(wl, dict) else []
    ctx.check("watch later list no longer includes video", s == 200 and vid not in ids, f"got {s}, ids={ids}")

    # add requires auth
    s, body = c.post("/api/watch-later", body={"videoId": vid})
    ctx.check("add to watch later (no token) -> 401", s == 401, f"got {s}: {body}")
