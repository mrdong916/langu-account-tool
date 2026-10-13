import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

const scripts = ['read-auth', 'bootstrap', 'clear-auth'].map(name => readFileSync(new URL(`../src-tauri/scripts/${name}.js`, import.meta.url), 'utf8')).join('\n');
function context(origin = 'http://languwin.com') {
  const cache = new Map();
  const window = {};
  window.top = window;
  const sandbox = vm.createContext({ location: { origin }, window, atob: value => Buffer.from(value, 'base64').toString('binary'), sessionStorage: { removeItem() {} }, localStorage: { getItem: key => cache.get(key) ?? null, setItem: (key, value) => cache.set(key, String(value)), removeItem: key => cache.delete(key) } });
  vm.runInContext(scripts, sandbox);
  return { sandbox, cache };
}
const payload = { result: { token: 'test-only-token', userInfo: { id: 'child-1', username: 'test.child' }, accounts: [{ id: 'account-1' }], sysAllDictItems: { role: [] }, erpUserInfo: [] }, tenant: '0' };

test('bootstrap restores complete Jeecg login data and does not mutate main profile', () => {
  const main = context();
  const child = context();
  main.cache.set('pro__Access-Token', 'original-main-token');
  child.sandbox.bootstrapLanguAccount('http://languwin.com', payload);
  assert.equal(main.cache.get('pro__Access-Token'), 'original-main-token');
  assert.equal(child.sandbox.readLanguAuth('http://languwin.com').user.username, 'test.child');
  assert.equal(child.cache.get('USER_ID'), 'child-1');
  assert.deepEqual(JSON.parse(child.cache.get('pro__Login_settlementOrg')).value, payload.result.accounts);
  assert.equal(JSON.parse(child.cache.get('pro__canLogin')).value, true);
});

test('primary expiry clears only auth and never another origin or child profile', () => {
  const main = context();
  const child = context();
  main.sandbox.bootstrapLanguAccount('http://languwin.com', payload);
  child.sandbox.bootstrapLanguAccount('http://languwin.com', payload);
  main.cache.set('draft', 'keep-me');
  assert.equal(main.sandbox.clearLanguAuth('http://other.example'), false);
  assert.equal(main.sandbox.clearLanguAuth('http://languwin.com'), true);
  assert.equal(main.sandbox.readLanguAuth('http://languwin.com'), null);
  assert.equal(main.cache.get('draft'), 'keep-me');
  assert.equal(child.sandbox.readLanguAuth('http://languwin.com').user.username, 'test.child');
});

test('credentials are neither injected nor read at another origin or inside an iframe', () => {
  const { sandbox, cache } = context('http://other.example');
  assert.equal(sandbox.bootstrapLanguAccount('http://languwin.com', payload), false);
  assert.equal(sandbox.readLanguAuth('http://languwin.com'), null);
  assert.equal(cache.size, 0);
  sandbox.location.origin = 'http://languwin.com';
  sandbox.window.top = {};
  assert.equal(sandbox.bootstrapLanguAccount('http://languwin.com', payload), false);
  assert.equal(cache.size, 0);
});

test('expired storage, expired JWT, and malformed login state cannot authorize account search', () => {
  const { sandbox, cache } = context();
  sandbox.bootstrapLanguAccount('http://languwin.com', payload);
  cache.set('pro__Access-Token', JSON.stringify({ value: 'expired', expire: 1 }));
  assert.equal(sandbox.readLanguAuth('http://languwin.com'), null);
  const token = `e30.${Buffer.from(JSON.stringify({ exp: 1 })).toString('base64url')}.test`;
  cache.set('pro__Access-Token', JSON.stringify({ value: token, expire: null }));
  assert.equal(sandbox.readLanguAuth('http://languwin.com'), null);
  cache.set('pro__Login_Userinfo', 'invalid-json');
  assert.equal(sandbox.readLanguAuth('http://languwin.com'), null);
});
