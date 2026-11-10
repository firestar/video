#!/usr/bin/env python3
"""Wait for the age-gate backend deploy (DOB-clear fix) to go live, then run
both the API and UI e2e suites.

Deploy detection: the fix makes `PATCH /api/me {dateOfBirth: null}` actually
clear the stored DOB. We log in as the FIXED viewer account (no throwaway
accounts), set a DOB, clear it, and check it is null. Once that works the new
backend is live (and the faster frontend certainly is too).

Run from the repo root:  python3 e2e/age_deploy_verify.py
"""
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

BASE = os.environ.get("E2E_BASE", "https://video.synload.com")
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
VIEWER = "e2e-api-viewer@synload.local"
PASS = "TestPass123!"
SQL_CMD = ('ssh -o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=no '
           'ops@192.168.1.43 "docker exec -i video-postgres psql -U postgres -d video -tA"')
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def req(method, path, token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(BASE + path, data=data, method=method)
    r.add_header("User-Agent", UA)
    if data is not None:
        r.add_header("Content-Type", "application/json")
    if token:
        r.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(r, timeout=20) as resp:
            txt = resp.read().decode()
            return resp.status, (json.loads(txt) if txt else None)
    except urllib.error.HTTPError as e:
        txt = e.read().decode()
        try:
            return e.code, json.loads(txt)
        except Exception:
            return e.code, txt
    except Exception as e:
        return 0, f"network error: {e}"


def deploy_live():
    """True once the DOB-clear fix is live. Leaves the viewer's DOB clean."""
    try:
        s, b = req("POST", "/api/auth/login", body={"email": VIEWER, "password": PASS})
        if s != 200 or not isinstance(b, dict):
            return False
        tok = b.get("token")
        if not tok:
            return False
        # Set a DOB, then clear it.
        s, _ = req("PATCH", "/api/me", token=tok, body={"dateOfBirth": "2000-01-01"})
        if s != 200:
            return False
        s, _ = req("PATCH", "/api/me", token=tok, body={"dateOfBirth": None})
        if s != 200:
            return False
        s, me = req("GET", "/api/me", token=tok)
        if s != 200 or not isinstance(me, dict):
            return False
        return me.get("dateOfBirth") is None
    except Exception:
        return False


def main():
    t0 = time.time()
    timeout = 840  # 14 min
    print(f"base: {BASE}", flush=True)
    print("Waiting for the age-gate backend deploy (DOB-clear fix) to go live...",
          flush=True)
    live = False
    i = 0
    while time.time() - t0 < timeout:
        i += 1
        if deploy_live():
            live = True
            print(f"AGE-GATE BACKEND LIVE after ~{int(time.time() - t0)}s", flush=True)
            break
        print(f"[{time.strftime('%H:%M:%S')}] not live yet ({i}, {int(time.time() - t0)}s)",
              flush=True)
        time.sleep(30)
    if not live:
        print("WARNING: backend fix not confirmed live after timeout — running suites anyway",
              flush=True)

    env = dict(os.environ)
    env["E2E_SQL_CMD"] = SQL_CMD

    print("\n=== API e2e suite ===", flush=True)
    p1 = subprocess.run([sys.executable, os.path.join(HERE, "run.py")],
                        env=env, capture_output=True, text=True)
    print(p1.stdout)
    if p1.stderr:
        print(p1.stderr)

    print("\n=== UI e2e suite ===", flush=True)
    p2 = subprocess.run([sys.executable, os.path.join(ROOT, "e2e-ui", "run.py")],
                        env=env, capture_output=True, text=True)
    print(p2.stdout)
    if p2.stderr:
        print(p2.stderr)

    print("\n=== DONE ===", flush=True)
    print(f"API exit={p1.returncode}, UI exit={p2.returncode}", flush=True)
    sys.exit(0 if (p1.returncode == 0 and p2.returncode == 0) else 1)


if __name__ == "__main__":
    main()
