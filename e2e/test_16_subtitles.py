"""Subtitles: upload (VTT + SRT), list, overwrite, ownership, delete.

The API suite never PUTs video bytes (Cloudflare bot protection), so the
video row stays `failed` — subtitle endpoints work on any status. Subtitle
bytes ARE PUT to the presigned URL (same mechanism the UI suite uses for
video uploads, which works from this host).
"""
import urllib.error
import urllib.request

from harness import Ctx, UA

VTT_EN = (
    "WEBVTT\n\n"
    "00:00:00.500 --> 00:00:02.000\n"
    "Hello from the e2e suite\n\n"
    "00:00:02.500 --> 00:00:04.000\n"
    "Second cue\n"
)

SRT_DE = (
    "1\n"
    "00:00:00,500 --> 00:00:02,000\n"
    "Hallo aus dem e2e-Test\n\n"
    "2\n"
    "00:00:02,500 --> 00:00:04,000\n"
    "Zweite Zeile\n"
)


def _put(url, data: bytes, ctype: str) -> int:
    r = urllib.request.Request(url, data=data, method="PUT")
    r.add_header("User-Agent", UA)
    r.add_header("Content-Type", ctype)
    try:
        with urllib.request.urlopen(r, timeout=60) as resp:
            return resp.status
    except urllib.error.HTTPError as e:
        return e.code


def _get(url) -> tuple:
    if not url:
        return 0, b""
    r = urllib.request.Request(url, method="GET")
    r.add_header("User-Agent", UA)
    try:
        with urllib.request.urlopen(r, timeout=60) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:
        return 0, str(e).encode()


def _upload_subtitle(c, token, video_id, language, filename, data: bytes, ctype: str):
    """presign (purpose=subtitle) -> PUT bytes -> register. Returns (ok, detail)."""
    s, ps = c.post("/api/uploads/presign", token=token, body={
        "file_name": filename, "content_type": ctype, "size": len(data),
        "purpose": "subtitle", "videoId": video_id, "language": language,
    })
    if s != 200 or not isinstance(ps, dict) or "upload_url" not in ps:
        return False, f"presign got {s}: {ps}"
    put_status = _put(ps["upload_url"], data, ctype)
    if put_status not in (200, 201):
        return False, f"PUT got {put_status}"
    s, reg = c.post(f"/api/videos/{video_id}/subtitles", token=token, body={
        "language": language, "objectKey": ps["object_key"],
    })
    if s not in (200, 201) or not isinstance(reg, dict):
        return False, f"register got {s}: {reg}"
    sub = reg.get("subtitle")
    if not isinstance(sub, dict):
        return False, f"register response missing 'subtitle': {reg}"
    return True, sub

def _list_subs(c, vid):
    """GET the track list; the endpoint wraps it as {"subtitles": [...]}."""
    s, body = c.get(f"/api/videos/{vid}/subtitles")
    if s == 200 and isinstance(body, dict) and isinstance(body.get("subtitles"), list):
        return s, body["subtitles"]
    return s, None

def run(ctx: Ctx):
    c = ctx.client
    owner = ctx.user("owner")
    viewer = ctx.user("viewer")
    if not owner or not viewer:
        return

    vid = ctx.video("subtitles", title=f"E2E Subtitles {ctx.run_id}")
    if not vid:
        return

    # --- VTT upload (English) ---
    ok, reg = _upload_subtitle(c, owner["token"], vid, "en",
                               f"e2e_{ctx.run_id}.vtt", VTT_EN.encode(), "text/vtt")
    ctx.check("register VTT (en)", ok, str(reg))
    if not ok:
        return
    ctx.check("register returns language+format+url",
              reg.get("language") == "en" and reg.get("format") == "vtt" and reg.get("url"),
              f"got {reg}")

    # --- list (public) ---
    s, lst = _list_subs(c, vid)
    ctx.check("list subtitles -> 200", s == 200 and lst is not None, f"got {s}")
    if s == 200 and lst is not None:
        ctx.check("list has exactly one track (en)",
                  len(lst) == 1 and lst[0].get("language") == "en",
                  f"got {lst}")
        # signed URL serves the exact bytes
        st, body = _get(lst[0].get("url"))
        ctx.check("signed URL serves the VTT bytes",
                  st == 200 and body == VTT_EN.encode(),
                  f"status={st} bytes={len(body)}")

    # --- SRT upload (German) -> converted to WebVTT ---
    ok, reg = _upload_subtitle(c, owner["token"], vid, "de",
                               f"e2e_{ctx.run_id}.srt", SRT_DE.encode(), "text/srt")
    ctx.check("register SRT (de)", ok, str(reg))
    if ok:
        ctx.check("SRT registered as vtt format", reg.get("format") == "vtt", f"got {reg}")
    s, lst = _list_subs(c, vid)
    if s == 200 and lst is not None:
        ctx.check("list now has two tracks (en, de)",
                  sorted(t.get("language") for t in lst) == ["de", "en"],
                  f"got {[t.get('language') for t in lst]}")
        de = next((t for t in lst if t.get("language") == "de"), None)
        if de:
            st, body = _get(de.get("url"))
            text = body.decode("utf-8", "replace")
            ctx.check("SRT converted to WebVTT (served body starts with WEBVTT)",
                      st == 200 and text.startswith("WEBVTT"),
                      f"status={st} head={text[:40]!r}")
            ctx.check("converted VTT has dot-millisecond timestamps",
                      "00:00:00.500 --> 00:00:02.000" in text,
                      f"head={text[:80]!r}")
            ctx.check("converted VTT keeps the cue text",
                      "Hallo aus dem e2e-Test" in text, f"body={text[:120]!r}")

    # --- overwrite: re-upload the same language ---
    VTT_EN2 = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nReplaced cue\n"
    ok, _ = _upload_subtitle(c, owner["token"], vid, "en",
                             f"e2e_{ctx.run_id}_2.vtt", VTT_EN2.encode(), "text/vtt")
    ctx.check("re-register VTT (en) overwrites", ok, str(_))
    if ok:
        s, lst = _list_subs(c, vid)
        if s == 200 and lst is not None:
            ctx.check("still two tracks after overwrite (no duplicate en)",
                      sorted(t.get("language") for t in lst) == ["de", "en"],
                      f"got {[t.get('language') for t in lst]}")
            en = next((t for t in lst if t.get("language") == "en"), None)
            if en:
                st, body = _get(en.get("url"))
                ctx.check("overwritten track serves the new bytes",
                          st == 200 and body == VTT_EN2.encode(),
                          f"status={st} head={body[:40]!r}")

    # --- the video detail response carries the tracks ---
    s, detail = c.get(f"/api/videos/{vid}", token=owner["token"])
    if s == 200 and isinstance(detail, dict):
        ctx.check("GET /api/videos/{id} includes subtitles[]",
                  isinstance(detail.get("subtitles"), list)
                  and len(detail.get("subtitles", [])) == 2,
                  f"got {detail.get('subtitles')}")

    # --- ownership: non-owner cannot register or delete ---
    s, ps = c.post("/api/uploads/presign", token=viewer["token"], body={
        "file_name": "x.vtt", "content_type": "text/vtt", "size": 10,
        "purpose": "subtitle", "videoId": vid, "language": "fr",
    })
    ctx.check("presign subtitle (non-owner) -> 403", s == 403, f"got {s}: {ps}")
    s, body = c.post(f"/api/videos/{vid}/subtitles", token=viewer["token"], body={
        "language": "fr", "objectKey": "subtitles/whatever.vtt",
    })
    ctx.check("register subtitle (non-owner) -> 403", s == 403, f"got {s}: {body}")
    s, body = c.delete(f"/api/videos/{vid}/subtitles/en", token=viewer["token"])
    ctx.check("delete subtitle (non-owner) -> 403", s == 403, f"got {s}: {body}")

    # --- invalid language rejected ---
    s, body = c.post(f"/api/videos/{vid}/subtitles", token=owner["token"], body={
        "language": "not a language!", "objectKey": "subtitles/whatever.vtt",
    })
    ctx.check("register subtitle (bad language) -> 400", s == 400, f"got {s}: {body}")

    # --- delete a track ---
    s, lst = _list_subs(c, vid)
    de_url = None
    if s == 200 and lst is not None:
        de = next((t for t in lst if t.get("language") == "de"), None)
        de_url = de.get("url") if de else None
    s, body = c.delete(f"/api/videos/{vid}/subtitles/de", token=owner["token"])
    ctx.check("delete subtitle (de) -> 200", s == 200, f"got {s}: {body}")
    s, lst = _list_subs(c, vid)
    if s == 200 and lst is not None:
        ctx.check("only en remains after delete",
                  [t.get("language") for t in lst] == ["en"], f"got {lst}")
    if de_url:
        st, _ = _get(de_url)
        ctx.check("deleted track's URL is gone (403/404)", st in (403, 404),
                  f"status={st}")

    # --- deleting the video removes its subtitle objects ---
    s, body = c.delete(f"/api/videos/{vid}", token=owner["token"])
    ctx.check("delete video -> 200", s == 200, f"got {s}: {body}")
    if s == 200 and isinstance(body, dict):
        ctx.check("video delete reports subtitlesDeleted",
                  body.get("subtitlesDeleted") == [True],
                  f"got {body.get('subtitlesDeleted')}")
    # the video is gone from ctx's cache so cleanup won't double-delete
    ctx.videos.pop("subtitles", None)
