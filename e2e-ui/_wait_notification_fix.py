#!/usr/bin/env python3
"""Wait for the notification-actor fix to deploy, then run the e2e-ui suite.

REUSES two fixed probe accounts — registered once, then logged in on every
poll. It NEVER signs up a new account per poll (those can't be deleted and
would pile up on the live site). Each poll creates one throwaway video, has
the viewer like it, checks the owner's notification, then deletes the video
(which cascades the like + notification away).

Probe (per poll):
  - log in as owner + viewer (register once if the account doesn't exist yet)
  - owner creates a throwaway video (fake object key — likes don't need it ready)
  - viewer likes it
  - owner reads notifications; the fix is LIVE when the new_like notification's
    channelName is the VIEWER's channel (the actor). The old backend names the
    owner's channel (the video's channel).
  - owner deletes the video

Run from the repo root:  python3 e2e-ui/_wait_notification_fix.py
"""
import json, os, time, urllib.request, urllib.error, subprocess, sys

BASE = os.environ.get("E2E_UI_BASE", "https://video.synload.com")
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36"

# Fixed probe accounts — created once, reused forever. Override via env if needed.
OWNER_EMAIL = os.environ.get("PROBE_OWNER_EMAIL", "deploy-probe-owner@synload.local")
VIEWER_EMAIL = os.environ.get("PROBE_VIEWER_EMAIL", "deploy-probe-viewer@synload.local")
PROBE_PASSWORD = os.environ.get("PROBE_PASSWORD", "DeployProbe!2026")
OWNER_NAME = "Deploy Probe Owner"
VIEWER_NAME = "Deploy Probe Viewer"


def req(method, path, token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(BASE + path, data=data, method=method)
    r.add_header("User-Agent", UA)
    if data:
        r.add_header("Content-Type", "application/json")
    if token:
        r.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(r, timeout=20) as resp:
            txt = resp.read().decode()
            return resp.status, (json.loads(txt) if txt else None)
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read().decode())
        except Exception:
            return e.code, None
    except Exception:
        return 0, None


def login(email):
    s, me = req("POST", "/api/auth/login", body={"email": email, "password": PROBE_PASSWORD})
    if s == 200 and isinstance(me, dict) and "token" in me:
        return me
    return None


def ensure_account(email, name):
    """Log in; register ONCE if the account doesn't exist yet. Returns the
    {token, user} object or None. Never registers more than once per account."""
    me = login(email)
    if me is None:
        s, me = req("POST", "/api/auth/register", body={
            "name": name, "email": email,
            "password": PROBE_PASSWORD, "channelName": name})
        if s != 200 or not isinstance(me, dict) or "token" not in me:
            return None
    return me


def notification_fixed():
    """True when a like notification names the LIKER's channel (the fix)."""
    owner = ensure_account(OWNER_EMAIL, OWNER_NAME)
    viewer = ensure_account(VIEWER_EMAIL, VIEWER_NAME)
    if not owner or not viewer:
        return False
    owner_tok = owner["token"]
    viewer_tok = viewer["token"]
    viewer_channel = viewer["user"]["channelName"]

    s, v = req("POST", "/api/videos", owner_tok, body={
        "title": "Deploy probe", "objectKey": f"deployprobe/{int(time.time() * 1000)}.mp4",
        "visibility": "public"})
    if s not in (200, 201) or not isinstance(v, dict):
        return False
    vid = (v.get("video") or {}).get("id")
    if not vid:
        return False
    try:
        s, _ = req("POST", f"/api/videos/{vid}/like", viewer_tok)
        if s != 204:
            return False
        s, n = req("GET", "/api/notifications", owner_tok)
        if s != 200 or not isinstance(n, dict):
            return False
        for item in n.get("notifications", []):
            if item.get("type") == "new_like" and item.get("videoId") == vid:
                return item.get("channelName") == viewer_channel
        return False
    finally:
        req("DELETE", f"/api/videos/{vid}", owner_tok)


def main():
    if "--check" in sys.argv:
        ok = notification_fixed()
        print(f"\n{'PASS' if ok else 'FAIL'}: like notification names the viewer's channel: {ok}")
        sys.exit(0 if ok else 1)
    print(f"base: {BASE}", flush=True)
    print(f"probe accounts: {OWNER_EMAIL} / {VIEWER_EMAIL} (reused, not re-registered)", flush=True)
    print("Waiting for the notification-actor fix to deploy...", flush=True)
    live = False
    for i in range(1, 41):  # up to ~13 min at 20s
        if notification_fixed():
            print(f"NOTIFICATION FIX LIVE after ~{i * 20}s", flush=True)
            live = True
            break
        print(f"[{time.strftime('%H:%M:%S')}] not live yet ({i})", flush=True)
        time.sleep(20)
    if not live:
        print("notification fix NOT live after 800s — running suite anyway", flush=True)

    print("\n=== Running e2e-ui suite ===", flush=True)
    here = os.path.dirname(os.path.abspath(__file__))
    r = subprocess.run([sys.executable, os.path.join(here, "run.py")])
    sys.exit(r.returncode)


if __name__ == "__main__":
    main()
