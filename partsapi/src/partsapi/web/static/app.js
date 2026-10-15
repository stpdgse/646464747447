(() => {
  'use strict';
  const $ = (s, r = document) => r.querySelector(s);

  // ---- tiny DOM helper: every vendor string goes in as text, never as HTML ------------------------------------
  function h(tag, attrs, ...kids) {
    const el = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (v === null || v === undefined || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'text') el.textContent = v;
      else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v === true ? '' : v);
    }
    for (const kid of kids.flat()) if (kid !== null && kid !== undefined && kid !== false) el.append(kid.nodeType ? kid : document.createTextNode(String(kid)));
    return el;
  }

  // ---- state -------------------------------------------------------------------------------------------------
  const store = {
    get(k, d) { try { const v = localStorage.getItem('pa_' + k); return v === null ? d : JSON.parse(v); } catch { return d; } },
    set(k, v) { try { localStorage.setItem('pa_' + k, JSON.stringify(v)); } catch { /* private mode */ } },
  };
  let KEY = ''; try { KEY = sessionStorage.getItem('pa_key') || ''; } catch { /* ignore */ }
  let vendors = [];
  let selected = new Set(store.get('vendors', []));
  let generation = 0;

  // ---- API ---------------------------------------------------------------------------------------------------
  class ApiProblem extends Error { constructor(msg, kind, status) { super(msg); this.kind = kind; this.status = status; } }
  async function api(path, params, raw) {
    const url = '/api/' + path + '?' + new URLSearchParams(Object.entries(params || {}).filter(([, v]) => v !== '' && v !== null && v !== undefined));
    let resp;
    try { resp = await fetch(url, { headers: { 'X-Access-Key': KEY } }); }
    catch { throw new ApiProblem('could not reach the server', 'network', 0); }
    if (resp.status === 401) { showLogin(KEY ? 'Wrong access key' : ''); throw new ApiProblem('access key required', 'auth', 401); }
    if (raw && resp.ok) return resp;
    let body = null; try { body = await resp.json(); } catch { /* not json */ }
    if (!resp.ok) throw new ApiProblem((body && body.error) || ('server error ' + resp.status), (body && body.kind) || 'server', resp.status);
    return body;
  }

  // ---- formatting --------------------------------------------------------------------------------------------
  function money(v, cur) {
    if (v === null || v === undefined) return '—';
    try { return new Intl.NumberFormat(undefined, { style: 'currency', currency: cur || 'USD', minimumFractionDigits: 2, maximumFractionDigits: 5 }).format(v); }
    catch { return v + (cur ? ' ' + cur : ''); }
  }
  const taxNote = (incl, vat) => incl === true ? ('incl. VAT' + (vat ? ' ' + Math.round(vat * 100) + '%' : '')) : (incl === false ? 'net' : '');
  function stockText(r) {
    if (r.in_stock === null || r.in_stock === undefined) return '—';
    if (!r.in_stock) return 'Out of stock';
    return r.stock_qty !== null && r.stock_qty !== undefined ? r.stock_qty.toLocaleString() + ' in stock' : 'In stock';
  }
  function toast(msg) {
    const t = $('#toast'); t.textContent = msg; t.hidden = false;
    clearTimeout(toast.t); toast.t = setTimeout(() => { t.hidden = true; }, 3500);
  }
  const vname = (id) => (vendors.find((v) => v.id === id) || { name: id }).name;

  // ---- login -------------------------------------------------------------------------------------------------
  function showLogin(error) {
    $('#login').hidden = false; $('#login-error').textContent = error || ''; $('#key').value = ''; $('#key').focus();
  }
  $('#login-form').addEventListener('submit', async (e) => {
    e.preventDefault(); KEY = $('#key').value;
    try { sessionStorage.setItem('pa_key', KEY); } catch { /* ignore */ }
    $('#login').hidden = true; await loadVendors();
  });
  $('#lock').addEventListener('click', () => { KEY = ''; try { sessionStorage.removeItem('pa_key'); } catch { /* ignore */ } $('#results').replaceChildren(); showLogin(''); });

  // ---- vendors -----------------------------------------------------------------------------------------------
  async function loadVendors() {
    try {
      vendors = (await api('vendors')).vendors;
    } catch (e) { if (e.kind !== 'auth') toast(e.message); return; }
    const health = await fetch('/api/health').then((r) => r.json()).catch(() => ({}));
    $('#lock').hidden = !health.password_required;
    if (!selected.size) selected = new Set(vendors.filter((v) => v.configured && !['adafruit', 'sparkfun'].includes(v.id)).map((v) => v.id));
    selected = new Set([...selected].filter((id) => vendors.some((v) => v.id === id && v.configured)));
    renderVendors();
  }
  function renderVendors() {
    $('#vendors').replaceChildren(...vendors.map((v) => h('button', {
      type: 'button', class: 'vendor' + (v.configured ? '' : ' nokey'), 'aria-pressed': String(selected.has(v.id)), title: v.configured ? v.note : 'Needs an API key on the server. ' + v.note,
      onclick: () => {
        if (!v.configured) { toast(v.name + ' needs an API key on the server'); return; }
        selected.has(v.id) ? selected.delete(v.id) : selected.add(v.id); store.set('vendors', [...selected]); renderVendors();
      },
    }, h('span', { class: 'dot' }), v.name)));
  }

  // ---- options toggle (phones) ---------------------------------------------------------------------------------
  const mobile = window.matchMedia('(max-width:720px)');
  function applyCollapse() { $('#options').classList.toggle('collapsed', mobile.matches && $('#opt-toggle').getAttribute('aria-expanded') !== 'true'); }
  $('#opt-toggle').addEventListener('click', () => { const t = $('#opt-toggle'); const open = t.getAttribute('aria-expanded') !== 'true'; t.setAttribute('aria-expanded', String(open)); t.textContent = open ? 'Options ▴' : 'Options ▾'; applyCollapse(); });
  mobile.addEventListener('change', applyCollapse); applyCollapse();

  // ---- search ------------------------------------------------------------------------------------------------
  function whereExpr() {
    const bits = [];
    if ($('#instock').checked) bits.push('is:instock');
    const mp = $('#maxprice').value.trim(); if (mp) bits.push('price:<=' + mp);
    const w = $('#where').value.trim(); if (w) bits.push(w);
    return bits.join(' ');
  }
  function updateEffective() {
    const q = $('#q').value.trim(), w = whereExpr();
    $('#effective').textContent = (q || w) ? 'Searching: ' + [q, w].filter(Boolean).join('  ') + (+$('#qty').value > 1 ? '   (prices at ' + $('#qty').value + ' pcs)' : '') : '';
  }
  for (const id of ['q', 'where', 'qty', 'maxprice', 'instock']) $('#' + id).addEventListener('input', updateEffective);

  function rowEl(r) {
    const thumb = r.image ? h('img', { class: 'thumb', src: r.image, alt: '', loading: 'lazy', referrerpolicy: 'no-referrer',
      onerror: (e) => e.target.replaceWith(h('div', { class: 'thumb none', text: 'no image' })) }) : h('div', { class: 'thumb none', text: 'no image' });
    const badges = h('div', { class: 'badges' },
      r.library ? h('span', { class: 'badge ' + (r.library === 'basic' ? 'good' : 'warn'), text: r.library }) : null,
      r.preferred ? h('span', { class: 'badge accent', text: 'preferred' }) : null,
      r.rohs ? h('span', { class: 'badge', text: 'RoHS' }) : null,
      r.package ? h('span', { class: 'badge', text: r.package }) : null,
      r.discontinued ? h('span', { class: 'badge bad', text: 'discontinued' }) : null);
    const el = h('div', { class: 'row', tabindex: '0', role: 'button', 'aria-label': 'Open ' + (r.mpn || r.code) },
      thumb,
      h('div', null,
        h('div', { class: 'title', text: r.description || r.mpn || r.code }),
        h('div', { class: 'sub' }, h('span', { class: 'mono', text: r.code }), r.mpn && r.mpn !== r.code ? ' · ' : '', r.mpn && r.mpn !== r.code ? h('span', { class: 'mono', text: r.mpn }) : '',
          r.manufacturer ? ' · ' + r.manufacturer : '', r.category ? ' · ' + r.category : ''), badges),
      h('div', { class: 'price' }, r.has_price ? [h('span', { class: 'big', text: money(r.price, r.currency) }),
        h('span', { class: 'small', text: [r.price_qty > 1 ? 'each at ' + r.price_qty : 'each', taxNote(r.incl_vat, r.vat_rate)].filter(Boolean).join(' · ') })] : h('span', { class: 'sub', text: 'no price' })),
      h('div', { class: 'stock' }, h('span', { class: 'badge ' + (r.in_stock ? 'good' : r.in_stock === false ? 'bad' : ''), text: stockText(r) }),
        r.delivery ? h('span', { class: 'small sub', text: r.delivery }) : null));
    const open = () => openDetail(r.vendor, r.code);
    el.addEventListener('click', open); el.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); open(); } });
    return el;
  }

  async function runVendor(v, params, gen, qs) {
    const meta = h('span', { class: 'meta', text: 'searching…' });
    const spin = h('span', { class: 'spinner', 'aria-label': 'loading' });
    const facetBtn = h('button', { type: 'button', class: 'btn', text: 'Filters', hidden: true, onclick: () => openFacets(v.id, qs) });
    const body = h('div');
    const block = h('section', { class: 'block' }, h('header', null, h('h2', { text: v.name }), meta, spin, facetBtn), body);
    $('#results').append(block);
    try {
      const data = await api('search', { ...params, vendor: v.id });
      if (gen !== generation) return;
      spin.remove(); facetBtn.hidden = false;
      meta.textContent = data.count + (data.count === 1 ? ' result' : ' results') + ' · ' + (data.ms / 1000).toFixed(1) + ' s';
      body.replaceChildren(...(data.parts.length ? data.parts.map(rowEl) : [h('div', { class: 'empty', text: 'Nothing matched. Loosen a filter, or check Filters to see which values exist.' })]));
    } catch (e) {
      if (gen !== generation) return;
      spin.remove(); meta.textContent = '';
      const soft = ['filter', 'credentials', 'notfound'].includes(e.kind);
      body.replaceChildren(h('div', { class: 'err' + (soft ? ' soft' : ''), text: e.message }));
    }
  }

  $('#search-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const q = $('#q').value.trim(), where = whereExpr();
    if (!q && !where) { toast('Type something to search for'); return; }
    if (!selected.size) { toast('Pick at least one vendor'); return; }
    store.set('opts', { qty: $('#qty').value, sort: $('#sort').value, limit: $('#limit').value, desc: $('#desc').checked });
    const gen = ++generation;
    $('#results').replaceChildren();
    if (mobile.matches) setTimeout(() => $('#results').scrollIntoView({ behavior: 'smooth', block: 'start' }), 50);
    const params = { q, where, qty: $('#qty').value || 1, sort: $('#sort').value, reverse: $('#desc').checked ? '1' : '', limit: $('#limit').value };
    const btn = $('#go'); btn.disabled = true; btn.textContent = 'Searching…';
    await Promise.all(vendors.filter((v) => selected.has(v.id)).map((v) => runVendor(v, params, gen, { q, where })));
    if (gen === generation) { btn.disabled = false; btn.textContent = 'Search'; }
  });

  // ---- dialogs -----------------------------------------------------------------------------------------------
  function openDialog(...content) { $('#dlg-body').replaceChildren(...content); $('#overlay').hidden = false; $('.dialog', $('#overlay')).scrollTop = 0; $('#dlg-close').focus(); }
  function closeDialog() { $('#overlay').hidden = true; }
  $('#dlg-close').addEventListener('click', closeDialog);
  $('#overlay').addEventListener('click', (e) => { if (e.target === $('#overlay')) closeDialog(); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') closeDialog(); });

  async function downloadModel(vendor, code, fmt, btn) {
    const label = btn.textContent; btn.textContent = 'Fetching…'; btn.disabled = true;
    try {
      const resp = await api('model3d', { vendor, code, fmt }, true);
      const src = resp.headers.get('X-Model-Source');
      const blob = await resp.blob();
      const a = h('a', { href: URL.createObjectURL(blob), download: (src || code) + '.' + fmt }); document.body.append(a); a.click(); a.remove();
      toast('3D model from LCSC part ' + (src || '?') + ' (exact part-number match)');
    } catch (e) { toast(e.message); }
    btn.textContent = label; btn.disabled = false;
  }

  async function openDetail(vendor, code) {
    openDialog(h('p', { class: 'hint', text: 'Loading ' + code + '…' }));
    let d;
    try { d = await api('part', { vendor, code, qty: $('#qty').value || 1 }); }
    catch (e) { openDialog(h('h2', { text: code }), h('div', { class: 'err soft', text: e.message })); return; }
    const qty = +($('#qty').value || 1);
    const ladder = d.ladder.length ? h('table', null, h('tr', null, h('th', { text: 'From' }), h('th', { text: 'Each' })),
      ...d.ladder.map((b, i) => h('tr', { class: qty >= b.min && (b.max === null || qty <= b.max) ? 'hit' : '' }, h('td', { text: b.min.toLocaleString() + (b.max ? ' – ' + b.max.toLocaleString() : '+') }),
        h('td', { class: 'num', text: money(b.price, b.currency) })))) : h('p', { class: 'hint', text: 'No price.' });
    const attrs = Object.entries(d.attributes || {});
    const docs = (d.documents || []).filter((x) => x.url);
    const model = !['adafruit', 'sparkfun'].includes(vendor);
    openDialog(
      h('h2', { text: d.description || d.mpn || d.code }),
      h('p', { class: 'sub' }, vname(vendor), ' · ', h('span', { class: 'mono', text: d.code }), d.mpn && d.mpn !== d.code ? ' · ' : '', d.mpn && d.mpn !== d.code ? h('span', { class: 'mono', text: d.mpn }) : '',
        d.manufacturer ? ' · ' + d.manufacturer : '', d.category ? ' · ' + d.category : ''),
      d.images.length ? h('div', { class: 'gallery' }, ...d.images.map((u) => h('img', { src: u, alt: '', loading: 'lazy', referrerpolicy: 'no-referrer', onerror: (e) => e.target.remove() }))) : null,
      h('div', { class: 'actions' },
        d.url ? h('a', { class: 'btn primary', href: d.url, target: '_blank', rel: 'noopener noreferrer', text: 'Open at ' + vname(vendor) }) : null,
        d.datasheet ? h('a', { class: 'btn', href: d.datasheet, target: '_blank', rel: 'noopener noreferrer', text: 'Datasheet' }) : null,
        ...docs.filter((x) => x.url !== d.datasheet).slice(0, 4).map((x) => h('a', { class: 'btn', href: x.url, target: '_blank', rel: 'noopener noreferrer', text: x.kind || x.type || 'Document' })),
        model ? h('button', { class: 'btn', type: 'button', text: '3D STEP', onclick: (e) => downloadModel(vendor, d.code, 'step', e.target) }) : null,
        model ? h('button', { class: 'btn', type: 'button', text: '3D OBJ', onclick: (e) => downloadModel(vendor, d.code, 'obj', e.target) }) : null),
      h('div', { class: 'cols' },
        h('div', null, h('h3', { text: 'Price' }), ladder,
          h('p', { class: 'sub', text: [taxNote(d.incl_vat, d.vat_rate), d.ladder[0] ? d.ladder[0].currency : ''].filter(Boolean).join(' · ') }),
          h('h3', { text: 'Stock' }), h('p', null, h('span', { class: 'badge ' + (d.in_stock ? 'good' : 'bad'), text: stockText(d) }), d.delivery ? ' ' + d.delivery : '')),
        h('div', null, h('h3', { text: 'Attributes' }),
          attrs.length ? h('table', null, ...attrs.map(([k, v]) => h('tr', null, h('th', { text: k }), h('td', { text: v })))) : h('p', { class: 'hint', text: 'This vendor lists no technical attributes for this part.' }))),
      h('details', null, h('summary', { text: 'Raw details' }), h('pre', { text: JSON.stringify(d.extra, null, 2) })));
  }

  async function openFacets(vendor, qs) {
    openDialog(h('p', { class: 'hint', text: 'Loading filters…' }));
    let f;
    try { f = await api('facets', { vendor, q: qs.q, where: qs.where }); }
    catch (e) { openDialog(h('h2', { text: vname(vendor) + ' filters' }), h('div', { class: 'err soft', text: e.message })); return; }
    openDialog(h('h2', { text: vname(vendor) + ' filters' }),
      h('p', { class: 'hint', text: (f.total !== null && f.total !== undefined ? f.total.toLocaleString() + ' matches. ' : '') + 'Click a value to add it as a filter.' }),
      ...f.groups.map((g) => h('div', { class: 'facet-group' }, h('h3', { text: g.title }),
        h('div', { class: 'facet-items' }, ...g.items.map(([label, count, token]) => h('button', {
          type: 'button', class: 'chip' + (token ? '' : ' plain'), disabled: !token,
          onclick: () => { const w = $('#where'); w.value = (w.value.trim() + ' ' + token).trim(); updateEffective(); closeDialog(); toast('Added ' + token + ' — press Search'); },
        }, String(label), count !== null && count !== undefined ? h('small', { text: Number(count).toLocaleString() }) : null))))));
  }

  // ---- compare prices ----------------------------------------------------------------------------------------
  $('#compare-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const q = $('#cq').value.trim(), qty = $('#cqty').value || 1, box = $('#compare-results');
    if (!q) { toast('Enter a part number'); return; }
    if (!selected.size) { toast('Pick vendors on the Search tab first'); return; }
    box.replaceChildren(); const rows = new Map(); const cards = [];
    const list = vendors.filter((v) => selected.has(v.id));
    const table = h('table', null, h('tr', null, ['Vendor', 'Part', 'Unit price', 'Stock', ''].map((t) => h('th', { text: t }))));
    const body = h('tbody'); table.append(body); box.append(h('div', { class: 'block' }, table));
    await Promise.all(list.map(async (v) => {
      const tr = h('tr', null, h('td', { text: v.name }), h('td', { colspan: '4', text: 'looking up…' })); body.append(tr);
      try {
        const data = await api('prices', { vendor: v.id, q, qty });
        rows.set(v.id, { tr, row: data.rows[0], v });
      } catch (err) { tr.replaceChildren(h('td', { text: v.name }), h('td', { colspan: '4', class: 'err', text: err.message })); }
    }));
    // cheapest per currency/tax basis only (never across currencies)
    const best = new Map();
    for (const { row } of rows.values()) if (row && row.unit_price !== null && row.unit_price !== undefined) {
      const k = row.currency + '|' + row.incl_vat; if (!best.has(k) || row.unit_price < best.get(k)) best.set(k, row.unit_price);
    }
    for (const { tr, row, v } of rows.values()) {
      if (!row || row.error) { tr.replaceChildren(h('td', { text: v.name }), h('td', { colspan: '4', class: 'sub', text: (row && row.error) || 'not found' })); continue; }
      const isBest = row.unit_price === best.get(row.currency + '|' + row.incl_vat) && best.size >= 1 && [...rows.values()].filter((x) => x.row && x.row.currency === row.currency && x.row.incl_vat === row.incl_vat).length > 1;
      tr.className = isBest ? 'hit' : '';
      tr.replaceChildren(h('td', { text: v.name }), h('td', null, h('span', { class: 'mono', text: row.code }), row.mpn && row.mpn !== row.code ? h('div', { class: 'sub', text: row.mpn }) : null),
        h('td', { class: 'num' }, money(row.unit_price, row.currency), h('div', { class: 'sub', text: [taxNote(row.incl_vat), isBest ? 'cheapest of same basis' : ''].filter(Boolean).join(' · ') })),
        h('td', { text: row.in_stock === null ? '—' : row.stock_qty !== null && row.stock_qty !== undefined ? row.stock_qty.toLocaleString() : (row.in_stock ? 'in stock' : 'out of stock') }),
        h('td', null, row.url ? h('a', { href: row.url, target: '_blank', rel: 'noopener noreferrer', text: 'open' }) : ''));
    }
    void cards;
  });

  // ---- tabs, examples, restore -------------------------------------------------------------------------------
  for (const b of document.querySelectorAll('.tabs button')) b.addEventListener('click', () => {
    for (const x of document.querySelectorAll('.tabs button')) x.setAttribute('aria-selected', String(x === b));
    for (const v of document.querySelectorAll('.view')) v.hidden = v.id !== 'view-' + b.dataset.tab;
  });
  const EXAMPLES = [
    { q: 'resistor 10k 0603', where: 'lib:basic is:instock', vendors: ['jlc'], sort: 'price' },
    { q: 'lm358', where: 'is:instock', vendors: null, sort: 'price' },
    { q: 'capacitor cap:100n pkg:0603', where: 'cat:="MLCC SMD capacitors" is:instock', vendors: ['tme'], sort: 'price' },
    { q: 'humidity sensor', where: 'cat:"Environmental Sensors"', vendors: ['sparkfun'], sort: '' },
    { q: 'rp2040', where: 'is:instock', vendors: ['adafruit'], sort: 'price' },
  ];
  for (const b of document.querySelectorAll('.chip[data-ex]')) b.addEventListener('click', () => {
    const ex = EXAMPLES[+b.dataset.ex];
    const want = ex.vendors || vendors.filter((v) => v.configured && !['adafruit', 'sparkfun'].includes(v.id)).map((v) => v.id);
    const usable = want.filter((id) => vendors.some((v) => v.id === id && v.configured));
    if (!usable.length) { toast('That example needs ' + vname(want[0]) + ', which has no API key on the server'); return; }
    selected = new Set(usable); renderVendors();
    $('#q').value = ex.q; $('#where').value = ex.where; $('#sort').value = ex.sort; $('#instock').checked = false; $('#maxprice').value = ''; updateEffective();
    $('#search-form').requestSubmit();
  });

  const o = store.get('opts', {});
  if (o.qty) $('#qty').value = o.qty; if (o.sort) $('#sort').value = o.sort; if (o.limit) $('#limit').value = o.limit; $('#desc').checked = !!o.desc;
  loadVendors().then(() => { if (!vendors.length && !KEY) fetch('/api/health').then((r) => r.json()).then((hh) => { if (hh.password_required) showLogin(''); }); });
  // an unauthenticated first load: /api/vendors answers 401, which opens the login dialog by itself
})();
