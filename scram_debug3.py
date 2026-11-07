import socket, struct, os, base64, hashlib, hmac

s = socket.create_connection(("127.0.0.1", 55432), timeout=5)
buf = b""

def sendraw(msg):
    s.sendall(struct.pack(">I", len(msg)) + msg)

def readall():
    global buf
    s.settimeout(3)
    try:
        while True:
            c = s.recv(65536)
            if not c:
                break
            buf += c
    except socket.timeout:
        pass

def msgs():
    p = 0
    out = []
    while p + 5 <= len(buf):
        ln = struct.unpack(">I", buf[p:p+4])[0]
        if p + 5 + ln > len(buf):
            break
        out.append((buf[p+4:p+5], buf[p+5:p+5+ln]))
        p += 5 + ln
    return out

body = struct.pack(">I", 196608) + b"user\x00postgres\x00" + b"database\x00postgres\x00" + b"\x00"
sendraw(body)
readall()
ms = msgs()
print("startup msgs:", [t for t, _ in ms])
scram_payload = None
for t, p in ms:
    if t == b"R":
        code = struct.unpack(">I", p[:4])[0]
        print("R code", code)
        if code == 10:
            scram_payload = p[4:]
assert scram_payload, "no scram request"
# server-first: r=<nonce>,s=<b64>,i=<iters>
fields = {}
for part in scram_payload.split(b"\x00"):
    if part:
        k, v = part.split(b"=", 1)
        fields[k] = v
print("r:", fields.get(b"r")[:40], "s_len", len(fields.get(b"s", b"")), "i", fields.get(b"i"))

user = b"postgres"
pw = b""
client_nonce = os.urandom(18)
first_bare = b"n=" + user + b",r=" + client_nonce
first = b"n,," + first_bare
sendraw(b"p" + first + b"\x00")
print("sent SCRAM-first")
readall()
ms = msgs()
print("after-first msgs:", [t for t, _ in ms])
for t, p in ms:
    if t == b"p":
        print("server-first:", p.decode(errors="replace"))
    elif t == b"E":
        print("ERROR:", p.decode(errors="replace"))
