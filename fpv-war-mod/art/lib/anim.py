"""Build Bedrock/GeckoLib animation JSON from sampled functions."""
import json, math

class Anim:
    def __init__(self, name, length, loop=True):
        self.name, self.length, self.loop = name, length, loop
        self.bones = {}

    def key(self, bone, channel, t, value):
        """value is in the authoring convention (+x pitch swings a limb forward, +x position is the entity's right).
        Bedrock/GeckoLib animation files mirror x: rotation (-x, -y, z), position (-x, y, z). Verified against Blockbench's animator."""
        if channel == "rotation": value = (-value[0], -value[1], value[2])
        elif channel == "position": value = (-value[0], value[1], value[2])
        self.bones.setdefault(bone, {}).setdefault(channel, {})[round(t, 4)] = [round(v, 3) for v in value]

    def sample(self, bone, channel, fn, n=12):
        """fn(phase 0..1) -> (x, y, z). Samples n+1 keys over the length (loops close seamlessly)."""
        for i in range(n + 1):
            ph = i / n
            self.key(bone, channel, ph * self.length, fn(ph))

    def to_json(self):
        bones = {}
        for b, chans in self.bones.items():
            bones[b] = {c: {str(t): v for t, v in sorted(keys.items())} for c, keys in chans.items()}
        loop = True if self.loop is True else ("hold_on_last_frame" if self.loop == "hold" else False)
        return {"loop": loop, "animation_length": self.length, "bones": bones}

def save(path, prefix, anims):
    data = {"format_version": "1.8.0", "animations": {f"animation.{prefix}.{a.name}": a.to_json() for a in anims}}
    with open(path, "w") as f:
        json.dump(data, f, indent=1)

def sin(ph, amp, phase=0.0, offset=0.0, cycles=1):
    return offset + amp * math.sin(2 * math.pi * (ph * cycles + phase))

def lerp(a, b, t): return a + (b - a) * t
def smooth(t): return t * t * (3 - 2 * t)
