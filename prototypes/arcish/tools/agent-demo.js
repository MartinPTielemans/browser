// Scripted user for recording the agent demo: ARCISH_AGENT_DEMO=<out-dir> npm start
// Records the window while an agent works. The "user" stays in their own space for a while,
// then switches over to watch, answers requests through the sidebar's buttons, signs in when
// asked, and quits once the agent disconnects. tools/agent-video.sh turns the frames into a video.
// ARCISH_AGENT_ANSWER=deny makes the user refuse every confirmation, for runs on real sites.
const { app } = require('electron');
const { execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

module.exports = async (h) => {
  const out = process.env.ARCISH_AGENT_DEMO;
  const frames = path.join(out, 'frames');
  fs.mkdirSync(frames, { recursive: true });
  const windowId = h.win.getMediaSourceId().split(':')[1];
  const capture = (file) => new Promise((resolve) => execFile('screencapture', ['-x', '-o', '-t', 'jpg', '-l', windowId, file], () => resolve()));
  const still = (name) => capture(path.join(out, `${name}.jpg`)).then(() => console.log('still', name));
  const click = (selector) => h.ui.webContents.executeJavaScript(`document.querySelector(${JSON.stringify(selector)})?.click()`);

  // Record frames with their timestamps so the video plays back at real speed.
  let recording = true;
  const times = [];
  (async () => {
    while (recording) {
      const t = Date.now();
      await capture(path.join(frames, `${String(times.length).padStart(5, '0')}.jpg`));
      times.push(t);
      await wait(Math.max(0, 300 - (Date.now() - t)));
    }
  })();

  // A person typing, not a script setting values. Replaces what's there: the agent may have
  // filled in the username already.
  async function typeInto(wc, selector, text) {
    await wc.executeJavaScript(`{ const el = document.querySelector(${JSON.stringify(selector)}); el.focus(); el.select(); }`);
    wc.focus();
    for (const ch of text) {
      wc.sendInputEvent({ type: 'char', keyCode: ch });
      await wait(60);
    }
  }

  async function answer(r) {
    await wait(1200);
    await still(`confirm-${r.id}`);
    await wait(2800);
    const reply = process.env.ARCISH_AGENT_ANSWER || 'once';
    await click(`[data-answer="${reply}"][data-req="${r.id}"]`);
    console.log(reply, r.text);
  }

  async function takeOver(r) {
    await wait(1200);
    await still('handoff');
    await wait(2800);
    await click(`[data-show="${r.id}"]`);
    await wait(1800);
    const wc = h.views.get(r.tabId)?.view.webContents;
    if (wc && /the-internet/.test(wc.getURL())) {
      await typeInto(wc, '#username', 'tomsmith');
      await typeInto(wc, '#password', 'SuperSecretPassword!');
      await still('user-signing-in');
      await wait(800);
      await wc.executeJavaScript(`document.querySelector('button[type=submit]').click()`);
    }
    await wait(2500);
    await click(`[data-hand-back="${r.spaceId}"]`);
    console.log('handed back');
  }

  await wait(1500);
  await still('start');
  const seen = new Set();
  let started = 0;
  let watching = false;
  let connected = false;
  for (;;) {
    await wait(250);
    const ui = h.agents.uiState();
    const space = h.state().spaces.find((s) => s.kind === 'agent');
    const info = space && ui.spaces[space.id];
    if (info?.connected) connected = true;
    if (info?.working && !started) {
      started = Date.now();
      setTimeout(() => still('working-in-background'), 4000);
    }
    if (started && !watching && Date.now() - started > 10000) {
      watching = true;
      h.activateSpace(space.id);
      setTimeout(() => still('watching'), 2500);
    }
    for (const r of ui.requests) {
      if (seen.has(r.id)) continue;
      seen.add(r.id);
      (r.kind === 'handoff' ? takeOver : answer)(r);
    }
    if (connected && !info?.connected && !ui.requests.length) break;
  }
  await wait(2500);
  await still('done');
  recording = false;
  await wait(600);

  // ffmpeg concat list: each frame shown until the next one was taken.
  const list = times.map((t, i) => `file 'frames/${String(i).padStart(5, '0')}.jpg'\nduration ${(((times[i + 1] ?? t + 300) - t) / 1000).toFixed(3)}`);
  fs.writeFileSync(path.join(out, 'frames.txt'), `${list.join('\n')}\n`);
  console.log('frames', times.length);
  app.quit();
};
