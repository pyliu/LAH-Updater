const path = require('path');
const GUARD_NAME = 'lah-http-header-guard.cjs';
class ProtectionError extends Error {}
const canonical = value => path.resolve(value).replace(/\\/g, '/').toLowerCase();

function normalizeArgs(raw, config) {
  if (raw == null) return [];
  let args = raw;
  if (typeof raw === 'string') {
    const checked = config.validateJSON({ script: __filename, node_args: raw });
    if (checked.errors.length) throw new ProtectionError('無法解析既有 Node 啟動參數，未修改設定');
    args = checked.config.node_args;
  }
  if (!Array.isArray(args) || args.some(value => typeof value !== 'string')) {
    throw new ProtectionError('不支援既有 Node 啟動參數格式，未修改設定');
  }
  return args.slice();
}

function protectionArgs(args, guard, enabled) {
  const result = [];
  const own = value => typeof value === 'string' && canonical(value) === canonical(guard);
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if ((arg === '--require' || arg === '-r') && own(args[i + 1])) { i++; continue; }
    if (arg.startsWith('--require=') && own(arg.slice(10))) continue;
    result.push(arg);
  }
  if (enabled) result.push('--require', guard);
  return result;
}

function validateProcess(proc, expectedPath, requireOnline = true) {
  const env = proc && proc.pm2_env;
  if (!env || canonical(env.pm_exec_path || '') !== canonical(expectedPath)) throw new ProtectionError('程序路徑已變更，請重新整理後再操作');
  const interpreter = String(env.exec_interpreter || 'node').replace(/\\/g, '/').split('/').pop().toLowerCase();
  if (env.pmx_module || (env.axm_options && env.axm_options.isModule) || proc.name === 'pm2-logrotate') throw new ProtectionError('不支援 PM2 模組');
  if (!['node', 'node.exe'].includes(interpreter)) throw new ProtectionError('僅支援 Node.js 應用');
  const allowed = requireOnline ? ['online'] : ['online', 'stopped', 'errored'];
  if (!allowed.includes(env.status)) throw new ProtectionError(requireOnline ? '請先確認程序已上線，再變更標頭防護' : '程序狀態正在切換，請稍後重新整理再停用標頭防護');
  return env;
}

const call = (pm2, method, ...args) => new Promise((resolve, reject) => {
  pm2[method](...args, (error, result) => error ? reject(new ProtectionError(`PM2 ${method} 操作失敗，程序設定可能已變更，請重新整理確認`)) : resolve(result));
});

async function setProtection(pm2, config, { id, enabled, expectedPath, guard }) {
  if (!Number.isSafeInteger(id) || id < 0 || typeof enabled !== 'boolean' || !expectedPath) throw new ProtectionError('無效的標頭防護操作參數');
  const describe = async (requireOnline = true) => {
    const list = await call(pm2, 'describe', String(id));
    const proc = list.find(value => value.pm_id === id);
    if (!proc) throw new ProtectionError('找不到指定程序，請重新整理');
    return validateProcess(proc, expectedPath, requireOnline);
  };
  const env = await describe(enabled);
  const previous = normalizeArgs(env.node_args, config);
  const next = protectionArgs(previous, guard, enabled);
  const changed = JSON.stringify(previous) !== JSON.stringify(next);
  if (changed) {
    await call(pm2, 'restart', String(id), { node_args: next });
    let verified = false;
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        const updated = await describe();
        verified = JSON.stringify(normalizeArgs(updated.node_args, config)) === JSON.stringify(next);
        if (verified) break;
      } catch (error) {
        if (!String(error.message).includes('已上線')) throw error;
      }
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    if (!verified) throw new ProtectionError('程序已重新啟動，但防護設定或上線狀態未通過驗證，請重新整理檢查');
  }
  try { await call(pm2, 'dump'); } catch (error) {
    return { success: false, message: '標頭防護已套用，但快照未儲存；請按「儲存快照」完成持久化。' };
  }
  return { success: true, message: `已${enabled ? '啟用' : '停用'} ID ${id} 的標頭防護，並儲存 PM2 快照。請再次執行資安自檢確認實際回應。` };
}

async function main() {
  const [root, rawId, operation, expectedPath] = process.argv.slice(2);
  if (!root || !/^\d+$/.test(rawId || '') || !['enable', 'disable'].includes(operation) || !expectedPath) throw new ProtectionError('無效的標頭防護操作參數');
  const pm2Path = path.join(root, 'node', 'node_modules', 'pm2');
  const pm2 = require(pm2Path);
  const config = require(path.join(pm2Path, 'lib', 'tools', 'Config'));
  try {
    await call(pm2, 'connect');
    const result = await setProtection(pm2, config, {
      id: Number(rawId), enabled: operation === 'enable', expectedPath,
      guard: path.join(root, 'node', GUARD_NAME),
    });
    console.log(JSON.stringify(result));
  } finally { pm2.disconnect(); }
}

module.exports = { normalizeArgs, protectionArgs, validateProcess, setProtection };
if (require.main === module) main().catch(error => {
  console.log(JSON.stringify({ success: false, message: error instanceof ProtectionError ? error.message : '標頭防護操作未完成，程序設定可能已變更，請重新整理並檢查服務日誌。' }));
  process.exitCode = 1;
});
