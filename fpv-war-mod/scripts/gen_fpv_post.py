#!/usr/bin/env python3
"""Generate the analog-FPV post-effect variants: one JSON per (signal level, noise phase).

Post-effect uniforms are fixed when the chain loads, and shaders get no clock, so the client animates the noise by
switching between PHASES variants every tick and picks the level from the radio link quality.
Levels match com.kamikazeskies.sim.SignalLevel: CLEAN, LIGHT_NOISE, NOISY, BREAKUP, HEAVY_BREAKUP, LOST.
"""
import json, os

PHASES = 8
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "main", "resources", "assets", "kamikaze_skies", "post_effect")

#            Noise  Scan   Bleed   Bar   Glitch Snow
LEVELS = [
    (0.035, 0.10, 0.0006, 0.00, 0.00, 0.00),   # clean: still a little analog
    (0.080, 0.12, 0.0012, 0.06, 0.00, 0.00),   # light noise
    (0.160, 0.16, 0.0025, 0.18, 0.06, 0.00),   # noisy
    (0.280, 0.20, 0.0050, 0.35, 0.22, 0.04),   # break-up
    (0.450, 0.24, 0.0090, 0.55, 0.45, 0.30),   # heavy break-up
    (1.000, 0.20, 0.0000, 0.00, 0.00, 1.00),   # lost: static
]

def variant(level, phase):
    n, scan, bleed, bar, glitch, snow = LEVELS[level]
    seed = 1.0 + phase * 7.31 + level * 0.37
    cfg = [("Noise", n), ("Scan", scan), ("Bleed", bleed), ("Bar", bar), ("Glitch", glitch), ("Snow", snow), ("Seed", seed)]
    return {
        "targets": {"temp": {}},
        "passes": [
            {
                "vertex_shader": "minecraft:core/screenquad",
                "fragment_shader": "kamikaze_skies:post/fpv_analog",
                "inputs": [{"sampler_name": "In", "target": "minecraft:main"}],
                "output": "temp",
                "uniforms": {"AnalogConfig": [{"name": k, "type": "float", "value": round(v, 5)} for k, v in cfg]},
            },
            {
                "vertex_shader": "minecraft:core/screenquad",
                "fragment_shader": "minecraft:post/blit",
                "inputs": [{"sampler_name": "In", "target": "temp"}],
                "output": "minecraft:main",
                "uniforms": {"BlitConfig": [{"name": "ColorModulate", "type": "vec4", "value": [1.0, 1.0, 1.0, 1.0]}]},
            },
        ],
    }

os.makedirs(OUT, exist_ok=True)
for lv in range(len(LEVELS)):
    for ph in range(PHASES):
        with open(os.path.join(OUT, f"fpv_{lv}_{ph}.json"), "w") as f:
            json.dump(variant(lv, ph), f, indent=2)
print(f"wrote {len(LEVELS) * PHASES} post effects to {os.path.normpath(OUT)}")
