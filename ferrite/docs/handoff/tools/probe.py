import sys,time; sys.path.insert(0,'/tmp/claude-0/-home-user-646464747447/8a0e81a2-3f0d-5a29-873d-2f03db265420/scratchpad')
from mc import *
r=Rcon()
script=sys.argv[1]
r.cmd("fill 0 100 0 15 104 15 air"); r.cmd("tick step 10"); time.sleep(0.3)
tag=0
def snap():
    global tag; tag+=1
    r.cmd("setblock 0 200 0 stone" if tag%2 else "setblock 0 200 0 air"); r.cmd("save-all flush")
    b=blocks(read_chunk(W,0,0))
    for (x,y,z),v in sorted(b.items(), key=lambda kv:(kv[0][1],kv[0][2],kv[0][0])):
        if y>=100 and y<103:
            if v.startswith("redstone_wire["):
                d=dict(kv.split("=") for kv in v[v.index("[")+1:-1].split(","))
                print(f"   ({x},{y},{z}) e={d['east']} n={d['north']} p={d['power']} s={d['south']} w={d['west']}")
            else: print(f"   ({x},{y},{z}) {v}")
for cmd in script.split(";"):
    c=cmd.strip().split(" ")
    if c[0]=="set": r.cmd(f"setblock {c[1]} {c[2]} {c[3]} {c[4]}")
    elif c[0]=="step": r.cmd(f"tick step {c[1]}"); time.sleep(0.05*int(c[1])+0.2)
    print("> "+cmd); snap()
