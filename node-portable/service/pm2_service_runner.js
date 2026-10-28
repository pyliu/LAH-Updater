'use strict';
/**
 * 可攜 PM2 開機自啟服務入口 (WinSW 呼叫，無須人員登入桌面)。
 * 1. 鎖定 PATH 最前端為可攜 node 目錄，並統一 PM2_HOME 至 <根目錄>\.pm2。
 * 2. 以 TCP 探測 PM2 Daemon (預設 127.0.0.1:42681)，離線時執行 pm2 resurrect 還原 dump.pm2。
 * 3. 每 60 秒重複探測，Daemon 掉線即自動 resurrect。
 */
const fs = require('fs');
const path = require('path');
const net = require('net');
const { spawnSync } = require('child_process');

const NODE_DIR = path.resolve(__dirname, '..');
const ROOT_DIR = path.resolve(NODE_DIR, '..');
const PM2_CMD = path.join(NODE_DIR, 'pm2.cmd');
const LOG_DIR = path.join(ROOT_DIR, 'logs', 'pm2-service');
const LOG_FILE = path.join(LOG_DIR, 'pm2-service-runner.log');
const CHECK_INTERVAL_MS = 60 * 1000;

process.env.PATH = NODE_DIR + ';' + (process.env.PATH || '');
process.env.PM2_HOME = process.env.PM2_HOME || path.join(ROOT_DIR, '.pm2');

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

function resurrect() {
  log('PM2 Daemon 離線，執行 pm2 resurrect...');
  const res = spawnSync('"' + PM2_CMD + '" resurrect', { shell: true, encoding: 'utf8', windowsHide: true, timeout: 120000 });
  if (res.status === 0) {
    log('pm2 resurrect 完成', 'SUCCESS');
  } else {
    log('pm2 resurrect 回傳碼 ' + res.status + ' ' + (res.stderr || res.stdout || '').trim(), 'WARN');
  }
}

async function tick() {
  if (!(await isDaemonAlive())) resurrect();
}

log('PM2_Portable_Service 啟動，根目錄 ' + ROOT_DIR + '，PM2_HOME ' + process.env.PM2_HOME);
tick();
setInterval(tick, CHECK_INTERVAL_MS);
