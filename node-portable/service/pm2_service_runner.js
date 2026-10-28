'use strict';
/**
 * 可攜 PM2 開機自啟服務入口 (WinSW 呼叫，無須人員登入桌面)。
 * 1. 鎖定 PATH 最前端為可攜 node 目錄，並統一 PM2_HOME 至 <根目錄>\.pm2。
 * 2. 檢測現有 PM2 Daemon 是否由非目標 node.exe (如舊全域 Node v22) 啟動，若是則先終止切換。
 * 3. 以 TCP 探測 PM2 Daemon (預設 127.0.0.1:42681)，離線時執行 pm2 resurrect 還原 dump.pm2。
 * 4. 確保 pm2-logrotate 日誌輪替模組離線安裝並啟動。
 * 5. 每 60 秒重複探測，Daemon 或 pm2-logrotate 掉線即自動喚醒。
 */
const fs = require('fs');
const path = require('path');
const net = require('net');
const { spawnSync } = require('child_process');

const NODE_DIR = path.resolve(__dirname, '..');
const ROOT_DIR = path.resolve(NODE_DIR, '..');
const TARGET_NODE_EXE = path.join(NODE_DIR, 'node.exe');
const PM2_CMD = path.join(NODE_DIR, 'pm2.cmd');
const LOG_DIR = path.join(ROOT_DIR, 'logs', 'pm2-service');
const LOG_FILE = path.join(LOG_DIR, 'pm2-service-runner.log');
const CHECK_INTERVAL_MS = 60 * 1000;

delete process.env.PM2_INSTALL_DIRECTORY;
delete process.env.PM2_SERVICE_DIRECTORY;
process.env.PATH = NODE_DIR + ';' + (process.env.PATH || '');
process.env.PM2_HOME = path.join(ROOT_DIR, '.pm2');

const rpcMatch = /^tcp:\/\/([^:]+):(\d+)$/.exec(process.env.PM2_DAEMON_RPC_PORT || '');
const RPC_HOST = rpcMatch ? rpcMatch[1] : '127.0.0.1';
const RPC_PORT = rpcMatch ? Number(rpcMatch[2]) : 42681;

function log(msg, level) {
  const line = '[' + new Date().toISOString().replace('T', ' ').substring(0, 19) + '] [' + (level || 'INFO') + '] ' + msg;
  console.log(line);
  try {
    fs.mkdirSync(LOG_DIR, { recursive: true });
    fs.appendFileSync(LOG_FILE, line + '\r\n', 'utf8');
  } catch (e) {}
}

function isDaemonAlive() {
  return new Promise((resolve) => {
    const sock = net.connect({ host: RPC_HOST, port: RPC_PORT });
    const done = (ok) => { sock.destroy(); resolve(ok); };
    sock.setTimeout(2000);
    sock.once('connect', () => done(true));
    sock.once('timeout', () => done(false));
    sock.once('error', () => done(false));
  });
}

function ensurePortableDaemonRuntime() {
  try {
    const psCmd = 'Get-CimInstance Win32_Process | Where-Object { $_.Name -like "*node*" -and $_.CommandLine -like "*Daemon.js*" } | Select-Object -First 1 -Property ProcessId, ExecutablePath | ConvertTo-Json';
    const res = spawnSync('powershell', ['-NoProfile', '-Command', psCmd], { encoding: 'utf8', windowsHide: true, timeout: 8000 });
    if (res.status === 0 && res.stdout && res.stdout.trim()) {
      const data = JSON.parse(res.stdout.trim());
      if (data && data.ExecutablePath) {
        const cur = path.normalize(data.ExecutablePath).toLowerCase();
        const exp = path.normalize(TARGET_NODE_EXE).toLowerCase();
        if (cur !== exp) {
          log('偵測到 PM2 Daemon 由非可攜 Node 運行 (' + data.ExecutablePath + ')，正在終止並切換...', 'WARN');
          spawnSync('"' + PM2_CMD + '" kill --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 15000 });
          spawnSync('powershell', ['-NoProfile', '-Command', 'Stop-Process -Id ' + data.ProcessId + ' -Force -ErrorAction SilentlyContinue'], { windowsHide: true, timeout: 5000 });
          return false;
        }
      }
    }
  } catch (e) {}
  return true;
}

function wireBundledLogrotate() {
  try {
    const localMod = path.join(NODE_DIR, 'node_modules', 'pm2-logrotate');
    const srcPkg = path.join(localMod, 'package.json');
    const srcApp = path.join(localMod, 'app.js');
    if (!fs.existsSync(srcPkg) || !fs.existsSync(srcApp)) return false;

    const pm2Home = process.env.PM2_HOME;
    const targetModDir = path.join(pm2Home, 'modules', 'pm2-logrotate', 'node_modules', 'pm2-logrotate');
    fs.mkdirSync(targetModDir, { recursive: true });
    fs.copyFileSync(srcPkg, path.join(targetModDir, 'package.json'));
    fs.writeFileSync(path.join(targetModDir, 'app.js'), 'require(' + JSON.stringify(srcApp) + ');\n', 'utf8');

    const confPath = path.join(pm2Home, 'module_conf.json');
    let conf = {};
    if (fs.existsSync(confPath)) {
      try { conf = JSON.parse(fs.readFileSync(confPath, 'utf8')) || {}; } catch (e) { conf = {}; }
    }
    if (!conf['module-db-v2'] || typeof conf['module-db-v2'] !== 'object') {
      conf['module-db-v2'] = {};
    }
    if (!conf['module-db-v2']['pm2-logrotate']) {
      conf['module-db-v2']['pm2-logrotate'] = { uid: null, gid: null };
    }
    const defaults = {
      max_size: '10M',
      retain: '30',
      compress: true,
      dateFormat: 'YYYY-MM-DD_HH-mm-ss',
      workerInterval: '30',
      rotateInterval: '0 0 * * *',
      rotateModule: true
    };
    conf['pm2-logrotate'] = Object.assign({}, defaults, conf['pm2-logrotate'] || {}, { compress: true });
    fs.writeFileSync(confPath, JSON.stringify(conf, null, 2), 'utf8');
    return true;
  } catch (e) {
    return false;
  }
}

function isLogrotateOnline() {
  try {
    const jres = spawnSync('"' + PM2_CMD + '" jlist', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 15000 });
    if (jres.status === 0 && jres.stdout && jres.stdout.trim()) {
      const list = JSON.parse(jres.stdout.trim());
      if (Array.isArray(list)) {
        return list.some((item) => item && item.name === 'pm2-logrotate' && item.pm2_env && item.pm2_env.status === 'online');
      }
    }
  } catch (e) {}
  return false;
}

function ensureLogrotate() {
  wireBundledLogrotate();
  if (isLogrotateOnline()) return;
  const localMod = path.join(NODE_DIR, 'node_modules', 'pm2-logrotate');
  log('偵測到 pm2-logrotate 尚未在線，正在由便攜版 ' + localMod + ' 啟動...', 'INFO');
  let res = spawnSync('"' + PM2_CMD + '" restart pm2-logrotate --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 25000 });
  if (res.status !== 0 || !isLogrotateOnline()) {
    const targetPkg = fs.existsSync(localMod) ? localMod : 'pm2-logrotate';
    res = spawnSync('"' + PM2_CMD + '" install "' + targetPkg + '" --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 60000 });
    if (res.status === 0) {
      spawnSync('"' + PM2_CMD + '" set pm2-logrotate:compress true --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 15000 });
    }
  }
  if (isLogrotateOnline()) {
    log('便攜版 pm2-logrotate 模組已在線運作', 'SUCCESS');
    spawnSync('"' + PM2_CMD + '" save --force --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 15000 });
  } else {
    log('啟動 pm2-logrotate 警示: ' + ((res && (res.stderr || res.stdout)) || '').trim(), 'WARN');
  }
}

function resurrect() {
  log('執行 pm2 resurrect 還原受管程序...');
  const res = spawnSync('"' + PM2_CMD + '" resurrect --no-color', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 120000 });
  if (res.status === 0) {
    log('pm2 resurrect 完成', 'SUCCESS');
  } else {
    log('pm2 resurrect 回傳碼 ' + res.status + ' ' + (res.stderr || res.stdout || '').trim(), 'WARN');
  }
}

async function tick() {
  const runtimeOk = ensurePortableDaemonRuntime();
  if (!runtimeOk || !(await isDaemonAlive())) {
    resurrect();
  }
  ensureLogrotate();
}

log('PM2_Portable_Service 啟動，根目錄 ' + ROOT_DIR + '，PM2_HOME ' + process.env.PM2_HOME);
tick();
setInterval(tick, CHECK_INTERVAL_MS);
