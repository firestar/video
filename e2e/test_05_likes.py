"""Likes/dislikes: like, unlike, dislike, hasLiked/hasDisliked flags."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # like (viewer)
    s, body = c.post(f"/api/videos/{vid}/like", token=viewer["token"])
    ctx.check("like video -> 204", s == 204, f"got {s}: {body}")

    # get video shows hasLiked=true for viewer
    s, v = c.get(f"/api/videos/{vid}", token=viewer["token"])
    ctx.check("hasLiked=true after like", s == 200 and v.get("hasLiked") is True, f"got {s}: hasLiked={v.get('hasLiked')}")

    # unlike (viewer)
    s, body = c.delete(f"/api/videos/{vid}/like", token=viewer["token"])
    ctx.check("unlike video -> 204", s == 204, f"got {s}: {body}")
    s, v = c.get(f"/api/videos/{vid}", token=viewer["token"])
    ctx.check("hasLiked=false after unlike", s == 200 and v.get("hasLiked") is False, f"got {s}: hasLiked={v.get('hasLiked')}")

    # dislike (viewer)
    s, body = c.post(f"/api/videos/{vid}/dislike", token=viewer["token"])
    ctx.check("dislike video -> 204", s == 204, f"got {s}: {body}")
    s, v = c.get(f"/api/videos/{vid}", token=viewer["token"])
    ctx.check("hasDisliked=true after dislike", s == 200 and v.get("hasDisliked") is True, f"got {s}: hasDisliked={v.get('hasDisliked')}")

    # undislike (viewer)
    s, body = c.delete(f"/api/videos/{vid}/dislike", token=viewer["token"])
    ctx.check("undislike video -> 204", s == 204, f"got {s}: {body}")

    # like requires auth
    s, body = c.post(f"/api/videos/{vid}/like")
    ctx.check("like (no token) -> 401", s == 401, f"got {s}: {body}")
