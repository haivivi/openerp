/**
 * E2E Test: Dashboard CRUD (schema-driven)
 *
 * Tests that the schema-driven dashboard can:
 * 1. Load modules from /meta/schema
 * 2. Navigate between resources via sidebar
 * 3. Create a record via the generic create dialog
 * 4. See it in the table
 * 5. Delete a record
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

describe('Dashboard CRUD (schema-driven)', () => {
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

    // Wait for schema to load and sidebar to render.
    await page.waitForFunction(
      () => document.querySelectorAll('.sidebar .nav-item').length > 0,
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

  it('loads modules from schema', async () => {
    const modules = await page.$$eval('.module-btn', els => els.map(e => e.textContent));
    assert.ok(modules.length >= 1, 'At least one module loaded');
    assert.ok(modules.some(m => /auth/i.test(m)), 'Auth module present');
  });

  it('shows sidebar resources for selected module', async () => {
    const items = await page.$$eval('.sidebar .nav-item', els => els.map(e => e.textContent));
    assert.ok(items.length >= 2, `Expected >= 2 sidebar items, got: ${items}`);
    assert.ok(items.some(i => /user/i.test(i)), 'Users in sidebar');
    assert.ok(items.some(i => /role/i.test(i)), 'Roles in sidebar');
  });

  it('navigates to a resource and shows table', async () => {
    // Click on Users in sidebar.
    await page.evaluate(() => {
      const items = document.querySelectorAll('.sidebar .nav-item');
      for (const item of items) {
        if (/user/i.test(item.textContent)) { item.click(); break; }
      }
    });
    await new Promise(r => setTimeout(r, 500));

    // Table should exist.
    const table = await page.$('.table-card table');
    assert.ok(table, 'Table rendered for resource');
  });

  it('creates a user via the generic create dialog', async () => {
    // Ensure we're on Users page.
    await page.evaluate(() => {
      const items = document.querySelectorAll('.sidebar .nav-item');
      for (const item of items) {
        if (/user/i.test(item.textContent)) { item.click(); break; }
      }
    });
    await new Promise(r => setTimeout(r, 500));

    // Click "+ Add" button.
    await page.evaluate(() => {
      const btn = document.querySelector('.btn-sm-primary');
      if (btn) btn.click();
    });

    // Wait for dialog to open.
    await page.waitForFunction(
      () => document.getElementById('createDlg')?.classList.contains('open'),
      { timeout: 3000 },
    );

    // Fill in the name field (should be the first input).
    const inputs = await page.$$('#dlgForm input');
    assert.ok(inputs.length >= 1, 'Create dialog has input fields');

    // Type name into the first input (which should be "name").
    await inputs[0].type('E2E Test User');

    // Submit.
    await page.click('#dlgSubmit');

    // Wait for dialog to close.
    await page.waitForFunction(
      () => !document.getElementById('createDlg')?.classList.contains('open'),
      { timeout: 5000 },
    );

    // Verify user appears in table.
    await new Promise(r => setTimeout(r, 500));
    const tableText = await page.$eval('#resBody', el => el.textContent);
    assert.ok(tableText.includes('E2E Test User'), 'Created user appears in table');
  });

  it('deletes a record via the table', async () => {
    // Find and click delete button for our test user.
    page.once('dialog', async dialog => await dialog.accept());

    const deleted = await page.evaluate(() => {
      const rows = document.querySelectorAll('#resBody tr');
      for (const row of rows) {
        if (row.textContent.includes('E2E Test User')) {
          const btn = row.querySelector('.btn-ghost-destructive');
          if (btn) { btn.click(); return true; }
        }
      }
      return false;
    });
    assert.ok(deleted, 'Found and clicked delete button');

    // Wait for the user to disappear.
    await page.waitForFunction(
      () => !document.getElementById('resBody')?.textContent.includes('E2E Test User'),
      { timeout: 5000 },
    );
  });
});
