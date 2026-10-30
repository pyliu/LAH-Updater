/**
 * 智慧控管與監控系統 - 管理儀表板 (Dashboard) 前端主程式
 * 100% 純原生 JavaScript，零外連依賴，Tauri 2 IPC 橋接
 */

// 安全調用 Tauri 2 API
const invoke = async (cmd, args = {}) => {
  try {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      return await window.__TAURI__.core.invoke(cmd, args);
    }
    if (window.__TAURI__ && window.__TAURI__.invoke) {
      return await window.__TAURI__.invoke(cmd, args);
    }
    console.warn('[IPC] Tauri 環境尚未就緒：', cmd, args);
    return null;
  } catch (err) {
    console.error(`[IPC Error] 指令 [${cmd}] 執行失敗:`, err);
    throw err;
  }
};

// 全域狀態機
const state = {
  rootDir: '',
  isPolling: true,
  pollingInterval: 5000,
  isBackgroundPaused: false,
  activeTab: 'tab-services',
  pollingTimer: null,
  autoTail: true,
  currentLogPath: '',
  availableLogs: [],
  rawLogLines: [],
  lastLogFingerprint: '',
};

// UI 元素快取
const el = {
  rootDirText: document.getElementById('rootDirText'),
  rootBadge: document.getElementById('rootBadge'),
  selectPollingInterval: document.getElementById('selectPollingInterval'),
  btnRestartAll: document.getElementById('btnRestartAll'),
  btnTogglePolling: document.getElementById('btnTogglePolling'),
  btnManualRefresh: document.getElementById('btnManualRefresh'),
  selectFontScale: document.getElementById('selectFontScale'),
  btnToggleTheme: document.getElementById('btnToggleTheme'),
  themeIcon: document.getElementById('themeIcon'),
  themeText: document.getElementById('themeText'),
  pollingStatusIcon: document.getElementById('pollingStatusIcon'),
  pollingStatusText: document.getElementById('pollingStatusText'),
  footerStatusText: document.getElementById('footerStatusText'),
  lastUpdatedTime: document.getElementById('lastUpdatedTime'),

  // 分頁元素
  tabBtns: document.querySelectorAll('.tab-btn'),
  tabPanes: document.querySelectorAll('.tab-pane'),

  // 服務表格
  systemServicesTableBody: document.getElementById('systemServicesTableBody'),
  pm2TableBody: document.getElementById('pm2TableBody'),

  // 日誌檢視器
  logSourceSelect: document.getElementById('logSourceSelect'),
  chkAutoTail: document.getElementById('chkAutoTail'),
  txtLogFilter: document.getElementById('txtLogFilter'),
  btnClearLogCanvas: document.getElementById('btnClearLogCanvas'),
  btnOpenNotepad: document.getElementById('btnOpenNotepad'),
  btnCopyLog: document.getElementById('btnCopyLog'),
  logPathInfo: document.getElementById('logPathInfo'),
  logMetaInfo: document.getElementById('logMetaInfo'),
  logViewport: document.getElementById('logViewport'),
  logContent: document.getElementById('logContent'),

  // 系統指標
  valCpu: document.getElementById('valCpu'),
  barCpu: document.getElementById('barCpu'),
  valMem: document.getElementById('valMem'),
  barMem: document.getElementById('barMem'),
  textMemDetails: document.getElementById('textMemDetails'),
  valDiskFree: document.getElementById('valDiskFree'),
  barDisk: document.getElementById('barDisk'),
  textDiskDetails: document.getElementById('textDiskDetails'),
  portProbeGrid: document.getElementById('portProbeGrid'),

  // 維運工具按鈕
  btnLaunchSandboxCmd: document.getElementById('btnLaunchSandboxCmd'),
  btnOpenFrontendWeb: document.getElementById('btnOpenFrontendWeb'),
  btnCheckApacheSyntax: document.getElementById('btnCheckApacheSyntax'),
  btnTriggerLogrotate: document.getElementById('btnTriggerLogrotate'),
  btnOpenLogsFolder: document.getElementById('btnOpenLogsFolder'),

  // 彈跳視窗與 Toast
  toast: document.getElementById('toast'),
  confirmModal: document.getElementById('confirmModal'),
  modalTitle: document.getElementById('modalTitle'),
  modalBody: document.getElementById('modalBody'),
  btnModalCancel: document.getElementById('btnModalCancel'),
  btnModalConfirm: document.getElementById('btnModalConfirm'),
};

// 格式化輔助函式
const formatBytes = (bytes) => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatUptime = (timestampMs) => {
  if (!timestampMs || timestampMs === 0) return '-';
  const uptimeSec = Math.floor((Date.now() - timestampMs) / 1000);
  if (uptimeSec < 60) return `${uptimeSec} 秒`;
  const minutes = Math.floor(uptimeSec / 60);
  if (minutes < 60) return `${minutes} 分鐘`;
  const hours = Math.floor(minutes / 60);
  const remMinutes = minutes % 60;
  if (hours < 24) return `${hours} 小時 ${remMinutes} 分`;
  const days = Math.floor(hours / 24);
  return `${days} 天 ${hours % 24} 時`;
};

// Toast 提示
let toastTimeout = null;
const showToast = (message) => {
  if (toastTimeout) clearTimeout(toastTimeout);
  el.toast.textContent = message;
  el.toast.classList.add('show');
  toastTimeout = setTimeout(() => {
    el.toast.classList.remove('show');
  }, 3200);
};

// 二次確認對話框
let confirmCallback = null;
const showConfirmModal = (title, message, onConfirm) => {
  el.modalTitle.textContent = title;
  el.modalBody.textContent = message;
  confirmCallback = onConfirm;
  el.confirmModal.classList.remove('hidden');
};

el.btnModalCancel.addEventListener('click', () => {
  el.confirmModal.classList.add('hidden');
  confirmCallback = null;
});

el.btnModalConfirm.addEventListener('click', async () => {
  el.confirmModal.classList.add('hidden');
  if (confirmCallback) {
    const cb = confirmCallback;
    confirmCallback = null;
    await cb();
  }
});

// ==========================================================================
// 0. 主題色調與字體縮放管理 (本地持久化)
// ==========================================================================
const initThemeAndFont = () => {
  // 字體大小：依使用者指示預設為 1.15 ("中")
  const savedScale = localStorage.getItem('lah_dashboard_font_scale') || '1.15';
  document.documentElement.style.setProperty('--font-scale', savedScale);
  if (el.selectFontScale) {
    el.selectFontScale.value = savedScale;
  }

  // 主題色調：預設為 dark
  const savedTheme = localStorage.getItem('lah_dashboard_theme') || 'dark';
  applyTheme(savedTheme);

  // 輪詢週期：依使用者指示預設為 5000ms (5 秒)
  const savedInterval = parseInt(localStorage.getItem('lah_dashboard_polling_interval'), 10) || 5000;
  state.pollingInterval = savedInterval;
  if (el.selectPollingInterval) {
    el.selectPollingInterval.value = String(savedInterval);
  }
};

const applyTheme = (theme) => {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('lah_dashboard_theme', theme);
  if (theme === 'light') {
    el.themeIcon.textContent = '🌙';
    el.themeText.textContent = '深色';
    el.btnToggleTheme.title = '切換至深邃主題';
  } else {
    el.themeIcon.textContent = '☀️';
    el.themeText.textContent = '明亮';
    el.btnToggleTheme.title = '切換至明亮主題';
  }
};

// ==========================================================================
// 1. 初始化與安裝根目錄載入
// ==========================================================================
const initApp = async () => {
  initThemeAndFont();

  // 綁定字體切換事件
  if (el.selectFontScale) {
    el.selectFontScale.addEventListener('change', (e) => {
      const scale = e.target.value;
      document.documentElement.style.setProperty('--font-scale', scale);
      localStorage.setItem('lah_dashboard_font_scale', scale);
      const label = scale === '1' ? '一般 (100%)' : scale === '1.15' ? '中 (+15%)' : '大 (+25%)';
      showToast(`✓ 已套用字體大小：${label}`);
    });
  }

  // 綁定主題切換事件
  if (el.btnToggleTheme) {
    el.btnToggleTheme.addEventListener('click', () => {
      const currentTheme = document.documentElement.getAttribute('data-theme') || 'dark';
      const newTheme = currentTheme === 'light' ? 'dark' : 'light';
      applyTheme(newTheme);
      showToast(newTheme === 'light' ? '✓ 已切換為明亮主題' : '✓ 已切換為深色主題');
    });
  }

  // 綁定輪詢時間切換事件
  if (el.selectPollingInterval) {
    el.selectPollingInterval.addEventListener('change', (e) => {
      const val = parseInt(e.target.value, 10) || 5000;
      state.pollingInterval = val;
      localStorage.setItem('lah_dashboard_polling_interval', String(val));
      if (state.isPolling) {
        startPollingScheduler();
      }
      showToast(`✓ 已調整自動輪詢週期為 ${val / 1000} 秒`);
    });
  }

  // 效能保護：監聽頁面能見度 (Visibility API)
  // 當儀表板視窗最小化或退到背景時自動暫停輪詢，徹底避免不必要的 CPU 與 PM2 呼叫開銷
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') {
      if (state.isPolling) {
        stopPollingScheduler();
        state.isBackgroundPaused = true;
      }
    } else {
      if (state.isBackgroundPaused && state.isPolling) {
        state.isBackgroundPaused = false;
        startPollingScheduler();
        refreshAllData();
      }
    }
  });
  try {
    const info = await invoke('get_app_info');
    if (info) {
      state.rootDir = info.root_dir;
      el.rootDirText.textContent = info.root_dir;
      if (info.is_valid_root) {
        el.rootBadge.title = `已鎖定標準安裝根目錄 (${info.root_dir})`;
      } else {
        el.rootBadge.title = `注意：目錄缺少標準環境腳本 (${info.root_dir})`;
      }
    }
  } catch (err) {
    el.rootDirText.textContent = '無法辨識安裝目錄';
  }

  // 綁定分頁切換
  el.tabBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      const targetTab = btn.getAttribute('data-tab');
      switchTab(targetTab);
    });
  });

  // 綁定全域按鈕
  el.btnTogglePolling.addEventListener('click', togglePolling);
  el.btnManualRefresh.addEventListener('click', () => refreshAllData(true));
  el.btnRestartAll.addEventListener('click', () => {
    showConfirmModal(
      '一鍵重啟全部服務確認',
      '即將重新啟動底層 Apache 系統服務 (LAH_Web_Service) 與所有 PM2 應用進程（前端、即時通、排程與 FastCGI）。是否確定執行？',
      async () => {
        showToast('正在執行全域服務重啟...');
        try {
          const res = await invoke('restart_all_services');
          if (res && res.success) {
            showToast('✓ 全域服務重啟指令已成功送出！');
          } else {
            showToast(`⚠️ 重啟警示：${res ? res.message : '未知狀態'}`);
          }
          await refreshAllData();
        } catch (e) {
          showToast(`❌ 重啟失敗: ${e}`);
        }
      }
    );
  });

  // 快捷維運工具箱
  el.btnLaunchSandboxCmd.addEventListener('click', async () => {
    try {
      const res = await invoke('open_lah_cmd');
      if (res && res.success) showToast('✓ ' + res.message);
      else showToast('⚠️ ' + (res ? res.message : '無法開啟終端'));
    } catch (e) {
      showToast('❌ 開啟失敗：' + e);
    }
  });

  el.btnOpenFrontendWeb.addEventListener('click', async () => {
    try {
      const res = await invoke('open_web_frontend');
      if (res && res.success) showToast('✓ ' + res.message);
    } catch (e) {
      showToast('❌ 開啟瀏覽器失敗：' + e);
    }
  });

  el.btnCheckApacheSyntax.addEventListener('click', async () => {
    try {
      const res = await invoke('check_apache_syntax');
      if (res && res.success) {
        showToast('✓ ' + res.message);
      } else {
        showConfirmModal('Apache 組態語法警告', res ? res.message : '語法異常', () => {});
      }
    } catch (e) {
      showToast('❌ 檢查失敗：' + e);
    }
  });

  el.btnTriggerLogrotate.addEventListener('click', async () => {
    try {
      showToast('正在觸發日誌輪替...');
      const res = await invoke('trigger_logrotate');
      if (res && res.success) showToast('✓ ' + res.message);
      else showToast('⚠️ ' + (res ? res.message : '觸發失敗'));
    } catch (e) {
      showToast('❌ 觸發失敗：' + e);
    }
  });

  el.btnOpenLogsFolder.addEventListener('click', async () => {
    try {
      const res = await invoke('open_logs_folder');
      if (res && res.success) showToast('✓ ' + res.message);
    } catch (e) {
      showToast('❌ 開啟失敗：' + e);
    }
  });

  // 日誌檢視器事件
  el.logSourceSelect.addEventListener('change', (e) => {
    state.currentLogPath = e.target.value;
    updateLogMetaDisplay();
    fetchLogTail(true);
  });

  el.chkAutoTail.addEventListener('change', (e) => {
    state.autoTail = e.target.checked;
    if (state.autoTail) fetchLogTail(false);
  });

  el.txtLogFilter.addEventListener('input', () => {
    renderFilteredLog();
  });

  el.btnClearLogCanvas.addEventListener('click', () => {
    state.rawLogLines = [];
    el.logContent.textContent = '【已清空日誌畫布】';
  });

  el.btnOpenNotepad.addEventListener('click', async () => {
    if (!state.currentLogPath) {
      showToast('⚠️ 尚未選擇日誌檔案');
      return;
    }
    try {
      const res = await invoke('open_log_in_notepad', { path: state.currentLogPath });
      if (res && res.success) showToast('✓ 已啟動記事本開啟日誌');
      else showToast('⚠️ ' + (res ? res.message : '無法開啟'));
    } catch (e) {
      showToast('❌ 開啟失敗：' + e);
    }
  });

  el.btnCopyLog.addEventListener('click', async () => {
    const text = el.logContent.textContent;
    if (!text) {
      showToast('⚠️ 無日誌內容可複製');
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      showToast('✓ 日誌已複製到剪貼簿！');
    } catch (err) {
      showToast('❌ 複製失敗');
    }
  });

  // 初始讀取各項清單與排程輪詢
  await loadAvailableLogsList();
  await refreshAllData();
  startPollingScheduler();
};

// ==========================================================================
// 2. 分頁切換
// ==========================================================================
const switchTab = (tabId) => {
  state.activeTab = tabId;
  el.tabBtns.forEach((btn) => {
    btn.classList.toggle('active', btn.getAttribute('data-tab') === tabId);
  });
  el.tabPanes.forEach((pane) => {
    pane.classList.toggle('active', pane.id === tabId);
  });

  if (tabId === 'tab-services') {
    loadSystemServices();
    loadPm2Processes();
  } else if (tabId === 'tab-logs') {
    fetchLogTail(true);
  } else if (tabId === 'tab-metrics') {
    loadHostMetrics();
  }
};

// ==========================================================================
// 3. 服務狀態查詢與渲染 (分頁一)
// ==========================================================================
const loadSystemServices = async () => {
  try {
    const list = await invoke('get_system_services');
    if (!list || list.length === 0) {
      el.systemServicesTableBody.innerHTML = '<tr><td colspan="4" class="text-center loading-row">未偵測到註冊之系統服務</td></tr>';
      return;
    }

    let html = '';
    list.forEach((s) => {
      const isRun = s.is_running;
      const statusPill = isRun
        ? '<span class="status-pill running"><span class="status-dot"></span>正在執行</span>'
        : s.is_installed
        ? '<span class="status-pill stopped"><span class="status-dot"></span>已停止</span>'
        : '<span class="status-pill unknown"><span class="status-dot"></span>未註冊</span>';

      const actions = `
        <div class="action-buttons">
          ${isRun ? `<button class="btn btn-secondary btn-sm" onclick="handleServiceAction('stop', '${s.name}')">停止</button>` : ''}
          ${!isRun && s.is_installed ? `<button class="btn btn-primary btn-sm" onclick="handleServiceAction('start', '${s.name}')">啟動</button>` : ''}
          ${s.is_installed ? `<button class="btn btn-warning btn-sm" onclick="handleServiceAction('restart', '${s.name}')">重啟</button>` : ''}
        </div>
      `;

      html += `
        <tr>
          <td><span class="service-name">${s.name}</span></td>
          <td><span class="service-desc">${s.display_name}</span></td>
          <td>${statusPill}</td>
          <td>${actions}</td>
        </tr>
      `;
    });
    el.systemServicesTableBody.innerHTML = html;
  } catch (err) {
    el.systemServicesTableBody.innerHTML = `<tr><td colspan="4" class="text-center loading-row text-danger">讀取系統服務失敗：${err}</td></tr>`;
  }
};

window.handleServiceAction = async (action, svcName) => {
  const actionText = action === 'start' ? '啟動' : action === 'stop' ? '停止' : '重啟';
  showToast(`正在${actionText}系統服務 [${svcName}]...`);
  try {
    let res;
    if (action === 'start') res = await invoke('start_system_service', { name: svcName });
    else if (action === 'stop') res = await invoke('stop_system_service', { name: svcName });
    else res = await invoke('restart_system_service', { name: svcName });

    if (res && res.success) {
      showToast(`✓ ${res.message}`);
    } else {
      showToast(`⚠️ ${res ? res.message : '操作未完成'}`);
    }
    await loadSystemServices();
  } catch (e) {
    showToast(`❌ ${actionText}失敗: ${e}`);
  }
};

const loadPm2Processes = async () => {
  try {
    const list = await invoke('get_pm2_processes');
    if (!list || list.length === 0) {
      el.pm2TableBody.innerHTML = '<tr><td colspan="8" class="text-center loading-row">目前無 PM2 進程在線或守護進程尚未就緒</td></tr>';
      return;
    }

    let html = '';
    list.forEach((p) => {
      const isOnline = p.status === 'online';
      const statusPill = isOnline
        ? '<span class="status-pill online"><span class="status-dot"></span>在線</span>'
        : `<span class="status-pill stopped"><span class="status-dot"></span>${p.status}</span>`;

      const memFormatted = formatBytes(p.memory);
      const uptimeStr = formatUptime(p.uptime);

      const actions = `
        <div class="action-buttons">
          ${isOnline ? `<button class="btn btn-secondary btn-sm" onclick="handlePm2Action('stop', '${p.name}')">停止</button>` : ''}
          ${!isOnline ? `<button class="btn btn-primary btn-sm" onclick="handlePm2Action('start', '${p.name}')">啟動</button>` : ''}
          <button class="btn btn-warning btn-sm" onclick="handlePm2Action('restart', '${p.name}')">重啟</button>
        </div>
      `;

      html += `
        <tr>
          <td><span class="text-dim">${p.pm_id}</span></td>
          <td><span class="service-name">${p.name}</span></td>
          <td><span class="font-mono text-dim">${p.pid > 0 ? p.pid : '-'}</span></td>
          <td>${statusPill}</td>
          <td><span class="font-mono">${p.cpu.toFixed(1)}%</span></td>
          <td><span class="font-mono">${memFormatted}</span></td>
          <td><span class="font-mono ${p.restarts > 5 ? 'text-warning' : ''}">${p.restarts}</span></td>
          <td>${actions}</td>
        </tr>
      `;
    });
    el.pm2TableBody.innerHTML = html;
  } catch (err) {
    el.pm2TableBody.innerHTML = `<tr><td colspan="8" class="text-center loading-row text-danger">讀取 PM2 進程失敗：${err}</td></tr>`;
  }
};

window.handlePm2Action = async (action, target) => {
  const actionText = action === 'start' ? '啟動' : action === 'stop' ? '停止' : '重啟';
  showToast(`正在${actionText} PM2 進程 [${target}]...`);
  try {
    let res;
    if (action === 'start') res = await invoke('start_pm2_process', { target });
    else if (action === 'stop') res = await invoke('stop_pm2_process', { target });
    else res = await invoke('restart_pm2_process', { target });

    if (res && res.success) {
      showToast(`✓ ${res.message}`);
    } else {
      showToast(`⚠️ ${res ? res.message : '操作失敗'}`);
    }
    await loadPm2Processes();
  } catch (e) {
    showToast(`❌ ${actionText}失敗: ${e}`);
  }
};

// ==========================================================================
// 4. 即時日誌檢視器 (分頁二)
// ==========================================================================
const loadAvailableLogsList = async () => {
  try {
    const list = await invoke('get_available_logs');
    if (!list || list.length === 0) return;

    state.availableLogs = list;
    let html = '';
    list.forEach((item) => {
      const optLabel = item.exists ? `${item.label} (${formatBytes(item.size_bytes)})` : `${item.label} [無日誌]`;
      html += `<option value="${item.path}" ${!item.exists ? 'disabled' : ''}>${optLabel}</option>`;
    });
    el.logSourceSelect.innerHTML = html;

    // 優先選取第一個存在的日誌
    const firstExist = list.find((i) => i.exists);
    if (firstExist) {
      el.logSourceSelect.value = firstExist.path;
      state.currentLogPath = firstExist.path;
      updateLogMetaDisplay();
    }
  } catch (err) {
    console.error('讀取可用日誌清單失敗：', err);
  }
};

const updateLogMetaDisplay = () => {
  const item = state.availableLogs.find((i) => i.path === state.currentLogPath);
  if (item) {
    el.logPathInfo.textContent = `路徑：${item.path}`;
    el.logMetaInfo.textContent = `大小：${formatBytes(item.size_bytes)} ｜ 最後修改：${item.modified}`;
  } else {
    el.logPathInfo.textContent = `路徑：${state.currentLogPath || '-'}`;
    el.logMetaInfo.textContent = '大小：- ｜ 最後修改：-';
  }
};

const fetchLogTail = async (forceScroll = false) => {
  if (!state.currentLogPath) return;

  try {
    const res = await invoke('get_log_tail', { path: state.currentLogPath, maxLines: 300 });
    if (res && res.lines) {
      const lineCount = res.lines.length;
      const lastLine = lineCount > 0 ? res.lines[lineCount - 1] : '';
      const currentFingerprint = `${lineCount}:${lastLine}:${res.file_size}`;

      // 效能優化：若日誌內容無變動且非強制捲動，跳過 DOM 重繪，極致保持平滑
      if (!forceScroll && currentFingerprint === state.lastLogFingerprint) {
        return;
      }

      state.lastLogFingerprint = currentFingerprint;
      state.rawLogLines = res.lines;
      renderFilteredLog(forceScroll);
    }
  } catch (err) {
    console.warn('讀取日誌末尾失敗：', err);
  }
};

const renderFilteredLog = (forceScroll = false) => {
  const kw = el.txtLogFilter.value.trim().toLowerCase();
  let filtered = state.rawLogLines;
  if (kw) {
    filtered = filtered.filter((line) => line.toLowerCase().includes(kw));
  }

  // 判斷使用者是否正在往上滾動檢視舊內容：
  // 若距底部超過 40px，表示使用者主動向上翻閱，此時不強制跳到底部，避免打斷閱讀
  const isNearBottom =
    el.logViewport.scrollHeight - el.logViewport.scrollTop - el.logViewport.clientHeight < 40;

  el.logContent.textContent = filtered.join('\n');

  if (forceScroll || (state.autoTail && isNearBottom)) {
    el.logViewport.scrollTop = el.logViewport.scrollHeight;
  }
};

// ==========================================================================
// 5. 系統指標與通訊埠探針 (分頁三)
// ==========================================================================
const loadHostMetrics = async () => {
  try {
    const m = await invoke('get_host_metrics');
    if (!m) return;

    // CPU 指標
    const cpuPct = Math.round(m.cpu_usage);
    el.valCpu.textContent = cpuPct;
    el.barCpu.style.width = `${Math.min(cpuPct, 100)}%`;
    el.barCpu.className = cpuPct > 80 ? 'progress-bar-fill warning' : 'progress-bar-fill';

    // 記憶體指標
    const memUsedMb = Math.round(m.mem_used / 1024 / 1024);
    const memTotalMb = Math.round(m.mem_total / 1024 / 1024);
    el.valMem.textContent = memUsedMb;
    el.barMem.style.width = `${Math.min(m.mem_load_pct, 100)}%`;
    el.barMem.className = m.mem_load_pct > 85 ? 'progress-bar-fill warning' : 'progress-bar-fill';
    el.textMemDetails.textContent = `已用 ${memUsedMb} MB / 總計 ${memTotalMb} MB (${m.mem_load_pct}%)`;

    // 磁碟空間指標
    const diskFreeGb = (m.disk_free / 1024 / 1024 / 1024).toFixed(1);
    const diskTotalGb = (m.disk_total / 1024 / 1024 / 1024).toFixed(1);
    el.valDiskFree.textContent = diskFreeGb;
    el.barDisk.style.width = `${Math.min(m.disk_used_pct, 100)}%`;
    el.barDisk.className = m.disk_used_pct > 90 ? 'progress-bar-fill warning' : 'progress-bar-fill';
    el.textDiskDetails.textContent = `剩餘 ${diskFreeGb} GB / 總計 ${diskTotalGb} GB (已用 ${m.disk_used_pct}%)`;

    // 通訊埠探針
    if (m.ports && m.ports.length > 0) {
      let portsHtml = '';
      m.ports.forEach((p) => {
        const isOnline = p.is_listening;
        const statusBadge = isOnline
          ? '<span class="status-pill running"><span class="status-dot"></span>監聽中</span>'
          : '<span class="status-pill stopped"><span class="status-dot"></span>離線</span>';

        portsHtml += `
          <div class="port-card">
            <div class="port-card-header">
              <span class="port-num">Port ${p.port}</span>
              ${statusBadge}
            </div>
            <div class="port-card-desc" title="${p.name}">${p.name}</div>
          </div>
        `;
      });
      el.portProbeGrid.innerHTML = portsHtml;
    }
  } catch (err) {
    console.warn('讀取主機指標失敗：', err);
  }
};

// ==========================================================================
// 6. 全方位資料重新整理與排程機制
// ==========================================================================
const refreshAllData = async (isManual = false) => {
  if (isManual) showToast('正在更新服務與系統指標...');

  if (state.activeTab === 'tab-services') {
    await Promise.all([loadSystemServices(), loadPm2Processes()]);
  } else if (state.activeTab === 'tab-logs') {
    await fetchLogTail(false);
  } else if (state.activeTab === 'tab-metrics') {
    await loadHostMetrics();
  }

  const now = new Date();
  const timeStr = now.toLocaleTimeString('zh-TW', { hour12: false });
  el.lastUpdatedTime.textContent = `最後更新：${timeStr}`;

  if (isManual) showToast('✓ 狀態更新完成');
};

const togglePolling = () => {
  state.isPolling = !state.isPolling;
  if (state.isPolling) {
    el.pollingStatusIcon.textContent = '⏸️';
    el.pollingStatusText.textContent = '暫停輪詢';
    el.footerStatusText.textContent = '系統監控總線運作中';
    startPollingScheduler();
    showToast(`✓ 已恢復自動狀態輪詢 (每 ${state.pollingInterval / 1000} 秒)`);
  } else {
    el.pollingStatusIcon.textContent = '▶️';
    el.pollingStatusText.textContent = '恢復輪詢';
    el.footerStatusText.textContent = '自動輪詢已暫停';
    stopPollingScheduler();
    showToast('已暫停自動輪詢');
  }
};

const startPollingScheduler = () => {
  stopPollingScheduler();

  // 主輪詢迴圈：依使用者設定之週期 (預設 5000ms) 探測當前分頁資料
  state.pollingTimer = setInterval(async () => {
    if (!state.isPolling) return;
    await refreshAllData();
  }, state.pollingInterval);
};

const stopPollingScheduler = () => {
  if (state.pollingTimer) {
    clearInterval(state.pollingTimer);
    state.pollingTimer = null;
  }
};

// 視窗載入即刻啟動
window.addEventListener('DOMContentLoaded', () => {
  initApp();
});
