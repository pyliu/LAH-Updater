/**
 * LAH 智慧管控系統 - PM2 Windows 原生系統服務守護腳本 (Service Runner & Watchdog)
 * 職責：
 * 1. 於 Windows 開機（免人員登入桌面）時由 WinSW 調用執行。
 * 2. 鎖定環境變數 PATH 最前端為 C:\AppServ\node。
 * 3. 智能比對 PM2 Daemon 核心執行路徑，杜絕外部 Node / 墊片干擾。
 * 4. 喚醒 (resurrect) 並巡檢補齊 5 大核心服務 (LAH-FE, LAH-SCHEDULER, LAH-WSS, lah-api-v85, pm2-logrotate)。
 * 5. 進入 Watchdog 常駐循環（每 60 秒），若有服務異常離線自動嘗試重啟並記錄。
 */
'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync, spawnSync } = require('child_process');

function r_path(p) {
  return path.normalize(p);
}

// 本腳本固定位於 <TARGET_ROOT>/node/service/service_runner.js，向上兩層自動推導真實根目錄
const SCRIPT_DIR = __dirname;
const AUTO_ROOT = path.resolve(SCRIPT_DIR, '..', '..');
const TARGET_ROOT = r_path(process.env.LAH_ROOT || AUTO_ROOT);
const NODE_DIR = path.join(TARGET_ROOT, 'node');
const TARGET_NODE_EXE = path.join(NODE_DIR, 'node.exe');
const PM2_BIN = path.join(NODE_DIR, 'pm2.cmd');
const LOG_DIR = path.join(TARGET_ROOT, 'logs', 'service');
const LOG_FILE = path.join(LOG_DIR, 'lah-service-runner.log');

function ensureDir(dir) {
  try {
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  } catch (e) {}
}

function log(msg, level) {
  level = level || 'INFO';
  const timeStr = new Date().toISOString().replace('T', ' ').substring(0, 19);
  const line = `[${timeStr}] [${level}] ${msg}`;
  console.log(line);
  try {
    ensureDir(LOG_DIR);
    fs.appendFileSync(LOG_FILE, line + '\r\n', 'utf8');
  } catch (e) {}
}

// 1. 環境變數鎖定（確保便攜 Node.js 與 Oracle Instant Client 優先於外部環境變數）
const OCI_DIR = path.join(TARGET_ROOT, 'instantclient_11_2');
const ociPath = fs.existsSync(OCI_DIR) ? `${OCI_DIR};` : (fs.existsSync('C:\\Tools\\instantclient_11_2') ? 'C:\\Tools\\instantclient_11_2;' : '');
process.env.PATH = `${NODE_DIR};${ociPath}${process.env.PATH || ''}`;

// 2. 處理 PM2_HOME 與 dump.pm2 跨使用者同步
function syncPm2Home() {
  const defaultPm2 = path.join(os.homedir(), '.pm2');
  const targetDump = path.join(defaultPm2, 'dump.pm2');

  if (!fs.existsSync(targetDump)) {
    log(`預設 PM2 存檔 [${targetDump}] 不存在，搜尋系統中現有 dump.pm2...`, 'INFO');
    const usersDir = 'C:\\Users';
    if (fs.existsSync(usersDir)) {
      try {
        const users = fs.readdirSync(usersDir);
        let newestFile = null;
        let newestMtime = 0;
        let newestModuleConf = null;

        for (const u of users) {
          const candidateDump = path.join(usersDir, u, '.pm2', 'dump.pm2');
          if (fs.existsSync(candidateDump)) {
            const stat = fs.statSync(candidateDump);
            if (stat.mtimeMs > newestMtime) {
              newestMtime = stat.mtimeMs;
              newestFile = candidateDump;
              const confCandidate = path.join(usersDir, u, '.pm2', 'module_conf.json');
              if (fs.existsSync(confCandidate)) newestModuleConf = confCandidate;
            }
          }
        }

        if (newestFile) {
          ensureDir(defaultPm2);
          fs.copyFileSync(newestFile, targetDump);
          log(`已自 [${newestFile}] 同步最新程序存檔至 [${targetDump}]`, 'SUCCESS');
          if (newestModuleConf) {
            fs.copyFileSync(newestModuleConf, path.join(defaultPm2, 'module_conf.json'));
            log(`已自 [${newestModuleConf}] 同步模組設定檔`, 'SUCCESS');
          }
        }
      } catch (err) {
        log(`搜尋/同步 dump.pm2 發生警示: ${err.message}`, 'WARN');
      }
    }
  }
}

// 3. 智能比對 PM2 Daemon 核心執行路徑
function auditDaemonRuntime() {
  log('正在檢測現有 PM2 守護程序 (Daemon) 執行路徑...', 'INFO');
  try {
    const psCmd = 'Get-CimInstance Win32_Process | Where-Object { $_.Name -like "*node*" -and $_.CommandLine -like "*Daemon.js*" } | Select-Object -First 1 -Property ProcessId, ExecutablePath | ConvertTo-Json';
    const res = spawnSync('powershell', ['-NoProfile', '-Command', psCmd], { encoding: 'utf8', timeout: 8000 });
    if (res.status === 0 && res.stdout.trim()) {
      const data = JSON.parse(res.stdout.trim());
      if (data && data.ExecutablePath) {
        const currentExe = path.normalize(data.ExecutablePath).toLowerCase();
        const expectedExe = TARGET_NODE_EXE.toLowerCase();
        if (currentExe !== expectedExe) {
          log(`⚠️ 偵測到現有 PM2 核心由非目標 Node 運行 [${data.ExecutablePath}] (PID: ${data.ProcessId})，正在切換...`, 'WARN');
          spawnSync(PM2_BIN, ['kill', '--no-color'], { timeout: 15000 });
          return false;
        } else {
          log(`✅ PM2 核心已確認由綠色 Node 運行 (PID: ${data.ProcessId})`, 'INFO');
          return true;
        }
      }
    }
  } catch (e) {
    log(`檢測 Daemon 發生例外: ${e.message}`, 'WARN');
  }
  return true;
}

// 4. 執行 PM2 喚醒與核心服務巡檢補啟動
function resurrectAndEnsureServices() {
  log('⏳ 正在執行 [pm2 resurrect] 喚醒受控服務...', 'INFO');
  try {
    const res = spawnSync(PM2_BIN, ['resurrect', '--no-color'], { encoding: 'utf8', timeout: 30000 });
    if (res.status === 0) {
      log('✅ 成功喚醒 PM2 服務清單！', 'SUCCESS');
    } else {
      log(`⚠️ pm2 resurrect 回傳: ${res.stderr || res.stdout}`, 'WARN');
    }
  } catch (e) {
    log(`❌ 呼叫 pm2 resurrect 失敗: ${e.message}`, 'ERROR');
  }

  // 取得目前在線列表
  const onlineApps = {};
  try {
    const jres = spawnSync(PM2_BIN, ['jlist'], { encoding: 'utf8', timeout: 15000 });
    if (jres.status === 0 && jres.stdout.trim()) {
      const list = JSON.parse(jres.stdout.trim());
      for (const item of list) {
        if (item.name && item.pm2_env && item.pm2_env.status === 'online') {
          onlineApps[item.name] = true;
        }
      }
    }
  } catch (e) {}

  const ecosystems = [
    { title: 'LAH-WSS', apps: ['LAH-WSS'], path: path.join(TARGET_ROOT, 'lah-messenger-server', 'ecosystem.config.js') },
    { title: 'LAH-FE / LAH-SCHEDULER', apps: ['LAH-FE', 'LAH-SCHEDULER'], path: path.join(TARGET_ROOT, 'LAH-NUXTJS', 'ecosystem.config.js') },
    { title: 'lah-api-v85', apps: ['lah-api-v85'], path: path.join(TARGET_ROOT, 'LAH-API', 'ecosystem.config.js') }
  ];

  let needSave = false;

  for (const eco of ecosystems) {
    if (!fs.existsSync(eco.path)) continue;
    const missing = eco.apps.filter(a => !onlineApps[a]);
    if (missing.length > 0) {
      log(`⚡ 偵測到 [${eco.title}] 之服務 ${JSON.stringify(missing)} 未在線，正在啟動 [${eco.path}]...`, 'WARN');
      try {
        const startRes = spawnSync(PM2_BIN, ['start', eco.path, '--no-color'], { encoding: 'utf8', timeout: 30000 });
        if (startRes.status === 0) {
          log(`  ✅ 成功啟動 [${eco.title}]！`, 'SUCCESS');
          needSave = true;
        } else {
          log(`  ⚠️ 啟動 [${eco.path}] 警示: ${startRes.stderr || startRes.stdout}`, 'WARN');
        }
      } catch (err) {
        log(`  ❌ 啟動失敗: ${err.message}`, 'ERROR');
      }
    }
  }

  // 檢查 pm2-logrotate 模組
  if (!onlineApps['pm2-logrotate']) {
    log('⚡ 偵測到日誌輪替模組 [pm2-logrotate] 尚未在線，正在嘗試啟動...', 'WARN');
    try {
      let modRes = spawnSync(PM2_BIN, ['restart', 'pm2-logrotate', '--no-color'], { encoding: 'utf8', timeout: 20000 });
      if (modRes.status !== 0) {
        const localMod = path.join(NODE_DIR, 'node_modules', 'pm2-logrotate');
        const targetPkg = fs.existsSync(localMod) ? localMod : 'pm2-logrotate';
        modRes = spawnSync(PM2_BIN, ['install', targetPkg, '--no-color'], { encoding: 'utf8', timeout: 45000 });
        if (modRes.status === 0) {
          spawnSync(PM2_BIN, ['set', 'pm2-logrotate:compress', 'true'], { timeout: 10000 });
        }
      }
      if (modRes.status === 0) {
        log('  ✅ 成功啟動 [pm2-logrotate] 模組！', 'SUCCESS');
        needSave = true;
      }
    } catch (err) {
      log(`  ❌ 啟動 pm2-logrotate 失敗: ${err.message}`, 'ERROR');
    }
  }

  if (needSave) {
    log('💾 正在儲存最新受控程序狀態 (pm2 save)...', 'INFO');
    try {
      spawnSync(PM2_BIN, ['save', '--force', '--no-color'], { timeout: 15000 });
    } catch (e) {}
  } else {
    log('✨ 受控服務巡檢完成，所有核心程序 (LAH-FE, LAH-SCHEDULER, LAH-WSS, lah-api-v85, pm2-logrotate) 均在線運行中！', 'SUCCESS');
  }
}

// 5. Watchdog 循環檢查
function watchdogCheck() {
  try {
    const res = spawnSync(PM2_BIN, ['jlist'], { encoding: 'utf8', timeout: 15000 });
    if (res.status === 0 && res.stdout.trim()) {
      const list = JSON.parse(res.stdout.trim());
      const apps = {};
      for (const item of list) {
        if (item.name) apps[item.name] = (item.pm2_env && item.pm2_env.status) || 'unknown';
      }

      const required = ['LAH-FE', 'LAH-SCHEDULER', 'LAH-WSS', 'lah-api-v85', 'pm2-logrotate'];
      const offline = required.filter(a => apps[a] !== 'online');

      if (offline.length > 0) {
        log(`⚠️ Watchdog 偵測到異常或離線服務: ${offline.join(', ')}，正在嘗試喚醒與重啟...`, 'WARN');
        resurrectAndEnsureServices();
      }
    }
  } catch (e) {
    log(`Watchdog 檢查循環警示: ${e.message}`, 'WARN');
  }
}

// 主流程啟動
log('=================================================================', 'INFO');
log('🚀 LAH PM2 Windows 原生系統服務已啟動 (Service Runner 運行中)', 'SUCCESS');
log(`執行檔: ${process.execPath} (PID: ${process.pid})`, 'INFO');

syncPm2Home();
auditDaemonRuntime();
resurrectAndEnsureServices();

log('🛡️ Watchdog 背景守護機制已啟動 (每 60 秒自動巡檢與修復)', 'INFO');

// 定時巡檢（每 60 秒）
const interval = setInterval(watchdogCheck, 60000);

// 優雅退出處理
function onShutdown(signal) {
  log(`接收到終止信號 [${signal}]，正在準備停止服務...`, 'INFO');
  clearInterval(interval);
  try {
    spawnSync(PM2_BIN, ['save', '--force', '--no-color'], { timeout: 10000 });
  } catch (e) {}
  log('LAH PM2 系統服務已安全退出。', 'INFO');
  process.exit(0);
}

process.on('SIGINT', () => onShutdown('SIGINT'));
process.on('SIGTERM', () => onShutdown('SIGTERM'));
