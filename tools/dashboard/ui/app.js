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

  // 組態編輯器狀態
  currentConfigPath: '',
  availableConfigs: [],
  originalConfigContent: '',
  isConfigDirty: false,
  currentConfigHasExample: false,
  isExamplePaneOpen: false,
  currentExampleContent: '',

  // 效能折線圖歷史緩衝區 (最多 60 筆取樣點)
  cpuHistory: [],
  memHistory: [],
  netRxHistory: [],
  netTxHistory: [],
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
  valNetRx: document.getElementById('valNetRx'),
  valNetTx: document.getElementById('valNetTx'),
  chartCpu: document.getElementById('chartCpu'),
  chartMem: document.getElementById('chartMem'),
  chartNet: document.getElementById('chartNet'),
  valLahMem: document.getElementById('valLahMem'),
  badgeLahMemPct: document.getElementById('badgeLahMemPct'),
  lahMemList: document.getElementById('lahMemList'),

  // 維運工具按鈕
  btnLaunchSandboxCmd: document.getElementById('btnLaunchSandboxCmd'),
  btnOpenFrontendWeb: document.getElementById('btnOpenFrontendWeb'),
  btnCheckApacheSyntax: document.getElementById('btnCheckApacheSyntax'),
  btnTriggerLogrotate: document.getElementById('btnTriggerLogrotate'),
  btnOpenLogsFolder: document.getElementById('btnOpenLogsFolder'),

  // 核心組態編輯器
  configSourceSelect: document.getElementById('configSourceSelect'),
  btnPickCustomConfig: document.getElementById('btnPickCustomConfig'),
  btnSaveConfig: document.getElementById('btnSaveConfig'),
  configBackupSelect: document.getElementById('configBackupSelect'),
  btnRestoreConfigBackup: document.getElementById('btnRestoreConfigBackup'),
  btnOpenConfigNotepad: document.getElementById('btnOpenConfigNotepad'),
  btnToggleExampleDiff: document.getElementById('btnToggleExampleDiff'),
  examplePane: document.getElementById('examplePane'),
  exampleFileName: document.getElementById('exampleFileName'),
  btnCopyExample: document.getElementById('btnCopyExample'),
  btnApplyExample: document.getElementById('btnApplyExample'),
  btnCloseExample: document.getElementById('btnCloseExample'),
  txtExampleViewer: document.getElementById('txtExampleViewer'),
  chkWrapConfig: document.getElementById('chkWrapConfig'),
  configPathInfo: document.getElementById('configPathInfo'),
  configDirtyFlag: document.getElementById('configDirtyFlag'),
  configMetaInfo: document.getElementById('configMetaInfo'),
  txtConfigEditor: document.getElementById('txtConfigEditor'),

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
  // 字體大小：基礎放大 20% (1.2x)，預設檔位為 1.15 ("中")
  const savedUserScale = localStorage.getItem('lah_dashboard_user_font_scale') || localStorage.getItem('lah_dashboard_font_scale') || '1.15';
  document.documentElement.style.setProperty('--font-user-scale', savedUserScale);
  document.documentElement.style.setProperty('--font-scale', `calc(1.2 * ${savedUserScale})`);
  if (el.selectFontScale) {
    el.selectFontScale.value = savedUserScale;
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
  document.documentElement.style.colorScheme = theme;
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
      document.documentElement.style.setProperty('--font-user-scale', scale);
      document.documentElement.style.setProperty('--font-scale', `calc(1.2 * ${scale})`);
      localStorage.setItem('lah_dashboard_user_font_scale', scale);
      localStorage.setItem('lah_dashboard_font_scale', scale);
      const label = scale === '1' ? '一般 (100%)' : scale === '1.15' ? '中 (+15%)' : '大 (+25%)';
      showToast(`✓ 已套用字體大小：${label} (基礎放大 20%)`);
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
        // 第 0 毫秒視覺即時回饋：所有狀態切換為重啟態
        setAllServicesRestartingVisual();

        try {
          const res = await invoke('restart_all_services');
          if (res && res.success) {
            showToast('✓ 重啟指令已送出，正在密集追蹤進程狀態...');
          } else {
            showToast(`⚠️ 重啟警示：${res ? res.message : '未知狀態'}`);
          }
          // 啟動 8 秒密集衝刺探測 (每 1.5 秒更新一次)
          triggerBurstPolling(8000, 1500);
        } catch (e) {
          showToast(`❌ 重啟失敗: ${e}`);
          await refreshAllData();
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
      showToast('正在執行日誌壓縮輪替與空間釋放...');
      const res = await invoke('trigger_logrotate');
      if (res && res.success) {
        showToast('✓ ' + res.message);
        // 即時刷新主機指標與磁碟空間
        await loadHostMetrics();
      } else {
        showToast('⚠️ ' + (res ? res.message : '觸發失敗'));
      }
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

  // ==========================================================================
  // 核心組態編輯器事件綁定
  // ==========================================================================
  el.configSourceSelect.addEventListener('change', async (e) => {
    const newPath = e.target.value;
    if (!newPath) return;

    if (state.isConfigDirty) {
      const prevPath = state.currentConfigPath;
      showConfirmModal(
        '未儲存變更提醒',
        '目前的組態檔案有尚未儲存的修改。若直接切換，修改將會遺失。\n是否確定捨棄並切換？',
        async () => {
          await fetchConfigContent(newPath);
        }
      );
      // 若取消，還原 select 選項
      el.configSourceSelect.value = prevPath;
    } else {
      await fetchConfigContent(newPath);
    }
  });

  el.btnPickCustomConfig.addEventListener('click', handlePickCustomConfig);
  el.btnSaveConfig.addEventListener('click', handleSaveConfig);
  el.btnRestoreConfigBackup.addEventListener('click', handleRestoreConfigBackup);

  el.btnOpenConfigNotepad.addEventListener('click', async () => {
    if (!state.currentConfigPath) {
      showToast('⚠️ 尚未選擇組態檔案');
      return;
    }
    try {
      const res = await invoke('open_log_in_notepad', { path: state.currentConfigPath });
      if (res && res.success) showToast('✓ 已使用記事本開啟組態');
      else showToast('⚠️ ' + (res ? res.message : '無法開啟'));
    } catch (e) {
      showToast('❌ 開啟失敗：' + e);
    }
  });

  el.btnToggleExampleDiff.addEventListener('click', () => {
    toggleExamplePane(!state.isExamplePaneOpen);
  });

  el.btnCloseExample.addEventListener('click', () => {
    toggleExamplePane(false);
  });

  el.btnCopyExample.addEventListener('click', async () => {
    const text = el.txtExampleViewer.value;
    if (!text) {
      showToast('⚠️ 無範本內容可複製');
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      showToast('✓ 已複製範本內容到剪貼簿！');
    } catch (err) {
      showToast('❌ 複製失敗');
    }
  });

  el.btnApplyExample.addEventListener('click', () => {
    if (!state.currentExampleContent) {
      showToast('⚠️ 目前無可套用的範本內容');
      return;
    }

    const currentVal = el.txtConfigEditor.value.trim();
    if (currentVal.length > 0) {
      showConfirmModal(
        '套用範本內容確認',
        '目前編輯區中已有文字內容。\n將範本套用至編輯區將會完全覆蓋現有內容。\n是否確定套用範本？',
        () => {
          el.txtConfigEditor.value = state.currentExampleContent;
          el.txtConfigEditor.dispatchEvent(new Event('input'));
          showToast('✓ 已成功將範本內容套入編輯區！');
        }
      );
    } else {
      el.txtConfigEditor.value = state.currentExampleContent;
      el.txtConfigEditor.dispatchEvent(new Event('input'));
      showToast('✓ 已成功將範本內容套入編輯區！');
    }
  });

  el.chkWrapConfig.addEventListener('change', (e) => {
    if (e.target.checked) {
      el.txtConfigEditor.classList.remove('no-wrap');
    } else {
      el.txtConfigEditor.classList.add('no-wrap');
    }
  });

  el.txtConfigEditor.addEventListener('input', () => {
    const isChanged = el.txtConfigEditor.value !== state.originalConfigContent;
    state.isConfigDirty = isChanged;
    if (isChanged) {
      el.configDirtyFlag.classList.remove('hidden');
    } else {
      el.configDirtyFlag.classList.add('hidden');
    }
    updateConfigEditorMeta();
  });

  el.txtConfigEditor.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = el.txtConfigEditor.selectionStart;
      const end = el.txtConfigEditor.selectionEnd;
      const val = el.txtConfigEditor.value;
      el.txtConfigEditor.value = val.substring(0, start) + '    ' + val.substring(end);
      el.txtConfigEditor.selectionStart = el.txtConfigEditor.selectionEnd = start + 4;
      el.txtConfigEditor.dispatchEvent(new Event('input'));
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      handleSaveConfig();
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
  } else if (tabId === 'tab-configs') {
    if (state.availableConfigs.length === 0) {
      loadCoreConfigsList();
    }
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

  if (action === 'restart') {
    // 視覺反饋
    const svcPills = el.systemServicesTableBody.querySelectorAll('.status-pill');
    svcPills.forEach((p) => {
      p.className = 'status-pill restarting';
      p.innerHTML = '<span class="spin-icon">🔄</span>重啟中...';
    });
  }

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
    if (action === 'restart') {
      triggerBurstPolling(6000, 1200);
    } else {
      await loadSystemServices();
    }
  } catch (e) {
    showToast(`❌ ${actionText}失敗: ${e}`);
    await loadSystemServices();
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

  if (action === 'restart') {
    // 視覺即時反饋：找到該目標所在的列轉為重啟態
    const rows = el.pm2TableBody.querySelectorAll('tr');
    rows.forEach((r) => {
      if (r.textContent.includes(target)) {
        const pill = r.querySelector('.status-pill');
        if (pill) {
          pill.className = 'status-pill restarting';
          pill.innerHTML = '<span class="spin-icon">🔄</span>重啟中...';
        }
      }
    });
  }

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
    if (action === 'restart') {
      triggerBurstPolling(6000, 1200);
    } else {
      await loadPm2Processes();
    }
  } catch (e) {
    showToast(`❌ ${actionText}失敗: ${e}`);
    await loadPm2Processes();
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
// 5. 系統指標與通訊埠探針 (分頁三 - Windows 效能風格)
// ==========================================================================
const drawPerfChart = (canvas, dataSeries) => {
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  const width = rect.width || 280;
  const height = rect.height || 65;

  if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) {
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
  }

  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, width, height);

  const isLight = document.documentElement.getAttribute('data-theme') === 'light';
  const gridColor = isLight ? 'rgba(0, 0, 0, 0.08)' : 'rgba(255, 255, 255, 0.07)';

  // 繪製工作管理員風格網格
  ctx.strokeStyle = gridColor;
  ctx.lineWidth = 1;
  ctx.beginPath();
  const hSteps = 3;
  for (let i = 1; i < hSteps; i++) {
    const y = Math.round((height / hSteps) * i);
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
  }
  const vSteps = 6;
  for (let i = 1; i < vSteps; i++) {
    const x = Math.round((width / vSteps) * i);
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
  }
  ctx.stroke();

  // 繪製波形曲線
  dataSeries.forEach((s) => {
    const pts = s.data || [];
    if (pts.length === 0) return;

    const maxVal = s.maxVal || 100;
    const maxPts = 60; // 60 筆採樣點
    const stepX = width / (maxPts - 1);
    const startIndex = maxPts - pts.length;

    let firstX = startIndex * stepX;
    let firstY = height;

    ctx.beginPath();
    pts.forEach((val, idx) => {
      const x = (startIndex + idx) * stepX;
      const normalized = Math.min(Math.max(val / maxVal, 0), 1);
      const y = height - normalized * (height - 6) - 3;

      if (idx === 0) {
        firstX = x;
        firstY = y;
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });

    // 漸層填充
    if (s.fillColor) {
      const fillPath = new Path2D();
      pts.forEach((val, idx) => {
        const x = (startIndex + idx) * stepX;
        const normalized = Math.min(Math.max(val / maxVal, 0), 1);
        const y = height - normalized * (height - 6) - 3;
        if (idx === 0) fillPath.moveTo(x, y);
        else fillPath.lineTo(x, y);
      });
      fillPath.lineTo(width, height);
      fillPath.lineTo(firstX, height);
      fillPath.closePath();

      const grad = ctx.createLinearGradient(0, 0, 0, height);
      grad.addColorStop(0, s.fillColor);
      grad.addColorStop(1, 'transparent');
      ctx.fillStyle = grad;
      ctx.fill(fillPath);
    }

    // 線條筆畫
    ctx.strokeStyle = s.strokeColor;
    ctx.lineWidth = 1.8;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    ctx.stroke();
  });

  ctx.restore();
};

const loadHostMetrics = async () => {
  try {
    const m = await invoke('get_host_metrics');
    if (!m) return;

    // 1. CPU 指標與趨勢圖
    const cpuPct = Math.round(m.cpu_usage);
    el.valCpu.textContent = cpuPct;
    state.cpuHistory.push(cpuPct);
    if (state.cpuHistory.length > 60) state.cpuHistory.shift();
    drawPerfChart(el.chartCpu, [
      {
        data: state.cpuHistory,
        strokeColor: '#0ea5e9',
        fillColor: 'rgba(14, 165, 233, 0.25)',
        maxVal: 100,
      },
    ]);

    // 2. 記憶體指標與趨勢圖
    const memUsedMb = Math.round(m.mem_used / 1024 / 1024);
    const memTotalMb = Math.round(m.mem_total / 1024 / 1024);
    el.valMem.textContent = memUsedMb;
    el.textMemDetails.textContent = `已用 ${memUsedMb} MB / 總計 ${memTotalMb} MB (${m.mem_load_pct}%)`;

    state.memHistory.push(m.mem_load_pct);
    if (state.memHistory.length > 60) state.memHistory.shift();
    drawPerfChart(el.chartMem, [
      {
        data: state.memHistory,
        strokeColor: '#a855f7',
        fillColor: 'rgba(168, 85, 247, 0.22)',
        maxVal: 100,
      },
    ]);

    // 3. 網路即時流量與雙色趨勢圖
    if (m.network) {
      el.valNetRx.textContent = m.network.rx_formatted;
      el.valNetTx.textContent = m.network.tx_formatted;

      state.netRxHistory.push(m.network.rx_bytes_per_sec);
      state.netTxHistory.push(m.network.tx_bytes_per_sec);
      if (state.netRxHistory.length > 60) state.netRxHistory.shift();
      if (state.netTxHistory.length > 60) state.netTxHistory.shift();

      // 動態上限，隨歷史流量波動自我適配
      const maxNet = Math.max(
        100 * 1024,
        ...state.netRxHistory,
        ...state.netTxHistory
      );

      drawPerfChart(el.chartNet, [
        {
          data: state.netRxHistory,
          strokeColor: '#38bdf8',
          fillColor: 'rgba(56, 189, 248, 0.25)',
          maxVal: maxNet,
        },
        {
          data: state.netTxHistory,
          strokeColor: '#f59e0b',
          fillColor: 'rgba(245, 158, 11, 0.15)',
          maxVal: maxNet,
        },
      ]);
    }

    // 4. LAH 服務群組記憶體占用卡片
    if (m.lah_memory) {
      const lahMb = (m.lah_memory.total_bytes / 1024 / 1024).toFixed(1);
      el.valLahMem.textContent = lahMb;
      el.badgeLahMemPct.textContent = `佔系統 ${m.lah_memory.system_pct}%`;

      if (m.lah_memory.items && m.lah_memory.items.length > 0) {
        let memHtml = '';
        const maxMem = m.lah_memory.items[0].memory_bytes || 1;
        m.lah_memory.items.forEach((item) => {
          const barWidth = Math.round((item.memory_bytes / maxMem) * 100);
          const badgeClass = item.category === 'apache' ? 'apache' : 'pm2';
          const badgeText = item.category === 'apache' ? 'Apache' : 'PM2';

          memHtml += `
            <div class="mem-item-row">
              <div class="mem-item-info">
                <span class="mem-item-badge ${badgeClass}">${badgeText}</span>
                <span class="mem-item-name" title="${item.name}">${item.name}</span>
              </div>
              <div class="mem-item-bar-box">
                <div class="mem-mini-bar-bg">
                  <div class="mem-mini-bar-fill" style="width: ${barWidth}%;"></div>
                </div>
                <span class="mem-item-size">${item.memory_formatted}</span>
              </div>
            </div>
          `;
        });
        el.lahMemList.innerHTML = memHtml;
      } else {
        el.lahMemList.innerHTML = '<div class="loading-mem-row">無運行中之 LAH 服務進程</div>';
      }
    }

    // 5. 磁碟空間指標
    const diskFreeGb = (m.disk_free / 1024 / 1024 / 1024).toFixed(1);
    const diskTotalGb = (m.disk_total / 1024 / 1024 / 1024).toFixed(1);
    el.valDiskFree.textContent = diskFreeGb;
    el.barDisk.style.width = `${Math.min(m.disk_used_pct, 100)}%`;
    el.barDisk.className = m.disk_used_pct > 90 ? 'progress-bar-fill warning' : 'progress-bar-fill';
    el.textDiskDetails.textContent = `剩餘 ${diskFreeGb} GB / 總計 ${diskTotalGb} GB (已用 ${m.disk_used_pct}%)`;

    // 6. 通訊埠探針
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

let burstTimer = null;

const triggerBurstPolling = (totalMs = 8000, intervalMs = 1500) => {
  if (burstTimer) clearInterval(burstTimer);
  stopPollingScheduler();

  const startTime = Date.now();
  burstTimer = setInterval(async () => {
    const elapsed = Date.now() - startTime;
    await refreshAllData();

    if (elapsed >= totalMs) {
      clearInterval(burstTimer);
      burstTimer = null;
      if (state.isPolling) {
        startPollingScheduler();
      }
      showToast('✓ 服務狀態已全面恢復上線');
    }
  }, intervalMs);
};

const setAllServicesRestartingVisual = () => {
  if (el.systemServicesTableBody) {
    const svcPills = el.systemServicesTableBody.querySelectorAll('.status-pill');
    svcPills.forEach((p) => {
      p.className = 'status-pill restarting';
      p.innerHTML = '<span class="spin-icon">🔄</span>重啟中...';
    });
  }
  if (el.pm2TableBody) {
    const pm2Pills = el.pm2TableBody.querySelectorAll('.status-pill');
    pm2Pills.forEach((p) => {
      p.className = 'status-pill restarting';
      p.innerHTML = '<span class="spin-icon">🔄</span>重啟中...';
    });
  }
};

const stopPollingScheduler = () => {
  if (state.pollingTimer) {
    clearInterval(state.pollingTimer);
    state.pollingTimer = null;
  }
  if (burstTimer) {
    clearInterval(burstTimer);
    burstTimer = null;
  }
};

// ==========================================================================
// 6. 核心組態編輯器 (分頁四)
// ==========================================================================
const loadCoreConfigsList = async () => {
  try {
    const list = await invoke('get_core_configs');
    if (!list || list.length === 0) {
      el.configSourceSelect.innerHTML = '<option value="">未找到任何核心組態</option>';
      return;
    }
    state.availableConfigs = list;

    let html = '';
    let firstExistingPath = '';

    list.forEach((cfg) => {
      const statusText = cfg.exists ? `${formatBytes(cfg.size_bytes)}` : '不存在';
      const disabledAttr = cfg.exists ? '' : ' disabled';
      html += `<option value="${cfg.path}"${disabledAttr}>${cfg.label} (${statusText})</option>`;
      if (cfg.exists && !firstExistingPath) {
        firstExistingPath = cfg.path;
      }
    });

    el.configSourceSelect.innerHTML = html;

    if (firstExistingPath) {
      el.configSourceSelect.value = firstExistingPath;
      await fetchConfigContent(firstExistingPath);
    }
  } catch (err) {
    el.configSourceSelect.innerHTML = `<option value="">讀取組態清單失敗: ${err}</option>`;
  }
};

const fetchConfigContent = async (filePath) => {
  if (!filePath) return;
  state.currentConfigPath = filePath;
  el.configPathInfo.textContent = `路徑：${filePath}`;
  el.txtConfigEditor.value = '正在讀取組態內容...';
  el.txtConfigEditor.disabled = true;

  try {
    const content = await invoke('read_config_content', { path: filePath });
    el.txtConfigEditor.value = content;
    el.txtConfigEditor.disabled = false;
    state.originalConfigContent = content;
    state.isConfigDirty = false;
    el.configDirtyFlag.classList.add('hidden');
    updateConfigEditorMeta();

    // 載入該檔案之歷史備份
    await loadConfigBackupsList(filePath);

    // 檢查是否有同名範本檔 (.env.example 等)
    const cfgInfo = state.availableConfigs.find((c) => c.path === filePath);
    const hasExample = cfgInfo ? cfgInfo.has_example : false;
    state.currentConfigHasExample = hasExample;

    if (hasExample) {
      el.btnToggleExampleDiff.classList.remove('hidden');
      if (state.isExamplePaneOpen) {
        await loadExampleContent(filePath);
      }
    } else {
      el.btnToggleExampleDiff.classList.add('hidden');
      el.examplePane.classList.add('hidden');
      state.isExamplePaneOpen = false;
      el.btnToggleExampleDiff.classList.remove('btn-primary');
      el.btnToggleExampleDiff.classList.add('btn-secondary');
    }
  } catch (err) {
    el.txtConfigEditor.value = `❌ 無法讀取檔案內容：\n${err}`;
    el.txtConfigEditor.disabled = true;
    state.originalConfigContent = '';
    state.isConfigDirty = false;
    el.configDirtyFlag.classList.add('hidden');
    el.btnToggleExampleDiff.classList.add('hidden');
    el.examplePane.classList.add('hidden');
    state.isExamplePaneOpen = false;
    updateConfigEditorMeta();
  }
};

const toggleExamplePane = async (open) => {
  state.isExamplePaneOpen = open;
  if (open) {
    el.examplePane.classList.remove('hidden');
    el.btnToggleExampleDiff.classList.add('btn-primary');
    el.btnToggleExampleDiff.classList.remove('btn-secondary');
    if (state.currentConfigPath) {
      await loadExampleContent(state.currentConfigPath);
    }
  } else {
    el.examplePane.classList.add('hidden');
    el.btnToggleExampleDiff.classList.remove('btn-primary');
    el.btnToggleExampleDiff.classList.add('btn-secondary');
  }
};

const loadExampleContent = async (filePath) => {
  try {
    el.txtExampleViewer.value = '正在讀取範本內容...';
    const content = await invoke('get_config_example_content', { path: filePath });
    state.currentExampleContent = content || '';
    el.txtExampleViewer.value = content || '(範本內容為空)';

    const found = state.availableConfigs.find((c) => c.path === filePath);
    if (found && found.example_path) {
      const parts = found.example_path.split('\\');
      el.exampleFileName.textContent = parts[parts.length - 1] || '.env.example';
    } else {
      el.exampleFileName.textContent = '.env.example';
    }
  } catch (err) {
    el.txtExampleViewer.value = `❌ 無法讀取範本檔案：\n${err}`;
    state.currentExampleContent = '';
  }
};

const updateConfigEditorMeta = () => {
  const text = el.txtConfigEditor.value || '';
  const lines = text.split('\n').length;
  const bytes = new Blob([text]).size;
  el.configMetaInfo.textContent = `大小：${formatBytes(bytes)} ｜ 行數：${lines} ｜ 編碼：UTF-8`;
};

const loadConfigBackupsList = async (filePath) => {
  try {
    const backups = await invoke('get_config_backups', { path: filePath });
    if (!backups || backups.length === 0) {
      el.configBackupSelect.innerHTML = '<option value="">(無歷史備份)</option>';
      el.btnRestoreConfigBackup.disabled = true;
      return;
    }

    let html = '';
    backups.forEach((b) => {
      html += `<option value="${b.full_path}">${b.created} (${formatBytes(b.size_bytes)})</option>`;
    });
    el.configBackupSelect.innerHTML = html;
    el.btnRestoreConfigBackup.disabled = false;
  } catch (err) {
    el.configBackupSelect.innerHTML = '<option value="">(讀取備份失敗)</option>';
    el.btnRestoreConfigBackup.disabled = true;
  }
};

const handleSaveConfig = async () => {
  if (!state.currentConfigPath) {
    showToast('⚠️ 請先選擇欲編輯的組態檔案');
    return;
  }

  const content = el.txtConfigEditor.value;
  showToast('正在儲存組態並執行安全檢核...');

  try {
    const res = await invoke('save_config_content', {
      path: state.currentConfigPath,
      content,
    });

    if (!res.success) {
      showConfirmModal('組態儲存失敗', res.message, () => {});
      return;
    }

    // 若 Apache 語法檢驗失敗，彈窗警告並提供一鍵自動還原
    if (res.syntax_checked && res.syntax_error) {
      showConfirmModal(
        '⚠️ Apache 組態語法驗證失敗',
        `檔案已寫入，但執行 httpd.exe -t 檢測未通過！\n\n錯誤細節：\n${res.syntax_output}\n\n為避免 Apache 服務啟動失敗，建議立即還原至儲存前自動建立的備份檔。\n是否立即自動還原？`,
        async () => {
          if (res.backup_path) {
            showToast('正在自動還原至備份檔...');
            await invoke('restore_config_backup', {
              target_path: state.currentConfigPath,
              backup_path: res.backup_path,
            });
            await fetchConfigContent(state.currentConfigPath);
            showToast('✓ 已成功復原至儲存前狀態');
          }
        }
      );
      return;
    }

    // 成功儲存
    state.originalConfigContent = content;
    state.isConfigDirty = false;
    el.configDirtyFlag.classList.add('hidden');
    await loadConfigBackupsList(state.currentConfigPath);

    // 關聯服務重啟提示
    if (res.related_service === 'apache') {
      showConfirmModal(
        '組態儲存成功 (Apache 語法正確)',
        `${res.message}\n\n此組態隸屬 Apache 系統服務 (LAH_Web_Service)。\n是否立即重新啟動 Apache 服務以套用新設定？`,
        async () => {
          await window.handleServiceAction('restart', 'LAH_Web_Service');
        }
      );
    } else if (res.related_service === 'pm2') {
      showConfirmModal(
        '組態儲存成功',
        `${res.message}\n\n此組態隸屬 PM2 應用服務。\n是否立即重新啟動 PM2 進程以載入新設定？`,
        async () => {
          showToast('正在重啟 PM2 應用進程...');
          await invoke('restart_all_services');
          triggerBurstPolling(8000, 1500);
        }
      );
    } else {
      showToast('✓ ' + res.message);
    }
  } catch (err) {
    showToast(`❌ 儲存失敗：${err}`);
  }
};

const handleRestoreConfigBackup = async () => {
  const selectedBackup = el.configBackupSelect.value;
  if (!selectedBackup) {
    showToast('⚠️ 請先選擇欲還原的歷史備份檔');
    return;
  }

  const selectedText = el.configBackupSelect.options[el.configBackupSelect.selectedIndex].text;

  showConfirmModal(
    '組態歷史備份還原確認',
    `即將以備份檔 [${selectedText}] 覆蓋還原目前檔案：\n${state.currentConfigPath}\n\n目前的編輯內容將被取代，是否確定還原？`,
    async () => {
      showToast('正在執行組態還原...');
      try {
        const res = await invoke('restore_config_backup', {
          target_path: state.currentConfigPath,
          backup_path: selectedBackup,
        });

        if (res && res.success) {
          showToast('✓ ' + res.message);
          await fetchConfigContent(state.currentConfigPath);
        } else {
          showToast('⚠️ ' + (res ? res.message : '還原失敗'));
        }
      } catch (err) {
        showToast(`❌ 還原失敗：${err}`);
      }
    }
  );
};

const handlePickCustomConfig = async () => {
  try {
    const pickedPath = await invoke('pick_custom_config');
    if (!pickedPath) return; // 使用者取消

    // 檢查是否已在選單中
    let foundIndex = -1;
    for (let i = 0; i < el.configSourceSelect.options.length; i++) {
      if (el.configSourceSelect.options[i].value === pickedPath) {
        foundIndex = i;
        break;
      }
    }

    if (foundIndex >= 0) {
      el.configSourceSelect.selectedIndex = foundIndex;
    } else {
      const fileName = pickedPath.split('\\').pop() || pickedPath;
      const opt = document.createElement('option');
      opt.value = pickedPath;
      opt.textContent = `📂 自訂：${fileName}`;
      el.configSourceSelect.prepend(opt);
      el.configSourceSelect.selectedIndex = 0;
    }

    await fetchConfigContent(pickedPath);
    showToast(`✓ 已載入自訂檔案：${pickedPath}`);
  } catch (err) {
    showToast(`❌ 選擇檔案失敗: ${err}`);
  }
};

// 視窗載入即刻啟動
window.addEventListener('DOMContentLoaded', () => {
  initApp();
});
