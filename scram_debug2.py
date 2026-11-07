import socket, struct, os, base64

s = socket.create_connection(("127.0.0.1", 55432), timeout=5)
body = struct.pack(">I", 196608)
body += b"user\x00" + b"postgres" + b"\x00"
body += b"database\x00" + b"postgres" + b"\x00"
body += b"\x00"
s.sendall(struct.pack(">I", len(body) + 4) + body)
buf = b""
s.settimeout(5)
while len(buf) < 600:
    try:
        c = s.recv(65536)
    except socket.timeout:
        break
    if not c:
        break
    buf += c

# walk messages
p = 0
while p + 5 <= len(buf):
    ln = struct.unpack(">I", buf[p:p+4])[0]
    t = buf[p+4:p+5].decode()
    if t == 'R':
        code = struct.unpack(">I", buf[p+5:p+9])[0]
        print("R code", code, "payload_len", ln-9, "payload_tail_hex", buf[p+ln-24:p+ln].hex())
    else:
        print("msg", t, "len", ln)
    p += 1 + ln
print("total", len(buf))
