"""Account settings: update profile (name/email/bio), change password."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    run_id = ctx.run_id
    email = f"acc-{run_id}@test.local"
    new_email = f"acc-renamed-{run_id}@test.local"

    s, me = c.post("/api/auth/register", body={
        "name": "Acc User", "email": email, "password": "TestPass123!",
        "channelName": f"Acc Channel {run_id}"})
    if s != 200:
        ctx.check("account: register", False, f"got {s}: {me}")
        return
    tok = me["token"]

    # initial /api/me
    s, m0 = c.get("/api/me", token=tok)
    ctx.check("initial /api/me bio is null", s == 200 and m0.get("bio") is None,
              f"got {s}: {m0}")

    # update profile
    s, up = c.patch("/api/me", token=tok, body={
        "name": "Acc User Renamed", "email": new_email, "bio": "hello world"})
    ctx.check("update profile -> 200", s == 200, f"got {s}: {up}")

    s, m1 = c.get("/api/me", token=tok)
    ctx.check("profile updated (name/email/bio)",
              s == 200 and m1.get("name") == "Acc User Renamed"
              and m1.get("email") == new_email and m1.get("bio") == "hello world",
              f"got {s}: {m1}")

    # invalid email -> 400
    s, body = c.patch("/api/me", token=tok, body={"email": "not-an-email"})
    ctx.check("update profile (invalid email) -> 400", s == 400, f"got {s}: {body}")

    # change password: wrong current -> 401
    s, body = c.post("/api/auth/change-password", token=tok, body={
        "currentPassword": "WrongPass999!", "newPassword": "NewPass456!"})
    ctx.check("change-password (wrong current) -> 401", s == 401, f"got {s}: {body}")

    # change password: correct -> 204
    s, body = c.post("/api/auth/change-password", token=tok, body={
        "currentPassword": "TestPass123!", "newPassword": "NewPass456!"})
    ctx.check("change-password (correct) -> 204", s == 204, f"got {s}: {body}")

    # login with new password -> 200
    s, body = c.post("/api/auth/login", body={"email": new_email, "password": "NewPass456!"})
    ctx.check("login (new password) -> 200", s == 200, f"got {s}: {body}")

    # login with old password -> 401
    s, body = c.post("/api/auth/login", body={"email": new_email, "password": "TestPass123!"})
    ctx.check("login (old password) -> 401", s == 401, f"got {s}: {body}")
