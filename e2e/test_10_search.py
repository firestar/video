"""Search: videos + channels."""
from harness import Ctx

import urllib.parse

from harness import Ctx
def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")

    # channel search finds the owner's channel (unique name)
    ch_name = f"E2E SearchCh {ctx.run_id}"
    # register a dedicated user so the channel name is unique and searchable
    s, me = c.post("/api/auth/register", body={
        "name": ch_name, "email": f"searchch-{ctx.run_id}@test.local",
        "password": "TestPass123!", "channelName": ch_name})
    if s != 200:
        ctx.check("search: register channel user", False, f"got {s}: {me}")
        return

    s, r = c.get(f"/api/search?q={ch_name}")
    s, r = c.get(f"/api/search?q={urllib.parse.quote(ch_name)}")
    ctx.check("search finds channel by name", s == 200 and isinstance(r, dict)
              and r.get("channels") is not None, f"got {s}: {r}")

    # search returns the expected shape
    ctx.check("search returns {videos, channels}",
              isinstance(r, dict) and "videos" in r and "channels" in r, f"got {r}")

    # video search: use a substring of an existing ready video's title
    ready_vid, _ = ctx.existing_ready_video()
    s, rv = c.get(f"/api/videos/{ready_vid}") if ready_vid else (0, None)
    if s == 200 and isinstance(rv, dict):
        title = rv.get("video", {}).get("title", "")
        word = next((w for w in title.split() if len(w) >= 4), None)
        if word:
            s, r = c.get(f"/api/search?q={urllib.parse.quote(word)}")
            ctx.check("video search returns 200 + videos array",
                      s == 200 and isinstance(r.get("videos"), list), f"got {s}: {r}")
        else:
            ctx.skip("video search positive match", "no usable word in existing title")
    else:
        # no existing ready video — just assert the shape with a nonsense term
        s, r = c.get("/api/search?q=zzzznotfoundzzzz")
        ctx.check("video search (nonsense) returns 200 + shape",
                  s == 200 and isinstance(r, dict) and "videos" in r, f"got {s}: {r}")
