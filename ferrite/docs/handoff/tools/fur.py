import sys,time,re
sys.path.insert(0,'/tmp/claude-0'); from rc import Rcon
r=Rcon()
def gt():
    return int(re.search(r"(\d+) tick", r.cmd("time query gametime")).group(1))
def step(n):
    t=gt()+n; r.cmd(f"tick step {n}")
    end=time.time()+120
    while time.time()<end:
        time.sleep(0.1); g=gt()
        if g>=t:
            break
    assert g==t, (g,t)
def nbt(pos):
    s=r.cmd(f"data get block {pos}")
    g=lambda k: re.search(k+r": ([-\d.]+)",s).group(1)
    items=re.findall(r"\{count: (\d+), Slot: (\d)b, id: \"minecraft:(\w+)\"\}",s)
    items=sorted((int(sl),n,int(c)) for c,sl,n in items)
    return (int(g("lit_time_remaining")),int(g("lit_total_time")),int(g("cooking_time_spent")),int(g("cooking_total_time")),items)
