/**
 * E2E Test: Login flow
 *
 * Tests:
 * 1. Login page renders correctly
 * 2. Wrong password shows error
 * 3. Correct password redirects to dashboard
 * 4. Dashboard shows user info
 * 5. Logout returns to login page
 * 6. Already-logged-in redirects to dashboard
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import {
  BASE_URL,
  ROOT_USER,
  ROOT_PASS,
  waitForServer,
  launchBrowser,
} from './helpers.mjs';

describe('Login flow', () => {
  let browser;
  let page;

  before(async () => {
    await waitForServer();
    browser = await launchBrowser();
    page = await browser.newPage();
    // Clear any lingering state.
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate(() => localStorage.clear());
  });

  after(async () => {
    if (browser) await browser.close();
  });

  it('renders login page with form', async () => {
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });

    // Title check.
    const title = await page.title();
    assert.match(title, /OpenERP/);

    // Form elements exist.
    const username = await page.$('#username');
    const password = await page.$('#password');
    const submit = await page.$('#submitBtn');
    assert.ok(username, 'username input exists');
    assert.ok(password, 'password input exists');
    assert.ok(submit, 'submit button exists');

    // Username is pre-filled with "root".
    const val = await page.$eval('#username', el => el.value);
    assert.equal(val, 'root');
  });

  it('shows error on wrong password', async () => {
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate(() => localStorage.removeItem('openerp_token'));
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });

    // Type wrong password.
    await page.type('#password', 'wrong-password');
    await page.click('#submitBtn');

    // Wait for error message to appear.
    await page.waitForSelector('#errorMsg', { visible: true, timeout: 5000 });
    const errorText = await page.$eval('#errorMsg', el => el.textContent);
    assert.match(errorText, /invalid/i);

    // Should still be on login page.
    assert.ok(page.url().endsWith('/') || page.url().endsWith(':8088/'));
  });

  it('redirects to dashboard on correct password', async () => {
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    await page.evaluate(() => localStorage.removeItem('openerp_token'));
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });

    // Fill in correct credentials.
    const usernameInput = await page.$('#username');
    await usernameInput.click({ clickCount: 3 });
    await usernameInput.type(ROOT_USER);
    await page.type('#password', ROOT_PASS);

    // Submit and wait for navigation.
    await Promise.all([
      page.waitForNavigation({ waitUntil: 'networkidle0' }),
      page.click('#submitBtn'),
    ]);

    // Should be on dashboard.
    assert.ok(page.url().includes('/dashboard'), `Expected /dashboard, got ${page.url()}`);

    // Token should be stored.
    const token = await page.evaluate(() => localStorage.getItem('openerp_token'));
    assert.ok(token, 'JWT token stored in localStorage');
    assert.ok(token.split('.').length === 3, 'Token is a valid JWT format');
  });

  it('dashboard shows signed-in user info', async () => {
    // We're already on the dashboard from the previous test.
    // New UI uses sidebar with #userName element.
    const userName = await page.$eval('#userName', el => el.textContent);
    assert.match(userName, /root/, 'Shows root user');
  });

  it('dashboard loads module data from schema', async () => {
    // Wait for schema to load and sidebar to appear.
    await page.waitForFunction(
      () => document.querySelectorAll('.sidebar .nav-item').length > 0,
      { timeout: 5000 },
    );

    // Should show module button(s) from schema.
    const moduleButtons = await page.$$eval('.module-btn', els => els.map(e => e.textContent));
    assert.ok(moduleButtons.length > 0, 'Module buttons rendered from schema');

    // Sidebar should have nav items.
    const navItems = await page.$$eval('.sidebar .nav-item', els => els.map(e => e.textContent));
    assert.ok(navItems.length > 0, 'Sidebar nav items rendered');
  });

  it('logout returns to login page', async () => {
    // Click logout and wait for the page to end up on login.
    await Promise.all([
      page.waitForNavigation({ waitUntil: 'networkidle0', timeout: 10000 }).catch(() => {}),
      page.click('#logoutBtn'),
    ]);

    // The redirect might take a moment — wait for the login form to appear.
    await page.waitForSelector('#loginForm', { timeout: 10000 });
    const url = page.url();
    assert.ok(
      url.endsWith('/') || url.endsWith(':8088/') || url.includes('login'),
      `Expected login page, got ${url}`,
    );

    // Token should be cleared.
    const token = await page.evaluate(() => localStorage.getItem('openerp_token'));
    assert.equal(token, null, 'Token cleared from localStorage');
  });

  it('redirects to dashboard when already logged in', async () => {
    // Login via API to set token.
    const resp = await fetch(`${BASE_URL}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: ROOT_USER, password: ROOT_PASS }),
    });
    const data = await resp.json();

    // Set token in localStorage.
    await page.evaluate((t) => localStorage.setItem('openerp_token', t), data.access_token);

    // Navigate to login page — should redirect to dashboard.
    await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });
    assert.ok(
      page.url().includes('/dashboard'),
      `Expected redirect to /dashboard, got ${page.url()}`,
    );
  });
});
