// OpenAL-style audio on Web Audio. The Java side (org.lwjgl.openal.*) drives this through
// org.teavm.mcweb.host.WebAudio; sources keep an AL buffer queue and schedule AudioBufferSourceNodes.
(function () {
  "use strict";
  const INITIAL = 0x1011, PLAYING = 0x1012, PAUSED = 0x1013, STOPPED = 0x1014;
  let ctx = null;
  let nextId = 1;
  const buffers = new Map();
  const sources = new Map();
  let unqueued = [];
  const listenerPos = [0, 0, 0];

  class Source {
    constructor(id) {
      this.id = id;
      this.items = [];            // {id, buf, node, start, end}
      this.state = INITIAL;
      this.gain = 1; this.pitch = 1; this.loop = false; this.relative = false;
      this.pos = [0, 0, 0]; this.vel = [0, 0, 0]; this.dir = [0, 0, 0];
      this.ref = 1; this.max = 100000; this.rolloff = 1; this.model = 0xD004;
      this.pauseItem = 0; this.pauseOffset = 0;
      this.gainNode = ctx.createGain();
      this.gainNode.connect(ctx.destination);
      this.panner = ctx.createPanner();
      this.panner.panningModel = "equalpower";
      this.panner.connect(this.gainNode);
      this.applyPanner();
    }
    applyPanner() {
      const p = this.panner;
      p.distanceModel = this.model === 0xD004 || this.model === 0xD003 ? "linear"
        : this.model === 0xD001 || this.model === 0xD002 ? "inverse" : "linear";
      p.refDistance = this.ref;
      p.maxDistance = this.max;
      p.rolloffFactor = this.model === 0 ? 0 : this.rolloff;
      setPos(p, this.pos);
      this.gainNode.gain.value = this.gain;
    }
    target(buf) {
      return buf.numberOfChannels === 1 && !this.relative ? this.panner : this.gainNode;
    }
    stopNodes() {
      for (const it of this.items) {
        if (it.node) {
          try { it.node.stop(); } catch (_) {}
          try { it.node.disconnect(); } catch (_) {}
          it.node = null;
        }
      }
    }
    schedule(from, offset) {
      const now = ctx.currentTime;
      let t = now + 0.005;
      if (from > 0 && this.items[from - 1].end > t) t = this.items[from - 1].end;
      for (let i = from; i < this.items.length; i++) {
        const it = this.items[i];
        const node = ctx.createBufferSource();
        node.buffer = it.buf;
        node.playbackRate.value = this.pitch;
        node.loop = this.loop && this.items.length === 1;
        node.connect(this.target(it.buf));
        node.start(t, offset);
        it.node = node;
        it.start = t;
        const dur = (it.buf.duration - offset) / this.pitch;
        it.end = node.loop ? Infinity : t + dur;
        t += dur;
        offset = 0;
      }
    }
    refresh() {
      if (this.state === PLAYING && this.items.length && !this.loop) {
        const now = ctx.currentTime;
        if (this.items.every((it) => it.end <= now)) this.state = STOPPED;
      }
    }
    processed() {
      this.refresh();
      if (this.state === INITIAL) return 0;
      if (this.state === STOPPED) return this.items.length;
      if (this.loop && this.items.length === 1) return 0;
      const now = ctx.currentTime;
      let n = 0;
      for (const it of this.items) { if (it.end <= now) n++; else break; }
      if (this.state === PAUSED) return this.pauseItem;
      return n;
    }
  }

  function setPos(p, v) {
    if (p.positionX) { p.positionX.value = v[0]; p.positionY.value = v[1]; p.positionZ.value = v[2]; }
    else p.setPosition(v[0], v[1], v[2]);
  }

  const api = {
    open() {
      if (ctx) return true;
      const AC = globalThis.AudioContext || globalThis.webkitAudioContext;
      if (!AC) return false;
      ctx = new AC({ latencyHint: "interactive" });
      return true;
    },
    resume() { if (ctx && ctx.state === "suspended") ctx.resume().catch(() => {}); },
    close() { /* keep the context: the game may reopen the device */ },
    sampleRate() { return ctx ? ctx.sampleRate : 48000; },
    genBuffer() { const id = nextId++; buffers.set(id, null); return id; },
    deleteBuffer(id) { buffers.delete(id); },
    bufferData(id, format, bytes, freq) {
      const ch = format === 0x1100 || format === 0x1101 ? 1 : 2;
      const sixteen = format === 0x1101 || format === 0x1103;
      const frames = Math.floor(bytes.byteLength / (ch * (sixteen ? 2 : 1)));
      const buf = ctx.createBuffer(ch, Math.max(1, frames), Math.min(Math.max(freq, 8000), 96000));
      const data = sixteen
        ? new Int16Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + frames * ch * 2))
        : new Uint8Array(bytes.buffer, bytes.byteOffset, frames * ch);
      for (let c = 0; c < ch; c++) {
        const out = buf.getChannelData(c);
        if (sixteen) for (let i = 0; i < frames; i++) out[i] = data[i * ch + c] / 32768;
        else for (let i = 0; i < frames; i++) out[i] = (data[i * ch + c] - 128) / 128;
      }
      buffers.set(id, buf);
    },
    genSource() { const id = nextId++; sources.set(id, new Source(id)); return id; },
    deleteSource(id) {
      const s = sources.get(id);
      if (s) { s.stopNodes(); try { s.gainNode.disconnect(); s.panner.disconnect(); } catch (_) {} }
      sources.delete(id);
    },
    sourcei(id, param, v) {
      const s = sources.get(id); if (!s) return;
      switch (param) {
        case 0x1009: // AL_BUFFER
          s.stopNodes();
          s.items = v ? [{ id: v, buf: buffers.get(v), node: null, start: 0, end: 0 }] : [];
          if (s.state !== INITIAL) s.state = STOPPED;
          break;
        case 0x1007: s.loop = v !== 0; break;               // AL_LOOPING
        case 0x202: s.relative = v !== 0; break;            // AL_SOURCE_RELATIVE
        case 0x200: s.model = v; s.applyPanner(); break;    // AL_SOURCE_DISTANCE_MODEL
        default: break;
      }
    },
    sourcef(id, param, v) {
      const s = sources.get(id); if (!s) return;
      switch (param) {
        case 0x1003: s.pitch = v; for (const it of s.items) if (it.node) it.node.playbackRate.value = v; break;
        case 0x100A: s.gain = v; s.gainNode.gain.value = v; break;
        case 0x1020: s.ref = v; s.applyPanner(); break;
        case 0x1021: s.rolloff = v; s.applyPanner(); break;
        case 0x1023: s.max = v; s.applyPanner(); break;
        default: break;
      }
    },
    sourcefv(id, param, x, y, z) {
      const s = sources.get(id); if (!s) return;
      if (param === 0x1004) { s.pos = [x, y, z]; setPos(s.panner, s.pos); }
      else if (param === 0x1006) s.vel = [x, y, z];
      else if (param === 0x1005) s.dir = [x, y, z];
    },
    getSourcei(id, param) {
      const s = sources.get(id); if (!s) return 0;
      switch (param) {
        case 0x1010: s.refresh(); return s.state;             // AL_SOURCE_STATE
        case 0x1015: return s.items.length;                   // AL_BUFFERS_QUEUED
        case 0x1016: return s.processed();                    // AL_BUFFERS_PROCESSED
        case 0x1009: return s.items.length ? s.items[0].id : 0;
        case 0x1007: return s.loop ? 1 : 0;
        case 0x202: return s.relative ? 1 : 0;
        default: return 0;
      }
    },
    play(id) {
      const s = sources.get(id); if (!s) return;
      api.resume();
      s.refresh();
      if (s.state === PLAYING) return;
      if (s.state === PAUSED) {
        s.stopNodes();
        s.state = PLAYING;
        s.schedule(s.pauseItem, s.pauseOffset);
        return;
      }
      if (!s.items.length) return;
      s.stopNodes();
      for (const it of s.items) it.end = 0;
      s.state = PLAYING;
      s.schedule(0, 0);
    },
    pause(id) {
      const s = sources.get(id); if (!s) return;
      s.refresh();
      if (s.state !== PLAYING) return;
      const now = ctx.currentTime;
      let idx = s.items.findIndex((it) => it.end > now);
      if (idx < 0) idx = s.items.length;
      s.pauseItem = idx;
      s.pauseOffset = idx < s.items.length ? Math.max(0, (now - s.items[idx].start) * s.pitch) % (s.items[idx].buf.duration || 1) : 0;
      s.stopNodes();
      s.state = PAUSED;
    },
    stop(id) {
      const s = sources.get(id); if (!s) return;
      s.stopNodes();
      if (s.state !== INITIAL) s.state = STOPPED;
    },
    queue(id, bufId) {
      const s = sources.get(id); if (!s) return;
      s.items.push({ id: bufId, buf: buffers.get(bufId), node: null, start: 0, end: 0 });
      if (s.state === PLAYING) s.schedule(s.items.length - 1, 0);
    },
    unqueue(id, count) {
      const s = sources.get(id);
      unqueued = [];
      if (!s) return 0;
      const n = Math.min(count, s.processed());
      for (let i = 0; i < n; i++) {
        const it = s.items.shift();
        unqueued.push(it.id);
      }
      if (s.state === PAUSED) s.pauseItem = Math.max(0, s.pauseItem - n);
      return n;
    },
    unqueuedId(i) { return unqueued[i] | 0; },
    listenerPos(x, y, z) {
      if (!ctx) return;
      const l = ctx.listener;
      listenerPos[0] = x; listenerPos[1] = y; listenerPos[2] = z;
      if (l.positionX) { l.positionX.value = x; l.positionY.value = y; l.positionZ.value = z; }
      else l.setPosition(x, y, z);
    },
    listenerOrientation(fx, fy, fz, ux, uy, uz) {
      if (!ctx) return;
      const l = ctx.listener;
      if (l.forwardX) {
        l.forwardX.value = fx; l.forwardY.value = fy; l.forwardZ.value = fz;
        l.upX.value = ux; l.upY.value = uy; l.upZ.value = uz;
      } else l.setOrientation(fx, fy, fz, ux, uy, uz);
    }
  };
  globalThis.mcaudio = api;
})();
