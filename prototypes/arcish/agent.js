// Agents: Claude Code, Codex or any MCP client works in the user's real browser, each in its
// own Agent Space, through a local MCP server. See decision 8 in docs/decisions.md.
//
// The server listens on a socket only this user can open; tools/mcp-bridge.js connects an
// MCP client's stdio to it. Pages are read and driven through Chromium's DevTools protocol
// from inside the browser, so there is no remote-debugging port.
const { app } = require('electron');
const net = require('node:net');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { McpServer } = require('@modelcontextprotocol/sdk/server/mcp.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');
const { z } = require('zod');

const MIN = 60 * 1000;
const IDLE_AFTER = 15 * 1000; // between tool calls the agent is thinking, not idle
const WORLD = 1337; // isolated JS world for our own page scripts, invisible to the page
const AGENT_COLORS = { 'claude-code': '#d97757', codex: '#10a37f' };

const INSTRUCTIONS = `Arcish is the user's real browser. You work in your own Agent Space: tabs you open show up in the user's sidebar but never take their focus, and you are signed in wherever the user is.

Loop: open_tab, then click / type / select_option by ref (like e12), using the snapshot each action returns. Refs stay the same for an element until its page navigates. Use read_page for long text and screenshot when layout matters.

Payments, sending messages and banking sites ask the user first. If they say no, don't retry. You can never type into password fields: at a sign-in, 2FA prompt or CAPTCHA, call request_human and wait. You can only see tabs in your own space.`;

// Roles an agent can act on. Each gets a ref.
const INTERACTIVE = new Set([
  'button', 'link', 'textbox', 'searchbox', 'combobox', 'checkbox', 'radio', 'switch', 'slider', 'spinbutton',
  'listbox', 'option', 'menuitem', 'menuitemcheckbox', 'menuitemradio', 'tab', 'treeitem',
]);
// Wrappers that only add nesting. Their children are printed one level up.
const FLATTEN = new Set(['none', 'generic', 'presentation', 'LayoutTable', 'LayoutTableRow', 'LayoutTableCell', 'InlineTextBox', 'RootWebArea']);
// Landmarks worth printing even without a name.
const LANDMARKS = new Set(['form', 'dialog', 'alertdialog', 'alert', 'navigation', 'main', 'banner', 'search', 'contentinfo']);
const STATES = ['checked', 'selected', 'expanded', 'disabled', 'required', 'level'];

const PAYMENT = /\b(pay|buy|purchase|check ?out|place (your )?order|submit order|order now|subscribe|donate|transfer)\b/i;
const SEND = /\b(send|submit|post|publish|reply|tweet|comment)\b/i;
const MESSAGING = /(^|\.)(mail\.google\.com|outlook\.(live|office)\.com|slack\.com|discord\.com|x\.com|twitter\.com|linkedin\.com|facebook\.com|messenger\.com|instagram\.com|whatsapp\.com|web\.telegram\.org|reddit\.com)$/i;
const BANKING = /(^|\.)(chase|wellsfargo|bankofamerica|citi|capitalone|schwab|fidelity|vanguard|paypal|venmo|revolut|wise|monzo|coinbase)\.com$|bank/i;

const KEYS = { Enter: [13, '\r'], Tab: [9], Escape: [27], Backspace: [8], Delete: [46], ArrowUp: [38], ArrowDown: [40], ArrowLeft: [37], ArrowRight: [39], PageUp: [33], PageDown: [34], Home: [36], End: [35], Space: [32, ' '] };

const SELECT_CONTENTS = `function () {
  this.focus();
  if (typeof this.select === 'function') return this.select();
  const range = document.createRange();
  range.selectNodeContents(this);
  getSelection().removeAllRanges();
  getSelection().addRange(range);
}`;

const CHOOSE_OPTION = `function (wanted) {
  const want = wanted.trim().toLowerCase();
  const option = [...(this.options || [])].find((o) => o.label.trim().toLowerCase() === want || o.value.toLowerCase() === want);
  if (!option) return null;
  this.value = option.value;
  this.dispatchEvent(new Event('input', { bubbles: true }));
  this.dispatchEvent(new Event('change', { bubbles: true }));
  return option.label;
}`;

const ripple = (x, y, color) => `(() => {
  const d = document.createElement('div');
  d.style.cssText = 'position:fixed;left:${x - 18}px;top:${y - 18}px;width:36px;height:36px;border-radius:50%;box-sizing:border-box;border:3px solid ${color};background:${color}33;z-index:2147483647;pointer-events:none;transition:transform 450ms ease-out,opacity 450ms ease-out';
  document.documentElement.appendChild(d);
  requestAnimationFrame(() => requestAnimationFrame(() => { d.style.transform = 'scale(1.9)'; d.style.opacity = '0'; }));
  setTimeout(() => d.remove(), 600);
})()`;

const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const clean = (s) => String(s ?? '').replace(/\s+/g, ' ').trim();
const clip = (s, n = 60) => (clean(s).length > n ? `${clean(s).slice(0, n - 1)}…` : clean(s));
const hostOf = (url) => {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
};
const fail = (outcome, message) => Object.assign(new Error(message), { outcome });

// Agents only get web pages. file:, chrome:, view-source: and the like would let them read
// local files or the browser's own pages, such as saved passwords.
function webOnly(url) {
  let scheme = '';
  try {
    scheme = new URL(url).protocol;
  } catch {}
  if (scheme === 'http:' || scheme === 'https:' || url === 'about:blank') return;
  throw fail('denied', `Agents can only use http and https pages, not ${scheme || url}`);
}

function socketPath(userData) {
  if (process.platform === 'win32') return `\\\\.\\pipe\\arcish-agent-${os.userInfo().username}`;
  return path.join(userData, 'agent.sock');
}

// Why an action needs the user's OK, or null if it doesn't.
function risk(host, label) {
  if (PAYMENT.test(label)) return 'Looks like a payment or order';
  if (MESSAGING.test(host) && SEND.test(label)) return 'Sends a message or post';
  return null;
}

function settled(wc, ms) {
  if (wc.isDestroyed() || !wc.isLoading()) return Promise.resolve();
  return new Promise((resolve) => {
    const done = () => {
      clearTimeout(timer);
      wc.off('did-stop-loading', done);
      resolve();
    };
    const timer = setTimeout(done, ms);
    wc.on('did-stop-loading', done);
  });
}

// Input can start a navigation a moment later. Wait for it if it does.
async function afterInput(wc) {
  await wait(400);
  await settled(wc, 15000);
}

async function press(cdp, key) {
  const [code, text] = KEYS[key];
  const base = { key: key === 'Space' ? ' ' : key, code: key, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code };
  await cdp('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', text, ...base });
  await cdp('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
}

module.exports = function startAgents(h) {
  const sessions = new Map(); // connection id -> { key, name, spaceId, busy, doing, lastAt }
  const requests = []; // waiting on the user: confirmations and handoffs
  const refs = new Map(); // tab id -> { next, byNode, byRef }
  const takenOver = new Set(); // agent space ids the user is driving
  const S = h.state;

  const prefs = (key) => ((S().agents ??= {})[key] ??= { sites: {} });

  // Newest first, by when the action started: a confirmation or handoff finishes after the
  // user's own actions in between.
  function log(entry) {
    const list = (S().activity ??= []);
    const e = { at: Date.now(), ...entry };
    const i = list.findIndex((x) => x.at <= e.at);
    list.splice(i < 0 ? list.length : i, 0, e);
    if (list.length > 300) list.length = 300;
    h.changed();
  }

  // ------------------------------------------------------------ spaces and tabs

  function spaceOf(session) {
    const s = S();
    let space = s.spaces.find((sp) => sp.kind === 'agent' && sp.agent === session.key);
    if (!space) {
      const color = AGENT_COLORS[session.key] || h.colors[s.spaces.length % h.colors.length];
      space = { id: h.uid(), name: session.name, color, activeTabId: null, split: null, kind: 'agent', agent: session.key };
      s.spaces.push(space);
      h.toast(`${session.name} is working in its own space`);
      h.changed();
    }
    session.spaceId = space.id;
    return space;
  }

  function tabOf(session, id) {
    const space = spaceOf(session);
    const tab = h.getTab(id || space.activeTabId);
    if (!tab) throw new Error(id ? `There's no tab ${id}. Call list_tabs.` : 'You have no tab open. Call open_tab first.');
    if (tab.spaceId !== space.id) throw new Error(`Tab ${id} isn't in your space. You can only use tabs in your own space.`);
    return { space, tab };
  }

  // The agent's space shows the tab it's working on, so a user watching follows along.
  function follow(space, tab) {
    tab.lastActive = Date.now();
    if (space.activeTabId === tab.id || takenOver.has(space.id)) return;
    space.activeTabId = tab.id;
    if (S().activeSpaceId === space.id) h.layout();
    h.changed();
  }

  async function page(tab) {
    webOnly(tab.url); // also covers a tab the user dragged into the agent's space
    h.ensureView(tab); // wakes a sleeping tab
    const wc = h.views.get(tab.id).view.webContents;
    if (!wc.debugger.isAttached()) {
      wc.debugger.attach('1.3');
      wc.on('did-navigate', () => refs.delete(tab.id));
      // Pages behave as if focused even though the user's focus is elsewhere.
      await wc.debugger.sendCommand('Emulation.setFocusEmulationEnabled', { enabled: true });
    }
    await settled(wc, 20000);
    return { wc, cdp: (method, params) => wc.debugger.sendCommand(method, params) };
  }

  // ---------------------------------------------------------------- snapshots

  async function snapshot(tab) {
    const { cdp } = await page(tab);
    const { nodes } = await cdp('Accessibility.getFullAXTree');
    const byId = new Map(nodes.map((n) => [n.nodeId, n]));
    const r = refs.get(tab.id) ?? { next: 1, byNode: new Map(), byRef: new Map() };
    refs.set(tab.id, r);

    // Agents may know a password field exists, never what's in it.
    const secret = new Set();
    await Promise.all(
      nodes
        .filter((n) => n.role?.value === 'textbox' && n.backendDOMNodeId)
        .map(async (n) => {
          const { node } = await cdp('DOM.describeNode', { backendNodeId: n.backendDOMNodeId }).catch(() => ({}));
          const a = node?.attributes ?? [];
          for (let i = 0; i < a.length; i += 2) if (a[i] === 'type' && a[i + 1].toLowerCase() === 'password') secret.add(n.backendDOMNodeId);
        }),
    );

    // Label text repeats the name of the field it labels.
    const fieldNames = new Set(nodes.filter((n) => INTERACTIVE.has(n.role?.value)).map((n) => clean(n.name?.value)));
    const lines = [];
    let size = 0;
    let captcha = false;
    const add = (depth, line) => {
      if (size > 30000) return;
      lines.push('  '.repeat(depth) + line);
      size += line.length;
    };

    const walk = (n, depth, parentName) => {
      const role = n.role?.value ?? '';
      const name = clean(n.name?.value);
      if (/captcha|not a robot|verify you are human/i.test(name)) captcha = true;
      let next = depth;
      let nextName = parentName;
      if (!n.ignored && role === 'StaticText') {
        if (name && !parentName.includes(name) && !fieldNames.has(name)) add(depth, `- text: ${name}`);
        return;
      }
      if (!n.ignored && !FLATTEN.has(role) && (INTERACTIVE.has(role) || name || LANDMARKS.has(role))) {
        let line = `- ${role}${name ? ` "${clip(name, 160)}"` : ''}`;
        if (INTERACTIVE.has(role) && n.backendDOMNodeId) {
          let ref = r.byNode.get(n.backendDOMNodeId);
          if (!ref) r.byNode.set(n.backendDOMNodeId, (ref = `e${r.next++}`));
          const password = secret.has(n.backendDOMNodeId);
          r.byRef.set(ref, { ref, node: n.backendDOMNodeId, role, name, password });
          line += ` [${ref}]`;
          if (password) line += ' (password: off limits)';
        }
        for (const p of n.properties ?? []) {
          if (!STATES.includes(p.name)) continue;
          const v = p.value?.value;
          if (p.name === 'level') line += ` (level ${v})`;
          else if (v === true || v === 'true') line += ` (${p.name})`;
        }
        const value = clean(n.value?.value);
        if (value && !secret.has(n.backendDOMNodeId) && value !== name) line += `: ${clip(value, 120)}`;
        add(depth, line);
        next = depth + 1;
        nextName = name;
      }
      for (const id of n.childIds ?? []) {
        const child = byId.get(id);
        if (child) walk(child, next, nextName);
      }
    };
    walk(nodes[0], 0, '');

    const notes = [];
    if (size > 30000) notes.push('Snapshot truncated. Use read_page for the full text.');
    if (secret.size) notes.push('This page has a password field. You can\'t sign in yourself: call request_human.');
    if (captcha) notes.push('This page shows a CAPTCHA: call request_human.');
    return [`Tab ${tab.id}: ${h.views.get(tab.id)?.view.webContents.getTitle() || tab.title}`, `URL: ${tab.url}`, '', ...lines, ...(notes.length ? ['', ...notes] : [])].join('\n');
  }

  function element(tab, ref) {
    const el = refs.get(tab.id)?.byRef.get(ref);
    if (!el) throw new Error(`There's no element ${ref} on this page. Use the refs from the latest snapshot.`);
    return el;
  }

  async function center(cdp, el) {
    await cdp('DOM.getDocument', { depth: 0 });
    await cdp('DOM.scrollIntoViewIfNeeded', { backendNodeId: el.node }).catch(() => {});
    const { quads } = await cdp('DOM.getContentQuads', { backendNodeId: el.node }).catch(() => ({ quads: [] }));
    const q = quads[0];
    if (!q) throw new Error(`${el.ref} is gone or hidden. Take a new snapshot.`);
    return { x: (q[0] + q[2] + q[4] + q[6]) / 4, y: (q[1] + q[3] + q[5] + q[7]) / 4 };
  }

  // Show the user where the agent clicks, if they're watching.
  async function pointAt(tab, wc, p, color) {
    if (!h.isShown(tab.id)) return;
    await wc.executeJavaScriptInIsolatedWorld(WORLD, [{ code: ripple(p.x, p.y, color) }]).catch(() => {});
    await wait(180);
  }

  // What an action did: a snapshot if it navigated or opened a tab, otherwise a short note.
  async function result(space, tab, before, note, cdp, el) {
    const current = h.getTab(space.activeTabId);
    if (current && current.id !== tab.id) return `${note} It opened a new tab, ${current.id}, which is now your current tab.\n\n${await snapshot(current)}`;
    if (tab.url !== before) return `${note}\n\n${await snapshot(tab)}`;
    if (el && /checkbox|radio|switch/.test(el.role)) {
      const { nodes } = await cdp('Accessibility.getPartialAXTree', { backendNodeId: el.node, fetchRelatives: false }).catch(() => ({ nodes: [] }));
      const checked = nodes[0]?.properties?.find((p) => p.name === 'checked')?.value?.value;
      if (checked !== undefined) note += ` It is now ${checked === 'true' ? 'checked' : 'unchecked'}.`;
    }
    return `${note} The page didn't navigate.`;
  }

  // ------------------------------------------------------- asking the user

  function ask(session, ctx, req) {
    return new Promise((resolve) => {
      const r = { id: h.uid(), at: Date.now(), session: session.id, agent: session.key, name: session.name, spaceId: session.spaceId, ...req };
      const timer = setTimeout(() => r.finish('timeout'), req.kind === 'handoff' ? 15 * MIN : 5 * MIN);
      // Progress notifications keep clients that reset their timeout on progress waiting.
      const token = ctx.extra?._meta?.progressToken;
      let ticks = 0;
      const progress = token === undefined ? null : setInterval(() => {
        ctx.extra.sendNotification({ method: 'notifications/progress', params: { progressToken: token, progress: ++ticks, message: 'Waiting for the user' } }).catch(() => {});
      }, 5000);
      r.finish = (answer) => {
        clearTimeout(timer);
        clearInterval(progress);
        const i = requests.indexOf(r);
        if (i < 0) return;
        requests.splice(i, 1);
        h.changed();
        resolve(answer);
      };
      requests.push(r);
      if (process.platform === 'darwin') app.dock?.bounce('informational');
      h.changed();
    });
  }

  // Payments, sending messages and banking sites need the user's OK, unless they've
  // allowed this agent on the site for good.
  async function check(session, ctx, tab, reason, what) {
    if (!reason || prefs(session.key).sites[ctx.host] === 'allow') return;
    const answer = await ask(session, ctx, { kind: 'confirm', tabId: tab?.id ?? null, host: ctx.host, text: what, reason });
    ctx.asked = answer;
    if (answer === 'always') prefs(session.key).sites[ctx.host] = 'allow';
    if (answer === 'once' || answer === 'always') return;
    if (answer === 'timeout') throw fail('denied', `The user didn't answer in time, so you can't ${what}.`);
    throw fail('denied', `The user said no: you can't ${what}. Don't try again.`);
  }

  // ------------------------------------------------------------------- tools

  function identify(session, server) {
    const raw = server.server.getClientVersion()?.name || 'Agent';
    session.name = /claude/i.test(raw) ? 'Claude Code' : /codex/i.test(raw) ? 'Codex' : raw.replace(/[-_]+/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
    session.key = session.name.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  }

  function register(server, session) {
    const say = (ctx, summary, host) => {
      ctx.summary = summary;
      ctx.host = host;
      session.doing = summary;
      h.changed();
    };

    // acts: changes something, so it's paused while the user drives. quiet: not logged.
    const tool = (name, description, shape, fn, { acts = true, quiet = false } = {}) =>
      server.registerTool(name, { description, inputSchema: shape }, async (args, extra) => {
        const ctx = { summary: name, host: '', asked: null, extra, at: Date.now() };
        let outcome = 'ok';
        session.busy++;
        session.doing = 'Working';
        h.changed();
        try {
          if (acts && takenOver.has(spaceOf(session).id)) throw fail('paused', 'Paused: the user has taken over your space. Wait a minute and try again.');
          const out = await fn(args, ctx);
          return typeof out === 'string' ? { content: [{ type: 'text', text: out }] } : out;
        } catch (e) {
          outcome = e.outcome || 'failed';
          return { content: [{ type: 'text', text: e.message }], isError: true };
        } finally {
          session.busy--;
          session.lastAt = Date.now();
          if (!quiet) {
            log({
              at: ctx.at,
              agent: session.key,
              name: session.name,
              spaceId: session.spaceId,
              summary: ctx.summary,
              host: ctx.host,
              outcome: outcome === 'ok' && ctx.asked ? 'approved' : outcome,
              read: !acts,
            });
          }
          setTimeout(h.changed, IDLE_AFTER + 100);
          h.changed();
        }
      });

    const tabArg = z.string().optional().describe('Tab id from open_tab or list_tabs. Defaults to your current tab.');
    const refArg = z.string().describe('Element ref from the latest snapshot, like e12');

    tool('list_tabs', 'List the tabs in your space. * marks your current tab.', {}, async () => {
      const space = spaceOf(session);
      const tabs = S().tabs.filter((t) => t.spaceId === space.id);
      if (!tabs.length) return 'Your space has no tabs. Use open_tab.';
      return tabs.map((t) => `${t.id === space.activeTabId ? '*' : ' '} ${t.id}  ${clip(t.title, 80)}  ${t.url}`).join('\n');
    }, { acts: false, quiet: true });

    tool('open_tab', 'Open a URL, or search for words, in a new tab in your space. It becomes your current tab. Returns a snapshot of the page.', { url: z.string().describe('URL or search words') }, async ({ url }, ctx) => {
      const space = spaceOf(session);
      const target = h.toURL(url);
      say(ctx, `Open ${clip(target.replace(/^https?:\/\//, ''), 50)}`, hostOf(target));
      webOnly(target);
      await check(session, ctx, null, BANKING.test(ctx.host) && 'Banking site', `open ${ctx.host}`);
      const tab = h.newTab(target, { background: true, spaceId: space.id });
      follow(space, tab);
      return snapshot(tab);
    });

    tool('navigate', 'Go to a URL or search for words in a tab. Use "back" or "forward" to move through its history. Returns a snapshot.', { url: z.string().describe('URL, search words, "back" or "forward"'), tab: tabArg }, async ({ url, tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      const { wc } = await page(tab);
      if (url === 'back' || url === 'forward') {
        say(ctx, url === 'back' ? 'Go back' : 'Go forward', hostOf(tab.url));
        if (url === 'back') wc.navigationHistory.goBack();
        else wc.navigationHistory.goForward();
        await afterInput(wc);
      } else {
        const target = h.toURL(url);
        say(ctx, `Go to ${clip(target.replace(/^https?:\/\//, ''), 50)}`, hostOf(target));
        webOnly(target);
        await check(session, ctx, tab, BANKING.test(ctx.host) && 'Banking site', `open ${ctx.host}`);
        await wc.loadURL(target).catch(() => {}); // redirects reject; the page is what matters
        await settled(wc, 20000);
      }
      return snapshot(tab);
    });

    tool('snapshot', 'Get the page as an accessibility tree. Elements you can act on have refs like [e12].', { tab: tabArg }, async ({ tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      say(ctx, `Look at ${clip(tab.title, 40)}`, hostOf(tab.url));
      return snapshot(tab);
    }, { acts: false });

    tool('read_page', "Get the page's visible text, for reading articles and long pages.", { tab: tabArg }, async ({ tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      say(ctx, `Read ${clip(tab.title, 40)}`, hostOf(tab.url));
      const { wc } = await page(tab);
      const text = await wc.executeJavaScriptInIsolatedWorld(WORLD, [{ code: 'document.body ? document.body.innerText : ""' }]);
      return `Tab ${tab.id}: ${tab.title}\nURL: ${tab.url}\n\n${String(text).slice(0, 40000)}`;
    }, { acts: false });

    tool('screenshot', 'Take a screenshot of a tab, for when layout or images matter.', { tab: tabArg }, async ({ tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      say(ctx, `Screenshot ${clip(tab.title, 40)}`, hostOf(tab.url));
      const { wc } = await page(tab);
      let img = await wc.capturePage(undefined, { stayHidden: true });
      if (img.getSize().width > 1280) img = img.resize({ width: 1280 });
      return { content: [{ type: 'image', data: img.toJPEG(75).toString('base64'), mimeType: 'image/jpeg' }] };
    }, { acts: false });

    tool('click', 'Click an element by its ref.', { ref: refArg, tab: tabArg }, async ({ ref, tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      const el = element(tab, ref);
      say(ctx, `Click “${clip(el.name, 40) || el.role}”`, hostOf(tab.url));
      await check(session, ctx, tab, risk(ctx.host, el.name), `click “${clip(el.name, 40)}” on ${ctx.host}`);
      const { wc, cdp } = await page(tab);
      const before = tab.url;
      const p = await center(cdp, el);
      await pointAt(tab, wc, p, space.color);
      await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y });
      for (const type of ['mousePressed', 'mouseReleased']) await cdp('Input.dispatchMouseEvent', { type, x: p.x, y: p.y, button: 'left', clickCount: 1 });
      await afterInput(wc);
      return result(space, tab, before, `Clicked ${el.role} “${clip(el.name)}”.`, cdp, el);
    });

    tool('type', 'Type text into a field by its ref, replacing what is there. Set submit to press Enter afterwards.', { ref: refArg, text: z.string(), submit: z.boolean().optional(), tab: tabArg }, async ({ ref, text, submit, tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      const el = element(tab, ref);
      say(ctx, `Type “${clip(text, 30)}”${submit ? ' and submit' : ''}`, hostOf(tab.url));
      if (el.password) throw fail('denied', 'Password fields are off limits to agents. Call request_human so the user can sign in.');
      if (submit) await check(session, ctx, tab, risk(ctx.host, 'send'), `send “${clip(text, 40)}” on ${ctx.host}`);
      const { wc, cdp } = await page(tab);
      const before = tab.url;
      const p = await center(cdp, el);
      await pointAt(tab, wc, p, space.color);
      const { object } = await cdp('DOM.resolveNode', { backendNodeId: el.node });
      await cdp('Runtime.callFunctionOn', { objectId: object.objectId, functionDeclaration: SELECT_CONTENTS });
      await cdp('Input.insertText', { text });
      if (submit) {
        await press(cdp, 'Enter');
        await afterInput(wc);
      }
      return result(space, tab, before, `Typed into “${clip(el.name)}”${submit ? ' and pressed Enter' : ''}.`, cdp);
    });

    tool('select_option', 'Choose an option in a dropdown by its ref and the option text.', { ref: refArg, option: z.string(), tab: tabArg }, async ({ ref, option, tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      const el = element(tab, ref);
      say(ctx, `Choose “${clip(option, 30)}”`, hostOf(tab.url));
      const { cdp } = await page(tab);
      const { object } = await cdp('DOM.resolveNode', { backendNodeId: el.node });
      const { result: chosen } = await cdp('Runtime.callFunctionOn', { objectId: object.objectId, functionDeclaration: CHOOSE_OPTION, arguments: [{ value: option }], returnByValue: true });
      if (!chosen.value) throw new Error(`“${option}” isn't an option in ${ref}. Check the snapshot for the options.`);
      return `Chose “${chosen.value}” in “${clip(el.name)}”.`;
    });

    tool('press_key', 'Press a key in a tab.', { key: z.enum(Object.keys(KEYS)), tab: tabArg }, async ({ key, tab: id }, ctx) => {
      const { space, tab } = tabOf(session, id);
      follow(space, tab);
      say(ctx, `Press ${key}`, hostOf(tab.url));
      if (key === 'Enter') await check(session, ctx, tab, risk(ctx.host, 'send'), `press Enter to send on ${ctx.host}`);
      const { wc, cdp } = await page(tab);
      const before = tab.url;
      await press(cdp, key);
      await afterInput(wc);
      return result(space, tab, before, `Pressed ${key}.`, cdp);
    });

    tool('close_tab', 'Close a tab in your space. It moves to the archive, where the user can still find it.', { tab: tabArg }, async ({ tab: id }, ctx) => {
      const { tab } = tabOf(session, id);
      say(ctx, `Close ${clip(tab.title, 40)}`, hostOf(tab.url));
      h.archiveTab(tab.id, 'agent');
      return 'Closed.';
    });

    tool(
      'request_human',
      "Hand a tab to the user: to sign in, pass 2FA or a CAPTCHA, or decide something you shouldn't. The tab is shown to them, and this waits until they hand it back (up to 15 minutes).",
      { reason: z.string().describe('What you need the user to do, in one short sentence'), tab: tabArg },
      async ({ reason, tab: id }, ctx) => {
        const { space, tab } = tabOf(session, id);
        follow(space, tab);
        say(ctx, `Asked you: ${clip(reason, 60)}`, hostOf(tab.url));
        const answer = await ask(session, ctx, { kind: 'handoff', tabId: tab.id, host: ctx.host, text: reason });
        if (answer !== 'done') return `The user didn't take over (${answer}). Carry on without it, or stop and tell them.`;
        return `The user handed the tab back.\n\n${await snapshot(h.getTab(tab.id) ?? tab)}`;
      },
      { acts: false },
    );
  }

  // ------------------------------------------------------------------ server

  function serve(socket) {
    const session = { id: h.uid(), key: null, name: 'Agent', spaceId: null, busy: 0, doing: '', lastAt: 0 };
    const server = new McpServer({ name: 'arcish', version: '0.1.0' }, { instructions: INSTRUCTIONS });
    server.server.oninitialized = () => {
      identify(session, server);
      sessions.set(session.id, session);
      h.changed();
    };
    register(server, session);
    const transport = new StdioServerTransport(socket, socket);
    server.connect(transport).catch((e) => console.error('Agent connection failed:', e.message));
    socket.on('error', () => {});
    socket.on('close', () => {
      sessions.delete(session.id);
      for (const r of requests.filter((r) => r.session === session.id)) r.finish('gone');
      server.close().catch(() => {});
      h.changed();
    });
  }

  const sock = socketPath(h.userData);
  fs.mkdirSync(h.userData, { recursive: true });
  fs.rmSync(sock, { force: true });
  const listener = net.createServer(serve);
  listener.on('error', (e) => console.error('Agent server unavailable:', e.message));
  // The socket file is created rw------- so only this user can connect. Binding happens
  // inside listen(), so the umask only needs to change around that call.
  const mask = process.umask(0o177);
  try {
    listener.listen(sock);
  } finally {
    process.umask(mask);
  }

  // ------------------------------------------------------------- UI surface

  const agentOf = (spaceId) => h.getSpace(spaceId)?.agent;

  return {
    socket: sock,

    stop() {
      listener.close();
      fs.rmSync(sock, { force: true });
    },

    // A page in an agent's tab opened a popup: the agent works there next.
    opened(opener, tab) {
      const space = h.getSpace(opener.spaceId);
      if (space) follow(space, tab);
    },

    answer(id, answer) {
      requests.find((r) => r.id === id)?.finish(answer);
    },

    show(id) {
      const r = requests.find((r) => r.id === id);
      if (!r?.tabId) return;
      if (r.kind === 'handoff') takenOver.add(r.spaceId);
      h.activateTab(r.tabId);
      h.changed();
    },

    takeOver(spaceId) {
      takenOver.add(spaceId);
      log({ agent: agentOf(spaceId), name: 'You', spaceId, summary: 'Took over', outcome: 'you' });
    },

    handBack(spaceId) {
      takenOver.delete(spaceId);
      for (const r of requests.filter((r) => r.kind === 'handoff' && r.spaceId === spaceId)) r.finish('done');
      log({ agent: agentOf(spaceId), name: 'You', spaceId, summary: 'Handed back', outcome: 'you' });
    },

    revoke(agent, host) {
      delete prefs(agent).sites[host];
      h.changed();
    },

    uiState() {
      const s = S();
      const now = Date.now();
      const spaces = {};
      for (const sp of s.spaces.filter((x) => x.kind === 'agent')) {
        const live = [...sessions.values()].filter((x) => x.key === sp.agent);
        const busy = live.find((x) => x.busy);
        const thinking = live.some((x) => now - x.lastAt < IDLE_AFTER);
        const waiting = requests.find((r) => r.spaceId === sp.id);
        const status = waiting
          ? waiting.kind === 'handoff' ? 'Waiting for you' : 'Waiting for your OK'
          : takenOver.has(sp.id) ? "Paused while you're driving"
          : busy ? busy.doing
          : thinking ? 'Thinking…'
          : live.length ? 'Idle'
          : 'Not connected';
        spaces[sp.id] = {
          status,
          connected: live.length > 0,
          working: !waiting && !takenOver.has(sp.id) && (!!busy || thinking),
          needsYou: !!waiting,
          takenOver: takenOver.has(sp.id),
          sites: Object.keys(prefs(sp.agent).sites),
        };
      }
      return {
        spaces,
        requests: requests.map(({ finish, session, ...r }) => r),
        activity: (s.activity ?? []).slice(0, 60),
      };
    },
  };
};
