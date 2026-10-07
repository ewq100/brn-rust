// Interaction check for the BRN design prototype (docs/design/src/prototypes).
// Not a product test: it verifies the prototype's documented journeys, states,
// keyboard focus and absence of console errors, and saves screenshots.
//
// Usage (no repository dependency is added):
//   npm install --prefix /tmp/brn-pw playwright-core
//   NODE_PATH=/tmp/brn-pw/node_modules node scripts/design-prototype-check.js OUTPUT_DIR
// Browser: BRN_CHROMIUM, else the newest Playwright headless shell in the user cache.
const { chromium } = require('playwright-core');
const fs = require('fs');
const path = require('path');
function browserPath() {
  if (process.env.BRN_CHROMIUM) return process.env.BRN_CHROMIUM;
  const cache = path.join(process.env.HOME, 'Library/Caches/ms-playwright');
  for (const dir of fs.readdirSync(cache).filter(d => d.startsWith('chromium_headless_shell-')).sort().reverse()) {
    for (const sub of fs.readdirSync(path.join(cache, dir))) {
      const exe = path.join(cache, dir, sub, 'chrome-headless-shell');
      if (fs.existsSync(exe)) return exe;
    }
  }
  throw new Error('Set BRN_CHROMIUM to a Chromium executable');
}
const exe = browserPath();
const file = 'file://' + path.resolve(__dirname, '../docs/design/src/prototypes/brn-prototype.html');
const out = process.argv[2] || '.';
fs.mkdirSync(out, { recursive: true });
(async () => {
  const browser = await chromium.launch({ executablePath: exe });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  const ok = (cond, msg) => { if (!cond) { console.log('FAIL', msg); process.exitCode = 1; } else console.log('ok  ', msg); };
  const shot = n => page.screenshot({ path: path.join(out, n + '.png') });
  await page.goto(file + '#today'); await page.waitForTimeout(200);
  ok(await page.isVisible('text=PROTOTYPE'), 'prototype banner visible');
  ok(await page.isVisible('text=Suggested focus'), 'Today shows suggested focus');
  await shot('proto-01-today');
  // Journey A: draft reply -> review -> comment/rewrite -> approve -> sent
  await page.click('text=Draft reply'); await page.waitForTimeout(300);
  ok(await page.isVisible('#stop'), 'Stop visible while working');
  await page.waitForTimeout(3600);
  ok(await page.isVisible('text=Open reply proposal'), 'draft answer completes with proposal link');
  ok(await page.isHidden('#stop'), 'Stop hidden after completion');
  await shot('proto-02-chat-provenance');
  await page.click('text=Open reply proposal'); await page.waitForSelector('.doc .label');
  ok((await page.textContent('.doc .label >> nth=0')).includes('version 1'), 'review opens at v1');
  await page.click('.draft .chip.t-danger'); ok(await page.isVisible('text=Where this claim comes from'), 'claim chip explains provenance');
  await page.keyboard.press('Escape');
  await shot('proto-03-review-v1');
  await page.click('text=Rewrite with comments'); await page.waitForTimeout(200);
  ok((await page.textContent('#phase')).includes('Rewriting'), 'Rewrite shows phase');
  await page.waitForTimeout(1800);
  ok((await page.textContent('.doc .label >> nth=0')).includes('version 2'), 'Rewrite produces v2');
  await page.click('text=Review exact approval…');
  ok(await page.isVisible('text=Approve reply version 2?'), 'approval binds displayed version');
  await page.click('#confirm');
  ok(await page.isVisible('text=Proposal · Approved'), 'approved state');
  ok(await page.isVisible('text=The Action stays Open'), 'action stays open after approval');
  await page.click('text=I sent it — complete Action…'); await page.click('#confirm');
  ok(await page.isVisible('button:has-text("Action completed")'), 'explicit completion');
  await shot('proto-04-review-approved');
  // Inbox group approval
  await page.goto(file + '#inbox'); await page.waitForTimeout(150);
  ok(await page.isVisible('text=Partial conversion.'), 'partial conversion flagged');
  await page.click('text=Approve 5 selected…'); await page.click('#confirm');
  ok(await page.isVisible('text=Review original-copy cleanup…'), 'cleanup offered only after approval');
  await page.goto(file + '#inbox'); await page.reload(); await page.waitForTimeout(150);
  await shot('proto-05-inbox-group');
  // Needs review
  await page.goto(file + '#needs-review'); await page.waitForTimeout(150);
  await shot('proto-06-needs-review');
  await page.click('text=Review proposed update…'); await page.click('#confirm');
  ok((await page.textContent('#statusText')).includes('history'), 'supersession keeps history');
  for (const v of ['project', 'person', 'graph', 'sessions', 'activity']) {
    await page.goto(file + '#' + v); await page.waitForTimeout(120);
    ok(await page.isVisible('.view-head h1'), v + ' renders');
    if (v === 'project' || v === 'sessions') await shot('proto-07-' + v);
  }
  await page.goto(file + '#sessions'); await page.waitForTimeout(120); await page.click('button:has-text("Delete…") >> nth=3'); ok(await page.isVisible('text=likely outcomes'), 'delete warns about uncaptured outcomes'); await page.keyboard.press('Escape');
  // States
  for (const s of ['empty', 'loading', 'error', 'offline']) {
    await page.goto(file.replace('#', '') + '?state=' + s + '#today'); await page.waitForTimeout(150);
    ok((await page.$eval('#state', e => e.value)) === s, 'state ' + s);
  }
  await page.goto(file + '?state=offline#review'); await page.waitForTimeout(150);
  ok(await page.isDisabled('text=Rewrite with comments'), 'offline disables Rewrite only');
  ok(await page.isEnabled('text=Review exact approval…'), 'offline keeps approval');
  await shot('proto-08-offline-review');
  await page.goto(file + '?state=error#chat'); await page.waitForTimeout(150);
  await shot('proto-09-error-chat');
  await page.goto(file + '?scheme=light#today'); await page.waitForTimeout(150);
  await shot('proto-10-today-light');
  // Keyboard: tab reaches nav and focus ring is visible
  await page.goto(file + '#today'); await page.keyboard.press('Tab'); await page.keyboard.press('Tab');
  const outline = await page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
  ok(outline !== 'none', 'keyboard focus ring visible');
  await page.keyboard.press('Meta+n'); await page.waitForTimeout(100);
  ok(await page.isVisible('text=What are you working on?'), '⌘N opens a new chat');
  // narrow
  await page.setViewportSize({ width: 900, height: 700 }); await page.goto(file + '#review'); await page.waitForTimeout(150);
  await shot('proto-11-narrow-review');
  ok(errors.length === 0, 'no console/page errors ' + JSON.stringify(errors));
  await browser.close();
})();
