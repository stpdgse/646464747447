#!/usr/bin/env python3
"""Build the rifleman: bake the low-ready rest pose into the model, save .bbmodel + texture, generate animations."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import rifleman as R
import rifleman_pose as RP

OUT = os.path.join(os.path.dirname(__file__), "out")
REST = RP.solve(**RP.LOW_READY)

def build():
    for b, v in REST.items():
        if "rot" in v and b in R.m.bones: R.m.bones[b].rotation = [round(x, 3) for x in v["rot"]]
    R.m.save(OUT)
    try:
        import rifleman_anim as RA
        RA.build(REST, OUT)
    except ImportError:
        pass

if __name__ == "__main__":
    build()
    print("rifleman built")
