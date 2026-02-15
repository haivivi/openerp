/**
 * E2E Browser Test: Dashboard DSL Polish features.
 *
 * Drives Lightpanda via Puppeteer to verify:
 * - Login + dashboard loads
 * - Pagination (hasMore, Prev/Next buttons)
 * - @count badges
 * - PATCH partial update + rev
 * - Optimistic locking 409
 *
 * Usage:
 *   LIGHTPANDA_WS=ws://127.0.0.1:9222 BASE_URL=http://localhost:8088 \
 *   node --test dashboard.test.mjs
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import puppeteer from 'puppeteer';

const BASE_URL = process.env.BASE_URL || 'http://localhost:8088';
const ROOT_USER = 'root';
const ROOT_PASS = process.env.ROOT_PASS || 'openerp123';

/** Make an API call directly (bypassing browser). */
async function api(method, path, body, token) {
  const opts = {
    method,
    headers: { 'Content-Type': 'application/json' },
  };
  if (token) opts.headers['Authorization'] = `Bearer ${token}`;
  if (body) opts.body = JSON.stringify(body);
  const resp = await fetch(`${BASE_URL}${path}`, opts);
  const text = await resp.text();
  let data = null;
  if (text) { try { data = JSON.parse(text); } catch(e) {} }
  return { status: resp.status, data };
}

/** Login via API, return JWT token. */
async function getToken() {
  const { data } = await api('POST', '/auth/login', {
    username: ROOT_USER,
    password: ROOT_PASS,
  });
  return data?.access_token || data?.token;
}

describe('Dashboard DSL Polish (Lightpanda)', () => {
  let browser;
  let context;
  let page;
  let token;

  before(async () => {
    // Launch Chromium (puppeteer auto-downloads it).
    browser = await puppeteer.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage'],
    });

    // Get API token.
    token = await getToken();
    assert.ok(token, 'should get JWT token');

    page = await browser.newPage();

    // Navigate to login page, inject token, then go to dashboard.
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate((t) => {
      localStorage.setItem('openerp_token', t);
    }, token);
    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle0' });

    // Wait for schema to load and sidebar to render.
    await page.waitForFunction(
      () => document.querySelectorAll('.sidebar .nav-item').length > 0,
      { timeout: 10000 },
    );
  });

  after(async () => {
    // Clean up test records.
    if (token) {
      const { data } = await api('GET', '/admin/auth/users', null, token);
      if (data?.items) {
        for (const u of data.items) {
          if (u.displayName?.startsWith('E2E LP')) {
            await api('DELETE', `/admin/auth/users/${u.id}`, null, token);
          }
        }
      }
    }
    if (browser) await browser.close();
  });

  // ── 1. Dashboard loads ──

  it('loads schema and renders sidebar', async () => {
    const items = await page.$$eval('.sidebar .nav-item', els => els.map(e => e.textContent));
    assert.ok(items.length >= 2, `Expected >= 2 sidebar items, got: ${items}`);
  });

  // ── 2. @count badges ──

  it('shows @count badges on sidebar', async () => {
    // Wait a moment for async count requests.
    await new Promise(r => setTimeout(r, 1000));
    const badges = await page.$$('.sidebar-count');
    assert.ok(badges.length > 0, 'Expected sidebar count badges');
  });

  // ── 3. Pagination UI ──

  it('has pagination controls', async () => {
    // Click Users in sidebar.
    await page.evaluate(() => {
      const items = document.querySelectorAll('.sidebar .nav-item');
      for (const i of items) { if (/user/i.test(i.textContent)) { i.click(); break; } }
    });
    await new Promise(r => setTimeout(r, 1000));

    // Pagination bar should exist.
    const prevBtn = await page.$('#prevBtn');
    const nextBtn = await page.$('#nextBtn');
    assert.ok(prevBtn, 'Prev button exists');
    assert.ok(nextBtn, 'Next button exists');
  });

  // ── 4. List API uses hasMore ──

  it('list API returns hasMore field', async () => {
    const { status, data } = await api('GET', '/admin/auth/users?limit=1&offset=0', null, token);
    assert.equal(status, 200);
    assert.ok('hasMore' in data, 'response should have hasMore');
    assert.ok('items' in data, 'response should have items');
  });

  // ── 5. @count endpoint ──

  it('@count endpoint returns count', async () => {
    const { status, data } = await api('GET', '/admin/auth/users/@count', null, token);
    assert.equal(status, 200);
    assert.ok(typeof data.count === 'number', `count should be number, got: ${typeof data.count}`);
  });

  // ── 6. Create record ──

  let createdId;

  it('creates a record via dialog', async () => {
    // Click Add button.
    await page.evaluate(() => {
      const btn = document.querySelector('.btn-sm-primary');
      if (btn) btn.click();
    });

    // Wait for dialog.
    await page.waitForFunction(
      () => document.getElementById('createDlg')?.classList.contains('open'),
      { timeout: 3000 },
    );

    // Fill display_name.
    await page.type('#dlgForm input[name="display_name"]', 'E2E LP Dashboard');

    // Submit.
    await page.click('#dlgSubmit');

    // Wait for dialog to close.
    await page.waitForFunction(
      () => !document.getElementById('createDlg')?.classList.contains('open'),
      { timeout: 5000 },
    );
    await new Promise(r => setTimeout(r, 500));

    // Verify via API.
    const { data } = await api('GET', '/admin/auth/users', null, token);
    const user = data.items.find(u => u.displayName === 'E2E LP Dashboard');
    assert.ok(user, 'Created user should exist');
    createdId = user.id;
  });

  // ── 7. rev=1 on create ──

  it('created record has rev=1', async () => {
    assert.ok(createdId, 'need created record');
    const { data } = await api('GET', `/admin/auth/users/${createdId}`, null, token);
    assert.equal(data.rev, 1, 'rev should be 1 after create');
  });

  // ── 8. PATCH partial update ──

  it('PATCH updates only changed fields', async () => {
    assert.ok(createdId, 'need created record');
    const patch = { displayName: 'E2E LP Patched', rev: 1 };
    const { status, data } = await api('PATCH', `/admin/auth/users/${createdId}`, patch, token);
    assert.equal(status, 200, 'PATCH should succeed');
    assert.equal(data.displayName, 'E2E LP Patched');
    assert.equal(data.rev, 2, 'rev should be bumped to 2');
  });

  // ── 9. Stale rev → 409 ──

  it('stale rev returns 409 Conflict', async () => {
    assert.ok(createdId, 'need created record');
    const patch = { displayName: 'Should Fail', rev: 1 };
    const { status } = await api('PATCH', `/admin/auth/users/${createdId}`, patch, token);
    assert.equal(status, 409, 'stale rev should return 409');
  });

  // ── 10. Count badge updates ──

  it('count badge shows in section header', async () => {
    // Navigate to Users to trigger count load.
    await page.evaluate(() => {
      const items = document.querySelectorAll('.sidebar .nav-item');
      for (const i of items) { if (/user/i.test(i.textContent)) { i.click(); break; } }
    });
    await new Promise(r => setTimeout(r, 1000));

    const badge = await page.$('#countBadge');
    assert.ok(badge, 'Count badge should exist in section header');
  });
});
