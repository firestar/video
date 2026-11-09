"""History: record watch position, list, record view, clear."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # record a watch position
    s, body = c.post("/api/history", token=viewer["token"], body={"videoId": vid, "positionSecs": 42})
    ctx.check("record history -> 204", s == 204, f"got {s}: {body}")

    # list history includes the video with watchedUntilSecs
    s, h = c.get("/api/history", token=viewer["token"])
    item = next((x for x in h if x.get("videoId") == vid), None) if isinstance(h, list) else None
    ctx.check("history list includes video", s == 200 and item is not None, f"got {s}: {h}")
    if item is not None:
        ctx.check("history watchedUntilSecs recorded", item.get("watchedUntilSecs") == 42,
                  f"watchedUntilSecs={item.get('watchedUntilSecs')}")

    # record a view (increments view count)
    s, body = c.post(f"/api/videos/{vid}/view", token=viewer["token"])
    ctx.check("record view -> 204", s == 204, f"got {s}: {body}")

    # clear history
    s, body = c.delete("/api/history", token=viewer["token"])
    ctx.check("clear history -> 204", s == 204, f"got {s}: {body}")
    s, h = c.get("/api/history", token=viewer["token"])
    ids = [x.get("videoId") for x in h] if isinstance(h, list) else []
    ctx.check("history empty after clear", s == 200 and vid not in ids, f"got {s}, ids={ids}")
