"""Playlists: create, list, get, add/remove item, update, delete, visibility."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # create playlist
    s, p = c.post("/api/playlists", token=owner["token"], body={"title": "E2E Playlist", "isPublic": True})
    ctx.check("create playlist -> 200 + id", s == 200 and p.get("id"), f"got {s}: {p}")
    pl_id = p.get("id") if isinstance(p, dict) else None
    if not pl_id:
        return

    # empty title -> 400
    s, body = c.post("/api/playlists", token=owner["token"], body={"title": "   "})
    ctx.check("create playlist (empty title) -> 400", s == 400, f"got {s}: {body}")

    # list playlists includes it
    s, lst = c.get("/api/playlists", token=owner["token"])
    ids = [x.get("id") for x in lst.get("playlists", [])] if isinstance(lst, dict) else []
    ctx.check("playlist list includes new playlist", s == 200 and pl_id in ids, f"got {s}, ids={ids}")

    # add a video to the playlist
    s, r = c.post(f"/api/playlists/{pl_id}/items", token=owner["token"], body={"videoId": vid})
    ctx.check("add playlist item -> 200 + videoCount=1", s == 200 and r.get("videoCount") == 1, f"got {s}: {r}")

    # get playlist (public) includes the video
    s, gp = c.get(f"/api/playlists/{pl_id}", token=viewer["token"])
    vids = [v.get("id") for v in gp.get("videos", [])] if isinstance(gp, dict) else []
    ctx.check("get playlist (public) includes video", s == 200 and vid in vids, f"got {s}, vids={vids}")

    # add a nonexistent video -> 404
    s, body = c.post(f"/api/playlists/{pl_id}/items", token=owner["token"],
                     body={"videoId": "00000000-0000-0000-0000-000000000000"})
    ctx.check("add nonexistent video to playlist -> 404", s == 404, f"got {s}: {body}")

    # remove the video
    s, r = c.delete(f"/api/playlists/{pl_id}/items/{vid}", token=owner["token"])
    ctx.check("remove playlist item -> 200 + videoCount=0", s == 200 and r.get("videoCount") == 0, f"got {s}: {r}")

    # update playlist
    s, p2 = c.patch(f"/api/playlists/{pl_id}", token=owner["token"], body={"title": "E2E Playlist (edited)"})
    ctx.check("update playlist -> 200", s == 200 and p2.get("title") == "E2E Playlist (edited)", f"got {s}: {p2}")

    # non-owner cannot update -> 404 (playlists.update returns None for non-owner)
    s, body = c.patch(f"/api/playlists/{pl_id}", token=viewer["token"], body={"title": "hax"})
    ctx.check("update playlist (non-owner) -> 404", s == 404, f"got {s}: {body}")

    # delete playlist
    s, r = c.delete(f"/api/playlists/{pl_id}", token=owner["token"])
    ctx.check("delete playlist -> 200 + deleted=true", s == 200 and r.get("deleted") is True, f"got {s}: {r}")
    s, body = c.get(f"/api/playlists/{pl_id}", token=owner["token"])
    ctx.check("deleted playlist -> 404", s == 404, f"got {s}: {body}")
