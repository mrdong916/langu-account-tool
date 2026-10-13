function clearLanguAuth(expectedOrigin) {
  if (location.origin !== expectedOrigin || window !== window.top) return false;
  for (const key of ['Access-Token', 'Login_Userinfo', 'canLogin']) {
    localStorage.removeItem('pro__' + key);
  }
  sessionStorage.removeItem('USER_AUTH');
  sessionStorage.removeItem('SYS_BUTTON_AUTH');
  return true;
}
