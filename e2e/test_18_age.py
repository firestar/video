"""Age restriction (18+): an age-restricted video's streams are withheld from
non-adult viewers and released after a valid age verification; the stored DOB
always wins over the self-declaration. Uses the fixed viewer but resets its age
state (DOB + confirmation) before and after, so no new accounts are created.

Stream-withholding checks need a *ready* video, which requires E2E_SQL_CMD to
flip the (never-uploaded) presigned video to ready; without it they are skipped.
The account-side age endpoints (age-confirm, DOB validation) always run.
"""
from harness import Ctx

UNDER18_DOB = "2015-01-01"  # ~11 years old
ADULT_DOB = "2005-01-01"    # ~21 years old


def _reset_age(c, token):
    """Clear the viewer's DOB and 18+ confirmation (clean slate)."""
    c.patch("/api/me", token=token, body={"dateOfBirth": None, "ageConfirmed": False})


def _streams(c, token, vid):
    s, v = c.get(f"/api/videos/{vid}", token=token)
    if s != 200 or not isinstance(v, dict):
        return None
    st = v.get("streams")
    return st if isinstance(st, list) else None


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    if not owner or not viewer:
        return
    vtok = viewer["token"]

    # Deterministic start: clear any age state a previous run may have left.
    _reset_age(c, vtok)

    # Create an age-restricted video (owner).
    vid = ctx.video("age_restricted", title="E2E Age Restricted", age_restricted=True)
    ctx.check("create age-restricted video", bool(vid), "create failed")
    if not vid:
        return

    # --- account-side age endpoints (no ready video needed) ---
    # 1. age-confirm with no DOB -> 200 + ageConfirmed true.
    s, body = c.post("/api/age-confirm", token=vtok)
    ctx.check("age-confirm (no DOB) -> 200 + confirmed",
              s == 200 and isinstance(body, dict) and body.get("ageConfirmed") is True,
              f"got {s}: {body}")
    # 2. /api/me reflects the confirmation.
    s, me = c.get("/api/me", token=vtok)
    ctx.check("/api/me shows ageConfirmed=true",
              s == 200 and isinstance(me, dict) and me.get("ageConfirmed") is True,
              f"got {s}: {me}")
    # 3. Set an under-18 DOB -> 200.
    s, body = c.patch("/api/me", token=vtok, body={"dateOfBirth": UNDER18_DOB})
    ctx.check("set under-18 DOB -> 200",
              s == 200 and isinstance(body, dict) and body.get("dateOfBirth") == UNDER18_DOB,
              f"got {s}: {body}")
    # 4. age-confirm now refused (DOB says under 18).
    s, body = c.post("/api/age-confirm", token=vtok)
    ctx.check("age-confirm with under-18 DOB -> 400", s == 400, f"got {s}: {body}")
    # 5. DOB validation: a future date -> 400.
    s, body = c.patch("/api/me", token=vtok, body={"dateOfBirth": "2030-01-01"})
    ctx.check("future DOB -> 400", s == 400, f"got {s}: {body}")
    # 6. DOB validation: older than 120 years -> 400.
    s, body = c.patch("/api/me", token=vtok, body={"dateOfBirth": "1900-01-01"})
    ctx.check("DOB older than 120y -> 400", s == 400, f"got {s}: {body}")
    # 7. Set an 18+ DOB -> 200 (viewer is now an adult via DOB).
    s, body = c.patch("/api/me", token=vtok, body={"dateOfBirth": ADULT_DOB})
    ctx.check("set 18+ DOB -> 200",
              s == 200 and isinstance(body, dict) and body.get("dateOfBirth") == ADULT_DOB,
              f"got {s}: {body}")

    # --- stream withholding (needs a ready video via E2E_SQL_CMD) ---
    if ctx.flip_ready(vid):
        # With the 18+ DOB set, the viewer is an adult -> streams released.
        st = _streams(c, vtok, vid)
        ctx.check("adult (18+ DOB): streams released",
                  st is not None and len(st) > 0, f"streams={st}")
        # Owner is never gated on their own content.
        st = _streams(c, owner["token"], vid)
        ctx.check("owner bypass: streams visible",
                  st is not None and len(st) > 0, f"streams={st}")
        # Clear DOB + confirmation -> non-adult -> streams withheld.
        _reset_age(c, vtok)
        st = _streams(c, vtok, vid)
        ctx.check("non-adult: streams withheld",
                  st is not None and len(st) == 0, f"streams={st}")
        # A stale self-declaration must NOT override an under-18 DOB.
        c.patch("/api/me", token=vtok, body={"ageConfirmed": True})
        c.patch("/api/me", token=vtok, body={"dateOfBirth": UNDER18_DOB})
        st = _streams(c, vtok, vid)
        ctx.check("under-18 DOB wins over stale confirm: streams withheld",
                  st is not None and len(st) == 0, f"streams={st}")
    else:
        ctx.skip("age gate stream withholding", "set E2E_SQL_CMD to flip to ready")

    # Leave the fixed viewer clean for the next run.
    _reset_age(c, vtok)
