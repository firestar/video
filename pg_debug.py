import socket, struct

s = socket.create_connection(("127.0.0.1", 55432), timeout=3)
body = struct.pack(">I", 196608)
body += b"user\x00" + b"postgres" + b"\x00"
body += b"database\x00" + b"postgres" + b"\x00"
body += b"\x00"
s.sendall(struct.pack(">I", len(body) + 4) + body)
buf = b""
s.settimeout(3)
try:
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
        print("recv", len(chunk), "bytes; hex head:", buf[:24].hex())
except socket.timeout:
    print("TIMEOUT; total", len(buf), "bytes")
print("full:", buf)
s.close()
