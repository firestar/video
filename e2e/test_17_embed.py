"""Embed: anonymous access to the public video endpoint + CSP frame-ancestors
headers on the embeddable player route vs the rest of the site.

The embed page (GET /embed/{id}) is a client component: the server always
serves the 200 HTML shell and the player itself shows "Video not available"
for missing/private videos, so the API-level assertions here are about the
data endpoint (what the embed fetches) and the CSP headers that make
third-party iframing possible.
"""
import urllib.request

from harness import BASE, UA, Ctx


def _headers(path):
    """GET `path` anonymously; return (status, headers-dict)."""
    r = urllib.request.Request(BASE + path, method="GET")
    r.add_header("User-Agent", UA)
    with urllib.request.urlopen(r, timeout=30) as resp:
        return resp.status, {k.lower(): v for k, v in resp.headers.items()}


def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    if not owner:
        return

    # --- public video: the embed's data source works anonymously ---
    vid = ctx.video("embed_public", title=f"E2E Embed Public {ctx.run_id}")
    if not vid:
        return

    s, v = c.get(f"/api/videos/{vid}")
    ctx.check("anonymous GET /api/videos/{id} -> 200",
              s == 200 and (v.get("video") or {}).get("id") == vid, f"got {s}")
    ctx.check("anonymous detail has streams array",
              s == 200 and isinstance(v.get("streams"), list),
              f"streams={v.get('streams') if s == 200 else s}")
    ctx.check("anonymous detail has channel",
              s == 200 and isinstance(v.get("channel"), dict),
              f"channel={v.get('channel') if s == 200 else s}")

    # --- private video: invisible to anonymous viewers (404, no leak) ---
    pvid = ctx.video("embed_private", visibility="private",
                     title=f"E2E Embed Private {ctx.run_id}")
    if pvid:
        s, body = c.get(f"/api/videos/{pvid}")
        ctx.check("anonymous GET private video -> 404", s == 404, f"got {s}: {body}")
        s, body = c.get(f"/api/videos/{pvid}", token=owner["token"])
        ctx.check("owner GET private video -> 200",
                  s == 200 and (body.get("video") or {}).get("id") == pvid, f"got {s}")

    # --- CSP: /embed/ is frameable from any origin, everything else is not ---
    s, h = _headers(f"/embed/{vid}")
    csp = h.get("content-security-policy", "")
    ctx.check("GET /embed/{id} -> 200", s == 200, f"got {s}")
    ctx.check("embed page is HTML",
              "text/html" in h.get("content-type", ""), f"ct={h.get('content-type')}")
    ctx.check("embed page allows framing (frame-ancestors *)",
              "frame-ancestors *" in csp, f"csp={csp!r}")

    s, h = _headers(f"/watch/{vid}")
    csp = h.get("content-security-policy", "")
    ctx.check("GET /watch/{id} -> 200", s == 200, f"got {s}")
    ctx.check("watch page is not frameable (frame-ancestors 'self')",
              "frame-ancestors 'self'" in csp, f"csp={csp!r}")

    s, h = _headers("/")
    csp = h.get("content-security-policy", "")
    ctx.check("home page is not frameable (frame-ancestors 'self')",
              s == 200 and "frame-ancestors 'self'" in csp, f"got {s}, csp={csp!r}")

    # Unknown video: the shell still serves (the player shows the error state
    # client-side) and keeps the embeddable CSP.
    s, h = _headers("/embed/00000000-0000-0000-0000-000000000000")
    csp = h.get("content-security-policy", "")
    ctx.check("GET /embed/<unknown> -> 200 shell", s == 200, f"got {s}")
    ctx.check("unknown embed keeps frame-ancestors *",
              "frame-ancestors *" in csp, f"csp={csp!r}")
