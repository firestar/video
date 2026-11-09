"""Auth: register, login, /api/me, duplicate email, bad credentials."""
from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client
    run_id = ctx.run_id

    # register
    s, me = c.post("/api/auth/register", body={
        "name": f"Auth User {run_id}", "email": f"auth-{run_id}@test.local",
        "password": "TestPass123!", "channelName": f"Auth Channel {run_id}"})
    ctx.check("register returns 200 + token + user", s == 200 and "token" in me and "user" in me,
              f"got {s}: {me}")

    # duplicate email -> 409
    s, body = c.post("/api/auth/register", body={
        "name": "Dup", "email": f"auth-{run_id}@test.local",
        "password": "TestPass123!", "channelName": "Dup"})
    ctx.check("duplicate email -> 409", s == 409, f"got {s}: {body}")

    # weak password -> 400
    s, body = c.post("/api/auth/register", body={
        "name": "Weak", "email": f"weak-{run_id}@test.local",
        "password": "short", "channelName": "Weak"})
    ctx.check("weak password -> 400", s == 400, f"got {s}: {body}")

    # login with correct credentials
    s, tok = c.post("/api/auth/login", body={
        "email": f"auth-{run_id}@test.local", "password": "TestPass123!"})
    ctx.check("login (correct) -> 200 + token", s == 200 and "token" in tok, f"got {s}: {tok}")

    # login with wrong password -> 401
    s, body = c.post("/api/auth/login", body={
        "email": f"auth-{run_id}@test.local", "password": "WrongPass999!"})
    ctx.check("login (wrong password) -> 401", s == 401, f"got {s}: {body}")

    # /api/me with token
    token = tok.get("token") if isinstance(tok, dict) else None
    s, me2 = c.get("/api/me", token=token)
    ctx.check("/api/me -> 200 + channelId", s == 200 and me2.get("channelId"),
              f"got {s}: {me2}")

    # /api/me without token -> 401
    s, body = c.get("/api/me")
    ctx.check("/api/me (no token) -> 401", s == 401, f"got {s}: {body}")
