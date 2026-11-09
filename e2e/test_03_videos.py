"""Videos: create, get, public list, update, delete, visibility, related, channel-videos."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")

    # create a public video
    vid = ctx.video("public", title="E2E Public Video")
    ctx.check("create video public", bool(vid), "create failed")
    if not vid:
        return

    # get video by id
    s, v = c.get(f"/api/videos/{vid}")
    ctx.check("get video -> 200", s == 200 and (v.get("video") or {}).get("id") == vid, f"got {s}")
    ctx.check("get video has streams", s == 200 and isinstance(v.get("streams"), list),
              f"streams={v.get('streams') if s == 200 else s}")

    # public list works and returns a list
    s, lst = c.get("/api/videos?limit=50")
    ctx.check("list videos -> 200 + array", s == 200 and isinstance(lst, list), f"got {s}")

    # if a DB hook is available, flip to ready and assert it appears in the public list
    if ctx.flip_ready(vid):
        s, lst = c.get("/api/videos?limit=100")
        ids = [v.get("id") for v in lst] if isinstance(lst, list) else []
        ctx.check("ready video appears in public list", vid in ids, f"got {s}, ids={ids[:5]}")
    else:
        ctx.skip("ready video appears in public list", "set E2E_SQL_CMD to flip to ready")

    # update video (owner)
    s, up = c.patch(f"/api/videos/{vid}", token=owner["token"],
                    body={"title": "E2E Public Video (edited)", "description": "edited desc"})
    ctx.check("update video (owner) -> 200", s == 200 and up.get("title") == "E2E Public Video (edited)",
              f"got {s}: {up}")

    # update by non-owner -> 403
    s, body = c.patch(f"/api/videos/{vid}", token=viewer["token"], body={"title": "hack"})
    ctx.check("update video (non-owner) -> 403", s == 403, f"got {s}: {body}")

    # private video is not in the public list
    pvid = ctx.video("private", visibility="private", title="E2E Private Video")
    if pvid:
        s, lst = c.get("/api/videos?limit=100")
        ids = [v.get("id") for v in lst] if isinstance(lst, list) else []
        ctx.check("private video NOT in public list", pvid not in ids, f"leaked {pvid}")

    # channel-videos (all statuses) includes the owner's processing video
    s, cv = c.get(f"/api/channels/{owner['channel_id']}/videos?channelId={owner['channel_id']}")
    ids = [v.get("id") for v in cv] if isinstance(cv, list) else []
    ctx.check("channel-videos lists owner's video", s == 200 and vid in ids,
              f"got {s}, ids={ids[:5]}")

    # related videos: use an existing ready video (related filters on ready)
    ready_vid, _ = ctx.existing_ready_video()
    if ready_vid:
        s, rel = c.get(f"/api/videos/{ready_vid}/related")
        ctx.check("related videos -> 200 + array", s == 200 and isinstance(rel, list),
                  f"got {s}: type={type(rel).__name__}")
    else:
        ctx.skip("related videos", "no ready video to base it on")

    # delete a dedicated video (owner) -> 200 + deleted, then 404
    del_vid = ctx.video("todelete", title="E2E To Delete")
    if del_vid:
        s, body = c.delete(f"/api/videos/{del_vid}", token=owner["token"])
        ctx.check("delete video (owner) -> 200 + deleted", s == 200 and body.get("deleted") is True,
                  f"got {s}: {body}")
        s, body = c.get(f"/api/videos/{del_vid}")
        ctx.check("deleted video -> 404", s == 404, f"got {s}: {body}")
        ctx.videos.pop("todelete", None)

    # delete by non-owner -> 403
    s, body = c.delete(f"/api/videos/{vid}", token=viewer["token"])
    ctx.check("delete video (non-owner) -> 403", s == 403, f"got {s}: {body}")
