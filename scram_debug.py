import socket, struct, hashlib, hmac, os, base64

s = socket.create_connection(("127.0.0.1", 55432), timeout=5)
body = struct.pack(">I", 196608)
body += b"user\x00" + b"postgres" + b"\x00"
body += b"database\x00" + b"postgres" + b"\x00"
body += b"\x00"
s.sendall(struct.pack(">I", len(body) + 4) + body)
buf = b""

def readmsg():
    global buf
    while True:
        if len(buf) >= 4:
            ln = struct.unpack(">I", buf[:4])[0]
            if len(buf) >= 1 + ln:
                m = buf[:1 + ln]
                buf = buf[1 + ln:]
                return m
        buf += s.recv(65536)

m = readmsg()
print("msg", m[0:1], "code", struct.unpack(">I", m[1:5])[0])
# S messages
while True:
    m = readmsg()
    if m[0:1] == b"S":
        continue
    break
print("got", m[0:1], "code", struct.unpack(">I", m[1:5])[0])
salt_b64 = m[5:]
print("salt/nonce area len", len(salt_b64), "tail", salt_b64[-20:])

user = b"postgres"
pw = b""
client_nonce = os.urandom(18)
first_bare = b"n=" + user + b",r=" + client_nonce
first = b"n,," + first_bare
s.sendall(struct.pack(">I", len(first) + 5) + b"p" + first + b"\x00")
print("sent first:", first[:40])
m = readmsg()
print("server reply type", m[0:1])
if m[0:1] == b"E":
    print("ERROR", m[1:].decode(errors="replace"))
else:
    code = struct.unpack(">I", m[1:5])[0]
    print("code", code, "data", m[5:].decode(errors="replace"))
