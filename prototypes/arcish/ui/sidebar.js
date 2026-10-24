const $ = (id) => document.getElementById(id);
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

let S = null;
let pending = null;
let dragging = null;
let renaming = false;

const host = (url) => {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
};

function iconHTML(tab) {
  const letter = esc((host(tab.url)[0] || '•').toUpperCase());
  if (tab.favicon) return `<img class="icon" src="${esc(tab.favicon)}" data-letter="${letter}">`;
  return `<span class="icon letter">${letter}</span>`;
}

// Broken favicons fall back to the site's first letter (inline handlers are blocked by the CSP).
document.addEventListener(
  'error',
  (e) => {
    const img = e.target;
    if (img.tagName !== 'IMG' || !img.dataset.letter) return;
    img.replaceWith(Object.assign(document.createElement('span'), { className: 'icon letter', textContent: img.dataset.letter }));
  },
  true,
);

const closeSVG = '<svg viewBox="0 0 16 16"><path d="m4 4 8 8M12 4l-8 8"/></svg>';

// Tabs that never loaded only know their URL; show the domain instead.
const label = (tab) => (tab.title && tab.title !== tab.url ? tab.title : host(tab.url));

function tabHTML(tab) {
  const cls = [
    'tab',
    tab.id === S.activeTabId && 'active',
    S.visible.length === 2 && S.visible.includes(tab.id) && 'in-split',
    !tab.live && 'sleeping',
    tab.loading && 'loading',
  ].filter(Boolean).join(' ');
  return `<div class="${cls}" draggable="true" data-id="${tab.id}" title="${esc(tab.title)}\n${esc(tab.url)}">
    ${iconHTML(tab)}<span class="title">${esc(label(tab))}</span>
    <button class="close" data-close="${tab.id}" title="Archive tab">${closeSVG}</button>
  </div>`;
}

const ago = (t) => {
  const s = (Date.now() - t) / 1000;
  return s < 60 ? 'now' : s < 3600 ? `${Math.floor(s / 60)}m` : `${Math.floor(s / 3600)}h`;
};

const OUTCOME = { approved: 'You allowed', denied: 'Denied', failed: 'Failed', paused: 'Paused' };

function activityHTML(space, info) {
  const items = S.agents.activity.filter((a) => a.agent === space.agent).slice(0, 30);
  const sites = info.sites
    .map((h) => `<span class="site">${esc(h)}<button data-revoke="${esc(h)}" data-agent="${esc(space.agent)}" title="Ask again on ${esc(h)}">${closeSVG}</button></span>`)
    .join('');
  const rows = items
    .map((a) => {
      const tag = OUTCOME[a.outcome] ? `<span class="act-tag">${OUTCOME[a.outcome]}</span>` : '';
      return `<div class="act ${a.outcome} ${a.read ? 'read' : ''}" title="${esc(a.host || '')}">
        <span class="act-dot"></span><span class="act-text">${a.outcome === 'you' ? '<b>You</b> ' : ''}${esc(a.summary)}${tag}</span><span class="act-time">${ago(a.at)}</span>
      </div>`;
    })
    .join('');
  return `<div class="section-head">Activity</div>
    ${sites ? `<div class="sites"><span class="sites-label">Always allowed</span>${sites}</div>` : ''}
    ${rows || '<div class="act-empty">Nothing yet</div>'}`;
}

function requestHTML(r) {
  const color = S.spaces.find((s) => s.id === r.spaceId)?.color || 'var(--space)';
  const head = `<div class="req-head"><span class="req-glyph"></span><b>${esc(r.name)}</b>&nbsp;${r.kind === 'handoff' ? 'needs you' : 'wants to'}</div>`;
  if (r.kind === 'handoff') {
    const driving = S.agents.spaces[r.spaceId]?.takenOver;
    return `<div class="req handoff" style="--c:${color}">${head}
      <div class="req-text">${esc(r.text)}</div>
      <div class="req-host">${esc(r.host)}</div>
      <div class="req-buttons"><button class="${driving ? '' : 'primary'}" data-show="${r.id}">Show tab</button><button class="${driving ? 'primary' : ''}" data-hand-back="${r.spaceId}">Hand back</button></div>
    </div>`;
  }
  return `<div class="req" style="--c:${color}">${head}
    <div class="req-text">${esc(r.text)}</div>
    <div class="req-host">${esc(r.reason)}${r.tabId ? ` · <button class="link" data-show="${r.id}">Show tab</button>` : ''}</div>
    <div class="req-buttons"><button data-answer="deny" data-req="${r.id}">Deny</button><button class="primary" data-answer="once" data-req="${r.id}">Allow</button></div>
    <button class="req-always" data-answer="always" data-req="${r.id}">Always allow on ${esc(r.host)}</button>
  </div>`;
}

function favHTML(tab) {
  const cls = ['fav', tab.id === S.activeTabId && 'active', !tab.live && 'sleeping', tab.loading && 'loading'].filter(Boolean).join(' ');
  return `<div class="${cls}" draggable="true" data-id="${tab.id}" title="${esc(label(tab))}">${iconHTML(tab)}</div>`;
}

function render(state) {
  if (dragging || renaming) {
    pending = state;
    return;
  }
  S = state;
  const space = S.spaces.find((s) => s.id === S.activeSpaceId) || S.spaces[0];
  const tabs = S.tabs;
  const active = tabs.find((t) => t.id === S.activeTabId);

  document.body.classList.toggle('mac', S.platform === 'darwin');
  document.body.classList.toggle('no-sidebar', !S.settings.sidebar);
  document.body.classList.toggle('is-empty', !active);
  document.documentElement.style.setProperty('--space', space.color);
  document.documentElement.style.setProperty('--sidebar-w', `${S.sidebarW}px`);
  document.documentElement.style.setProperty('--gap', `${S.gap}px`);
  if (S.platform !== 'darwin') $('kbd-t').textContent = 'Ctrl T';

  $('url-text').textContent = active ? host(active.url) : 'Search or enter URL…';
  $('back').disabled = !S.canGoBack;
  $('forward').disabled = !S.canGoForward;
  $('reload').disabled = !active;

  $('favorites').innerHTML = tabs.filter((t) => t.favorite).map(favHTML).join('');
  $('space-name').textContent = space.name;
  $('pinned').innerHTML = tabs.filter((t) => t.spaceId === space.id && t.pinned && !t.favorite).map(tabHTML).join('');
  $('today').innerHTML = tabs.filter((t) => t.spaceId === space.id && !t.pinned && !t.favorite).map(tabHTML).join('');

  $('space-dots').innerHTML = S.spaces
    .map((s, i) => {
      const a = s.kind === 'agent' ? S.agents?.spaces[s.id] : null;
      const cls = ['space-dot', s.id === space.id && 'active', a && 'agent', a?.working && 'working', a?.needsYou && 'needs'].filter(Boolean).join(' ');
      return `<button class="${cls}" data-space="${s.id}" title="${esc(s.name)}${a ? ` · ${esc(a.status)}` : ''} (Ctrl ${i + 1})" style="--c:${s.color}"><span></span></button>`;
    })
    .join('');

  // Agent spaces: what the agent is doing, a way to take the wheel, and what it has done.
  const agent = space.kind === 'agent' ? S.agents?.spaces[space.id] : null;
  document.body.classList.toggle('agent-space', !!agent);
  $('empty-title').textContent = agent ? `${space.name} hasn't opened anything yet` : 'Nothing open in this space';
  if (agent) {
    $('agent-bar').className = ['agent-bar', agent.working && 'working', agent.needsYou && 'needs', !agent.connected && 'offline'].filter(Boolean).join(' ');
    $('agent-status').textContent = agent.status;
    $('agent-control').textContent = agent.takenOver ? 'Hand back' : 'Take over';
    $('agent-control').dataset.action = agent.takenOver ? 'agent:hand-back' : 'agent:take-over';
    $('agent-control').hidden = !agent.connected && !agent.takenOver;
    $('activity').innerHTML = activityHTML(space, agent);
  } else {
    $('activity').innerHTML = '';
  }
  $('inbox').innerHTML = (S.agents?.requests ?? []).map(requestHTML).join('');
}

// ------------------------------------------------------------- events

document.addEventListener('click', (e) => {
  const answer = e.target.closest('[data-answer]');
  if (answer) return arc.send('agent:answer', { id: answer.dataset.req, answer: answer.dataset.answer });
  const handBack = e.target.closest('[data-hand-back]');
  if (handBack) return arc.send('agent:hand-back', handBack.dataset.handBack);
  const show = e.target.closest('[data-show]');
  if (show) return arc.send('agent:show', show.dataset.show);
  const revoke = e.target.closest('[data-revoke]');
  if (revoke) return arc.send('agent:revoke', { agent: revoke.dataset.agent, host: revoke.dataset.revoke });
  const close = e.target.closest('[data-close]');
  if (close) return arc.send('tab:archive', close.dataset.close);
  const tab = e.target.closest('[data-id]');
  if (tab) return arc.send('tab:activate', tab.dataset.id);
  const space = e.target.closest('[data-space]');
  if (space) return arc.send('space:activate', space.dataset.space);
});

document.addEventListener('auxclick', (e) => {
  const tab = e.target.closest('[data-id]');
  if (tab && e.button === 1) arc.send('tab:archive', tab.dataset.id);
});

document.addEventListener('contextmenu', (e) => {
  const tab = e.target.closest('[data-id]');
  if (tab) return arc.send('tab:menu', tab.dataset.id);
  const space = e.target.closest('[data-space]');
  if (space) arc.send('space:menu', space.dataset.space);
});

$('toggle').onclick = () => arc.send('command', 'toggle-sidebar');
$('back').onclick = () => arc.send('nav:back');
$('forward').onclick = () => arc.send('nav:forward');
$('reload').onclick = () => arc.send('nav:reload');
$('url').onclick = () => arc.send('palette:open', S && S.activeTabId ? 'edit' : 'new');
$('newtab').onclick = () => arc.send('palette:open', 'new');
$('clear').onclick = () => arc.send('command', 'archive-idle-now');
$('add-space').onclick = () => arc.send('palette:open', 'space');
// The button does what its label says, even if a state update is on its way.
$('agent-control').onclick = (e) => arc.send(e.currentTarget.dataset.action, S.activeSpaceId);

// Rename the space by double-clicking its name.
$('space-name').addEventListener('dblclick', () => {
  const el = $('space-name');
  const id = S.activeSpaceId;
  renaming = true;
  el.contentEditable = 'true';
  el.focus();
  document.getSelection().selectAllChildren(el);
  const done = (commit) => {
    el.contentEditable = 'false';
    el.onkeydown = el.onblur = null;
    renaming = false;
    if (commit) arc.send('space:rename', { id, name: el.textContent });
    render(pending || S);
    pending = null;
  };
  el.onkeydown = (e) => {
    if (e.key === 'Enter') (e.preventDefault(), done(true));
    if (e.key === 'Escape') done(false);
  };
  el.onblur = () => done(true);
});

// Two-finger horizontal swipe on the sidebar switches spaces.
let swipe = 0;
let swipeLock = 0;
$('sidebar').addEventListener(
  'wheel',
  (e) => {
    if (Math.abs(e.deltaX) <= Math.abs(e.deltaY)) return;
    if (Date.now() < swipeLock) return;
    swipe += e.deltaX;
    if (Math.abs(swipe) > 90) {
      arc.send('space:cycle', swipe > 0 ? 1 : -1);
      swipe = 0;
      swipeLock = Date.now() + 450;
    }
  },
  { passive: true },
);

// ---------------------------------------------------------- drag & drop

function clearDropMarks() {
  document.querySelectorAll('.drop-before,.drop-after,.drop-target').forEach((el) => el.classList.remove('drop-before', 'drop-after', 'drop-target'));
}

function dropInfo(e) {
  const section = e.target.closest('[data-section]');
  const spaceDot = e.target.closest('[data-space]');
  if (spaceDot) return { spaceDot };
  if (!section) return null;
  const over = e.target.closest('[data-id]');
  if (!over || over.dataset.id === dragging) return { section, beforeId: null };
  const r = over.getBoundingClientRect();
  const after = section.id === 'favorites' ? e.clientX > r.left + r.width / 2 : e.clientY > r.top + r.height / 2;
  const next = after ? over.nextElementSibling?.dataset.id ?? null : over.dataset.id;
  return { section, over, after, beforeId: next };
}

document.addEventListener('dragstart', (e) => {
  const el = e.target.closest?.('[data-id]');
  if (!el) return;
  dragging = el.dataset.id;
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/plain', dragging);
});

document.addEventListener('dragover', (e) => {
  if (!dragging) return;
  const info = dropInfo(e);
  clearDropMarks();
  if (!info) return;
  e.preventDefault();
  if (info.spaceDot) info.spaceDot.classList.add('drop-target');
  else if (info.over) info.over.classList.add(info.after ? 'drop-after' : 'drop-before');
  else info.section.classList.add('drop-target');
});

document.addEventListener('drop', (e) => {
  if (!dragging) return;
  e.preventDefault();
  const info = dropInfo(e);
  const id = dragging;
  if (info?.spaceDot) {
    const tab = S.tabs.find((t) => t.id === id);
    arc.send('tab:move', { id, section: tab?.pinned ? 'pinned' : 'today', spaceId: info.spaceDot.dataset.space });
  } else if (info) {
    arc.send('tab:move', { id, beforeId: info.beforeId, section: info.section.dataset.section, spaceId: S.activeSpaceId });
  }
});

document.addEventListener('dragend', () => {
  dragging = null;
  clearDropMarks();
  if (pending) {
    const p = pending;
    pending = null;
    render(p);
  }
});

// ------------------------------------------------------------ metrics

arc.onMetrics((m) => {
  const blocked = m.adblock ? ` · ${m.blocked.toLocaleString()} blocked` : ' · blocking off';
  $('stats').textContent = `${m.memoryMB.toLocaleString()} MB · ${m.liveTabs}/${m.totalTabs} tabs awake${blocked}`;
  $('stats').title = 'Total memory of all Arcish processes. Sleeping tabs use no memory and reload when opened.';
});

let toastTimer;
arc.onToast((text) => {
  const t = $('toast');
  t.textContent = text;
  t.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove('show'), 1800);
});

arc.onState(render);
arc.send('ready');
