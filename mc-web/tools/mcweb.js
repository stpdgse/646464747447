// Browser side of the SDL shim: owns the canvas and WebGL2 context and turns DOM events into a queue of
// SDL3-style events that the Java side (org.teavm.mcweb.host.Sdl / org.lwjgl.sdl.SDLEvents) polls.
(function () {
  "use strict";
  const hasDom = typeof document !== "undefined";

  // KeyboardEvent.code -> SDL3 scancode
  const SC = {};
  for (let i = 0; i < 26; i++) SC["Key" + String.fromCharCode(65 + i)] = 4 + i;
  for (let i = 1; i <= 9; i++) SC["Digit" + i] = 29 + i;
  SC.Digit0 = 39;
  Object.assign(SC, {
    Enter: 40, Escape: 41, Backspace: 42, Tab: 43, Space: 44, Minus: 45, Equal: 46, BracketLeft: 47,
    BracketRight: 48, Backslash: 49, Semicolon: 51, Quote: 52, Backquote: 53, Comma: 54, Period: 55,
    Slash: 56, CapsLock: 57, PrintScreen: 70, ScrollLock: 71, Pause: 72, Insert: 73, Home: 74,
    PageUp: 75, Delete: 76, End: 77, PageDown: 78, ArrowRight: 79, ArrowLeft: 80, ArrowDown: 81,
    ArrowUp: 82, NumLock: 83, NumpadDivide: 84, NumpadMultiply: 85, NumpadSubtract: 86, NumpadAdd: 87,
    NumpadEnter: 88, Numpad0: 98, NumpadDecimal: 99, IntlBackslash: 100, ContextMenu: 101,
    NumpadEqual: 103, ControlLeft: 224, ShiftLeft: 225, AltLeft: 226, MetaLeft: 227, ControlRight: 228,
    ShiftRight: 229, AltRight: 230, MetaRight: 231
  });
  for (let i = 1; i <= 12; i++) SC["F" + i] = 57 + i;
  for (let i = 13; i <= 24; i++) SC["F" + i] = 91 + i;
  for (let i = 1; i <= 9; i++) SC["Numpad" + i] = 88 + i;

  const EXT = 0x40000000;
  const FIXED_KEYCODE = { 40: 13, 41: 27, 42: 8, 43: 9, 44: 32, 76: 127 };

  const MOD_BITS = { 225: 0x1, 229: 0x2, 224: 0x40, 228: 0x80, 226: 0x100, 230: 0x200, 227: 0x400, 231: 0x800 };

  const queue = [];
  let cur = { type: 0, ints: [0, 0, 0, 0], floats: [0, 0, 0, 0], str: "" };
  const keys = new Uint8Array(512);
  let modBits = 0;
  let canvas = null;
  let textInput = false;
  let wantRelative = false;
  let wantGrab = false;
  let mx = 0, my = 0, buttons = 0;
  let clipboard = "";
  let frameCallbacks = [];
  let frameRequested = false;

  function ev(type, ints, floats, str) {
    queue.push({ type, ints: ints || [0, 0, 0, 0], floats: floats || [0, 0, 0, 0], str: str || "" });
  }

  function keycodeFor(e, sc) {
    if (FIXED_KEYCODE[sc] !== undefined) return FIXED_KEYCODE[sc];
    const k = e.key;
    if (k && k.length >= 1 && [...k].length === 1 && sc < 224) return [...k.toLowerCase()][0].codePointAt(0);
    return sc | EXT;
  }

  function cssSize() {
    return hasDom ? { w: Math.max(1, Math.floor(window.innerWidth)), h: Math.max(1, Math.floor(window.innerHeight)) }
                  : { w: 854, h: 480 };
  }

  function density() {
    return hasDom ? window.devicePixelRatio || 1 : 1;
  }

  function applySize(emit) {
    if (!canvas) return;
    const s = cssSize();
    const d = density();
    canvas.style.width = s.w + "px";
    canvas.style.height = s.h + "px";
    canvas.width = Math.round(s.w * d);
    canvas.height = Math.round(s.h * d);
    if (emit) {
      ev(0x206, [s.w, s.h, 0, 0]);          // SDL_EVENT_WINDOW_RESIZED
      ev(0x207, [canvas.width, canvas.height, 0, 0]); // SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED
    }
  }

  function setKey(sc, down) {
    keys[sc] = down ? 1 : 0;
    const bit = MOD_BITS[sc];
    if (bit) modBits = down ? (modBits | bit) : (modBits & ~bit);
  }

  function onKey(e, down) {
    if (down && globalThis.mcaudio) globalThis.mcaudio.resume();
    const sc = SC[e.code];
    if (sc === undefined) return;
    const allowDefault = e.code === "F5" || e.code === "F11" || e.code === "F12" ||
      (e.code === "KeyV" && (e.ctrlKey || e.metaKey)) || (e.code === "KeyR" && (e.ctrlKey || e.metaKey));
    if (!allowDefault) e.preventDefault();
    if (down && e.code === "KeyV" && (e.ctrlKey || e.metaKey)) refreshClipboard();
    setKey(sc, down);
    ev(down ? 0x300 : 0x301, [sc, keycodeFor(e, sc), modBits, e.repeat ? 1 : 0]);
    if (down && textInput && e.key && [...e.key].length === 1 && !e.ctrlKey && !e.metaKey) {
      ev(0x303, [0, 0, 0, 0], null, e.key);
    }
  }

  function refreshClipboard() {
    if (hasDom && navigator.clipboard && navigator.clipboard.readText) {
      navigator.clipboard.readText().then((t) => { clipboard = t; }, () => {});
    }
  }

  function requestLockIfWanted() {
    if (hasDom && wantRelative && canvas && document.pointerLockElement !== canvas) {
      try {
        const p = canvas.requestPointerLock({ unadjustedMovement: true });
        if (p && p.catch) p.catch(() => { try { canvas.requestPointerLock(); } catch (_) {} });
      } catch (_) {}
    }
  }

  const api = {
    gl: null,
    canvas: null,

    createWindow(title, width, height) {
      if (hasDom) {
        canvas = document.getElementById("game") || document.body.appendChild(document.createElement("canvas"));
        document.title = title;
        applySize(false);
        const attrs = { antialias: false, alpha: false, depth: true, stencil: true,
                        preserveDrawingBuffer: false, powerPreference: "high-performance", desynchronized: false };
        api.gl = canvas.getContext("webgl2", attrs);
        api.canvas = canvas;
        canvas.tabIndex = 0;
        canvas.focus();
        window.addEventListener("keydown", (e) => onKey(e, true));
        window.addEventListener("keyup", (e) => onKey(e, false));
        canvas.addEventListener("mousemove", (e) => {
          const locked = document.pointerLockElement === canvas;
          if (locked) { mx += e.movementX; my += e.movementY; } else { mx = e.offsetX; my = e.offsetY; }
          const last = queue[queue.length - 1];
          if (last && last.type === 0x400) {
            last.floats[0] = mx; last.floats[1] = my;
            last.floats[2] += e.movementX; last.floats[3] += e.movementY;
          } else {
            ev(0x400, null, [mx, my, e.movementX, e.movementY]);
          }
        });
        canvas.addEventListener("mousedown", (e) => {
          e.preventDefault();
          if (globalThis.mcaudio) globalThis.mcaudio.resume();
          canvas.focus();
          requestLockIfWanted();
          buttons |= 1 << e.button;
          ev(0x401, [e.button + 1, 0, 0, 0], [mx, my, 0, 0]);
        });
        window.addEventListener("mouseup", (e) => {
          if (!(buttons & (1 << e.button))) return;
          buttons &= ~(1 << e.button);
          ev(0x402, [e.button + 1, 0, 0, 0], [mx, my, 0, 0]);
        });
        canvas.addEventListener("contextmenu", (e) => e.preventDefault());
        canvas.addEventListener("wheel", (e) => {
          e.preventDefault();
          const scale = e.deltaMode === 0 ? 100 : e.deltaMode === 1 ? 3 : 1;
          ev(0x403, null, [-e.deltaX / scale, -e.deltaY / scale, 0, 0]);
        }, { passive: false });
        window.addEventListener("resize", () => applySize(true));
        window.addEventListener("blur", () => {
          keys.fill(0); modBits = 0; buttons = 0;
          ev(0x20f, [0, 0, 0, 0]); // SDL_EVENT_WINDOW_FOCUS_LOST
        });
        window.addEventListener("focus", () => ev(0x20e, [0, 0, 0, 0])); // FOCUS_GAINED
        document.addEventListener("fullscreenchange", () => applySize(true));
        document.addEventListener("pointerlockchange", () => {
          if (document.pointerLockElement !== canvas && wantRelative) {
            // Escape released the lock without a key event: give the game the Escape press it would have seen.
            ev(0x300, [41, 27, modBits, 0]);
            ev(0x301, [41, 27, modBits, 0]);
          }
        });
        document.addEventListener("paste", (e) => {
          const t = e.clipboardData && e.clipboardData.getData("text");
          if (t) clipboard = t;
        });
        window.addEventListener("beforeunload", (e) => { /* unsaved worlds live in browser storage */ });
        refreshClipboard();
      }
    },

    setTitle(title) { if (hasDom) document.title = title; },
    width() { return cssSize().w; },
    height() { return cssSize().h; },
    pixelDensity() { return density(); },
    screenWidth() { return hasDom ? screen.width : 1920; },
    screenHeight() { return hasDom ? screen.height : 1080; },
    setSize(w, h) { /* the canvas always fills the page */ },
    setFullscreen(on) {
      if (!hasDom) return;
      if (on && !document.fullscreenElement) {
        const p = document.documentElement.requestFullscreen({ navigationUI: "hide" });
        if (p && p.catch) p.catch(() => {});
      } else if (!on && document.fullscreenElement) {
        document.exitFullscreen().catch(() => {});
      }
    },
    isFullscreen() { return hasDom && !!document.fullscreenElement; },
    setRelativeMouse(on) {
      wantRelative = on;
      if (!hasDom) return;
      if (on) requestLockIfWanted();
      else if (document.pointerLockElement === canvas) document.exitPointerLock();
    },
    setMouseGrab(on) { wantGrab = on; },
    setCursor(c) { if (canvas) canvas.style.cursor = c; },
    setTextInput(on) { textInput = on; },
    openUrl(url) { if (hasDom) window.open(url, "_blank", "noopener"); },
    alert(msg) { if (hasDom) window.alert(msg); else console.log("[alert] " + msg); },

    poll() {
      const e = queue.shift();
      if (!e) return false;
      cur = e;
      return true;
    },
    evType() { return cur.type; },
    evInt(k) { return cur.ints[k] | 0; },
    evFloat(k) { return cur.floats[k]; },
    evString() { return cur.str; },
    flushEvents() {
      for (let i = queue.length - 1; i >= 0; i--) {
        if (queue[i].type >= 0x300 && queue[i].type <= 0x1313) queue.splice(i, 1);
      }
    },
    keyDown(sc) { return sc >= 0 && sc < 512 && keys[sc] === 1; },
    mods() { return modBits; },
    mouseX() { return mx; },
    mouseY() { return my; },
    mouseButtons() { return buttons; },

    requestFrame(cb) {
      frameCallbacks.push(cb);
      if (frameRequested) return;
      frameRequested = true;
      const run = () => {
        frameRequested = false;
        const cbs = frameCallbacks;
        frameCallbacks = [];
        for (const f of cbs) f();
      };
      if (hasDom && typeof requestAnimationFrame === "function") requestAnimationFrame(run);
      else setTimeout(run, 16);
    },

    setClipboard(text) {
      clipboard = text;
      if (hasDom && navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).catch(() => {});
    },
    getClipboard() { return clipboard; }
  };

  globalThis.mcweb = api;
})();
