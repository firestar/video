"""Notifications: new_comment + new_like, mark read, read-all, unreadCount."""
from harness import Ctx


def _types(ctx, token):
    s, n = ctx.client.get("/api/notifications", token=token)
    if s != 200 or not isinstance(n, dict):
        return s, [], n
    return s, [x.get("type") for x in n.get("notifications", [])], n


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # viewer comments on owner's video -> owner gets new_comment
    s, r = c.post(f"/api/videos/{vid}/comments", token=viewer["token"], body={"body": "notify me"})
    ctx.check("comment (triggers new_comment)", s == 201, f"got {s}: {r}")

    s, types, n = _types(ctx, owner["token"])
    ctx.check("owner has new_comment notification", s == 200 and "new_comment" in types,
              f"got {s}, types={types}")

    # viewer likes owner's video -> owner gets new_like
    s, body = c.post(f"/api/videos/{vid}/like", token=viewer["token"])
    ctx.check("like (triggers new_like)", s == 204, f"got {s}: {body}")
    s, types, n = _types(ctx, owner["token"])
    ctx.check("owner has new_like notification", s == 200 and "new_like" in types,
              f"got {s}, types={types}")

    # unreadCount > 0
    s, n = c.get("/api/notifications", token=owner["token"])
    ctx.check("unreadCount > 0", s == 200 and n.get("unreadCount", 0) > 0, f"got {s}: {n}")

    # mark one read
    first = (n.get("notifications") or [{}])[0]
    nid = first.get("id")
    if nid:
        s, r = c.post(f"/api/notifications/{nid}/read", token=owner["token"])
        ctx.check("mark notification read -> 200", s == 200 and "unreadCount" in r, f"got {s}: {r}")

    # mark all read -> unreadCount 0
    s, r = c.post("/api/notifications/read-all", token=owner["token"])
    ctx.check("read-all -> unreadCount 0", s == 200 and r.get("unreadCount") == 0, f"got {s}: {r}")

    # notifications require auth
    s, body = c.get("/api/notifications")
    ctx.check("notifications (no token) -> 401", s == 401, f"got {s}: {body}")
