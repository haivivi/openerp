/**
 * E2E Test: Facet API
 *
 * Tests the MFG facet (/mfg/pms/) returns correct field subsets
 * and is accessible independently.
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

describe('Facet API', () => {
  let rootToken;
  let deviceSn;

  before(async () => {
    await waitForServer();

    // Get root token.
    const resp = await apiCall('POST', '/auth/login', {
      username: ROOT_USER,
      password: ROOT_PASS,
    });
    rootToken = resp.data.access_token;

    // Create a test device via admin.
    deviceSn = 'E2E-SN-001';
    await apiCall('POST', '/admin/pms/devices', {
      sn: deviceSn,
      model: 42,
      status: 'active',
      sku: 'TEST-SKU',
      imei: ['860000001'],
      displayName: 'E2E Test Device',
    }, rootToken);

    // Create a model.
    await apiCall('POST', '/admin/pms/models', {
      code: 42,
      seriesName: 'E2E-Series',
      displayName: 'E2E Model',
    }, rootToken);
  });

  after(async () => {
    // Clean up.
    if (rootToken) {
      await apiCall('DELETE', `/admin/pms/devices/${deviceSn}`, null, rootToken);
      await apiCall('DELETE', `/admin/pms/models/42`, null, rootToken);
    }
  });

  it('MFG facet lists devices with field subset', async () => {
    const resp = await apiCall('GET', '/mfg/pms/devices');
    assert.equal(resp.status, 200);

    const items = resp.data.items;
    assert.ok(items.length >= 1, 'At least one device');

    const device = items.find(d => d.sn === deviceSn);
    assert.ok(device, `Device ${deviceSn} found in MFG facet`);

    // MFG facet fields present.
    assert.ok(device.sn, 'has sn');
    assert.equal(device.model, 42);
    assert.equal(device.status, 'active');
    assert.equal(device.sku, 'TEST-SKU');
    assert.ok(Array.isArray(device.imei), 'has imei array');
    assert.ok(device.displayName, 'has displayName');

    // Secret should NOT be in MFG facet.
    assert.equal(device.secret, undefined, 'secret not exposed in MFG facet');
    assert.equal(device.passwordHash, undefined, 'no passwordHash in device');
  });

  it('MFG facet gets single device', async () => {
    const resp = await apiCall('GET', `/mfg/pms/devices/${deviceSn}`);
    assert.equal(resp.status, 200);
    assert.equal(resp.data.sn, deviceSn);
    assert.equal(resp.data.secret, undefined, 'secret not exposed');
  });

  it('MFG facet lists models', async () => {
    const resp = await apiCall('GET', '/mfg/pms/models');
    assert.equal(resp.status, 200);
    assert.ok(resp.data.items.length >= 1);
    const model = resp.data.items.find(m => m.code === 42);
    assert.ok(model, 'Model found');
    assert.equal(model.seriesName, 'E2E-Series');
  });

  it('MFG facet is accessible without admin auth', async () => {
    // No Authorization header — should still work.
    const resp = await apiCall('GET', '/mfg/pms/devices');
    assert.equal(resp.status, 200);
  });

  it('admin API returns more fields than facet', async () => {
    // Admin includes all fields (secret, metadata, etc.).
    const adminResp = await apiCall('GET', `/admin/pms/devices/${deviceSn}`, null, rootToken);
    assert.equal(adminResp.status, 200);

    const admin = adminResp.data;
    const facetResp = await apiCall('GET', `/mfg/pms/devices/${deviceSn}`);
    const facet = facetResp.data;

    // Admin has more keys than facet.
    const adminKeys = Object.keys(admin);
    const facetKeys = Object.keys(facet);
    assert.ok(
      adminKeys.length > facetKeys.length,
      `Admin (${adminKeys.length} fields) should have more fields than facet (${facetKeys.length})`,
    );
  });

  it('schema includes facet info', async () => {
    const resp = await apiCall('GET', '/meta/schema');
    assert.equal(resp.status, 200);
    const facets = resp.data.facets;
    assert.ok(Array.isArray(facets));
    const mfg = facets.find(f => f.name === 'mfg');
    assert.ok(mfg, 'MFG facet in schema');
    assert.equal(mfg.module, 'pms');
    assert.equal(mfg.path, '/mfg/pms');
  });
});
