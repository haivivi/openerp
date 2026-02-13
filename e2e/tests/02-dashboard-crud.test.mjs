/**
 * E2E Test: Dashboard CRUD operations
 *
 * Tests:
 * 1. Create a user via dashboard UI
 * 2. User appears in the table
 * 3. Create a role via dashboard UI
 * 4. Role appears in the table
 * 5. Create a PMS model via dashboard UI
 * 6. Model appears in the table
 * 7. Stats update after creating resources
 * 8. Delete a user via dashboard UI
 * 9. Delete a role via dashboard UI
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
  ).catch(() => {}); // Ignore if no toast was visible.
}

describe('Dashboard CRUD', () => {
  let browser;
  let page;

  before(async () => {
    await waitForServer();
    browser = await launchBrowser();
    page = await browser.newPage();

    // Login first.
    await loginAsRoot(page);

    // Clean up any leftover test data.
    const token = await getToken(page);
    await cleanupTestData(token);

    // Reload dashboard to get fresh state.
    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle0' });
    // Wait for initial data load.
    await page.waitForFunction(
      () => document.getElementById('statUsers')?.textContent !== '-',
      { timeout: 5000 },
    );
  });

  after(async () => {
    // Clean up test data.
    if (page) {
      const token = await getToken(page);
      if (token) await cleanupTestData(token);
    }
    if (browser) await browser.close();
  });

  it('creates a user via the UI', async () => {
    await waitToastClear(page);

    await page.type('#newUserName', 'E2E Test User');
    await page.type('#newUserEmail', 'e2e@test.com');
    await page.click('button[onclick="createUser()"]');

    // Wait for user to appear in the table (definitive proof of success).
    await page.waitForFunction(
      () => document.getElementById('usersBody')?.textContent.includes('E2E Test User'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#usersBody tr', trs =>
      trs.map(tr => tr.textContent),
    );
    const found = rows.some(r => r.includes('E2E Test User') && r.includes('e2e@test.com'));
    assert.ok(found, 'User "E2E Test User" appears in table');
  });

  it('creates a role via the UI', async () => {
    await waitToastClear(page);

    await page.type('#newRoleId', 'e2e:tester');
    await page.type('#newRoleDesc', 'E2E test role');
    await page.type('#newRolePerms', 'e2e:test:read, e2e:test:write');
    await page.click('button[onclick="createRole()"]');

    // Wait for role to appear in the table.
    await page.waitForFunction(
      () => document.getElementById('rolesBody')?.textContent.includes('e2e:tester'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#rolesBody tr', trs =>
      trs.map(tr => tr.textContent),
    );
    const found = rows.some(r => r.includes('e2e:tester') && r.includes('E2E test role'));
    assert.ok(found, 'Role "e2e:tester" appears in table');
  });

  it('creates a PMS model via the UI', async () => {
    await waitToastClear(page);

    await page.type('#newModelCode', '999');
    await page.type('#newModelSeries', 'E2E');
    await page.type('#newModelDisplay', 'E2E Test Model');
    await page.click('button[onclick="createModel()"]');

    // Wait for model to appear in the table.
    await page.waitForFunction(
      () => document.getElementById('modelsBody')?.textContent.includes('999'),
      { timeout: 5000 },
    );

    const rows = await page.$$eval('#modelsBody tr', trs =>
      trs.map(tr => tr.textContent),
    );
    const found = rows.some(r => r.includes('999') && r.includes('E2E'));
    assert.ok(found, 'Model with code 999 appears in table');
  });

  it('stats update after creating resources', async () => {
    // Give stats a moment to refresh.
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
    await waitToastClear(page);

    // Set up dialog handler to accept confirmation.
    page.once('dialog', async dialog => await dialog.accept());

    // Find and click the delete button for our test user.
    const deleted = await page.evaluate(() => {
      const rows = document.querySelectorAll('#usersBody tr');
      for (const row of rows) {
        if (row.textContent.includes('E2E Test User')) {
          const btn = row.querySelector('.btn-danger');
          if (btn) { btn.click(); return true; }
        }
      }
      return false;
    });
    assert.ok(deleted, 'Found and clicked delete for E2E Test User');

    // Wait for the user to disappear from the table.
    await page.waitForFunction(
      () => !document.getElementById('usersBody')?.textContent.includes('E2E Test User'),
      { timeout: 5000 },
    );
  });

  it('deletes a role via the UI', async () => {
    await waitToastClear(page);

    page.once('dialog', async dialog => await dialog.accept());

    const deleted = await page.evaluate(() => {
      const rows = document.querySelectorAll('#rolesBody tr');
      for (const row of rows) {
        if (row.textContent.includes('e2e:tester')) {
          const btn = row.querySelector('.btn-danger');
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
