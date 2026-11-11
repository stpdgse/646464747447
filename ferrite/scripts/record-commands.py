#!/usr/bin/env python3
"""Records what the official server answers to commands, for `crates/ferrite-game/tests/commands.rs`.

A scripted client (an operator) joins the local vanilla server and runs each command line; the
system messages and a few watched packets it receives are written down.

    scripts/record-commands.py --interact-bin target/debug/ferrite-interact \
        --scenarios scripts/command-scenarios.txt --out crates/ferrite-game/tests/data/commands.txt

Scenario file: `scenario <name>` starts a group (the client rejoins), lines beginning `rcon ` run
on the vanilla server (setup that Ferrite's replay gets by other means), other lines are commands.
Needs the local vanilla server (offline, RCON). Local oracle testing only.
"""
import argparse, os, subprocess, sys, time

WATCH = "player_chat,ticking_state,ticking_step,set_time,player_abilities,game_event,player_info_update,change_difficulty,set_experience,set_health"
sys.path.insert(0, os.path.dirname(__file__))


class Rcon:
    def __init__(self, addr, pw):
        import socket, struct
        self.socket, self.struct = socket, struct
        h, p = addr.rsplit(":", 1); self.s = socket.create_connection((h, int(p))); self.i = 0
        self._send(3, pw); self._read()

    def _send(self, k, b):
        self.i += 1; b = b.encode(); p = self.struct.pack("<ii", self.i, k) + b + b"\0\0"
        self.s.sendall(self.struct.pack("<i", len(p)) + p)

    def _read(self):
        def rd(n):
            d = b""
            while len(d) < n:
                c = self.s.recv(n - len(d)); assert c; d += c
            return d
        n = self.struct.unpack("<i", rd(4))[0]; d = rd(n)
        return self.struct.unpack("<ii", d[:8])[0], d[8:-2].decode()

    def cmd(self, c):
        self._send(2, c); return self._read()[1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25632")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--game", default="127.0.0.1:25630")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--scenarios", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--name", default="InteractBot")
    args = ap.parse_args()
    r = Rcon(args.rcon, args.password)
    r.cmd("tick freeze")
    r.cmd(f"op {args.name}")
    groups = []
    for line in open(args.scenarios):
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        if line.startswith("scenario "):
            groups.append((line[9:], []))
        else:
            groups[-1][1].append(line)
    out = open(args.out, "w")
    out.write("# Recorded from the official server by scripts/record-commands.py\n")
    for name, lines in groups:
        env = dict(os.environ, FERRITE_INTERACT_WATCH=WATCH)
        bot = subprocess.Popen([args.interact_bin, args.game, "--name", args.name], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1, env=env)
        if bot.stdout.readline().strip() != "ready":
            sys.exit("the bot did not join")
        time.sleep(0.8)
        # Every scenario starts from the same state: survival mode, peaceful difficulty.
        r.cmd(f"gamemode survival {args.name}")
        r.cmd("difficulty peaceful")
        r.cmd("time of minecraft:overworld rate 1")
        r.cmd("time of minecraft:overworld resume")
        r.cmd("time of minecraft:the_end rate 1")
        r.cmd("time of minecraft:the_end resume")
        r.cmd("tick rate 20")
        r.cmd("tick step stop")
        r.cmd("tick freeze")
        time.sleep(0.4)
        out.write(f"\nscenario {name}\n")
        # Drain what the join left (until the first mark).
        bot.stdin.write("mark\n"); bot.stdin.flush()
        while bot.stdout.readline().strip() != "mark":
            pass
        for line in lines:
            if line.startswith("rcon "):
                r.cmd(line[5:]); time.sleep(0.3)
                out.write(f"rcon {line[5:]}\n")
                # Its broadcast to the operator is not part of what comes next.
                bot.stdin.write("mark\n"); bot.stdin.flush()
                while bot.stdout.readline().strip() != "mark":
                    pass
                continue
            bot.stdin.write(f"cmd {line}\n"); bot.stdin.flush()
            time.sleep(0.5)
            bot.stdin.write("mark\n"); bot.stdin.flush()
            out.write(f"> {line}\n")
            while True:
                got = bot.stdout.readline()
                if not got:
                    sys.exit("the bot disconnected")
                got = got.rstrip("\n")
                if got == "mark":
                    break
                # (A list-order update arrives some time after joining; it is not about the command.)
                if got.startswith("evt player_info_update 10"):
                    continue
                # (The time sync every 20 ticks carries no clock update; it is not about the command.)
                if got.startswith("evt set_time ") and got.endswith("00") and len(got.split()[-1]) == 18:
                    continue
                # (The game time at the front of a time packet depends on how old the world is.)
                if got.startswith("evt set_time "):
                    got = "evt set_time " + got.split()[2][16:]
                if got.startswith(("sys ", "evt ", "chat ")):
                    out.write(got + "\n")
        bot.stdin.write("quit\n"); bot.stdin.flush()
        bot.wait(timeout=10)
        time.sleep(0.3)
    out.close()


if __name__ == "__main__":
    main()
