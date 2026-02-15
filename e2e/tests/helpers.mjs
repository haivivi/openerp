/**
 * Shared helpers for E2E tests.
 *
 * Manages browser lifecycle, server health checks, and common operations.
 */

import puppeteer from 'puppeteer-core';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';

export const BASE_URL = process.env.BASE_URL || 'http://localhost:8088';
export const ROOT_USER = 'root';
export const ROOT_PASS = process.env.ROOT_PASS || 'openerp123';
export const HEADLESS = process.env.HEADLESS !== 'false';
export const SLOW_MO = parseInt(process.env.SLOW_MO || '0', 10);

/**
 * Find a usable Chrome/Chromium executable.
 */
function findChrome() {
  // 1. Explicit env var.
  if (process.env.CHROME_PATH && existsSync(process.env.CHROME_PATH)) {
    return process.env.CHROME_PATH;
  }

  // 2. Puppeteer cache.
  const home = process.env.HOME || '/tmp';
  const cached = `${home}/.cache/puppeteer/chrome`;
  if (existsSync(cached)) {
    // Find any installed Chrome directory.
    try {
      const dirs = execSync(`ls -d ${cached}/mac_arm-*/chrome-mac-arm64 2>/dev/null || ls -d ${cached}/mac-*/chrome-mac 2>/dev/null`, { encoding: 'utf8' }).trim().split('\n');
      for (const dir of dirs) {
        const app = `${dir}/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;
        if (existsSync(app)) return app;
      }
    } catch { /* ignore */ }
  }

  // 3. System Chrome (macOS).
  const systemChrome = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  if (existsSync(systemChrome)) return systemChrome;

  // 4. Chromium via brew.
  try {
    return execSync('which chromium', { encoding: 'utf8' }).trim();
  } catch { /* ignore */ }

  throw new Error('No Chrome/Chromium found. Set CHROME_PATH env var.');
}

/**
 * Wait for the server to be ready (health check).
 * Retries up to `maxRetries` times with `intervalMs` between.
 */
export async function waitForServer(maxRetries = 30, intervalMs = 1000) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const resp = await fetch(`${BASE_URL}/health`);
      if (resp.ok) return;
    } catch {
      // Server not ready yet.
    }
    await new Promise(r => setTimeout(r, intervalMs));
  }
  throw new Error(`Server at ${BASE_URL} not ready after ${maxRetries} retries`);
}

/**
 * Launch a browser instance.
 */
export async function launchBrowser() {
  const executablePath = findChrome();
  console.log(`Using Chrome: ${executablePath}`);
  return puppeteer.launch({
    executablePath,
    headless: HEADLESS,
    slowMo: SLOW_MO,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
    ],
  });
}

/**
 * Login as root via the UI, returning the page already on /dashboard.
 */
export async function loginAsRoot(page) {
  await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });

  // Clear any existing token first.
  await page.evaluate(() => localStorage.removeItem('openerp_token'));
  await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle0' });

  // Fill in credentials.
  const usernameInput = await page.$('#username');
  await usernameInput.click({ clickCount: 3 }); // select all
  await usernameInput.type(ROOT_USER);
  await page.type('#password', ROOT_PASS);

  // Submit and wait for navigation to dashboard.
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'networkidle0' }),
    page.click('#submitBtn'),
  ]);

  // Verify we're on the dashboard.
  const url = page.url();
  if (!url.includes('/dashboard')) {
    throw new Error(`Expected /dashboard, got ${url}`);
  }
}

/**
 * Get the JWT token from localStorage.
 */
export async function getToken(page) {
  return page.evaluate(() => localStorage.getItem('openerp_token'));
}

/**
 * Make an API call directly (bypassing UI).
 */
export async function apiCall(method, path, body, token) {
  const opts = {
    method,
    headers: {
      'Content-Type': 'application/json',
    },
  };
  if (token) opts.headers['Authorization'] = `Bearer ${token}`;
  if (body) opts.body = JSON.stringify(body);
  const resp = await fetch(`${BASE_URL}${path}`, opts);
  if (resp.status === 204) return { status: 204, data: null };
  const text = await resp.text();
  let data = null;
  if (text) { try { data = JSON.parse(text); } catch(e) {} }
  return { status: resp.status, data };
}

/**
 * Clean up test data: delete all users, roles, models created during tests.
 */
export async function cleanupTestData(token) {
  try {
    const users = await apiCall('GET', '/admin/auth/users', null, token);
    if (users?.data?.items) {
      for (const u of users.data.items) {
        await apiCall('DELETE', `/auth/users/${u.id}`, null, token);
      }
    }
    const roles = await apiCall('GET', '/admin/auth/roles', null, token);
    if (roles?.data?.items) {
      for (const r of roles.data.items) {
        await apiCall('DELETE', `/auth/roles/${r.id}`, null, token);
      }
    }
    const models = await apiCall('GET', '/admin/pms/models', null, token);
    if (models?.data?.items) {
      for (const m of models.data.items) {
        await apiCall('DELETE', `/pms/models/${m.code}`, null, token);
      }
    }
  } catch {
    // Best effort cleanup.
  }
}
