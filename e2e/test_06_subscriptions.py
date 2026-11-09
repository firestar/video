"""Subscriptions: subscribe, list, unsubscribe, cannot subscribe to own channel."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")

    # viewer subscribes to owner's channel
    s, r = c.post(f"/api/channels/{owner['channel_id']}/subscribe", token=viewer["token"])
    ctx.check("subscribe -> 200 + subscriberCount", s == 200 and "subscriberCount" in r, f"got {s}: {r}")

    # list subscriptions includes owner's channel
    s, subs = c.get("/api/subscriptions", token=viewer["token"])
    ch_ids = [x.get("channel", {}).get("id") for x in subs] if isinstance(subs, list) else []
    ctx.check("subscriptions list includes owner channel", s == 200 and owner["channel_id"] in ch_ids,
              f"got {s}, ch_ids={ch_ids}")

    # cannot subscribe to own channel -> 400
    s, body = c.post(f"/api/channels/{owner['channel_id']}/subscribe", token=owner["token"])
    ctx.check("subscribe to own channel -> 400", s == 400, f"got {s}: {body}")

    # unsubscribe
    s, r = c.delete(f"/api/channels/{owner['channel_id']}/subscribe", token=viewer["token"])
    ctx.check("unsubscribe -> 200", s == 200, f"got {s}: {r}")
    s, subs = c.get("/api/subscriptions", token=viewer["token"])
    ch_ids = [x.get("channel", {}).get("id") for x in subs] if isinstance(subs, list) else []
    ctx.check("subscriptions list no longer includes owner channel",
              s == 200 and owner["channel_id"] not in ch_ids, f"got {s}, ch_ids={ch_ids}")

    # subscribe to nonexistent channel -> 404
    s, body = c.post("/api/channels/00000000-0000-0000-0000-000000000000/subscribe", token=viewer["token"])
    ctx.check("subscribe to nonexistent channel -> 404", s == 404, f"got {s}: {body}")
