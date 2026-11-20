# Headless Blockbench pipeline

Blockbench 5.2.2 (Electron, built from source) runs on a virtual display (Xvfb) with software WebGL (SwiftShader).
It is driven over Chromium's DevTools port, so models can be generated, loaded, screenshotted and exported with no GUI.

| File | Purpose |
|---|---|
| `bbmodel.py` | Python writer for `.bbmodel` (groups, cubes, per-face UV, embedded PNG textures) |
| `gen_drone.py` | Example: generates a 5" FPV quadcopter (`out/fpv_quad.bbmodel`) |
| `run-blockbench.sh` | Starts Blockbench headless with debug port 9222 |
| `bb-driver.js` | Opens a `.bbmodel`, screenshots it, exports via Blockbench's own codecs (project, java_block, bedrock, obj) |

## Setup (one time)
```bash
git clone --depth 1 https://github.com/JannisX11/blockbench /home/user/jannisx11/blockbench
cd /home/user/jannisx11/blockbench && npm install && node node_modules/electron/install.js && npm run build-electron
apt-get install -y libgtk-3-0 libnss3 libasound2t64 libgbm1 libxss1 libxtst6 libatk-bridge2.0-0 xvfb
```

## Use
```bash
./run-blockbench.sh &                         # wait ~20s
python3 gen_drone.py                          # write out/fpv_quad.bbmodel
node bb-driver.js out/fpv_quad.bbmodel out/result   # screenshot.png + export_*.txt + result.json
```
