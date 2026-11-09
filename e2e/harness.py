"""Shared harness for the API e2e suite.

Targets a live backend (default https://video.synload.com; override with
E2E_BASE). Every test registers its own users and creates its own videos via
the public API, so the suite is self-contained and rerunnable.

Videos are created with a presigned object key but the bytes are never PUT to
the bucket (the upload endpoint sits behind Cloudflare bot protection, which
rejects non-browser clients). The worker therefore marks them `failed`, which
is fine: every feature endpoint (comments, likes, notifications, reports,
watch-later, history, playlists, search, channel-videos) works on a video of
any status. Only the *list/feed/trending/shorts* queries filter on
`status='ready'`, so those tests lean on the site's existing ready content,
and — when E2E_SQL_CMD is provided — can flip a video to `ready` to assert it
appears in the public list.

E2E_SQL_CMD (optional): a shell command that reads a SQL statement on stdin and
runs it against the app database, e.g.
  E2E_SQL_CMD='ssh -i ~/.ssh/key ops@192.168.1.43 "docker exec -i video-postgres psql -U postgres -d video -tA"'
"""
import json
import os
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request

BASE = os.environ.get("E2E_BASE", "https://video.synload.com")
UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)


class Client:
    def __init__(self, base=BASE):
        self.base = base.rstrip("/")

    def request(self, method, path, token=None, body=None, raw=None,
                ctype="application/json", timeout=30):
        url = self.base + path
        if raw is not None:
            data = raw
        elif body is not None:
            data = json.dumps(body).encode()
        else:
            data = None
        r = urllib.request.Request(url, data=data, method=method)
        r.add_header("User-Agent", UA)
        if data is not None:
            r.add_header("Content-Type", ctype)
        if token:
            r.add_header("Authorization", f"Bearer {token}")
        try:
            with urllib.request.urlopen(r, timeout=timeout) as resp:
                txt = resp.read().decode()
                return resp.status, (json.loads(txt) if txt else None)
        except urllib.error.HTTPError as e:
            txt = e.read().decode()
            try:
                return e.code, json.loads(txt)
            except Exception:
                return e.code, txt
        except Exception as e:  # network error
            return 0, f"network error: {e}"

    def get(self, path, token=None, **kw):
        return self.request("GET", path, token, **kw)

    def post(self, path, token=None, body=None, **kw):
        return self.request("POST", path, token, body=body, **kw)

    def patch(self, path, token=None, body=None, **kw):
        return self.request("PATCH", path, token, body=body, **kw)

    def delete(self, path, token=None, **kw):
        return self.request("DELETE", path, token, **kw)


class Results:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.skipped = 0
        self.failures = []

    def check(self, name, cond, detail=""):
        if cond:
            self.passed += 1
            print(f"  PASS  {name}")
            return True
        self.failed += 1
        self.failures.append((name, detail))
        print(f"  FAIL  {name}  {detail}")
        return False

    def skip(self, name, reason=""):
        self.skipped += 1
        print(f"  SKIP  {name}  {reason}")


class Ctx:
    """Test context: lazy user + video creation, shared across feature modules."""

    def __init__(self):
        self.client = Client()
        self.results = Results()
        self.run_id = int(time.time())
        self.users = {}
        self.videos = {}
        self._existing_ready = None

    # ---- users ----
    def register(self, role, name=None):
        name = name or f"E2E {role} {self.run_id}"
        email = f"e2e-{role}-{self.run_id}@test.local"
        password = "TestPass123!"
        s, me = self.client.post("/api/auth/register", body={
            "name": name, "email": email, "password": password, "channelName": name})
        self.results.check(f"register {role}", s == 200, f"got {s}: {me}")
        if s != 200:
            self.users[role] = None
            return None
        s2, me2 = self.client.get("/api/me", token=me["token"])
        self.users[role] = {
            "token": me["token"],
            "user_id": me2.get("id"),
            "channel_id": me2.get("channelId"),
            "email": email,
            "password": password,
            "name": name,
        }
        return self.users[role]

    def user(self, role):
        if role not in self.users:
            self.register(role)
        return self.users[role]

    # ---- videos ----
    def video(self, key, role="owner", title=None, visibility="public",
              is_short=False, tags=None):
        """Create (and cache) a video. Returns the video id or None."""
        if key in self.videos:
            return self.videos[key]
        u = self.user(role)
        if not u:
            return None
        title = title or f"E2E {key} {self.run_id}"
        s, ps = self.client.post("/api/uploads/presign", token=u["token"], body={
            "file_name": "e2e.mp4", "content_type": "video/mp4", "size": 1000})
        if s != 200 or not isinstance(ps, dict):
            self.results.check(f"presign {key}", False, f"got {s}: {ps}")
            return None
        body = {
            "title": title,
            "description": f"e2e {key}",
            "tags": tags or ["e2e", key],
            "object_key": ps["object_key"],
            "visibility": visibility,
        }
        if is_short:
            body["isShort"] = True
        s, v = self.client.post("/api/videos", token=u["token"], body=body)
        ok = s in (200, 201) and isinstance(v, dict) and "video" in v
        self.results.check(f"create video {key}", ok, f"got {s}: {v}")
        if ok:
            self.videos[key] = v["video"]["id"]
            return self.videos[key]
        return None

    # ---- helpers ----
    def check(self, name, cond, detail=""):
        return self.results.check(name, cond, detail)

    def skip(self, name, reason=""):
        self.results.skip(name, reason)

    def existing_ready_video(self):
        """(video_id, channel_id) of an existing ready video, or (None, None)."""
        if self._existing_ready is not None:
            return self._existing_ready
        s, lst = self.client.get("/api/videos?limit=50")
        if s == 200 and isinstance(lst, list) and lst:
            ch = (lst[0].get("channel") or {}).get("id")
            self._existing_ready = (lst[0]["id"], ch)
            return self._existing_ready
        self._existing_ready = (None, None)
        return self._existing_ready

    def flip_ready(self, video_id):
        """Flip a video to ready via E2E_SQL_CMD (if configured). Returns bool."""
        cmd = os.environ.get("E2E_SQL_CMD")
        if not cmd:
            return False
        sql = (f"UPDATE videos SET status='ready', duration_secs=10, file_size=1024, "
               f"published_at=now() WHERE id='{video_id}'")
        try:
            p = subprocess.run(cmd, shell=True, input=sql, capture_output=True,
                               text=True, timeout=30)
            return p.returncode == 0
        except Exception:
            return False

    def cleanup(self):
        """Best-effort: delete the videos this run created."""
        u = self.users.get("owner")
        if not u:
            return
        for key, vid in list(self.videos.items()):
            self.client.delete(f"/api/videos/{vid}", token=u["token"])
