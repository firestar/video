"""Home feed: personalized feed shape, reasons map."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    viewer = ctx.user("viewer")

    # feed requires auth
    s, body = c.get("/api/home/feed")
    # feed works without auth (returns trending only)
    s, body = c.get("/api/home/feed")
    ctx.check("home feed (no token) -> 200 (trending only)", s == 200 and isinstance(body, dict) and "videos" in body, f"got {s}: {body}")

    # authenticated feed
    s, f = c.get("/api/home/feed", token=viewer["token"])
    ctx.check("home feed -> 200 + {videos, reasons}",
              s == 200 and isinstance(f, dict) and "videos" in f and "reasons" in f,
              f"got {s}: {f}")
    if s == 200 and isinstance(f, dict):
        vids = f.get("videos", [])
        reasons = f.get("reasons", {})
        ctx.check("feed videos is a list", isinstance(vids, list), f"type={type(vids).__name__}")
        ctx.check("feed reasons is a map", isinstance(reasons, dict), f"type={type(reasons).__name__}")
        if vids:
            # every feed video id should have a reason
            missing = [v.get("id") for v in vids if v.get("id") not in reasons]
            ctx.check("every feed video has a reason", not missing, f"missing={missing[:5]}")
        else:
            ctx.skip("feed non-empty", "no ready content to feed yet")
