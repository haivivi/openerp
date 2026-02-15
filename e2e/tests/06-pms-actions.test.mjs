/**
 * E2E Test: PMS action API
 *
 * Tests:
 * 1. Create a batch, provision it → devices appear
 * 2. Activate a provisioned device
 * 3. Upload firmware
 * 4. Provision a fully-provisioned batch → error
 * 5. Activate an already-active device → error
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

describe('PMS Actions', () => {
  let rootToken;
  let batchId;
  const createdDeviceSns = [];

  before(async () => {
    await waitForServer();
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER, password: ROOT_PASS,
    });
    rootToken = resp.data.access_token;
  });

  after(async () => {
    // Clean up devices.
    for (const sn of createdDeviceSns) {
      await apiCall('DELETE', `/admin/pms/devices/${sn}`, null, rootToken);
    }
    if (batchId) {
      await apiCall('DELETE', `/admin/pms/batches/${batchId}`, null, rootToken);
    }
    // Clean up any firmwares.
    const fws = await apiCall('GET', '/admin/pms/firmwares', null, rootToken);
    for (const fw of (fws.data?.items || [])) {
      await apiCall('DELETE', `/admin/pms/firmwares/${fw.id}`, null, rootToken);
    }
  });

  it('create a batch and provision → devices appear', async () => {
    // Create batch.
    const batchResp = await apiCall('POST', '/admin/pms/batches', {
      model: 99, quantity: 3, provisionedCount: 0,
      status: 'pending', displayName: 'E2E Batch',
    }, rootToken);
    assert.equal(batchResp.status, 200);
    batchId = batchResp.data.id;
    assert.ok(batchId);

    // Provision.
    const provResp = await apiCall('POST', `/admin/pms/batches/${batchId}/@provision`, {}, rootToken);
    assert.equal(provResp.status, 200);
    assert.equal(provResp.data.provisioned, 3);
    assert.equal(provResp.data.devices.length, 3);
    createdDeviceSns.push(...provResp.data.devices);

    // Verify devices exist.
    const devResp = await apiCall('GET', '/admin/pms/devices', null, rootToken);
    assert.ok(devResp.data.items.length >= 3);
    for (const sn of provResp.data.devices) {
      const d = devResp.data.items.find(i => i.sn === sn);
      assert.ok(d, `Device ${sn} should exist`);
      assert.equal(d.status, 'provisioned');
      assert.equal(d.model, 99);
    }

    // Batch should be completed.
    const batchCheck = await apiCall('GET', `/admin/pms/batches/${batchId}`, null, rootToken);
    assert.equal(batchCheck.data.status, 'completed');
    assert.equal(batchCheck.data.provisionedCount, 3);
  });

  it('provision a fully-provisioned batch → error', async () => {
    const provResp = await apiCall('POST', `/admin/pms/batches/${batchId}/@provision`, {}, rootToken);
    assert.notEqual(provResp.status, 200, 'Should fail — batch already fully provisioned');
  });

  it('activate a provisioned device', async () => {
    const sn = createdDeviceSns[0];
    assert.ok(sn, 'Need a device SN');

    const actResp = await apiCall('POST', `/admin/pms/devices/${sn}/@activate`, {}, rootToken);
    assert.equal(actResp.status, 200);
    assert.equal(actResp.data.status, 'active');

    // Verify via GET.
    const devResp = await apiCall('GET', `/admin/pms/devices/${sn}`, null, rootToken);
    assert.equal(devResp.data.status, 'active');
  });

  it('activate an already-active device → error', async () => {
    const sn = createdDeviceSns[0];
    const actResp = await apiCall('POST', `/admin/pms/devices/${sn}/@activate`, {}, rootToken);
    assert.notEqual(actResp.status, 200, 'Should fail — device already active');
  });

  it('upload firmware', async () => {
    const fwResp = await apiCall('POST', '/admin/pms/firmwares/@upload', {
      model: 99, semver: '1.0.0', build: 1,
      releaseNotes: 'Initial release',
    }, rootToken);
    assert.equal(fwResp.status, 200);
    assert.equal(fwResp.data.status, 'uploaded');
    assert.equal(fwResp.data.semver, '1.0.0');
    assert.ok(fwResp.data.id);
  });
});
