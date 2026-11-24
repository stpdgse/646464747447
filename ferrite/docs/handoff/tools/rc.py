import socket, struct, sys
class Rcon:
    def __init__(s, addr="127.0.0.1:25632", pw="ferrite-oracle"):
        h, p = addr.rsplit(":", 1); s.s = socket.create_connection((h, int(p))); s.i = 0
        s._send(3, pw); s._read()
    def _send(s, k, b):
        s.i += 1; b = b.encode(); p = struct.pack("<ii", s.i, k) + b + b"\0\0"; s.s.sendall(struct.pack("<i", len(p)) + p)
    def _read(s):
        def rd(n):
            d = b""
            while len(d) < n:
                c = s.s.recv(n - len(d)); assert c; d += c
            return d
        n = struct.unpack("<i", rd(4))[0]; d = rd(n); return struct.unpack("<ii", d[:8])[0], d[8:-2].decode()
    def cmd(s, c):
        s._send(2, c); return s._read()[1]
if __name__ == "__main__":
    r = Rcon()
    for c in sys.argv[1:]: print(r.cmd(c))
