import { createServer } from 'node:http';
import { createHash } from 'node:crypto';

// Explicit local fixture. Never enabled or bundled in the application.
const port = Number(process.env.LANGU_FIXTURE_PORT ?? 1431);
const users = Array.from({ length: 45 }, (_, i) => ({ id: `fixture-${i + 1}`, username: `test.user.${String(i + 1).padStart(2, '0')}`, realname: `Test ${i + 1}`, orgCodeTxt: 'Local fixture', avatar: '' }));
const main = { id: 'fixture-main', username: 'test.admin', avatar: '' };
const secret = 'dd05f1c54d63749eda95f9fa6d49v442a';
const tokens = new Map([['fixture-main-token', 'test.admin']]);
const revoked = new Set();
let permitted = true;
let issued = 0;
const requests = [];
const server = createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  const json = (value, code = 200) => { res.writeHead(code, { 'Content-Type': 'application/json' }); res.end(JSON.stringify(value)); };
  if (url.pathname === '/fixture/state') return json({ permitted, requests });
  if (url.pathname.startsWith('/fixture/') && req.method === 'POST') {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    let body;
    try { body = JSON.parse(Buffer.concat(chunks).toString() || '{}'); }
    catch { return json({ success: false }, 400); }
    if (url.pathname === '/fixture/permission') permitted = body.allowed === true;
    if (url.pathname === '/fixture/expire') {
      for (const [token, username] of tokens) if (username === body.username) revoked.add(token);
    }
    if (url.pathname === '/fixture/login') {
      const token = `fixture-main-${++issued}`;
      tokens.set(token, main.username);
      return json({ token, user: main });
    }
    return json({ success: true });
  }
  if (url.pathname === '/sys/common/getServerTimestamp') return json({ success: true, result: Date.now() });
  if (url.pathname.startsWith('/sys/')) {
    const token = req.headers['x-access-token'];
    const username = tokens.get(token);
    requests.push({ path: url.pathname, username, time: Date.now() });
    if (!username || revoked.has(token)) return json({ success: false, code: 500, message: 'Token失效，请重新登录' }, 500);
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    const params = req.method === 'POST' ? JSON.parse(Buffer.concat(chunks).toString()) : Object.fromEntries([...url.searchParams].filter(([key]) => key !== '_t'));
    const sorted = Object.fromEntries(Object.keys(params).sort().map(key => [key, String(params[key])]));
    const sign = createHash('md5').update(JSON.stringify(sorted) + secret).digest('hex').toUpperCase();
    if (sign !== req.headers['x-sign'] || !/^\d{14}$/.test(req.headers['x-timestamp'] ?? '')) return json({ success: false, message: 'Invalid signature' }, 403);
    if (url.pathname === '/sys/sysConfig/getByCode') return json({ success: true, result: { code: params.code, value: 'fixture' } });
    if (username !== 'test.admin') return json({ success: false }, 403);
    if (url.pathname === '/sys/permission/getUserPermissionByToken') {
      const grant = { action: 'system:internalLoginByAdmin', type: '1' };
      return json({ success: true, result: { auth: permitted ? [grant] : [], allAuth: [grant] } });
    }
    if (!permitted) return json({ success: false }, 403);
    if (url.pathname === '/sys/user/list') {
      const query = (params.username ?? '').replaceAll('*', '');
      const matches = users.filter(user => user.username.includes(query));
      const page = Number(params.pageNo ?? 1);
      return json({ success: true, result: { records: matches.slice((page - 1) * 20, page * 20), total: matches.length } });
    }
    if (url.pathname === '/sys/internalLoginByAdmin') {
      const userInfo = users.find(user => user.username === params.username);
      if (!userInfo) return json({ success: false, message: 'User not found' });
      const token = `fixture-token-${userInfo.id}-${++issued}`;
      tokens.set(token, userInfo.username);
      return json({ success: true, result: { token, userInfo, accounts: [], sysAllDictItems: {}, erpUserInfo: [] } });
    }
    return json({ success: false }, 404);
  }
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(`<!doctype html><html lang="en"><meta charset="utf-8"><title>Local account isolation test</title><style>body{font:16px system-ui;padding:40px;color:#245347}h1{font-size:24px}input,button{padding:10px;margin:8px}pre{white-space:pre-wrap}</style><h1>Account isolation fixture</h1><p id="account"></p><label>Draft <input id="draft" placeholder="Session-specific draft"></label><p><a href="/second">Second page</a> | <a href="/popup" target="_blank">New-window link</a></p><button id="logout">Local logout</button><button id="login">Fixture main login</button><button id="expire">Expire current account on server</button><script>
  const set=(k,v)=>localStorage.setItem('pro__'+k,JSON.stringify({value:v,expire:null}));
  const get=k=>JSON.parse(localStorage.getItem('pro__'+k)||'null')?.value;
  if(!get('Access-Token')&&!sessionStorage.getItem('fixture-initialized')){set('Access-Token','fixture-main-token');set('Login_Userinfo',${JSON.stringify(main)});set('canLogin',true);}
  sessionStorage.setItem('fixture-initialized','1');
  document.getElementById('account').textContent='Current account: '+(get('Login_Userinfo')?.username??'signed out');
  const draft=document.getElementById('draft');draft.value=localStorage.getItem('draft')||'';draft.oninput=()=>localStorage.setItem('draft',draft.value);
  document.getElementById('logout').onclick=()=>{localStorage.removeItem('pro__Access-Token');localStorage.removeItem('pro__Login_Userinfo');sessionStorage.setItem('fixture-logged-out','1');location.reload();};
  document.getElementById('login').onclick=async()=>{const response=await fetch('/fixture/login',{method:'POST'});const data=await response.json();set('Access-Token',data.token);set('Login_Userinfo',data.user);set('canLogin',true);location.reload();};
  document.getElementById('expire').onclick=async()=>{await fetch('/fixture/expire',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({username:get('Login_Userinfo')?.username})});document.getElementById('expire').textContent='Expired on server';};
  </script></html>`);
});
server.listen(port, '127.0.0.1', () => console.log(`Local test system: http://127.0.0.1:${server.address().port}`));
