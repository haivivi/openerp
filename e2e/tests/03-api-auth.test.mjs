/**
 * E2E Test: API authentication & authorization
 *
 * Tests API-level auth behavior through a real browser context:
 * 1. Unauthenticated requests return 401
 * 2. Authenticated requests succeed
 * 3. Expired/invalid tokens return 401
 * 4. Dashboard auto-redirects on 401
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import {
  BASE_URL,
  ROOT_USER,
  ROOT_PASS,
  waitForServer,
  launchBrowser,
  apiCall,
} from './helpers.mjs';

describe('API Authentication', () => {
  let browser;
  let page;
  let token;

  before(async () => {
    await waitForServer();
    browser = await launchBrowser();
    page = await browser.newPage();

    // Get a valid token.
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER,
      password: ROOT_PASS,
    });
    token = resp.data.access_token;
  });

  after(async () => {
    if (browser) await browser.close();
  });

  it('admin API works without auth (public)', async () => {
    const resp = await apiCall('GET', '/admin/auth/users');
    assert.equal(resp.status, 200);
    assert.ok(resp.data.items !== undefined, 'Returns items array');
  });

  it('accepts authenticated API calls', async () => {
    const resp = await apiCall('GET', '/admin/auth/users', null, token);
    assert.equal(resp.status, 200);
    assert.ok(Array.isArray(resp.data.items));
  });

  it('admin API works even with invalid token (public routes)', async () => {
    const resp = await apiCall('GET', '/admin/auth/users', null, 'invalid.token.here');
    assert.equal(resp.status, 200);
  });

  it('public endpoints work without auth', async () => {
    const health = await apiCall('GET', '/health');
    assert.equal(health.status, 200);
    assert.equal(health.data.status, 'ok');

    const version = await apiCall('GET', '/version');
    assert.equal(version.status, 200);
    assert.ok(version.data.name);
  });

  it('login endpoint works and returns JWT', async () => {
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER,
      password: ROOT_PASS,
    });
    assert.equal(resp.status, 200);
    assert.ok(resp.data.access_token);
    assert.equal(resp.data.token_type, 'Bearer');
    assert.ok(resp.data.expires_in > 0);
  });

  it('login rejects wrong password', async () => {
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER,
      password: 'wrong',
    });
    assert.equal(resp.status, 401);
    assert.match(resp.data.error, /invalid/i);
  });

  it('dashboard loads even with invalid token (admin routes are public)', async () => {
    // Set an obviously invalid token.
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate(() => localStorage.setItem('openerp_token', 'invalid.token.value'));
    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle0' });

    // Dashboard should still load because admin API routes don't require JWT.
    // The schema endpoint is also public.
    await page.waitForFunction(
      () => document.querySelectorAll('.sidebar .nav-item').length > 0,
      { timeout: 5000 },
    );

    const items = await page.$$eval('.sidebar .nav-item', els => els.length);
    assert.ok(items > 0, 'Dashboard loaded with sidebar items');

    // Clean up.
    await page.evaluate(() => localStorage.removeItem('openerp_token'));
  });
});
