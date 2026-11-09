"""Comments: create, threaded list, reply, edit, delete, like (toggle), pin/unpin."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("commented", title="E2E Commented Video")
    if not vid:
        return

    # create a top-level comment (author = viewer)
    s, r = c.post(f"/api/videos/{vid}/comments", token=viewer["token"], body={"body": "top comment"})
    ctx.check("create comment -> 201", s == 201 and r.get("body") == "top comment", f"got {s}: {r}")
    top_id = r.get("id") if isinstance(r, dict) else None

    # empty comment -> 400
    s, body = c.post(f"/api/videos/{vid}/comments", token=viewer["token"], body={"body": "   "})
    ctx.check("empty comment -> 400", s == 400, f"got {s}: {body}")

    # create a reply to the top comment
    if top_id:
        s, r = c.post(f"/api/videos/{vid}/comments", token=owner["token"],
                      body={"body": "a reply", "parentId": top_id})
        ctx.check("create reply -> 201", s == 201 and r.get("parentId") == top_id, f"got {s}: {r}")

    # threaded list (public) includes the top comment + its reply
    s, data = c.get(f"/api/comments/{vid}")
    comments = data.get("comments") if isinstance(data, dict) else None
    top_found = any(cm.get("id") == top_id for cm in (comments or []))
    ctx.check("threaded list has top comment", s == 200 and top_found,
              f"got {s}, count={data.get('count') if isinstance(data, dict) else '?'}")
    reply_found = any(
        any(rp.get("parentId") == top_id for rp in (cm.get("replies") or []))
        for cm in (comments or [])
    )
    ctx.check("threaded list nests the reply", s == 200 and reply_found, f"got {s}")

    if not top_id:
        return

    # edit comment (author) -> {comment}
    s, r = c.patch(f"/api/comments/{top_id}", token=viewer["token"], body={"body": "top comment (edited)"})
    ctx.check("edit comment (author) -> 200", s == 200 and (r.get("comment") or {}).get("body") == "top comment (edited)",
              f"got {s}: {r}")

    # edit by non-author -> 403
    s, body = c.patch(f"/api/comments/{top_id}", token=owner["token"], body={"body": "hack"})
    ctx.check("edit comment (non-author) -> 403", s == 403, f"got {s}: {body}")

    # like (toggle) -> liked true, then false
    s, r = c.post(f"/api/comments/{top_id}/like", token=owner["token"])
    ctx.check("like comment -> liked=true", s == 200 and r.get("liked") is True, f"got {s}: {r}")
    s, r = c.post(f"/api/comments/{top_id}/like", token=owner["token"])
    ctx.check("unlike comment -> liked=false", s == 200 and r.get("liked") is False, f"got {s}: {r}")

    # pin (video owner) -> {comment} with pinnedAt
    s, r = c.post(f"/api/comments/{top_id}/pin", token=owner["token"])
    ctx.check("pin comment (owner) -> 200", s == 200 and (r.get("comment") or {}).get("pinnedAt"),
              f"got {s}: {r}")
    s, r = c.delete(f"/api/comments/{top_id}/pin", token=owner["token"])
    ctx.check("unpin comment (owner) -> 200", s == 200 and not (r.get("comment") or {}).get("pinnedAt"),
              f"got {s}: {r}")

    # pin by non-owner -> 403
    s, body = c.post(f"/api/comments/{top_id}/pin", token=viewer["token"])
    ctx.check("pin comment (non-owner) -> 403", s == 403, f"got {s}: {body}")

    # delete comment (author) -> 204
    s, body = c.delete(f"/api/comments/{top_id}", token=viewer["token"])
    ctx.check("delete comment (author) -> 204", s == 204, f"got {s}: {body}")
