"""Analytics: channel analytics shape + owner-only."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")

    # owner can view their own analytics
    s, a = c.get(f"/api/channels/{owner['channel_id']}/analytics", token=owner["token"])
    ctx.check("analytics (owner) -> 200", s == 200 and isinstance(a, dict), f"got {s}: {a}")
    if s == 200 and isinstance(a, dict):
        ctx.check("analytics has totalViews + totalSubscribers + viewsByDay",
                  "totalViews" in a and "totalSubscribers" in a and "viewsByDay" in a,
                  f"keys={list(a.keys())}")

    # non-owner cannot view analytics -> 403
    s, body = c.get(f"/api/channels/{owner['channel_id']}/analytics", token=viewer["token"])
    ctx.check("analytics (non-owner) -> 403", s == 403, f"got {s}: {body}")

    # no token -> 401
    s, body = c.get(f"/api/channels/{owner['channel_id']}/analytics")
    ctx.check("analytics (no token) -> 401", s == 401, f"got {s}: {body}")
