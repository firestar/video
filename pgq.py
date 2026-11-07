import socket, struct, sys

def build_startup(user, db):
    body = struct.pack(">I", 196608)  # protocol 3.0
    body += b"user\x00" + user.encode() + b"\x00"
    body += b"database\x00" + db.encode() + b"\x00"
    body += b"\x00"
    return struct.pack(">I", len(body) + 4) + body

class Pg:
    def __init__(self, host, port, user, pw, db):
        self.s = socket.create_connection((host, port), timeout=5)
        self.buf = b""
        self.s.sendall(build_startup(user, db))
        # handle auth
        while True:
            m = self.readmsg()
            t = m[0:1]
            if t == b"R":
                code = struct.unpack(">I", m[1:5])[0]
                if code == 0: break        # ok
                if code == 3:             # md5
                    import hashlib
                    inner = hashlib.md5((pw + user).encode()).hexdigest()
                    outer = "md5" + hashlib.md5((inner + pw).encode()).hexdigest()
                    msg = b"p" + outer.encode() + b"\x00"
                    self.s.sendall(struct.pack(">I", len(msg)+4) + msg)
                    continue
                else:
                    raise RuntimeError("auth code %d" % code)
            elif t == b"Z":
                break
            elif t == b"E":
                raise RuntimeError(m[1:].decode(errors="replace"))

    def readmsg(self):
        while True:
            if len(self.buf) >= 4:
                ln = struct.unpack(">I", self.buf[:4])[0]
                if len(self.buf) >= 1 + ln:
                    m = self.buf[:1+ln]; self.buf = self.buf[1+ln:]; return m
            self.buf += self.s.recv(4096)

    def query(self, q):
        rows = []
        msg = b"Q" + q.encode() + b"\x00"
        self.s.sendall(struct.pack(">I", len(msg)+4) + msg)
        while True:
            m = self.readmsg()
            t = m[0:1]
            if t == b"C": break
            if t == b"D":
                n = struct.unpack(">H", m[1:3])[0]; p = 3; vals = []
                for _ in range(n):
                    ln = struct.unpack(">i", m[p:p+4])[0]; p += 4
                    if ln == -1: vals.append(None)
                    else: vals.append(m[p:p+ln].decode(errors="replace")); p += ln
                rows.append(vals)
            if t == b"E": raise RuntimeError(m[1:].decode(errors="replace"))
        self.s.close()
        return rows

q = sys.argv[1]
for r in Pg("localhost", 5432, "postgres", "postgres", "youtube").query(q):
    print("\t".join(r))
