const { app, BaseWindow, WebContentsView, Menu, ipcMain, session, clipboard } = require('electron');
const path = require('node:path');
const fs = require('node:fs');
const startAgents = require('./agent');

app.setName('Arcish');
if (process.env.ARCISH_USER_DATA) app.setPath('userData', process.env.ARCISH_USER_DATA);
// Drop the Electron/app tokens so sites treat us like plain Chrome.
app.userAgentFallback = app.userAgentFallback
  .replace(/ Electron\/\S+/, '')
  .replace(/ arcish\/\S+/i, '');

const isMac = process.platform === 'darwin';
const SIDEBAR_W = 248;
const GAP = 8;
const RADIUS = 10;
const MIN = 60 * 1000;
const HOUR = 60 * MIN;
const COLORS = ['#7c5cff', '#2f80ed', '#e5484d', '#12a594', '#f76b15', '#d6409f', '#8e8c99'];

const statePath = () => path.join(app.getPath('userData'), 'state.json');
const uid = () => Math.random().toString(36).slice(2, 10);

// ---------------------------------------------------------------- state

let state;
let win;
let ui;
let palette = null;
let paletteOpen = false;
let blocker = null;
let blockedCount = 0;
let agents = null;
const views = new Map(); // tabId -> { view, loading }
const attached = new Set(); // tabIds whose view is currently in the window

function makeTab(spaceId, url, extra = {}) {
  const now = Date.now();
  return { id: uid(), spaceId, url, title: url, favicon: null, pinned: false, favorite: false, lastActive: now, createdAt: now, ...extra };
}

function defaultState() {
  const personal = { id: uid(), name: 'Personal', color: COLORS[0], activeTabId: null, split: null };
  const work = { id: uid(), name: 'Work', color: COLORS[1], activeTabId: null, split: null };
  const tabs = [
    makeTab(null, 'https://www.youtube.com', { favorite: true }),
    makeTab(personal.id, 'https://news.ycombinator.com', { pinned: true }),
    makeTab(personal.id, 'https://en.wikipedia.org/wiki/Arc_(web_browser)'),
    makeTab(work.id, 'https://github.com', { pinned: true }),
  ];
  personal.activeTabId = tabs[2].id;
  work.activeTabId = tabs[3].id;
  return {
    version: 1,
    spaces: [personal, work],
    activeSpaceId: personal.id,
    tabs,
    archive: [],
    history: [],
    settings: {
      sidebar: true,
      adblock: true,
      archiveAfterMs: 12 * HOUR,
      discardAfterMs: 10 * MIN,
      searchUrl: 'https://www.google.com/search?q=%s',
    },
    bounds: null,
  };
}

function loadState() {
  try {
    const s = JSON.parse(fs.readFileSync(statePath(), 'utf8'));
    if (s.version === 1) return { ...defaultState(), ...s, settings: { ...defaultState().settings, ...s.settings } };
  } catch {}
  return defaultState();
}

let saveTimer = null;
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNow, 500);
}
function saveNow() {
  clearTimeout(saveTimer);
  fs.mkdirSync(path.dirname(statePath()), { recursive: true });
  fs.writeFileSync(statePath(), JSON.stringify(state));
}

const getTab = (id) => state.tabs.find((t) => t.id === id);
const getSpace = (id) => state.spaces.find((s) => s.id === id);
const activeSpace = () => getSpace(state.activeSpaceId) || state.spaces[0];
const activeTab = () => getTab(activeSpace()?.activeTabId);

function visibleTabIds() {
  const space = activeSpace();
  const t = activeTab();
  if (!space || !t) return [];
  if (space.split && space.split.includes(t.id) && space.split.every(getTab)) return space.split;
  return [t.id];
}

// ------------------------------------------------------------- UI sync

let pushTimer = null;
function changed() {
  save();
  if (pushTimer) return;
  pushTimer = setTimeout(() => {
    pushTimer = null;
    pushState();
  }, 16);
}

function pushState() {
  if (!ui || ui.webContents.isDestroyed()) return;
  const t = activeTab();
  const wc = t && views.get(t.id)?.view.webContents;
  ui.webContents.send('state', {
    platform: process.platform,
    sidebarW: SIDEBAR_W,
    gap: GAP,
    spaces: state.spaces,
    activeSpaceId: state.activeSpaceId,
    activeTabId: t?.id ?? null,
    visible: visibleTabIds(),
    tabs: state.tabs.map((tab) => ({
      ...tab,
      live: views.has(tab.id),
      loading: views.get(tab.id)?.loading ?? false,
    })),
    settings: state.settings,
    canGoBack: wc ? wc.navigationHistory.canGoBack() : false,
    canGoForward: wc ? wc.navigationHistory.canGoForward() : false,
    agents: agents?.uiState() ?? null,
  });
}

function toast(text) {
  ui?.webContents.send('toast', text);
}

// ----------------------------------------------------------- tab views

const isAgentTab = (tab) => getSpace(tab.spaceId)?.kind === 'agent';

function ensureView(tab) {
  const existing = views.get(tab.id);
  if (existing) return existing.view;

  // Agents work in tabs nobody is looking at, so those must not be throttled.
  const view = new WebContentsView({ webPreferences: { sandbox: true, contextIsolation: true, backgroundThrottling: !isAgentTab(tab) } });
  view.setBorderRadius(RADIUS);
  view.setBackgroundColor('#ffffff');
  // A view outside the window still renders and takes input, as long as it has a size.
  if (win) view.setBounds(tabArea());
  views.set(tab.id, { view, loading: false });
  const wc = view.webContents;

  wc.on('page-title-updated', (_e, title) => {
    tab.title = title;
    recordHistory(tab);
    changed();
  });
  wc.on('page-favicon-updated', (_e, icons) => {
    tab.favicon = icons[0] || null;
    changed();
  });
  const onNav = (url) => {
    tab.url = url;
    recordHistory(tab);
    changed();
  };
  wc.on('did-navigate', (_e, url) => {
    tab.title = url; // a page without a <title> mustn't keep the previous page's
    onNav(url);
  });
  wc.on('did-navigate-in-page', (_e, url, isMain) => isMain && onNav(url));
  wc.on('did-start-loading', () => {
    views.get(tab.id) && (views.get(tab.id).loading = true);
    changed();
  });
  wc.on('did-stop-loading', () => {
    views.get(tab.id) && (views.get(tab.id).loading = false);
    changed();
  });
  wc.on('focus', () => {
    // Clicking into the other half of a split makes it the active tab.
    const space = activeSpace();
    if (space.split?.includes(tab.id) && space.activeTabId !== tab.id) {
      space.activeTabId = tab.id;
      changed();
    }
  });
  wc.on('render-process-gone', () => discard(tab.id));
  wc.setWindowOpenHandler(({ url, disposition }) => {
    // Popups stay in the opener's space, and an agent's popups never take focus.
    const agentTab = isAgentTab(tab);
    const opened = newTab(url, { background: agentTab || disposition === 'background-tab', spaceId: tab.spaceId || state.activeSpaceId });
    if (agentTab) agents?.opened(tab, opened);
    return { action: 'deny' };
  });
  wc.on('context-menu', (_e, p) => pageContextMenu(tab, wc, p));

  if (tab.nav) {
    wc.navigationHistory.restore(tab.nav).catch(() => wc.loadURL(tab.url));
    delete tab.nav;
  } else {
    wc.loadURL(tab.url).catch(() => {});
  }
  return view;
}

function discard(tabId) {
  const entry = views.get(tabId);
  if (!entry) return;
  const tab = getTab(tabId);
  const wc = entry.view.webContents;
  if (tab && !wc.isDestroyed()) {
    // Keep back/forward + scroll position in memory so a discarded tab comes back where it was.
    tab.nav = { entries: wc.navigationHistory.getAllEntries(), index: wc.navigationHistory.getActiveIndex() };
  }
  if (attached.has(tabId)) {
    win.contentView.removeChildView(entry.view);
    attached.delete(tabId);
  }
  views.delete(tabId);
  if (!wc.isDestroyed()) wc.close();
  changed();
}

function recordHistory(tab) {
  if (!/^https?:/.test(tab.url)) return;
  state.history = state.history.filter((h) => h.url !== tab.url);
  state.history.unshift({ url: tab.url, title: tab.title, favicon: tab.favicon, at: Date.now() });
  if (state.history.length > 1000) state.history.length = 1000;
}

function tabArea() {
  const [W, H] = win.getContentSize();
  const x0 = state.settings.sidebar ? SIDEBAR_W : GAP;
  return { x: x0, y: GAP, width: W - x0 - GAP, height: H - 2 * GAP };
}

function layout() {
  if (!win) return;
  if (isMac) win.setWindowButtonVisibility(state.settings.sidebar);
  const [W, H] = win.getContentSize();
  ui.setBounds({ x: 0, y: 0, width: W, height: H });

  const area = tabArea();
  const { x: x0, width: w, height: h } = area;
  const ids = visibleTabIds();
  const want = new Set(ids);

  for (const id of [...attached]) {
    if (!want.has(id)) {
      const v = views.get(id)?.view;
      if (v) win.contentView.removeChildView(v);
      attached.delete(id);
    }
  }
  const half = Math.floor((w - GAP) / 2);
  ids.forEach((id, i) => {
    const view = ensureView(getTab(id));
    const bounds = ids.length === 2
      ? { x: x0 + i * (half + GAP), y: GAP, width: i === 0 ? half : w - half - GAP, height: h }
      : { x: x0, y: GAP, width: w, height: h };
    view.setBounds(bounds);
    if (!attached.has(id)) {
      win.contentView.addChildView(view);
      attached.add(id);
    }
  });
  // Hidden tabs keep the size they'd have on screen, so agents see the same layout the user would.
  for (const [id, { view }] of views) if (!want.has(id)) view.setBounds(area);

  if (paletteOpen) {
    palette.setBounds({ x: 0, y: 0, width: W, height: H });
    win.contentView.addChildView(palette); // re-adding moves it to the top
  }
}

function focusActive() {
  if (paletteOpen) return palette.webContents.focus();
  const t = activeTab();
  const v = t && views.get(t.id)?.view;
  if (v) v.webContents.focus();
  else ui.webContents.focus();
}

// --------------------------------------------------------- tab actions

function activateTab(id) {
  const tab = getTab(id);
  if (!tab) return;
  const prev = activeTab();
  if (prev) prev.lastActive = Date.now();
  if (tab.spaceId && tab.spaceId !== state.activeSpaceId) state.activeSpaceId = tab.spaceId;
  activeSpace().activeTabId = tab.id;
  tab.lastActive = Date.now();
  layout();
  focusActive();
  changed();
}

function newTab(url, { background = false, spaceId = state.activeSpaceId } = {}) {
  const tab = makeTab(spaceId, url);
  // Newest tabs go to the top of the unpinned list, like Arc.
  state.tabs.unshift(tab);
  if (background) {
    ensureView(tab);
    changed();
  } else {
    activateTab(tab.id);
  }
  return tab;
}

function neighbourOf(tab) {
  const list = state.tabs.filter((t) => t.spaceId === tab.spaceId && t.id !== tab.id);
  const i = state.tabs.indexOf(tab);
  return list.find((t) => state.tabs.indexOf(t) > i && t.pinned === tab.pinned) || list.find((t) => !t.pinned) || list[list.length - 1] || null;
}

function removeFromLayout(tab) {
  for (const space of state.spaces) {
    if (space.split?.includes(tab.id)) space.split = null;
    if (space.activeTabId === tab.id) space.activeTabId = neighbourOf(tab)?.id ?? null;
  }
}

function archiveTab(id, reason) {
  const tab = getTab(id);
  if (!tab) return;
  const wasVisible = visibleTabIds().includes(id);
  discard(id);
  delete tab.nav;
  removeFromLayout(tab);
  state.tabs = state.tabs.filter((t) => t.id !== id);
  state.archive.unshift({ ...tab, archivedAt: Date.now(), reason });
  if (state.archive.length > 300) state.archive.length = 300;
  layout();
  if (wasVisible) focusActive();
  changed();
}

function restoreArchived(id) {
  const entry = state.archive.find((t) => t.id === id);
  if (!entry) return;
  state.archive = state.archive.filter((t) => t.id !== id);
  const { archivedAt, reason, ...tab } = entry;
  tab.spaceId = getSpace(tab.spaceId) || tab.favorite ? tab.spaceId : state.activeSpaceId;
  tab.lastActive = Date.now();
  state.tabs.unshift(tab);
  activateTab(tab.id);
}

function reopenClosed() {
  const last = state.archive.find((t) => t.reason === 'closed');
  if (last) restoreArchived(last.id);
}

function moveTab({ id, beforeId, section, spaceId }) {
  const tab = getTab(id);
  if (!tab || id === beforeId) return;
  const wasActiveIn = state.spaces.filter((s) => s.activeTabId === id);
  state.tabs = state.tabs.filter((t) => t.id !== id);
  tab.favorite = section === 'favorites';
  tab.pinned = section === 'pinned';
  tab.spaceId = tab.favorite ? null : spaceId || tab.spaceId || state.activeSpaceId;
  const at = beforeId ? state.tabs.findIndex((t) => t.id === beforeId) : -1;
  if (at >= 0) state.tabs.splice(at, 0, tab);
  else state.tabs.push(tab);
  // Moving the active tab to another space: follow it there.
  if (!tab.favorite && wasActiveIn.length && tab.spaceId !== state.activeSpaceId) {
    for (const s of wasActiveIn) {
      if (s.split?.includes(id)) s.split = null;
      if (s.activeTabId === id) s.activeTabId = null;
    }
    state.activeSpaceId = tab.spaceId;
    getSpace(tab.spaceId).activeTabId = tab.id;
  }
  layout();
  changed();
}

function splitWith(otherId) {
  const space = activeSpace();
  const cur = activeTab();
  const other = getTab(otherId);
  if (!cur || !other || cur.id === other.id) return;
  if (other.spaceId && other.spaceId !== space.id) {
    other.spaceId = space.id;
  }
  ensureView(other);
  space.split = [cur.id, other.id];
  layout();
  changed();
}

function unsplit() {
  activeSpace().split = null;
  layout();
  focusActive();
  changed();
}

function cycleTab(dir) {
  const space = activeSpace();
  const list = [
    ...state.tabs.filter((t) => t.favorite),
    ...state.tabs.filter((t) => t.spaceId === space.id && t.pinned),
    ...state.tabs.filter((t) => t.spaceId === space.id && !t.pinned && !t.favorite),
  ];
  if (!list.length) return;
  const i = list.findIndex((t) => t.id === space.activeTabId);
  activateTab(list[(i + dir + list.length) % list.length].id);
}

function activateSpace(id) {
  if (!getSpace(id)) return;
  const prev = activeTab();
  if (prev) prev.lastActive = Date.now();
  state.activeSpaceId = id;
  const t = activeTab();
  if (t) t.lastActive = Date.now();
  layout();
  focusActive();
  changed();
}

function cycleSpace(dir) {
  const i = state.spaces.findIndex((s) => s.id === state.activeSpaceId);
  activateSpace(state.spaces[(i + dir + state.spaces.length) % state.spaces.length].id);
}

function createSpace(name) {
  const color = COLORS[state.spaces.length % COLORS.length];
  const space = { id: uid(), name: name || `Space ${state.spaces.length + 1}`, color, activeTabId: null, split: null };
  state.spaces.push(space);
  activateSpace(space.id);
}

function deleteSpace(id) {
  if (state.spaces.length < 2) return toast('Can’t delete the last space');
  for (const t of state.tabs.filter((t) => t.spaceId === id)) archiveTab(t.id, 'space-deleted');
  state.spaces = state.spaces.filter((s) => s.id !== id);
  if (state.activeSpaceId === id) state.activeSpaceId = state.spaces[0].id;
  layout();
  changed();
}

function toggleSidebar() {
  state.settings.sidebar = !state.settings.sidebar;
  layout();
  changed();
}

function toURL(input) {
  const q = input.trim();
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(q) || /^(about|data|file):/i.test(q)) return q;
  if (/^localhost(:\d+)?(\/.*)?$/i.test(q) || /^[^\s]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(q)) return `https://${q}`;
  return state.settings.searchUrl.replace('%s', encodeURIComponent(q));
}

function withActiveWC(fn) {
  const t = activeTab();
  const wc = t && views.get(t.id)?.view.webContents;
  if (wc) fn(wc, t);
}

// ------------------------------------------------- memory & archiving

function maintenance() {
  const now = Date.now();
  const s = state.settings;
  const visible = new Set(visibleTabIds());
  const activeAnywhere = new Set(state.spaces.map((sp) => sp.activeTabId));
  for (const tab of [...state.tabs]) {
    if (visible.has(tab.id)) continue;
    const idle = now - tab.lastActive;
    const wc = views.get(tab.id)?.view.webContents;
    const audible = wc && !wc.isDestroyed() && wc.isCurrentlyAudible();
    if (audible) continue;
    if (s.archiveAfterMs && !tab.pinned && !tab.favorite && !activeAnywhere.has(tab.id) && idle > s.archiveAfterMs) {
      archiveTab(tab.id, 'auto');
    } else if (wc && s.discardAfterMs && idle > s.discardAfterMs) {
      discard(tab.id);
    }
  }
}

function discardBackground() {
  const visible = new Set(visibleTabIds());
  let n = 0;
  for (const id of [...views.keys()]) {
    const wc = views.get(id).view.webContents;
    if (!visible.has(id) && !wc.isCurrentlyAudible()) {
      discard(id);
      n++;
    }
  }
  toast(`Put ${n} background tab${n === 1 ? '' : 's'} to sleep`);
}

function sendMetrics() {
  if (!ui || ui.webContents.isDestroyed()) return;
  const kb = app.getAppMetrics().reduce((sum, m) => sum + (m.memory?.workingSetSize || 0), 0);
  ui.webContents.send('metrics', {
    memoryMB: Math.round(kb / 1024),
    liveTabs: views.size,
    totalTabs: state.tabs.length,
    blocked: blockedCount,
    adblock: state.settings.adblock && !!blocker,
  });
}

async function setupBlocker() {
  try {
    const { ElectronBlocker } = require('@ghostery/adblocker-electron');
    const cache = path.join(app.getPath('userData'), 'adblock-engine.bin');
    blocker = await ElectronBlocker.fromPrebuiltAdsAndTracking(fetch, {
      path: cache,
      read: fs.promises.readFile,
      write: fs.promises.writeFile,
    });
    blocker.on('request-blocked', () => blockedCount++);
    if (state.settings.adblock) blocker.enableBlockingInSession(session.defaultSession);
  } catch (err) {
    console.error('Ad blocker unavailable:', err.message);
  }
}

function toggleAdblock() {
  state.settings.adblock = !state.settings.adblock;
  if (blocker) {
    if (state.settings.adblock) blocker.enableBlockingInSession(session.defaultSession);
    else blocker.disableBlockingInSession(session.defaultSession);
  }
  toast(`Ad & tracker blocking ${state.settings.adblock ? 'on' : 'off'} (reload to apply)`);
  changed();
}

// -------------------------------------------------------- command bar

function openPalette(mode) {
  if (!palette) {
    palette = new WebContentsView({
      webPreferences: { preload: path.join(__dirname, 'preload.js'), sandbox: true, contextIsolation: true },
    });
    palette.setBackgroundColor('#00000000');
    logConsole(palette.webContents, 'palette');
    palette.webContents.loadFile(path.join(__dirname, 'ui', 'palette.html'));
  }
  paletteOpen = true;
  layout();
  const t = activeTab();
  const payload = {
    mode,
    query: mode === 'edit' && t ? t.url : '',
    platform: process.platform,
    activeTabId: t?.id ?? null,
    activeSpaceId: state.activeSpaceId,
    spaces: state.spaces,
    tabs: state.tabs,
    archive: state.archive,
    history: state.history.slice(0, 400),
    settings: state.settings,
    split: !!activeSpace().split,
  };
  const send = () => {
    palette.webContents.send('palette:open', payload);
    palette.webContents.focus();
  };
  if (palette.webContents.isLoading()) palette.webContents.once('did-finish-load', send);
  else send();
}

function closePalette() {
  if (!paletteOpen) return;
  paletteOpen = false;
  win.contentView.removeChildView(palette);
  focusActive();
}

function runPalette({ kind, value, mode }) {
  closePalette();
  switch (kind) {
    case 'go': {
      const url = toURL(value);
      if (mode === 'edit' && activeTab()) withActiveWC((wc) => wc.loadURL(url).catch(() => {}));
      else newTab(url);
      break;
    }
    case 'tab':
      return activateTab(value);
    case 'archived':
      return restoreArchived(value);
    case 'split':
      return splitWith(value);
    case 'space':
      return activateSpace(value);
    case 'new-space':
      return createSpace(value);
    case 'command':
      return runCommand(value);
  }
}

const ARCHIVE_OPTIONS = { 'archive-5m': 5 * MIN, 'archive-1h': HOUR, 'archive-12h': 12 * HOUR, 'archive-24h': 24 * HOUR, 'archive-off': 0 };

function runCommand(cmd) {
  if (cmd in ARCHIVE_OPTIONS) {
    state.settings.archiveAfterMs = ARCHIVE_OPTIONS[cmd];
    toast(ARCHIVE_OPTIONS[cmd] ? `Unpinned tabs archive after ${cmd.slice(8)} idle` : 'Auto-archive off');
    changed();
    return maintenance();
  }
  const actions = {
    'toggle-sidebar': toggleSidebar,
    'toggle-adblock': toggleAdblock,
    'sleep-tabs': discardBackground,
    'unsplit': unsplit,
    'copy-url': copyURL,
    'reopen': reopenClosed,
    'close-tab': () => activeTab() && archiveTab(activeTab().id, 'closed'),
    'pin-tab': () => activeTab() && togglePin(activeTab().id),
    'devtools': () => withActiveWC((wc) => wc.openDevTools({ mode: 'detach' })),
    'archive-idle-now': () => {
      const space = activeSpace();
      for (const t of state.tabs.filter((t) => t.spaceId === space.id && !t.pinned && t.id !== space.activeTabId)) archiveTab(t.id, 'manual');
    },
  };
  actions[cmd]?.();
}

function togglePin(id) {
  const tab = getTab(id);
  if (!tab) return;
  moveTab({ id, section: tab.pinned ? 'today' : 'pinned', spaceId: tab.spaceId || state.activeSpaceId });
}

function copyURL() {
  const t = activeTab();
  if (!t) return;
  clipboard.writeText(t.url);
  toast('Copied URL');
}

// ------------------------------------------------------- context menus

function pageContextMenu(tab, wc, p) {
  const t = [];
  if (p.linkURL) {
    t.push(
      { label: 'Open Link in New Tab', click: () => newTab(p.linkURL, { background: true }) },
      { label: 'Open Link in Split View', click: () => splitWith(newTab(p.linkURL, { background: true }).id) },
      { label: 'Copy Link', click: () => clipboard.writeText(p.linkURL) },
      { type: 'separator' },
    );
  }
  if (p.isEditable) t.push({ role: 'cut' }, { role: 'copy' }, { role: 'paste' }, { type: 'separator' });
  else if (p.selectionText) {
    const s = p.selectionText.trim().slice(0, 30);
    t.push({ role: 'copy' }, { label: `Search for “${s}”`, click: () => newTab(toURL(p.selectionText)) }, { type: 'separator' });
  }
  t.push(
    { label: 'Back', enabled: wc.navigationHistory.canGoBack(), click: () => wc.navigationHistory.goBack() },
    { label: 'Forward', enabled: wc.navigationHistory.canGoForward(), click: () => wc.navigationHistory.goForward() },
    { label: 'Reload', click: () => wc.reload() },
    { type: 'separator' },
    { label: 'Inspect Element', click: () => wc.inspectElement(p.x, p.y) },
  );
  Menu.buildFromTemplate(t).popup({ window: win });
}

function tabContextMenu(id) {
  const tab = getTab(id);
  if (!tab) return;
  const cur = activeTab();
  const others = state.spaces.filter((s) => s.id !== tab.spaceId);
  Menu.buildFromTemplate([
    { label: tab.pinned ? 'Unpin Tab' : 'Pin Tab', enabled: !tab.favorite, click: () => togglePin(id) },
    {
      label: tab.favorite ? 'Remove from Favorites' : 'Add to Favorites',
      click: () => moveTab({ id, section: tab.favorite ? 'today' : 'favorites', spaceId: state.activeSpaceId }),
    },
    { label: 'Open in Split View', enabled: !!cur && cur.id !== id, click: () => splitWith(id) },
    {
      label: 'Move to Space',
      enabled: others.length > 0 && !tab.favorite,
      submenu: others.map((s) => ({ label: s.name, click: () => moveTab({ id, section: tab.pinned ? 'pinned' : 'today', spaceId: s.id }) })),
    },
    { label: 'Copy URL', click: () => clipboard.writeText(tab.url) },
    { label: 'Put to Sleep', enabled: views.has(id) && !visibleTabIds().includes(id), click: () => discard(id) },
    { type: 'separator' },
    { label: 'Archive Tab', click: () => archiveTab(id, 'closed') },
  ]).popup({ window: win });
}

function spaceContextMenu(id) {
  const space = getSpace(id);
  if (!space) return;
  Menu.buildFromTemplate([
    ...COLORS.map((c, i) => ({
      label: ['Violet', 'Blue', 'Red', 'Teal', 'Orange', 'Pink', 'Grey'][i],
      type: 'radio',
      checked: space.color === c,
      click: () => {
        space.color = c;
        changed();
      },
    })),
    { type: 'separator' },
    { label: 'Delete Space', enabled: state.spaces.length > 1, click: () => deleteSpace(id) },
  ]).popup({ window: win });
}

// ---------------------------------------------------------------- menu

function buildMenu() {
  const nav = (fn) => () => withActiveWC(fn);
  const spaceItems = Array.from({ length: 9 }, (_, i) => ({
    label: `Space ${i + 1}`,
    accelerator: `Ctrl+${i + 1}`,
    click: () => state.spaces[i] && activateSpace(state.spaces[i].id),
  }));
  const template = [
    ...(isMac ? [{ role: 'appMenu' }] : []),
    {
      label: 'File',
      submenu: [
        { label: 'New Tab…', accelerator: 'CmdOrCtrl+T', click: () => openPalette('new') },
        { label: 'Open Location…', accelerator: 'CmdOrCtrl+L', click: () => openPalette('edit') },
        { label: 'Command Bar…', accelerator: 'CmdOrCtrl+K', click: () => openPalette('new') },
        { label: 'Close Tab', accelerator: 'CmdOrCtrl+W', click: () => (paletteOpen ? closePalette() : runCommand('close-tab')) },
        { label: 'Reopen Closed Tab', accelerator: 'CmdOrCtrl+Shift+T', click: reopenClosed },
        { type: 'separator' },
        { label: 'Copy URL', accelerator: 'CmdOrCtrl+Shift+C', click: copyURL },
        { label: 'Pin / Unpin Tab', accelerator: 'CmdOrCtrl+D', click: () => runCommand('pin-tab') },
        ...(isMac ? [] : [{ type: 'separator' }, { role: 'quit' }]),
      ],
    },
    { role: 'editMenu' },
    {
      label: 'View',
      submenu: [
        { label: 'Toggle Sidebar', accelerator: 'CmdOrCtrl+S', click: toggleSidebar },
        { label: 'Close Split View', accelerator: 'CmdOrCtrl+Shift+U', click: unsplit },
        { type: 'separator' },
        { label: 'Reload', accelerator: 'CmdOrCtrl+R', click: nav((wc) => wc.reload()) },
        { label: 'Hard Reload', accelerator: 'CmdOrCtrl+Shift+R', click: nav((wc) => wc.reloadIgnoringCache()) },
        { label: 'Back', accelerator: 'CmdOrCtrl+[', click: nav((wc) => wc.navigationHistory.goBack()) },
        { label: 'Forward', accelerator: 'CmdOrCtrl+]', click: nav((wc) => wc.navigationHistory.goForward()) },
        { type: 'separator' },
        { label: 'Zoom In', accelerator: 'CmdOrCtrl+=', click: nav((wc) => wc.setZoomLevel(wc.getZoomLevel() + 0.5)) },
        { label: 'Zoom Out', accelerator: 'CmdOrCtrl+-', click: nav((wc) => wc.setZoomLevel(wc.getZoomLevel() - 0.5)) },
        { label: 'Actual Size', accelerator: 'CmdOrCtrl+0', click: nav((wc) => wc.setZoomLevel(0)) },
        { type: 'separator' },
        { label: 'Developer Tools', accelerator: isMac ? 'Alt+Cmd+I' : 'Ctrl+Shift+I', click: () => runCommand('devtools') },
        { label: 'Sidebar Developer Tools', click: () => ui.webContents.openDevTools({ mode: 'detach' }) },
      ],
    },
    {
      label: 'Tabs',
      submenu: [
        { label: 'Next Tab', accelerator: 'Alt+CmdOrCtrl+Down', click: () => cycleTab(1) },
        { label: 'Previous Tab', accelerator: 'Alt+CmdOrCtrl+Up', click: () => cycleTab(-1) },
        { label: 'Next Tab', accelerator: 'Ctrl+Tab', visible: false, click: () => cycleTab(1) },
        { label: 'Previous Tab', accelerator: 'Ctrl+Shift+Tab', visible: false, click: () => cycleTab(-1) },
        { type: 'separator' },
        { label: 'Next Space', accelerator: 'Alt+CmdOrCtrl+Right', click: () => cycleSpace(1) },
        { label: 'Previous Space', accelerator: 'Alt+CmdOrCtrl+Left', click: () => cycleSpace(-1) },
        ...spaceItems,
        { type: 'separator' },
        { label: 'Put Background Tabs to Sleep', click: discardBackground },
      ],
    },
    { role: 'windowMenu' },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

// ----------------------------------------------------------------- IPC

ipcMain.on('ui', (_e, action, arg) => {
  const handlers = {
    'tab:activate': () => activateTab(arg),
    'tab:archive': () => archiveTab(arg, 'closed'),
    'tab:move': () => moveTab(arg),
    'tab:menu': () => tabContextMenu(arg),
    'space:activate': () => activateSpace(arg),
    'space:cycle': () => cycleSpace(arg),
    'space:menu': () => spaceContextMenu(arg),
    'space:rename': () => {
      const s = getSpace(arg.id);
      if (s && arg.name.trim()) s.name = arg.name.trim();
      changed();
    },
    'nav:back': () => withActiveWC((wc) => wc.navigationHistory.goBack()),
    'nav:forward': () => withActiveWC((wc) => wc.navigationHistory.goForward()),
    'nav:reload': () => withActiveWC((wc) => wc.reload()),
    'palette:open': () => openPalette(arg),
    'palette:close': closePalette,
    'palette:run': () => runPalette(arg),
    'command': () => runCommand(arg),
    'agent:answer': () => agents?.answer(arg.id, arg.answer),
    'agent:show': () => agents?.show(arg),
    'agent:take-over': () => agents?.takeOver(arg),
    'agent:hand-back': () => agents?.handBack(arg),
    'agent:revoke': () => agents?.revoke(arg.agent, arg.host),
    'ready': () => {
      pushState();
      sendMetrics();
    },
  };
  handlers[action]?.();
});

// -------------------------------------------------------------- window

// Surface errors from our own UI pages in the terminal.
function logConsole(wc, name) {
  wc.on('console-message', (e) => {
    if (e.level === 'error' || e.level === 'warning') console.log(`[${name}] ${e.level}: ${e.message}`);
  });
  wc.on('preload-error', (_e, p, err) => console.log(`[${name}] preload error: ${err.message}`));
}

function createWindow() {
  const b = state.bounds || { width: 1360, height: 860 };
  win = new BaseWindow({
    ...b,
    minWidth: 640,
    minHeight: 420,
    title: 'Arcish',
    backgroundColor: '#1d1b26',
    ...(isMac ? { titleBarStyle: 'hiddenInset', trafficLightPosition: { x: 16, y: 18 } } : {}),
  });

  ui = new WebContentsView({
    webPreferences: { preload: path.join(__dirname, 'preload.js'), sandbox: true, contextIsolation: true },
  });
  ui.setBackgroundColor('#00000000');
  logConsole(ui.webContents, 'sidebar');
  win.contentView.addChildView(ui);
  ui.webContents.loadFile(path.join(__dirname, 'ui', 'index.html'));

  const saveBounds = () => {
    if (!win.isMinimized() && !win.isFullScreen()) state.bounds = win.getBounds();
    save();
  };
  win.on('resize', () => {
    layout();
    saveBounds();
  });
  win.on('move', saveBounds);
  win.on('focus', focusActive);
  win.on('closed', () => {
    win = null;
    app.quit();
  });

  layout();
  focusActive();
}

app.whenReady().then(() => {
  state = loadState();
  // Restored tabs stay asleep until you open them, so startup only loads what's on screen.
  buildMenu();
  createWindow();
  setupBlocker();
  agents = startAgents({
    state: () => state,
    userData: app.getPath('userData'),
    colors: COLORS,
    uid,
    views,
    isShown: (id) => attached.has(id),
    getTab,
    getSpace,
    ensureView,
    newTab,
    archiveTab,
    activateTab,
    layout,
    changed,
    toast,
    toURL,
  });
  setInterval(maintenance, 30 * 1000);
  setInterval(sendMetrics, 2000);
  if (process.env.ARCISH_SMOKE) {
    require('./tools/smoke')({ win, ui, getPalette: () => palette, state: () => state, openPalette, closePalette, runPalette, splitWith, activateSpace, discardBackground, toggleSidebar, views });
  }
  if (process.env.ARCISH_AGENT_DEMO) {
    require('./tools/agent-demo')({ win, ui, state: () => state, agents, views, activateSpace });
  }
});

app.on('before-quit', () => {
  agents?.stop();
  if (state) saveNow();
});
app.on('window-all-closed', () => app.quit());
