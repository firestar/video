"""Shorts: upload with isShort flag, /api/shorts feed shape, ready short appears (if DB hook set)."""
import os

from harness import Ctx


def run(ctx: Ctx):
    c = ctx.client

    # /api/shorts returns the expected shape (may be empty)
    s, r = c.get("/api/shorts?limit=50")
    ctx.check("/api/shorts -> 200 + {shorts}", s == 200 and isinstance(r, dict) and "shorts" in r,
              f"got {s}: {r}")

    # create a short (isShort=true)
    short = ctx.video("short", is_short=True, title="E2E Short")
    if not short:
        return

    # the short is not ready (worker can't transcode the missing object), so it
    # won't appear in the ready-only /api/shorts feed unless we flip it ready.
    if os.environ.get("E2E_SQL_CMD"):
        flipped = ctx.flip_ready(short)
        ctx.check("flip short to ready (E2E_SQL_CMD)", flipped)
        if flipped:
            s, r = c.get("/api/shorts?limit=50")
            ids = [x.get("id") for x in r.get("shorts", [])] if isinstance(r, dict) else []
            ctx.check("ready short appears in /api/shorts", s == 200 and short in ids,
                      f"got {s}, ids={ids[:5]}")
    else:
        ctx.skip("ready short appears in /api/shorts",
                 "set E2E_SQL_CMD to flip the short to ready")

    # /api/shorts requires no auth (public)
    s, r = c.get("/api/shorts")
    ctx.check("/api/shorts (no token) -> 200", s == 200, f"got {s}: {r}")


