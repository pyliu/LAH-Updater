'use strict';
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');
const zlib = require('zlib');
const { auditLogLifecycle } = require('../tools/node-portable/service/pm2_service_runner');

console.log('--- 正在執行日誌生命週期治理單元測試 (test_log_lifecycle.js) ---');

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'lah_test_log_lifecycle_'));
const logsDir = path.join(tmpDir, 'logs');
const apacheLogsDir = path.join(logsDir, 'apache');

fs.mkdirSync(apacheLogsDir, { recursive: true });

const now = new Date();
const pad = (n) => String(n).padStart(2, '0');
const todayApacheTag = `${now.getFullYear()}.${pad(now.getMonth() + 1)}.${pad(now.getDate())}`;

// 昨天與 35 天前之日期字串計算
const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000);
const yesterdayApacheTag = `${yesterday.getFullYear()}.${pad(yesterday.getMonth() + 1)}.${pad(yesterday.getDate())}`;
const yesterdayIsoTag = `${yesterday.getFullYear()}-${pad(yesterday.getMonth() + 1)}-${pad(yesterday.getDate())}`;

const thirtyFiveDaysAgo = new Date(Date.now() - 35 * 24 * 60 * 60 * 1000);

// 1. 建立測試檔案
// A. Apache 日誌
const apacheOldExpired = path.join(apacheLogsDir, 'access.2026.09.01.log');
fs.writeFileSync(apacheOldExpired, 'expired 35 days ago content\n');
fs.utimesSync(apacheOldExpired, thirtyFiveDaysAgo, thirtyFiveDaysAgo);

const apacheYesterday = path.join(apacheLogsDir, `access.${yesterdayApacheTag}.log`);
const yesterdayContent = 'yesterday access log content line 1\nyesterday access log content line 2\n';
fs.writeFileSync(apacheYesterday, yesterdayContent);

const apacheToday = path.join(apacheLogsDir, `access.${todayApacheTag}.log`);
const todayContent = 'active today log content\n';
fs.writeFileSync(apacheToday, todayContent);

// B. PM2 應用日誌
const pm2OldGzExpired = path.join(logsDir, 'fe_out-2_2026-09-01_00-00-00.log.gz');
fs.writeFileSync(pm2OldGzExpired, zlib.gzipSync(Buffer.from('old expired gz')));
fs.utimesSync(pm2OldGzExpired, thirtyFiveDaysAgo, thirtyFiveDaysAgo);

const pm2Yesterday = path.join(logsDir, `fe_out-2_${yesterdayIsoTag}_00-00-00.log`);
const pm2YesterdayContent = 'pm2 yesterday output\n';
fs.writeFileSync(pm2Yesterday, pm2YesterdayContent);

const pm2Active = path.join(logsDir, 'fe_out-2.log');
fs.writeFileSync(pm2Active, 'pm2 active log\n');

// 2. 執行生命週期治理
const logs = [];
const mockLogger = (msg, level) => logs.push(`[${level}] ${msg}`);
const result = auditLogLifecycle(tmpDir, mockLogger);

console.log('治理結果回傳:', result);
console.log('記錄訊息:\n' + logs.join('\n'));

// 3. 斷言驗證
// A. 35 天前過期日誌與壓縮檔是否已被刪除
assert.strictEqual(fs.existsSync(apacheOldExpired), false, '35 天前過期之 Apache 日誌必須被安全刪除');
assert.strictEqual(fs.existsSync(pm2OldGzExpired), false, '35 天前過期之 PM2 壓縮檔必須被安全刪除');

// B. 昨日歷史日誌是否已完成 Gzip 壓縮，且原 .log 已刪除
const apacheYesterdayGz = apacheYesterday + '.gz';
assert.strictEqual(fs.existsSync(apacheYesterday), false, '昨日之 Apache .log 原檔必須被移除');
assert.strictEqual(fs.existsSync(apacheYesterdayGz), true, '昨日之 Apache 日誌必須被壓縮為 .log.gz');
const uncompressedApache = zlib.gunzipSync(fs.readFileSync(apacheYesterdayGz)).toString('utf8');
assert.strictEqual(uncompressedApache, yesterdayContent, '壓縮檔解壓內容必須與原內容完全相符');

const pm2YesterdayGz = pm2Yesterday + '.gz';
assert.strictEqual(fs.existsSync(pm2Yesterday), false, '昨日之 PM2 .log 原檔必須被移除');
assert.strictEqual(fs.existsSync(pm2YesterdayGz), true, '昨日之 PM2 日誌必須被壓縮為 .log.gz');
const uncompressedPm2 = zlib.gunzipSync(fs.readFileSync(pm2YesterdayGz)).toString('utf8');
assert.strictEqual(uncompressedPm2, pm2YesterdayContent, 'PM2 壓縮檔解壓內容必須與原內容完全相符');

// C. 當日活躍日誌是否完整保留純文字明文，絕不被誤壓縮
assert.strictEqual(fs.existsSync(apacheToday), true, '當日 Apache 活躍日誌必須完整保留');
assert.strictEqual(fs.readFileSync(apacheToday, 'utf8'), todayContent);

assert.strictEqual(fs.existsSync(pm2Active), true, '當日 PM2 活躍日誌必須完整保留');
assert.strictEqual(fs.readFileSync(pm2Active, 'utf8'), 'pm2 active log\n');

// D. 回傳計數斷言
assert.strictEqual(result.compressed, 2, '應壓縮 2 個昨日日誌');
assert.strictEqual(result.deleted, 2, '應刪除 2 個 35 天前過期日誌');

// 清理測試暫存目錄
fs.rmSync(tmpDir, { recursive: true, force: true });
console.log('✅ 所有日誌生命週期治理單元測試通過！');
