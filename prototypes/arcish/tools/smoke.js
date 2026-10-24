// Scripted run used during development: ARCISH_SMOKE=<out-dir> npm start
const { app } = require('electron');
const { execFileSync } = require('node:child_process');
const path = require('node:path');

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

module.exports = async (h) => {
  const out = process.env.ARCISH_SMOKE;
  const id = h.win.getMediaSourceId().split(':')[1];
  const shot = (name) => {
    try {
      execFileSync('screencapture', ['-x', '-o', '-l', id, path.join(out, `${name}.png`)]);
      console.log('shot', name);
    } catch (e) {
      console.log('screencapture failed', e.message);
    }
  };
  const S = h.state();
  const log = (label) =>
    console.log(label, JSON.stringify({ space: S.spaces.find((s) => s.id === S.activeSpaceId).name, live: [...h.views.keys()].length, tabs: S.tabs.map((t) => `${t.pinned ? 'P' : t.favorite ? 'F' : '-'} ${t.title.slice(0, 30)}`) }));

  h.win.focus();
  await wait(6000);
  log('start');
  shot('1-start');

  h.openPalette('new');
  await wait(800);
  await h.getPalette().webContents.executeJavaScript(`(() => { const i = document.getElementById('q'); i.value = 'wiki'; i.dispatchEvent(new Event('input')); return document.querySelectorAll('.item').length })()`).then((n) => console.log('palette items', n));
  await wait(400);
  shot('2-palette');
  h.closePalette();

  const hn = S.tabs.find((t) => t.url.includes('ycombinator'));
  h.splitWith(hn.id);
  await wait(3500);
  shot('3-split');

  const work = S.spaces[1];
  h.activateSpace(work.id);
  await wait(3500);
  log('work');
  shot('4-work-space');

  h.discardBackground();
  await wait(2500);
  log('after sleep');
  shot('5-after-sleep');

  h.toggleSidebar();
  await wait(600);
  shot('6-no-sidebar');
  h.toggleSidebar();
  await wait(300);
  app.quit();
};
