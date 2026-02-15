/**
 * E2E Test: Task action API
 *
 * Tests:
 * 1. Create task → claim → progress → complete (happy path)
 * 2. Create task → claim → fail → auto-retry (retry_count < max_retries)
 * 3. Cancel a pending task
 * 4. Invalid transitions rejected (claim a running task, complete a pending task)
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

describe('Task Actions', () => {
  let rootToken;
  const taskIds = [];

  before(async () => {
    await waitForServer();
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER, password: ROOT_PASS,
    });
    rootToken = resp.data.access_token;
  });

  after(async () => {
    for (const id of taskIds) {
      await apiCall('DELETE', `/admin/task/tasks/${id}`, null, rootToken);
    }
  });

  async function createTask(extra = {}) {
    const resp = await apiCall('POST', '/admin/task/tasks', {
      taskType: 'e2e-test', total: 10, status: 'pending',
      timeoutSecs: 60, maxRetries: 0,
      displayName: 'E2E Task', ...extra,
    }, rootToken);
    assert.equal(resp.status, 200);
    taskIds.push(resp.data.id);
    return resp.data;
  }

  it('happy path: create → claim → progress → complete', async () => {
    const task = await createTask();
    assert.ok(task.id);

    // Claim.
    const claim = await apiCall('POST', `/admin/task/tasks/${task.id}/@claim`,
      { workerId: 'e2e-worker' }, rootToken);
    assert.equal(claim.status, 200);
    assert.equal(claim.data.status, 'running');

    // Progress.
    const prog = await apiCall('POST', `/admin/task/tasks/${task.id}/@progress`,
      { success: 7, message: '70% done' }, rootToken);
    assert.equal(prog.status, 200);

    // Verify progress saved.
    const check = await apiCall('GET', `/admin/task/tasks/${task.id}`, null, rootToken);
    assert.equal(check.data.success, 7);
    assert.equal(check.data.message, '70% done');

    // Complete.
    const comp = await apiCall('POST', `/admin/task/tasks/${task.id}/@complete`, {}, rootToken);
    assert.equal(comp.status, 200);
    assert.equal(comp.data.status, 'completed');

    // Final state.
    const final_ = await apiCall('GET', `/admin/task/tasks/${task.id}`, null, rootToken);
    assert.equal(final_.data.status, 'completed');
    assert.ok(final_.data.endedAt, 'endedAt should be set');
  });

  it('fail with auto-retry: claim → fail → back to pending', async () => {
    const task = await createTask({ maxRetries: 2 });

    // Claim.
    await apiCall('POST', `/admin/task/tasks/${task.id}/@claim`,
      { workerId: 'w1' }, rootToken);

    // Fail.
    const fail = await apiCall('POST', `/admin/task/tasks/${task.id}/@fail`,
      { error: 'connection timeout' }, rootToken);
    assert.equal(fail.status, 200);
    assert.equal(fail.data.status, 'pending', 'Should retry — back to pending');

    // Check retry_count.
    const check = await apiCall('GET', `/admin/task/tasks/${task.id}`, null, rootToken);
    assert.equal(check.data.retryCount, 1);

    // Can claim again after retry.
    const claim2 = await apiCall('POST', `/admin/task/tasks/${task.id}/@claim`,
      { workerId: 'w2' }, rootToken);
    assert.equal(claim2.status, 200);
    assert.equal(claim2.data.status, 'running');
  });

  it('cancel a pending task', async () => {
    const task = await createTask();

    const cancel = await apiCall('POST', `/admin/task/tasks/${task.id}/@cancel`, {}, rootToken);
    assert.equal(cancel.status, 200);
    assert.equal(cancel.data.status, 'cancelled');

    // Verify.
    const check = await apiCall('GET', `/admin/task/tasks/${task.id}`, null, rootToken);
    assert.equal(check.data.status, 'cancelled');
    assert.ok(check.data.endedAt);
  });

  it('invalid transitions are rejected', async () => {
    const task = await createTask();

    // Can't complete a pending task.
    const comp = await apiCall('POST', `/admin/task/tasks/${task.id}/@complete`, {}, rootToken);
    assert.notEqual(comp.status, 200, 'Cannot complete a pending task');

    // Can't progress a pending task.
    const prog = await apiCall('POST', `/admin/task/tasks/${task.id}/@progress`,
      { success: 1 }, rootToken);
    assert.notEqual(prog.status, 200, 'Cannot progress a pending task');

    // Claim it.
    await apiCall('POST', `/admin/task/tasks/${task.id}/@claim`,
      { workerId: 'w1' }, rootToken);

    // Can't claim again (it's running).
    const claim2 = await apiCall('POST', `/admin/task/tasks/${task.id}/@claim`,
      { workerId: 'w2' }, rootToken);
    assert.notEqual(claim2.status, 200, 'Cannot claim a running task');
  });
});
