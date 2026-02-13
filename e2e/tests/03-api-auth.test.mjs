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

  it('rejects unauthenticated API calls with 401', async () => {
    const resp = await apiCall('GET', '/auth/users');
    assert.equal(resp.status, 401);
    assert.match(resp.data.error, /missing/i);
  });

  it('accepts authenticated API calls', async () => {
    const resp = await apiCall('GET', '/auth/users', null, token);
    assert.equal(resp.status, 200);
    assert.ok(Array.isArray(resp.data.items));
  });

  it('rejects invalid tokens with 401', async () => {
    const resp = await apiCall('GET', '/auth/users', null, 'invalid.token.here');
    assert.equal(resp.status, 401);
    assert.match(resp.data.error, /invalid/i);
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

  it('dashboard redirects to login when token expires in browser', async () => {
    // Set an obviously invalid token.
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate(() => localStorage.setItem('openerp_token', 'expired.token.value'));
    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle0' });

    // The dashboard JS should detect 401 from API calls and redirect to login.
    // Wait a moment for the redirect to happen.
    await new Promise(r => setTimeout(r, 2000));

    // Check: either redirected to login or still on dashboard with invalid token.
    // The dashboard makes API calls which will return 401 and trigger redirect.
    const url = page.url();
    const tokenGone = await page.evaluate(() => localStorage.getItem('openerp_token'));
    // The token should be cleared if any API call returned 401.
    assert.ok(
      url.endsWith('/') || tokenGone === null,
      `Expected redirect to login or token cleared, got url=${url}, token=${tokenGone}`,
    );
  });
});
