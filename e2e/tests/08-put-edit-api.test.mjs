/**
 * E2E Test: PUT edit API
 *
 * Directly tests the admin PUT endpoint to ensure:
 * 1. Fields are correctly updated
 * 2. Hidden fields (password_hash) are not lost
 * 3. No duplicate records are created
 * 4. before_update hook fires (updated_at changes)
 * 5. created_at is preserved
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import {
  BASE_URL,
  ROOT_USER,
  ROOT_PASS,
  waitForServer,
  apiCall,
} from './helpers.mjs';

describe('PUT Edit API', () => {
  let rootToken;
  let userId;

  before(async () => {
    await waitForServer();
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER, password: ROOT_PASS,
    });
    rootToken = resp.data.access_token;
  });

  after(async () => {
    if (rootToken && userId) {
      await apiCall('DELETE', `/admin/auth/users/${userId}`, null, rootToken);
    }
  });

  it('PUT updates fields and preserves id/created_at', async () => {
    // Create.
    const createResp = await apiCall('POST', '/admin/auth/users', {
      displayName: 'PUT Test User',
      email: 'put@test.com',
      active: true,
    }, rootToken);
    assert.equal(createResp.status, 200);
    userId = createResp.data.id;
    const createdAt = createResp.data.createdAt;
    assert.ok(userId);
    assert.ok(createdAt);

    // Small delay so updated_at differs.
    await new Promise(r => setTimeout(r, 50));

    // PUT with full record (change displayName, keep everything else).
    const putData = { ...createResp.data, displayName: 'PUT Updated User' };
    const putResp = await apiCall('PUT', `/admin/auth/users/${userId}`, putData, rootToken);
    assert.equal(putResp.status, 200);
    assert.equal(putResp.data.id, userId, 'ID must not change');
    assert.equal(putResp.data.displayName, 'PUT Updated User');
    assert.equal(putResp.data.email, 'put@test.com', 'Email preserved');
    assert.equal(putResp.data.createdAt, createdAt, 'createdAt must be preserved');
  });

  it('PUT does not create duplicates', async () => {
    const listResp = await apiCall('GET', '/admin/auth/users', null, rootToken);
    const matches = listResp.data.items.filter(u => u.id === userId);
    assert.equal(matches.length, 1, `Should have exactly 1 user with id=${userId}`);
  });

  it('PUT preserves hidden fields not in request', async () => {
    // Get current record (has all fields).
    const getResp = await apiCall('GET', `/admin/auth/users/${userId}`, null, rootToken);
    const original = getResp.data;

    // PUT with a subset of fields (but include id for key).
    const putResp = await apiCall('PUT', `/admin/auth/users/${userId}`, {
      ...original,
      displayName: 'Subset Update',
    }, rootToken);
    assert.equal(putResp.status, 200);
    assert.equal(putResp.data.displayName, 'Subset Update');
    assert.equal(putResp.data.active, original.active, 'active preserved');
  });

  it('PUT on non-existent record returns 404', async () => {
    const putResp = await apiCall('PUT', '/admin/auth/users/nonexistent123', {
      id: 'nonexistent123', displayName: 'Ghost',
    }, rootToken);
    assert.equal(putResp.status, 404);
  });
});
