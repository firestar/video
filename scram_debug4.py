import socket, struct, time

s = socket.create_connection(("127.0.0.1", 55432), timeout=5)
body = struct.pack(">I", 196608) + b"user\x00postgres\x00" + b"database\x00postgres\x00" + b"\x00"
print("startup len field:", struct.unpack(">I", body[:4])[0])
s.sendall(struct.pack(">I", len(body)) + body)
print("sent", 4 + len(body), "bytes")
s.settimeout(4)
buf = b""
t0 = time.time()
try:
    while time.time() - t0 < 3:
        c = s.recv(65536)
        if not c:
            print("closed after", len(buf))
            break
        buf += c
        print("recv", len(c), "total", len(buf), "first bytes:", buf[:8].hex())
        if buf[:1] == b"Z":
            break
except socket.timeout:
    print("timeout, total", len(buf))
print("final total", len(buf))
print("hex:", buf[:100].hex())
