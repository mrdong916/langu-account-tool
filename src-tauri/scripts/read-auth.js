function readLanguAuth(expectedOrigin) {
  if (location.origin !== expectedOrigin || window !== window.top) return null;
  const get = (key) => {
    try {
      const entry = JSON.parse(localStorage.getItem('pro__' + key));
      return entry && (entry.expire === null || entry.expire >= Date.now()) ? entry.value : null;
    } catch {
      return null;
    }
  };
  const token = get('Access-Token');
  const user = get('Login_Userinfo');
  if (!token || !user?.username || !get('canLogin')) return null;
  try {
    const claims = JSON.parse(atob(token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')));
    if (typeof claims.exp === 'number' && claims.exp * 1000 <= Date.now()) return null;
  } catch {
    /* Non-JWT tokens remain subject to server validation. */
  }
  return { token, user, tenant: String(get('TENANT_ID') ?? 0) };
}
