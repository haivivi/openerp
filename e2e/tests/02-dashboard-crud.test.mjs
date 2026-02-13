/**
 * E2E Test: Dashboard CRUD operations (shadcn sidebar layout)
 *
 * Tests:
 * 1. Create a user via Users page
 * 2. Create a role via Roles page
 * 3. Create a PMS model via Product Models page
 * 4. Stats update on Overview page
 * 5. Delete user
 * 6. Delete role
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import {
  BASE_URL,
  waitForServer,
  launchBrowser,
  loginAsRoot,
  getToken,
  cleanupTestData,
} from './helpers.mjs';

/** Wait for the toast to fully disappear before next action. */
async function waitToastClear(page) {
  await page.waitForFunction(
    () => !document.querySelector('.toast.show'),
    { timeout: 5000 },
  ).catch(() => {});
}

/** Click a sidebar nav item to switch pages. */
async function navigateTo(page, pageName) {
  await page.evaluate((name) => {
    const items = document.querySelectorAll('.nav-item[data-page]');
    for (const item of items) {
      if (item.dataset.page === name) { item.click(); break; }
    }
  }, pageName);
  // Wait for the target page to become visible.
  await page.waitForFunction(
    (name) => document.getElementById('page-' + name)?.classList.contains('active'),
    { timeout: 3000 },
    pageName,
  );
  // Brief pause for data to load.
  await new Promise(r => setTimeout(r, 500));
}

describe('Dashboard CRUD', () => {
  let browser;
  let page;

  before(async () => {
    await waitForServer();
    browser = await launchBrowser();
    page = await browser.newPage();

    await loginAsRoot(page);

    const token = await getToken(page);
    await cleanupTestData(token);

    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle0' });
    // Wait for initial overview data load (em-dash is placeholder).
    await page.waitForFunction(
      () => {
        const el = document.getElementById('statUsers');
        return el && el.textContent !== '\u2014' && el.textContent !== '-';
      },
      { timeout: 5000 },
    );
  });

  after(async () => {
    if (page) {
      const token = await getToken(page);
      if (token) await cleanupTestData(token);
    }
    if (browser) await browser.close();
  });

  it('creates a user via the UI', async () => {
    await navigateTo(page, 'users');
    await waitToastClear(page);

    await page.type('#newUserName', 'E2E Test User');
    await page.type('#newUserEmail', 'e2e@test.com');
    await page.click('button[onclick="createUser()"]');

    await page.waitForFunction(
      () => document.getElementById('usersBody')?.textContent.includes('E2E Test User'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#usersBody tr', trs => trs.map(tr => tr.textContent));
    const found = rows.some(r => r.includes('E2E Test User') && r.includes('e2e@test.com'));
    assert.ok(found, 'User "E2E Test User" appears in table');
  });

  it('creates a role via the UI', async () => {
    await navigateTo(page, 'roles');
    await waitToastClear(page);

    await page.type('#newRoleId', 'e2e:tester');
    await page.type('#newRoleDesc', 'E2E test role');
    await page.type('#newRolePerms', 'e2e:test:read, e2e:test:write');
    await page.click('button[onclick="createRole()"]');

    await page.waitForFunction(
      () => document.getElementById('rolesBody')?.textContent.includes('e2e:tester'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#rolesBody tr', trs => trs.map(tr => tr.textContent));
    const found = rows.some(r => r.includes('e2e:tester') && r.includes('E2E test role'));
    assert.ok(found, 'Role "e2e:tester" appears in table');
  });

  it('creates a PMS model via the UI', async () => {
    await navigateTo(page, 'models');
    await waitToastClear(page);

    await page.type('#newModelCode', '999');
    await page.type('#newModelSeries', 'E2E');
    await page.type('#newModelDisplay', 'E2E Test Model');
    await page.click('button[onclick="createModel()"]');

    await page.waitForFunction(
      () => document.getElementById('modelsBody')?.textContent.includes('999'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#modelsBody tr', trs => trs.map(tr => tr.textContent));
    const found = rows.some(r => r.includes('999') && r.includes('E2E'));
    assert.ok(found, 'Model with code 999 appears in table');
  });

  it('stats update after creating resources', async () => {
    // Switch to overview to see stats.
    await navigateTo(page, 'overview');

    await page.waitForFunction(
      () => parseInt(document.getElementById('statUsers')?.textContent || '0', 10) >= 1,
      { timeout: 5000 },
    );

    const users = parseInt(await page.$eval('#statUsers', el => el.textContent), 10);
    const roles = parseInt(await page.$eval('#statRoles', el => el.textContent), 10);
    const models = parseInt(await page.$eval('#statModels', el => el.textContent), 10);

    assert.ok(users >= 1, `Expected >= 1 user, got ${users}`);
    assert.ok(roles >= 1, `Expected >= 1 role, got ${roles}`);
    assert.ok(models >= 1, `Expected >= 1 model, got ${models}`);
  });

  it('deletes a user via the UI', async () => {
    await navigateTo(page, 'users');
    await waitToastClear(page);

    page.once('dialog', async dialog => await dialog.accept());

    const deleted = await page.evaluate(() => {
      const rows = document.querySelectorAll('#usersBody tr');
      for (const row of rows) {
        if (row.textContent.includes('E2E Test User')) {
          const btn = row.querySelector('.btn-ghost-destructive');
          if (btn) { btn.click(); return true; }
        }
      }
      return false;
    });
    assert.ok(deleted, 'Found and clicked delete for E2E Test User');

    await page.waitForFunction(
      () => !document.getElementById('usersBody')?.textContent.includes('E2E Test User'),
      { timeout: 5000 },
    );
  });

  it('deletes a role via the UI', async () => {
    await navigateTo(page, 'roles');
    await waitToastClear(page);

    page.once('dialog', async dialog => await dialog.accept());

    const deleted = await page.evaluate(() => {
      const rows = document.querySelectorAll('#rolesBody tr');
      for (const row of rows) {
        if (row.textContent.includes('e2e:tester')) {
          const btn = row.querySelector('.btn-ghost-destructive');
          if (btn) { btn.click(); return true; }
        }
      }
      return false;
    });
    assert.ok(deleted, 'Found and clicked delete for e2e:tester');

    await page.waitForFunction(
      () => !document.getElementById('rolesBody')?.textContent.includes('e2e:tester'),
      { timeout: 5000 },
    );
  });
});
