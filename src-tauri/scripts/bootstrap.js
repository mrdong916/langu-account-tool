function bootstrapLanguAccount(expectedOrigin, payload) {
  if (location.origin !== expectedOrigin || window !== window.top) return false;
  const result = payload.result;
  const expire = Date.now() + 7 * 24 * 60 * 60 * 1000;
  const set = (key, value) =>
    localStorage.setItem('pro__' + key, JSON.stringify({ value, expire }));
  set('Access-Token', result.token);
  set('Login_Username', result.userInfo.username);
  set('Login_Userinfo', result.userInfo);
  set('Login_settlementOrg', result.accounts ?? []);
  set('UI_CACHE_DB_DICT_DATA', result.sysAllDictItems ?? {});
  set('Login_ErpUserList', result.erpUserInfo ?? []);
  set('TENANT_ID', payload.tenant ?? '0');
  set('canLogin', true);
  localStorage.setItem('USER_ID', result.userInfo.id);
  return true;
}
