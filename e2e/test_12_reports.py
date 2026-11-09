"""Reports: report a video, cannot report own, report a comment, cannot report own comment."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    vid = ctx.video("public")
    if not vid:
        return

    # viewer reports owner's video -> 201
    s, body = c.post("/api/reports", token=viewer["token"], body={
        "targetType": "video", "targetId": vid, "reason": "Spam or deceptive", "detail": "test"})
    ctx.check("report video -> 201", s == 201, f"got {s}: {body}")

    # owner reports own video -> 400
    s, body = c.post("/api/reports", token=owner["token"], body={
        "targetType": "video", "targetId": vid, "reason": "Other"})
    ctx.check("report own video -> 400", s == 400, f"got {s}: {body}")

    # invalid targetType -> 400
    s, body = c.post("/api/reports", token=viewer["token"], body={
        "targetType": "banana", "targetId": vid, "reason": "Other"})
    ctx.check("report (invalid targetType) -> 400", s == 400, f"got {s}: {body}")

    # viewer comments; owner reports the comment -> 201
    s, r = c.post(f"/api/videos/{vid}/comments", token=viewer["token"], body={"body": "reportable"})
    cid = r.get("id") if isinstance(r, dict) else None
    if cid:
        s, body = c.post("/api/reports", token=owner["token"], body={
            "targetType": "comment", "targetId": cid, "reason": "Harassment or bullying"})
        ctx.check("report comment -> 201", s == 201, f"got {s}: {body}")
        # viewer reports own comment -> 400
        s, body = c.post("/api/reports", token=viewer["token"], body={
            "targetType": "comment", "targetId": cid, "reason": "Other"})
        ctx.check("report own comment -> 400", s == 400, f"got {s}: {body}")
    else:
        ctx.check("create reportable comment", False, f"got {s}: {r}")

    # reports require auth
    s, body = c.post("/api/reports", body={"targetType": "video", "targetId": vid, "reason": "x"})
    ctx.check("report (no token) -> 401", s == 401, f"got {s}: {body}")
