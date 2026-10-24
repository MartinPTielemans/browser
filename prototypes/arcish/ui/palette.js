const $ = (id) => document.getElementById(id);
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
const input = $('q');
const results = $('results');

let D = null; // data sent by main when the palette opens
let items = [];
let sel = 0;

const host = (url) => {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
};

const looksLikeURL = (q) =>
  /^[a-z][a-z0-9+.-]*:\/\//i.test(q) || /^localhost(:\d+)?(\/.*)?$/i.test(q) || /^[^\s]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(q);

const matches = (q, ...fields) => {
  const hay = fields.join(' ').toLowerCase();
  return q.toLowerCase().split(/\s+/).filter(Boolean).every((w) => hay.includes(w));
};

function commands() {
  const s = D.settings;
  const archive = s.archiveAfterMs;
  const tab = D.tabs.find((t) => t.id === D.activeTabId);
  const list = [
    { label: 'New Space…', kind: 'mode', value: 'space', glyph: '+', keys: 'create add space' },
    ...D.spaces.filter((sp) => sp.id !== D.activeSpaceId).map((sp) => ({ label: `Switch to ${sp.name}`, kind: 'space', value: sp.id, glyph: '●', color: sp.color, keys: 'space go' })),
    { label: 'Split View with…', kind: 'prefix', value: 'split ', glyph: '◫', keys: 'split side by side', hide: !tab },
    { label: 'Close Split View', kind: 'command', value: 'unsplit', glyph: '▢', keys: 'split exit unsplit', hide: !D.split },
    { label: tab?.pinned ? 'Unpin Tab' : 'Pin Tab', kind: 'command', value: 'pin-tab', glyph: '⌖', keys: 'pin', hide: !tab || tab.favorite },
    { label: 'Copy URL', kind: 'command', value: 'copy-url', glyph: '⧉', keys: 'copy link url', hide: !tab },
    { label: 'Toggle Sidebar', kind: 'command', value: 'toggle-sidebar', glyph: '⇤', keys: 'sidebar hide show' },
    { label: 'Reopen Closed Tab', kind: 'command', value: 'reopen', glyph: '↺', keys: 'undo reopen restore' },
    { label: 'Clear Unpinned Tabs in This Space', kind: 'command', value: 'archive-idle-now', glyph: '↓', keys: 'archive clear clean' },
    { label: 'Put Background Tabs to Sleep', kind: 'command', value: 'sleep-tabs', glyph: '☾', keys: 'sleep discard memory free' },
    { label: s.adblock ? 'Turn Ad Blocking Off' : 'Turn Ad Blocking On', kind: 'command', value: 'toggle-adblock', glyph: '⛨', keys: 'ads tracker block adblock' },
    { label: 'Auto-Archive After 5 Minutes (demo)', kind: 'command', value: 'archive-5m', glyph: '⏱', keys: 'archive auto setting', on: archive === 5 * 60e3 },
    { label: 'Auto-Archive After 1 Hour', kind: 'command', value: 'archive-1h', glyph: '⏱', keys: 'archive auto setting', on: archive === 3600e3 },
    { label: 'Auto-Archive After 12 Hours', kind: 'command', value: 'archive-12h', glyph: '⏱', keys: 'archive auto setting', on: archive === 12 * 3600e3 },
    { label: 'Auto-Archive After 24 Hours', kind: 'command', value: 'archive-24h', glyph: '⏱', keys: 'archive auto setting', on: archive === 24 * 3600e3 },
    { label: 'Never Auto-Archive', kind: 'command', value: 'archive-off', glyph: '⏱', keys: 'archive auto setting off', on: !archive },
    { label: 'Developer Tools', kind: 'command', value: 'devtools', glyph: '⌘', keys: 'inspect devtools debug', hide: !tab },
  ];
  return list.filter((c) => !c.hide).map((c) => ({ ...c, badge: c.on ? 'current' : 'command' }));
}

function spaceName(id) {
  return D.spaces.find((s) => s.id === id)?.name ?? 'Favorites';
}

function build(q) {
  q = q.trim();
  if (D.mode === 'space') {
    return [{ kind: 'new-space', value: q, label: q ? `Create space “${q}”` : 'Type a name for the new space', glyph: '+', badge: 'space', disabled: !q }];
  }

  // "split <query>" picks a tab to put next to the current one.
  const split = q.match(/^split\s+(.*)$/i);
  if (split || q.toLowerCase() === 'split') {
    const sq = split ? split[1] : '';
    return D.tabs
      .filter((t) => t.id !== D.activeTabId && matches(sq, t.title, t.url))
      .slice(0, 12)
      .map((t) => ({ kind: 'split', value: t.id, label: t.title || host(t.url), secondary: host(t.url), tab: t, badge: 'split with' }));
  }

  const out = [];
  if (q) {
    out.push(
      looksLikeURL(q)
        ? { kind: 'go', value: q, label: q, glyph: '→', badge: D.mode === 'edit' ? 'go' : 'open' }
        : { kind: 'go', value: q, label: q, secondary: '— Google Search', glyph: '⌕', badge: 'search' },
    );
  }

  const byRecent = [...D.tabs].sort((a, b) => b.lastActive - a.lastActive);
  const tabs = byRecent
    .filter((t) => (q ? matches(q, t.title, t.url) : t.id !== D.activeTabId))
    .slice(0, q ? 6 : 5)
    .map((t) => ({ kind: 'tab', value: t.id, label: t.title || host(t.url), secondary: `${host(t.url)} · ${spaceName(t.spaceId)}`, tab: t, badge: 'switch to tab' }));
  out.push(...tabs);

  if (!q) {
    out.push(
      ...D.archive.slice(0, 3).map((t) => ({ kind: 'archived', value: t.id, label: t.title || host(t.url), secondary: host(t.url), tab: t, badge: 'archived' })),
    );
    return out;
  }

  out.push(...commands().filter((c) => matches(q, c.label, c.keys)).slice(0, 6));

  const openURLs = new Set(D.tabs.map((t) => t.url));
  out.push(
    ...D.archive
      .filter((t) => matches(q, t.title, t.url))
      .slice(0, 4)
      .map((t) => ({ kind: 'archived', value: t.id, label: t.title || host(t.url), secondary: host(t.url), tab: t, badge: 'archived' })),
  );
  out.push(
    ...D.history
      .filter((h) => !openURLs.has(h.url) && matches(q, h.title, h.url))
      .slice(0, 6)
      .map((h) => ({ kind: 'go', value: h.url, label: h.title || host(h.url), secondary: host(h.url), tab: h, badge: 'history' })),
  );
  return out;
}

function iconHTML(item) {
  if (item.tab) {
    const letter = esc((host(item.tab.url)[0] || '•').toUpperCase());
    if (item.tab.favicon) return `<img class="icon" src="${esc(item.tab.favicon)}" data-letter="${letter}">`;
    return `<span class="icon letter">${letter}</span>`;
  }
  const style = item.color ? ` style="color:${item.color}"` : '';
  return `<span class="icon glyph"${style}>${esc(item.glyph || '•')}</span>`;
}

function render() {
  items = build(input.value);
  sel = Math.min(sel, Math.max(0, items.length - 1));
  results.innerHTML = items
    .map(
      (it, i) => `<div class="item ${i === sel ? 'sel' : ''}" data-i="${i}">
        ${iconHTML(it)}
        <div class="text"><span class="primary">${esc(it.label)}</span>${it.secondary ? `<span class="secondary">${esc(it.secondary)}</span>` : ''}</div>
        <span class="badge">${esc(it.badge || '')}</span>
      </div>`,
    )
    .join('');
  results.querySelector('.sel')?.scrollIntoView({ block: 'nearest' });
}

function run(item) {
  if (!item || item.disabled) return;
  if (item.kind === 'mode') {
    D.mode = item.value;
    input.value = '';
    input.placeholder = placeholder();
    return render();
  }
  if (item.kind === 'prefix') {
    input.value = item.value;
    return render();
  }
  arc.send('palette:run', { kind: item.kind, value: item.value, mode: D.mode });
}

function placeholder() {
  if (D.mode === 'space') return 'Name your new space…';
  if (D.mode === 'edit') return 'Enter URL or search…';
  return 'Search tabs, history, commands — or type a URL';
}

input.addEventListener('input', () => {
  sel = 0;
  render();
});

input.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowDown' || (e.ctrlKey && e.key === 'n')) {
    e.preventDefault();
    sel = Math.min(sel + 1, items.length - 1);
    render();
  } else if (e.key === 'ArrowUp' || (e.ctrlKey && e.key === 'p')) {
    e.preventDefault();
    sel = Math.max(sel - 1, 0);
    render();
  } else if (e.key === 'Enter') {
    e.preventDefault();
    run(items[sel]);
  } else if (e.key === 'Escape') {
    e.preventDefault();
    arc.send('palette:close');
  }
});

results.addEventListener('mousemove', (e) => {
  const el = e.target.closest('[data-i]');
  if (el && Number(el.dataset.i) !== sel) {
    sel = Number(el.dataset.i);
    results.querySelectorAll('.item').forEach((x, i) => x.classList.toggle('sel', i === sel));
  }
});

results.addEventListener('mousedown', (e) => {
  const el = e.target.closest('[data-i]');
  if (el) {
    e.preventDefault();
    run(items[Number(el.dataset.i)]);
  }
});

$('backdrop').addEventListener('mousedown', () => arc.send('palette:close'));

document.addEventListener(
  'error',
  (e) => {
    const img = e.target;
    if (img.tagName !== 'IMG' || !img.dataset.letter) return;
    img.replaceWith(Object.assign(document.createElement('span'), { className: 'icon letter', textContent: img.dataset.letter }));
  },
  true,
);

arc.onPalette((data) => {
  D = data;
  const space = D.spaces.find((s) => s.id === D.activeSpaceId);
  document.documentElement.style.setProperty('--accent', space?.color || '#7c5cff');
  input.placeholder = placeholder();
  input.value = D.query || '';
  sel = 0;
  // Restart the open animation each time.
  document.querySelectorAll('.card,.backdrop').forEach((el) => {
    el.style.animation = 'none';
    void el.offsetWidth;
    el.style.animation = '';
  });
  render();
  input.focus();
  input.select();
});
