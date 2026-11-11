import sys,time; sys.path.insert(0,'/tmp/claude-0/-home-user-646464747447/8a0e81a2-3f0d-5a29-873d-2f03db265420/scratchpad')
from mc import *
r=Rcon()
script=sys.argv[1]; Z=int(sys.argv[2]) if len(sys.argv)>2 else 8
r.cmd("fill 0 100 0 15 104 15 air"); r.cmd("tick step 10"); time.sleep(0.4)
tag=0
def snap():
    global tag; tag+=1
    r.cmd("setblock 0 200 0 stone" if tag%2 else "setblock 0 200 0 air"); r.cmd("save-all flush")
    c=read_chunk(W,0,0); b=blocks(c)
    row=[]
    for x in range(3,15):
        v=b.get((x,100,Z),"."); row.append(f"{x}:{v}")
    print("   "+" | ".join(row))
    for be in c.get("block_entities",[]):
        if 100<=be["y"]<=104: print("   BE",{k:v for k,v in be.items() if k not in("components",)})
    print("   bticks",[(t["x"],t["y"],t["z"],t["i"][10:],t["t"]) for t in c["block_ticks"] if 100<=t["y"]<=104])
for cmd in script.split(";"):
    c=cmd.strip().split(" ")
    if c[0]=="set": print("> "+cmd, "->", r.cmd(f"setblock {c[1]} {c[2]} {c[3]} {c[4]}"))
    elif c[0]=="step": r.cmd(f"tick step {c[1]}"); time.sleep(0.04*int(c[1])+0.15); print("> "+cmd); snap()
    elif c[0]=="cmd": print("> "+cmd, "->", r.cmd(" ".join(c[1:])))
