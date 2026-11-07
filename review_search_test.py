import socket, struct, hashlib, hmac, os, base64

class Pg:
    def __init__(self, user, db, host="127.0.0.1", port=55432):
        self.user = user.encode()
        self.s = socket.create_connection((host, port), timeout=5)
        self.buf = b""
        body = struct.pack(">I", 196608)
        body += b"user\x00" + self.user + b"\x00"
        body += b"database\x00" + db.encode() + b"\x00"
        body += b"\x00"
        self.s.sendall(struct.pack(">I", len(body) + 4) + body)
        self._auth()

    def readmsg(self):
        while True:
            if len(self.buf) >= 4:
                ln = struct.unpack(">I", self.buf[:4])[0]
                if len(self.buf) >= 5 + ln:
                    m = self.buf[:5 + ln]
                    self.buf = self.buf[5 + ln:]
                    return m
            chunk = self.s.recv(65536)
            if not chunk:
                raise RuntimeError("closed")
            self.buf += chunk

    def _auth(self):
        while True:
            m = self.readmsg()
            t = m[0:1]
            if t == b"R":
                code = struct.unpack(">I", m[1:5])[0]
                if code == 0:
                    return
                if code == 10:
                    self._scram(m[5:])
                    continue
                raise RuntimeError("auth code %d" % code)
            elif t == b"Z":
                return
            elif t in (b"S", b"K", b"N", b"A"):
                continue
            elif t == b"E":
                raise RuntimeError(m[1:].decode(errors="replace"))

    def _scram(self, area):
        # area ends with salt(18) | iters(4)
        iters = struct.unpack(">I", area[-4:])[0]
        salt = area[-24:-4]
        salt_b64 = base64.b64encode(salt).decode()
        client_nonce = os.urandom(18)
        first_bare = b"n=" + self.user + b",r=" + client_nonce
        first = b"n,," + first_bare
        self.s.sendall(struct.pack(">I", len(first) + 5) + b"p" + first + b"\x00")
        m = self.readmsg()
        if m[0:1] == b"E":
            raise RuntimeError(m[1:].decode(errors="replace"))
        assert m[0:1] == b"R", m[0:1]
        fields = {}
        for part in m[5:].split(b"\x00"):
            if not part:
                continue
            k, v = part.split(b"=", 1)
            fields[k] = v
        server_nonce = fields[b"r"]
        assert server_nonce.startswith(client_nonce), "nonce mismatch"
        assert base64.b64decode(fields[b"s"]) == salt
        assert int(fields[b"i"]) == iters
        pw = b""
        salted = hashlib.pbkdf2_hmac("sha256", pw, salt, iters)
        client_key = hmac.new(salted, b"Client Key", hashlib.sha256).digest()
        stored_key = hashlib.sha256(client_key).digest()
        auth_msg = first_bare + b"," + fields[b"r"] + b"," + fields[b"s"] + b"," + fields[b"i"]
        server_sig = hmac.new(stored_key, auth_msg, hashlib.sha256).digest()
        combined = hashlib.sha256(client_key + stored_key).digest()
        proof = bytes(a ^ b for a, b in zip(client_key, combined))
        final = b"c=" + base64.b64encode(b"n,,") + b",p=" + base64.b64encode(proof)
        self.s.sendall(struct.pack(">I", len(final) + 5) + b"p" + final + b"\x00")
        m = self.readmsg()
        if m[0:1] == b"E":
            raise RuntimeError(m[1:].decode(errors="replace"))
        assert m[0:1] == b"R"
        code = struct.unpack(">I", m[1:5])[0]
        if code == 12:  # server-final with v=
            return
        if code == 0:
            return
        raise RuntimeError("scram final code %d" % code)

    def exec(self, q):
        msg = b"Q" + q.encode() + b"\x00"
        self.s.sendall(struct.pack(">I", len(msg) + 4) + msg)
        while True:
            m = self.readmsg()
            t = m[0:1]
            if t == b"C":
                continue
            if t == b"E":
                raise RuntimeError(m[1:].decode(errors="replace"))
            if t == b"Z":
                return True

    def query(self, q):
        rows = []
        msg = b"Q" + q.encode() + b"\x00"
        self.s.sendall(struct.pack(">I", len(msg) + 4) + msg)
        while True:
            m = self.readmsg()
            t = m[0:1]
            if t == b"C":
                continue
            if t == b"D":
                n = struct.unpack(">H", m[1:3])[0]
                p = 3
                vals = []
                for _ in range(n):
                    ln = struct.unpack(">i", m[p:p + 4])[0]
                    p += 4
                    if ln == -1:
                        vals.append(None)
                    else:
                        vals.append(m[p:p + ln].decode(errors="replace"))
                        p += ln
                rows.append(vals)
            if t == b"E":
                raise RuntimeError(m[1:].decode(errors="replace"))
            if t == b"Z":
                break
        return rows


def main():
    base = Pg("postgres", "postgres")
    base.exec("DROP DATABASE IF EXISTS searchtest WITH (FORCE)")
    base.exec("CREATE DATABASE searchtest")
    print("created searchtest")
    base.s.close()

    import os
    c = Pg("postgres", "searchtest")
    base_dir = "backend/crates/db/migrations"
    for f in ["001_init.sql", "002_search.sql", "003_comments.sql"]:
        path = os.path.join(base_dir, f)
        with open(path) as fh:
            sql = fh.read()
        c.exec(sql)
        print("applied", f)

    cols = c.query("SELECT column_name FROM information_schema.columns WHERE table_name='videos' AND column_name='search_vector'")
    print("search_vector column:", cols)
    assert cols == [("search_vector",)], cols
    idx = c.query("SELECT indexname FROM pg_indexes WHERE tablename='videos' AND indexname='idx_videos_search'")
    print("GIN index:", idx)
    assert idx == [("idx_videos_search",)], idx

    c.exec("""
    DELETE FROM users; DELETE FROM channels; DELETE FROM videos;
    INSERT INTO users (id, email, name, channel_name)
      VALUES (gen_random_uuid(), 'a@x.com', 'Alice', 'alicechan'),
             (gen_random_uuid(), 'b@x.com', 'Bob', 'bobchan');
    INSERT INTO channels (id, user_id, name, handle)
      SELECT gen_random_uuid(), id, channel_name, channel_name FROM users;
    """)
    ch = c.query("SELECT c.id, c.name FROM channels c ORDER BY c.name")
    print("channels:", ch)
    c1, c2 = ch[0][0], ch[1][0]
    c.exec(
        "INSERT INTO videos (id, channel_id, title, description, status, object_key) VALUES "
        "(gen_random_uuid(), '" + c1 + "', 'Rust async runtime demo', 'tokio and axum deep dive', 'ready', 'videos/rust.mp4'),"
        "(gen_random_uuid(), '" + c1 + "', 'Cooking pasta', 'italian food', 'ready', 'videos/pasta.mp4'),"
        "(gen_random_uuid(), '" + c2 + "', 'Rust web framework review', 'axum and actix', 'pending', 'videos/rust2.mp4'),"
        "(gen_random_uuid(), '" + c2 + "', 'Music video', 'song', 'failed', 'videos/music.mp4')"
    )
    print("seeded")

    q = "SELECT title FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', 'rust') ORDER BY ts_rank(search_vector, plainto_tsquery('english', 'rust')) DESC LIMIT 10"
    rows = c.query(q)
    print("FTS rust ready-only:", rows)
    assert rows == [("Rust async runtime demo",)], rows

    cnt = c.query("SELECT count(*) FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', 'rust')")
    print("ready rust count:", cnt)
    assert cnt[0][0] == "1", cnt
    cnt3 = c.query("SELECT count(*) FROM videos WHERE search_vector @@ plainto_tsquery('english', 'rust')")
    print("all-status rust count (2 => filter matters):", cnt3)
    assert cnt3[0][0] == "2", cnt3

    rows4 = c.query("SELECT title FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', 'axum')")
    print("FTS axum (description, ready):", rows4)
    assert rows4 == [("Rust async runtime demo",)], rows4

    rows5 = c.query("SELECT count(*) FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', 'Rust')")
    print("FTS capitalized Rust (english lowercases):", rows5)
    assert rows5[0][0] == "1", rows5

    rows6 = c.query("SELECT name FROM channels WHERE name ILIKE '%alu%' ORDER BY name LIMIT 10")
    print("channel ILIKE %alu%:", rows6)
    assert any("alicechan" in r[0] for r in rows6), rows6

    rows7 = c.query("SELECT count(*) FROM channels WHERE name ILIKE '%_u%'")
    print("channel ILIKE underscore pattern:", rows7)

    print("ALL_MIGRATION_AND_QUERY_TESTS_PASSED")


main()
