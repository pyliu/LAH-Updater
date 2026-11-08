/**
 * 智慧控管與監控系統 - 管理儀表板 (Dashboard) 前端主程式
 * 100% 純原生 JavaScript，零外連依賴，Tauri 2 IPC 橋接
 */

// 安全調用 Tauri 2 API (附帶 15 秒防死當熔斷超時機制)
const invoke = async (cmd, args = {}, timeoutMs = 15000) => {
  const invokePromise = (async () => {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      return await window.__TAURI__.core.invoke(cmd, args);
    }
    if (window.__TAURI__ && window.__TAURI__.invoke) {
      return await window.__TAURI__.invoke(cmd, args);
    }
    console.warn('[IPC] Tauri 環境尚未就緒：', cmd, args);
    return null;
  })();

  if (!timeoutMs || timeoutMs <= 0) {
    try {
      return await invokePromise;
    } catch (err) {
      console.error(`[IPC Error] 指令 [${cmd}] 執行失敗:`, err);
      throw err;
    }
  }

  let timer = null;
  const timeoutPromise = new Promise((_, reject) => {
    timer = setTimeout(() => {
      reject(new Error(`指令 [${cmd}] 探測逾時 (超過 ${timeoutMs / 1000} 秒未回應)`));
    }, timeoutMs);
  });

  try {
    const res = await Promise.race([invokePromise, timeoutPromise]);
    clearTimeout(timer);
    return res;
  } catch (err) {
    clearTimeout(timer);
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
  isRestarting: false,
  isLoadingServices: false,
  isLoadingPm2: false,
  isRefreshingAll: false,
  activeTab: 'tab-metrics',
  restartingServices: new Set(),
  pollingTimer: null,
  autoTail: true,
  currentLogPath: '',
  availableLogs: [],
  apiLogFiles: [],
  rawLogLines: [],
  filteredLogLines: [],
  lastLogFingerprint: '',

  // 組態編輯器狀態
  currentConfigPath: '',
  availableConfigs: [],
  originalConfigContent: '',
  isConfigDirty: false,
  currentConfigHasExample: false,
  isExamplePaneOpen: false,
  currentExampleContent: '',
  configSyntaxLang: 'plain',
  configSearchQuery: '',
  configSearchMatches: [],
  configSearchCurrentIdx: -1,
  configActiveLineNum: 1,
  exampleActiveLineNum: 1,

  // 效能折線圖歷史緩衝區 (最多 60 筆取樣點)
  cpuHistory: [],
  memHistory: [],
  netRxHistory: [],
  netTxHistory: [],

  // 沙盒終端機狀態
  terminalTimer: null,
  terminalHistory: [],
  terminalHistoryIndex: -1,
  terminalIsInitialized: false,
  terminalInputBuffer: '',
  currentTerminalEncoding: 'utf-8',
  xtermInstance: null,
  fitAddonInstance: null,
  isTerminalMaximized: false,
};

// UI 元素快取
const el = {
  rootDirText: document.getElementById('rootDirText'),
  rootBadge: document.getElementById('rootBadge'),
  rootDot: document.getElementById('rootDot'),
  switchRootModal: document.getElementById('switchRootModal'),
  inputCustomRoot: document.getElementById('inputCustomRoot'),
  btnBrowseRootDir: document.getElementById('btnBrowseRootDir'),
  quickRootList: document.getElementById('quickRootList'),
  btnResetDefaultRoot: document.getElementById('btnResetDefaultRoot'),
  btnCancelSwitchRoot: document.getElementById('btnCancelSwitchRoot'),
  btnConfirmSwitchRoot: document.getElementById('btnConfirmSwitchRoot'),
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
  apiDateSelectGroup: document.getElementById('apiDateSelectGroup'),
  apiDateSelect: document.getElementById('apiDateSelect'),
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
  valNetTotal: document.getElementById('valNetTotal'),
  unitNetTotal: document.getElementById('unitNetTotal'),
  textNetDetails: document.getElementById('textNetDetails'),
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
  configSyntaxBadge: document.getElementById('configSyntaxBadge'),
  configCursorInfo: document.getElementById('configCursorInfo'),
  configMetaInfo: document.getElementById('configMetaInfo'),
  txtConfigEditor: document.getElementById('txtConfigEditor'),
  txtConfigSearch: document.getElementById('txtConfigSearch'),
  configSearchCount: document.getElementById('configSearchCount'),
  btnConfigSearchPrev: document.getElementById('btnConfigSearchPrev'),
  btnConfigSearchNext: document.getElementById('btnConfigSearchNext'),
  btnConfigSearchClear: document.getElementById('btnConfigSearchClear'),
  txtConfigGotoLine: document.getElementById('txtConfigGotoLine'),
  btnConfigGotoLine: document.getElementById('btnConfigGotoLine'),
  configLineGutter: document.getElementById('configLineGutter'),
  configBackdrop: document.getElementById('configBackdrop'),
  configActiveLine: document.getElementById('configActiveLine'),
  configHighlightPre: document.getElementById('configHighlightPre'),
  configHighlightCode: document.getElementById('configHighlightCode'),
  exampleLineGutter: document.getElementById('exampleLineGutter'),
  exampleBackdrop: document.getElementById('exampleBackdrop'),
  exampleActiveLine: document.getElementById('exampleActiveLine'),
  exampleHighlightPre: document.getElementById('exampleHighlightPre'),
  exampleHighlightCode: document.getElementById('exampleHighlightCode'),

  // 沙盒終端機元素
  terminalScreen: document.getElementById('terminalScreen'),
  terminalContainer: document.getElementById('terminalContainer'),
  terminalContainerBox: document.querySelector('.terminal-container'),
  selectTerminalEncoding: document.getElementById('selectTerminalEncoding'),
  btnToggleTerminalMaximize: document.getElementById('btnToggleTerminalMaximize'),
  btnResetTerminal: document.getElementById('btnResetTerminal'),
  btnClearTerminalCanvas: document.getElementById('btnClearTerminalCanvas'),
  btnLaunchExternalCmd: document.getElementById('btnLaunchExternalCmd'),
  btnTermQuicks: document.querySelectorAll('.btn-term-quick'),

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

const formatSpeed = (bytesPerSec) => {
  if (!bytesPerSec || bytesPerSec <= 0) return { val: '0', unit: 'B/s' };
  const k = 1024;
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
  const safeI = Math.min(Math.max(0, i), units.length - 1);
  const val = parseFloat((bytesPerSec / Math.pow(k, safeI)).toFixed(safeI === 0 ? 0 : 1));
  return { val: String(val), unit: units[safeI] };
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

  // 輪詢週期：依使用者指示最低與預設為 5000ms (5 秒)
  let savedInterval = parseInt(localStorage.getItem('lah_dashboard_polling_interval'), 10) || 5000;
  if (savedInterval < 5000) {
    savedInterval = 5000;
    localStorage.setItem('lah_dashboard_polling_interval', '5000');
  }
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
    el.themeIcon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 14A8 8 0 0110 4a8 8 0 1010 10z"/></svg>`;
    el.themeText.textContent = '深色';
    el.btnToggleTheme.title = '切換至深邃主題';
  } else {
    el.themeIcon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l2 2m10 10 2 2M19 5l-2 2M7 17l-2 2"/></svg>`;
    el.themeText.textContent = '明亮';
    el.btnToggleTheme.title = '切換至明亮主題';
  }
};

// ==========================================================================
// 1. 初始化與安裝根目錄載入 (含初次啟動微霧載入遮罩管理)
// ==========================================================================
let isInitialLoadingDismissed = false;

const hideInitialLoadingOverlay = () => {
  if (isInitialLoadingDismissed) return;
  isInitialLoadingDismissed = true;
  const overlay = document.getElementById('initialLoadingOverlay');
  if (overlay) {
    overlay.classList.add('fade-out');
    setTimeout(() => {
      overlay.classList.add('hidden');
    }, 380);
  }
};

const initApp = async () => {
  initThemeAndFont();

  // 安全超時防護 (8 秒)：若後端探測逾時未完成，自動淡出遮罩並提示
  const safetyTimeout = setTimeout(() => {
    if (!isInitialLoadingDismissed) {
      console.warn('[Init] 初始探測超過 8 秒，觸發安全淡出遮罩');
      hideInitialLoadingOverlay();
      showToast('警告：首次服務探測較久，已轉為背景探測模式');
    }
  }, 8000);

  try {

  // 綁定字體切換事件
  if (el.selectFontScale) {
    el.selectFontScale.addEventListener('change', (e) => {
      const scale = e.target.value;
      document.documentElement.style.setProperty('--font-user-scale', scale);
      document.documentElement.style.setProperty('--font-scale', `calc(1.2 * ${scale})`);
      localStorage.setItem('lah_dashboard_user_font_scale', scale);
      localStorage.setItem('lah_dashboard_font_scale', scale);
      const label = scale === '1' ? '一般 (100%)' : scale === '1.15' ? '中 (+15%)' : '大 (+25%)';
      showToast(`已套用字體大小：${label} (基礎放大 20%)`);
      scheduleConfigEditorSync();
    });
  }

  // 綁定主題切換事件
  if (el.btnToggleTheme) {
    el.btnToggleTheme.addEventListener('click', () => {
      const currentTheme = document.documentElement.getAttribute('data-theme') || 'dark';
      const newTheme = currentTheme === 'light' ? 'dark' : 'light';
      applyTheme(newTheme);
      showToast(newTheme === 'light' ? '已切換為明亮主題' : '已切換為深色主題');
    });
  }

  // 綁定輪詢時間切換事件
  if (el.selectPollingInterval) {
    el.selectPollingInterval.addEventListener('change', (e) => {
      let val = parseInt(e.target.value, 10) || 5000;
      if (val < 5000) val = 5000;
      state.pollingInterval = val;
      localStorage.setItem('lah_dashboard_polling_interval', String(val));
      if (state.isPolling) {
        startPollingScheduler();
      }
      showToast(`已調整自動輪詢週期為 ${val / 1000} 秒`);
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
    let info = await invoke('get_app_info');
    if (info && !info.cli_specified) {
      const savedRoot = localStorage.getItem('lah_dashboard_custom_root');
      if (savedRoot && savedRoot.trim() && savedRoot.trim().toLowerCase() !== info.root_dir.toLowerCase()) {
        try {
          const customInfo = await invoke('set_root_dir', { path: savedRoot.trim() });
          if (customInfo) {
            info = customInfo;
          }
        } catch (e) {
          console.warn('[Root] 套用上次儲存目錄失敗，維持預設：', e);
        }
      }
    }
    if (info) {
      updateRootDirDisplay(info);
    }
  } catch (err) {
    if (el.rootDirText) el.rootDirText.textContent = '無法辨識安裝目錄';
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
  if (el.rootBadge) {
    el.rootBadge.addEventListener('click', openSwitchRootModal);
  }
  if (el.btnBrowseRootDir) {
    el.btnBrowseRootDir.addEventListener('click', handleBrowseRoot);
  }
  if (el.btnConfirmSwitchRoot) {
    el.btnConfirmSwitchRoot.addEventListener('click', handleConfirmSwitchRoot);
  }
  if (el.btnCancelSwitchRoot) {
    el.btnCancelSwitchRoot.addEventListener('click', closeSwitchRootModal);
  }
  if (el.btnResetDefaultRoot) {
    el.btnResetDefaultRoot.addEventListener('click', handleResetDefaultRoot);
  }
  if (el.inputCustomRoot) {
    el.inputCustomRoot.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') handleConfirmSwitchRoot();
      if (e.key === 'Escape') closeSwitchRootModal();
    });
  }

  el.btnRestartAll.addEventListener('click', () => {
    showConfirmModal(
      '一鍵重啟全部服務確認',
      '即將重新啟動底層 Apache 系統服務 (LAH_Web_Service) 與所有 PM2 應用進程（前端、即時通、排程與 FastCGI）。是否確定執行？',
      async () => {
        await executeRestartAllPipeline();
      }
    );
  });

  const btnRestartClose = document.getElementById('btnRestartCloseOverlay');
  if (btnRestartClose) {
    btnRestartClose.addEventListener('click', () => {
      hideRestartProgressOverlay();
    });
  }

  const btnRestartRetryFailed = document.getElementById('btnRestartRetryFailed');
  if (btnRestartRetryFailed) {
    btnRestartRetryFailed.addEventListener('click', async () => {
      await retryUnreadyServicesInOverlay();
    });
  }

  const btnRestartContinuePoll = document.getElementById('btnRestartContinuePoll');
  if (btnRestartContinuePoll) {
    btnRestartContinuePoll.addEventListener('click', async () => {
      await continuePollingServicesInOverlay();
    });
  }

  // 快捷維運工具箱
  el.btnLaunchSandboxCmd.addEventListener('click', async () => {
    try {
      const res = await invoke('open_lah_cmd');
      if (res && res.success) showToast(res.message);
      else showToast('警告：' + (res ? res.message : '無法開啟終端'));
    } catch (e) {
      showToast('開啟失敗：' + e);
    }
  });

  el.btnOpenFrontendWeb.addEventListener('click', async () => {
    try {
      const res = await invoke('open_web_frontend');
      if (res && res.success) showToast(res.message);
    } catch (e) {
      showToast('開啟瀏覽器失敗：' + e);
    }
  });

  el.btnCheckApacheSyntax.addEventListener('click', async () => {
    try {
      const res = await invoke('check_apache_syntax');
      if (res && res.success) {
        showToast(res.message);
      } else {
        showConfirmModal('Apache 組態語法警告', res ? res.message : '語法異常', () => {});
      }
    } catch (e) {
      showToast('檢查失敗：' + e);
    }
  });

  el.btnTriggerLogrotate.addEventListener('click', async () => {
    try {
      showToast('正在執行日誌壓縮輪替與空間釋放...');
      const res = await invoke('trigger_logrotate');
      if (res && res.success) {
        showToast(res.message);
        // 即時刷新主機指標與磁碟空間
        await loadHostMetrics();
      } else {
        showToast('警告：' + (res ? res.message : '觸發失敗'));
      }
    } catch (e) {
      showToast('觸發失敗：' + e);
    }
  });

  el.btnOpenLogsFolder.addEventListener('click', async () => {
    try {
      const res = await invoke('open_logs_folder');
      if (res && res.success) showToast(res.message);
    } catch (e) {
      showToast('開啟失敗：' + e);
    }
  });

  // 日誌檢視器事件
  el.logSourceSelect.addEventListener('change', async (e) => {
    const selectedPath = e.target.value;
    const selectedItem = state.availableLogs.find((i) => i.path === selectedPath);
    await handleLogSourceSelection(selectedItem);
  });

  if (el.apiDateSelect) {
    el.apiDateSelect.addEventListener('change', (e) => {
      const path = e.target.value;
      if (path) {
        state.currentLogPath = path;
        updateLogMetaDisplay();
        fetchLogTail(true);
      }
    });
  }

  el.chkAutoTail.addEventListener('change', (e) => {
    state.autoTail = e.target.checked;
    if (state.autoTail) fetchLogTail(false);
  });

  el.txtLogFilter.addEventListener('input', () => {
    renderFilteredLog();
  });

  el.btnClearLogCanvas.addEventListener('click', () => {
    state.rawLogLines = [];
    state.filteredLogLines = [];
    el.logContent.innerHTML = highlightLogLine('【已清空日誌畫布】', '');
  });

  el.btnOpenNotepad.addEventListener('click', async () => {
    if (!state.currentLogPath) {
      showToast('警告：尚未選擇日誌檔案');
      return;
    }
    try {
      const res = await invoke('open_log_in_notepad', { path: state.currentLogPath });
      if (res && res.success) showToast('已啟動記事本開啟日誌');
      else showToast('警告：' + (res ? res.message : '無法開啟'));
    } catch (e) {
      showToast('開啟失敗：' + e);
    }
  });

  el.btnCopyLog.addEventListener('click', async () => {
    const text =
      state.filteredLogLines && state.filteredLogLines.length > 0
        ? state.filteredLogLines.join('\n')
        : el.logContent.textContent;
    if (!text) {
      showToast('警告：無日誌內容可複製');
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      showToast('日誌已複製到剪貼簿！');
    } catch (err) {
      showToast('複製失敗');
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
      showToast('警告：尚未選擇組態檔案');
      return;
    }
    try {
      const res = await invoke('open_log_in_notepad', { path: state.currentConfigPath });
      if (res && res.success) showToast('已使用記事本開啟組態');
      else showToast('警告：' + (res ? res.message : '無法開啟'));
    } catch (e) {
      showToast('開啟失敗：' + e);
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
      showToast('警告：無範本內容可複製');
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      showToast('已複製範本內容到剪貼簿！');
    } catch (err) {
      showToast('複製失敗');
    }
  });

  el.btnApplyExample.addEventListener('click', () => {
    if (!state.currentExampleContent) {
      showToast('警告：目前無可套用的範本內容');
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
          showToast('已成功將範本內容套入編輯區！');
        }
      );
    } else {
      el.txtConfigEditor.value = state.currentExampleContent;
      el.txtConfigEditor.dispatchEvent(new Event('input'));
      showToast('已成功將範本內容套入編輯區！');
    }
  });

  el.chkWrapConfig.addEventListener('change', (e) => {
    const isWrap = e.target.checked;
    el.txtConfigEditor.classList.toggle('no-wrap', !isWrap);
    if (el.configBackdrop) el.configBackdrop.classList.toggle('no-wrap', !isWrap);
    el.txtExampleViewer.classList.toggle('no-wrap', !isWrap);
    if (el.exampleBackdrop) el.exampleBackdrop.classList.toggle('no-wrap', !isWrap);
    scheduleConfigEditorSync();
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
    renderMainConfigHighlight(true);
    updateConfigCursorAndActiveLine('main');
  });

  el.txtConfigEditor.addEventListener('scroll', () => {
    syncEditorScroll('main');
  });

  ['click', 'keyup', 'select', 'focus'].forEach((evtName) => {
    el.txtConfigEditor.addEventListener(evtName, () => {
      updateConfigCursorAndActiveLine('main');
    });
  });

  el.txtExampleViewer.addEventListener('scroll', () => {
    syncEditorScroll('example');
  });

  ['click', 'keyup', 'select', 'focus'].forEach((evtName) => {
    el.txtExampleViewer.addEventListener(evtName, () => {
      updateConfigCursorAndActiveLine('example');
    });
  });

  if (el.configLineGutter) {
    el.configLineGutter.addEventListener('click', (e) => {
      const lineItem = e.target.closest('.gutter-line');
      if (lineItem && lineItem.dataset.line) {
        jumpToConfigLine(parseInt(lineItem.dataset.line, 10), true);
      }
    });
  }

  // 組態編輯器內搜尋與跳行事件綁定
  if (el.txtConfigSearch) {
    el.txtConfigSearch.addEventListener('input', () => {
      handleConfigSearchInput();
    });
    el.txtConfigSearch.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        navigateConfigSearch(e.shiftKey ? -1 : 1);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        clearConfigSearch(true);
      }
    });
  }

  if (el.btnConfigSearchPrev) {
    el.btnConfigSearchPrev.addEventListener('click', () => navigateConfigSearch(-1));
  }
  if (el.btnConfigSearchNext) {
    el.btnConfigSearchNext.addEventListener('click', () => navigateConfigSearch(1));
  }
  if (el.btnConfigSearchClear) {
    el.btnConfigSearchClear.addEventListener('click', () => clearConfigSearch(true));
  }

  if (el.txtConfigGotoLine) {
    el.txtConfigGotoLine.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const targetLine = parseInt(el.txtConfigGotoLine.value, 10);
        if (targetLine > 0) {
          jumpToConfigLine(targetLine, true);
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        el.txtConfigEditor.focus();
      }
    });
  }

  if (el.btnConfigGotoLine) {
    el.btnConfigGotoLine.addEventListener('click', () => {
      const targetLine = parseInt(el.txtConfigGotoLine.value, 10);
      if (targetLine > 0) {
        jumpToConfigLine(targetLine, true);
      } else if (el.txtConfigGotoLine) {
        el.txtConfigGotoLine.focus();
      }
    });
  }

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

  // ==========================================================================
  // 沙盒終端機事件綁定 (分頁五 - xterm.js 專業終端)
  // ==========================================================================
  if (el.selectTerminalEncoding) {
    el.selectTerminalEncoding.addEventListener('change', async (e) => {
      const enc = e.target.value;
      try {
        const res = await invoke('terminal_set_encoding', { encoding: enc });
        state.currentTerminalEncoding = enc;
        if (res && res.success) {
          showToast(res.message);
          if (state.xtermInstance) {
            state.xtermInstance.writeln(`\r\n\x1b[90m>>> ${res.message}\x1b[0m\r\n`);
          }
        }
      } catch (err) {
        showToast(`切換編碼失敗: ${err}`);
      }
    });
  }

  if (el.btnToggleTerminalMaximize) {
    el.btnToggleTerminalMaximize.addEventListener('click', () => {
      toggleTerminalMaximize();
    });
  }

  // 按 ESC 鍵快速退出沙盒終端最大化模式，並支援組態編輯器 Ctrl+F / Ctrl+G 快捷鍵
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && state.isTerminalMaximized) {
      toggleTerminalMaximize(false);
      return;
    }
    if (state.activeTab === 'tab-configs') {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        if (el.txtConfigSearch) {
          el.txtConfigSearch.focus();
          el.txtConfigSearch.select();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'g') {
        e.preventDefault();
        if (el.txtConfigGotoLine) {
          el.txtConfigGotoLine.focus();
          el.txtConfigGotoLine.select();
        }
      } else if (e.key === 'F3') {
        e.preventDefault();
        navigateConfigSearch(e.shiftKey ? -1 : 1);
      }
    }
  });

  if (el.btnResetTerminal) {
    el.btnResetTerminal.addEventListener('click', () => {
      showConfirmModal(
        '重設沙盒終端 Session 確認',
        '即將終止目前沙盒終端 Session 並重新建立全新進程。現有運行中之互動命令將被中斷。\n是否確定重設？',
        async () => {
          showToast('正在重設沙盒終端 Session...');
          try {
            const res = await invoke('terminal_reset_session');
            if (res && res.success) {
              if (state.xtermInstance) {
                state.xtermInstance.clear();
              }
              showToast(res.message);
              pollTerminalOutput();
            } else {
              showToast('警告：' + (res ? res.message : '重設失敗'));
            }
          } catch (err) {
            showToast(`重設失敗: ${err}`);
          }
        }
      );
    });
  }

  if (el.btnClearTerminalCanvas) {
    el.btnClearTerminalCanvas.addEventListener('click', () => {
      if (state.xtermInstance) {
        state.xtermInstance.clear();
      }
      showToast('終端機螢幕畫面已清空');
    });
  }

  if (el.btnLaunchExternalCmd) {
    el.btnLaunchExternalCmd.addEventListener('click', async () => {
      try {
        const res = await invoke('open_lah_cmd');
        if (res && res.success) showToast(res.message);
        else showToast('警告：' + (res ? res.message : '無法開啟終端'));
      } catch (e) {
        showToast('開啟失敗：' + e);
      }
    });
  }

  if (el.btnTermQuicks && el.btnTermQuicks.length > 0) {
    el.btnTermQuicks.forEach((btn) => {
      btn.addEventListener('click', () => {
        const cmd = btn.getAttribute('data-cmd');
        if (cmd) {
          runQuickCommand(cmd);
        }
      });
    });
  }

  // 視窗大小改變時自動調校 xterm 尺寸與組態編輯器行高對齊
  window.addEventListener('resize', () => {
    if (state.fitAddonInstance && state.activeTab === 'tab-terminal') {
      try {
        state.fitAddonInstance.fit();
      } catch (_) {}
    } else if (state.activeTab === 'tab-configs') {
      scheduleConfigEditorSync();
    }
  });

    // 初始讀取各項清單與首次探測
    await loadAvailableLogsList();
    await refreshAllData();
    startPollingScheduler();
  } catch (err) {
    console.error('[Init] 初始化過程發生異常：', err);
  } finally {
    clearTimeout(safetyTimeout);
    hideInitialLoadingOverlay();
  }
};

// ==========================================================================
// 2. 分頁切換
// ==========================================================================
// ==========================================================================
// 1.1 監控安裝目錄切換管理
// ==========================================================================
const updateRootDirDisplay = (info) => {
  state.rootDir = info.root_dir;
  if (el.rootDirText) el.rootDirText.textContent = info.root_dir;
  const rootDot = document.getElementById('rootDot');
  if (info.is_valid_root) {
    if (el.rootBadge) el.rootBadge.title = `點擊切換監控安裝目錄 (當前：${info.root_dir})`;
    if (rootDot) {
      rootDot.style.backgroundColor = 'var(--success)';
      rootDot.style.boxShadow = '0 0 6px var(--success)';
    }
  } else {
    if (el.rootBadge) el.rootBadge.title = `注意：目錄缺少標準環境腳本 (${info.root_dir})，點擊切換`;
    if (rootDot) {
      rootDot.style.backgroundColor = 'var(--warning)';
      rootDot.style.boxShadow = '0 0 6px var(--warning)';
    }
  }
};

const openSwitchRootModal = () => {
  if (!el.switchRootModal) return;
  el.inputCustomRoot.value = state.rootDir || '';
  renderQuickRoots();
  el.switchRootModal.classList.remove('hidden');
  el.inputCustomRoot.focus();
};

const closeSwitchRootModal = () => {
  if (el.switchRootModal) el.switchRootModal.classList.add('hidden');
};

const renderQuickRoots = () => {
  if (!el.quickRootList) return;
  const defaults = ['C:\\AppServ', 'D:\\AppServ', 'C:\\Temp\\WTF2'];
  let history = [];
  try {
    const raw = localStorage.getItem('lah_dashboard_root_history');
    if (raw) history = JSON.parse(raw);
  } catch (e) {}

  const allCandidates = Array.from(new Set([state.rootDir, ...defaults, ...history].filter(Boolean)));
  el.quickRootList.innerHTML = '';
  allCandidates.forEach((cand) => {
    const isActive = cand.toLowerCase() === (state.rootDir || '').toLowerCase();
    const btn = document.createElement('div');
    btn.className = `root-candidate-btn ${isActive ? 'active' : ''}`;
    btn.innerHTML = `
      <span><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h7l2 2h9v10H3z"/></svg> ${cand}</span>
      ${isActive ? '<span style="font-size: 11px; color: var(--primary);">目前監控中</span>' : ''}
    `;
    btn.addEventListener('click', () => {
      el.inputCustomRoot.value = cand;
    });
    btn.addEventListener('dblclick', async () => {
      el.inputCustomRoot.value = cand;
      await handleConfirmSwitchRoot();
    });
    el.quickRootList.appendChild(btn);
  });
};

const handleBrowseRoot = async () => {
  try {
    let selected = null;
    const dialog = window.__TAURI__?.plugin?.dialog || window.__TAURI__?.dialog;
    if (dialog && dialog.open) {
      selected = await dialog.open({ directory: true, multiple: false });
    }
    if (!selected) {
      selected = await invoke('pick_root_directory');
    }
    if (selected) {
      el.inputCustomRoot.value = selected;
    }
  } catch (err) {
    console.error('[BrowseRoot] 瀏覽目錄失敗：', err);
    showToast(`瀏覽目錄失敗：${err}`);
  }
};

const handleConfirmSwitchRoot = async () => {
  const targetPath = el.inputCustomRoot.value.trim();
  if (!targetPath) {
    showToast('請輸入或選擇有效目錄路徑！');
    return;
  }
  showToast(`正在切換至：${targetPath}...`);
  try {
    const newInfo = await invoke('set_root_dir', { path: targetPath });
    if (newInfo) {
      updateRootDirDisplay(newInfo);
      localStorage.setItem('lah_dashboard_custom_root', newInfo.root_dir);

      try {
        let history = [];
        const raw = localStorage.getItem('lah_dashboard_root_history');
        if (raw) history = JSON.parse(raw);
        history = [newInfo.root_dir, ...history.filter((p) => p.toLowerCase() !== newInfo.root_dir.toLowerCase())].slice(0, 5);
        localStorage.setItem('lah_dashboard_root_history', JSON.stringify(history));
      } catch (e) {}

      closeSwitchRootModal();
      showToast(`已成功切換監控目錄至：${newInfo.root_dir}`);

      await reloadAllAfterRootChange();
    }
  } catch (err) {
    console.error('[SwitchRoot] 切換目錄失敗：', err);
    showToast(`切換目錄失敗：${err}`);
  }
};

const handleResetDefaultRoot = async () => {
  showToast('正在還原自動探測目錄...');
  try {
    const newInfo = await invoke('reset_to_default_root');
    if (newInfo) {
      updateRootDirDisplay(newInfo);
      localStorage.removeItem('lah_dashboard_custom_root');
      closeSwitchRootModal();
      showToast(`已還原至預設探測目錄：${newInfo.root_dir}`);
      await reloadAllAfterRootChange();
    }
  } catch (err) {
    console.error('[ResetRoot] 還原預設失敗：', err);
    showToast(`還原預設目錄失敗：${err}`);
  }
};

const reloadAllAfterRootChange = async () => {
  await refreshAllData(true);
  if (state.activeTab === 'tab-logs') {
    if (typeof loadAvailableLogsList === 'function') await loadAvailableLogsList();
  }
  if (state.activeTab === 'tab-configs') {
    if (typeof loadCoreConfigsList === 'function') await loadCoreConfigsList();
  }
};

const switchTab = (tabId) => {
  state.activeTab = tabId;
  el.tabBtns.forEach((btn) => {
    btn.classList.toggle('active', btn.getAttribute('data-tab') === tabId);
  });
  el.tabPanes.forEach((pane) => {
    pane.classList.toggle('active', pane.id === tabId);
  });

  if (tabId === 'tab-services') {
    stopTerminalPolling();
    loadSystemServices();
    loadPm2Processes();
  } else if (tabId === 'tab-logs') {
    stopTerminalPolling();
    fetchLogTail(true);
  } else if (tabId === 'tab-metrics') {
    stopTerminalPolling();
    loadHostMetrics();
  } else if (tabId === 'tab-configs') {
    stopTerminalPolling();
    if (state.availableConfigs.length === 0) {
      loadCoreConfigsList();
    } else {
      scheduleConfigEditorSync();
    }
  } else if (tabId === 'tab-terminal') {
    startTerminalSession();
  }
};

// ==========================================================================
// 3. 服務狀態查詢與渲染 (分頁一)
// ==========================================================================
const renderSystemServicesList = (list) => {
  if (!list || list.length === 0) {
    if (!state.isRestarting) {
      el.systemServicesTableBody.innerHTML = '<tr><td colspan="4" class="text-center loading-row">未偵測到註冊之系統服務</td></tr>';
    }
    return;
  }

  let html = '';
  list.forEach((s) => {
    const isRun = s.is_running;
    const isTransitioning = s.status === 'STARTING' || s.status === 'STOPPING';
    const statusPill = isRun
      ? '<span class="status-pill running"><span class="status-dot"></span>正在執行</span>'
      : isTransitioning || state.isRestarting
      ? `<span class="status-pill restarting"><span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>${s.status === 'STOPPING' ? '停止中...' : '啟動中...'}</span>`
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

    let pathWarning = '';
    if (s.binary_path && state.rootDir) {
      const normBin = s.binary_path.toLowerCase().replace(/\\/g, '/');
      const normRoot = state.rootDir.toLowerCase().replace(/\\/g, '/');
      if (!normBin.includes(normRoot)) {
        pathWarning = `<div style="font-size: 11px; color: var(--warning); margin-top: 2px;" title="此 Windows 服務註冊路徑為：${s.binary_path}，與當前儀表板監控目錄 (${state.rootDir}) 不同">警告：服務註冊於其他目錄</div>`;
      }
    }

    html += `
      <tr>
        <td><span class="service-name">${s.name}</span></td>
        <td><span class="service-desc">${s.display_name}</span>${pathWarning}</td>
        <td>${statusPill}</td>
        <td>${actions}</td>
      </tr>
    `;
  });
  el.systemServicesTableBody.innerHTML = html;
};

const loadSystemServices = async () => {
  if (state.isLoadingServices) return;
  state.isLoadingServices = true;
  try {
    let list;
    try {
      list = await invoke('get_system_services', {}, 15000);
    } catch (firstErr) {
      await new Promise((r) => setTimeout(r, 1000));
      list = await invoke('get_system_services', {}, 15000);
    }
    renderSystemServicesList(list);
    if (el.footerStatusText && el.footerStatusText.textContent.includes('探測系統服務逾時')) {
      el.footerStatusText.textContent = '系統監控總線運作中';
    }
  } catch (err) {
    console.error('讀取系統服務失敗：', err);
    if (!state.isRestarting) {
      el.systemServicesTableBody.innerHTML = `<tr><td colspan="4" class="text-center loading-row text-danger" style="padding: 1.25rem;">警告：探測後端系統服務逾時或失敗 <button class="btn btn-primary btn-sm" style="margin-left: 0.75rem; padding: 2px 10px;" onclick="loadSystemServices()">立即重試</button></td></tr>`;
      if (el.footerStatusText) {
        el.footerStatusText.textContent = '警告：探測系統服務逾時';
      }
    }
  } finally {
    state.isLoadingServices = false;
  }
};

window.handleServiceAction = async (action, svcName) => {
  const actionText = action === 'start' ? '啟動' : action === 'stop' ? '停止' : '重啟';
  showToast(`正在${actionText}系統服務 [${svcName}]...`);

  stopPollingScheduler();
  if (action === 'restart' || action === 'start') {
    const rows = el.systemServicesTableBody.querySelectorAll('tr');
    rows.forEach((r) => {
      if (r.textContent.includes(svcName)) {
        const pill = r.querySelector('.status-pill');
        if (pill) {
          pill.className = 'status-pill restarting';
          pill.innerHTML = `<span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>${actionText}中...`;
        }
      }
    });
  }

  try {
    let res;
    if (action === 'start') res = await invoke('start_system_service', { name: svcName }, 45000);
    else if (action === 'stop') res = await invoke('stop_system_service', { name: svcName }, 45000);
    else res = await invoke('restart_system_service', { name: svcName }, 45000);

    if (res && res.success) {
      showToast(`${res.message}`);
    } else {
      showToast(`警告：${res ? res.message : '操作未完成'}`);
    }
    if (action === 'restart' || action === 'start') {
      triggerBurstPolling(8000, 1500);
    } else {
      await loadSystemServices();
      if (state.isPolling) startPollingScheduler();
    }
  } catch (e) {
    showToast(`${actionText}失敗: ${e}`);
    await loadSystemServices();
    if (state.isPolling) startPollingScheduler();
  }
};

const renderPm2ProcessesList = (list) => {
  if (!list || list.length === 0) {
    if (!state.isRestarting) {
      el.pm2TableBody.innerHTML = '<tr><td colspan="8" class="text-center loading-row">目前無 PM2 進程在線或守護進程尚未就緒</td></tr>';
    }
    return;
  }

  let html = '';
  list.forEach((p) => {
    const isOnline = p.status === 'online';
    const statusPill = isOnline
      ? '<span class="status-pill online"><span class="status-dot"></span>在線</span>'
      : state.isRestarting || p.status === 'launching'
      ? '<span class="status-pill restarting"><span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>啟動中...</span>'
      : `<span class="status-pill stopped"><span class="status-dot"></span>${p.status}</span>`;

    const memFormatted = formatBytes(p.memory);

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
};

const loadPm2Processes = async () => {
  if (state.isLoadingPm2) return;
  state.isLoadingPm2 = true;
  try {
    let list;
    try {
      list = await invoke('get_pm2_processes', {}, 15000);
    } catch (firstErr) {
      await new Promise((r) => setTimeout(r, 1000));
      list = await invoke('get_pm2_processes', {}, 15000);
    }
    renderPm2ProcessesList(list);
    if (el.footerStatusText && el.footerStatusText.textContent.includes('探測 PM2 進程逾時')) {
      el.footerStatusText.textContent = '系統監控總線運作中';
    }
  } catch (err) {
    console.error('讀取 PM2 進程失敗：', err);
    if (!state.isRestarting) {
      el.pm2TableBody.innerHTML = `<tr><td colspan="8" class="text-center loading-row text-danger" style="padding: 1.25rem;">警告：探測後端 PM2 進程逾時或失敗 <button class="btn btn-primary btn-sm" style="margin-left: 0.75rem; padding: 2px 10px;" onclick="loadPm2Processes()">立即重試</button></td></tr>`;
      if (el.footerStatusText) {
        el.footerStatusText.textContent = '警告：探測 PM2 進程逾時';
      }
    }
  } finally {
    state.isLoadingPm2 = false;
  }
};

window.handlePm2Action = async (action, target) => {
  const actionText = action === 'start' ? '啟動' : action === 'stop' ? '停止' : '重啟';
  showToast(`正在${actionText} PM2 進程 [${target}]...`);

  stopPollingScheduler();
  if (action === 'restart' || action === 'start') {
    const rows = el.pm2TableBody.querySelectorAll('tr');
    rows.forEach((r) => {
      if (r.textContent.includes(target)) {
        const pill = r.querySelector('.status-pill');
        if (pill) {
          pill.className = 'status-pill restarting';
          pill.innerHTML = `<span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>${actionText}中...`;
        }
      }
    });
  }

  try {
    let res;
    if (action === 'start') res = await invoke('start_pm2_process', { target }, 30000);
    else if (action === 'stop') res = await invoke('stop_pm2_process', { target }, 30000);
    else res = await invoke('restart_pm2_process', { target }, 30000);

    if (res && res.success) {
      showToast(`${res.message}`);
    } else {
      showToast(`警告：${res ? res.message : '操作失敗'}`);
    }
    if (action === 'restart' || action === 'start') {
      triggerBurstPolling(8000, 1500);
    } else {
      await loadPm2Processes();
      if (state.isPolling) startPollingScheduler();
    }
  } catch (e) {
    showToast(`${actionText}失敗: ${e}`);
    await loadPm2Processes();
    if (state.isPolling) startPollingScheduler();
  }
};

window.quickControlService = async (evt, action, category, serviceKey, displayName = '') => {
  if (evt) {
    evt.stopPropagation();
    evt.preventDefault();
  }
  const showName = displayName || serviceKey;
  const actionText = action === 'start' ? '啟動' : action === 'stop' ? '停止' : '重啟';
  showToast(`正在${actionText} [${showName}]...`);

  if (!state.restartingServices) state.restartingServices = new Set();
  state.restartingServices.add(serviceKey);

  // 畫面微型卡片即時過渡狀態反饋：優先使用觸發按鈕所在的卡片，若無則依 data-service-key 查詢
  const card = (evt && evt.currentTarget)
    ? evt.currentTarget.closest('.mem-micro-card')
    : document.querySelector(`.mem-micro-card[data-service-key="${serviceKey}"]`);

  if (card) {
    card.classList.add('is-restarting');
    const actBox = card.querySelector('.mem-micro-actions');
    if (actBox) {
      actBox.innerHTML = '<span class="micro-spin-label"><span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span></span>';
    }
  }

  try {
    let res;
    if (category === 'apache') {
      if (action === 'start') res = await invoke('start_system_service', { name: serviceKey }, 45000);
      else if (action === 'stop') res = await invoke('stop_system_service', { name: serviceKey }, 45000);
      else res = await invoke('restart_system_service', { name: serviceKey }, 45000);
    } else {
      if (action === 'start') res = await invoke('start_pm2_process', { target: serviceKey }, 45000);
      else if (action === 'stop') res = await invoke('stop_pm2_process', { target: serviceKey }, 45000);
      else res = await invoke('restart_pm2_process', { target: serviceKey }, 45000);
    }

    if (res && res.success) {
      showToast(`${res.message}`);
    } else {
      showToast(`警告：${res ? res.message : '操作完成'}`);
    }

    // 若為重啟或啟動，觸發密集輪詢追蹤進程與 PID 變化
    if (action === 'restart' || action === 'start') {
      triggerBurstPolling(8000, 1500);
    }
  } catch (err) {
    showToast(`${actionText}失敗: ${err}`);
  } finally {
    state.restartingServices.delete(serviceKey);
    await loadHostMetrics();
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
      await handleLogSourceSelection(firstExist);
    }
  } catch (err) {
    console.error('讀取可用日誌清單失敗：', err);
  }
};

const handleLogSourceSelection = async (item) => {
  if (!item) return;

  if (item.key === 'api_www_log') {
    // 顯示歷史日期下拉選單
    if (el.apiDateSelectGroup) el.apiDateSelectGroup.style.display = 'flex';
    await loadApiLogDates();
  } else {
    // 隱藏歷史日期下拉選單
    if (el.apiDateSelectGroup) el.apiDateSelectGroup.style.display = 'none';
    state.currentLogPath = item.path;
    updateLogMetaDisplay();
    fetchLogTail(true);
  }
};

const loadApiLogDates = async () => {
  try {
    const files = await invoke('get_api_log_files');
    state.apiLogFiles = files || [];
    if (files && files.length > 0) {
      let html = '';
      files.forEach((f) => {
        const sizeStr = formatBytes(f.size_bytes);
        html += `<option value="${f.path}">${f.label} (${sizeStr})</option>`;
      });
      if (el.apiDateSelect) {
        el.apiDateSelect.innerHTML = html;
        el.apiDateSelect.value = files[0].path;
      }
      state.currentLogPath = files[0].path;
    } else {
      if (el.apiDateSelect) {
        el.apiDateSelect.innerHTML = '<option value="">無歷史日誌</option>';
      }
      const mainItem = state.availableLogs.find((i) => i.key === 'api_www_log');
      state.currentLogPath = mainItem ? mainItem.path : '';
    }
    updateLogMetaDisplay();
    fetchLogTail(true);
  } catch (err) {
    console.error('讀取 API 歷史日誌檔案清單失敗：', err);
  }
};

const updateLogMetaDisplay = () => {
  let item = state.availableLogs.find((i) => i.path === state.currentLogPath);
  if (!item && state.apiLogFiles) {
    item = state.apiLogFiles.find((i) => i.path === state.currentLogPath);
  }
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

const escapeAndMark = (text, filterKw) => {
  if (!text) return '';
  if (!filterKw) return escapeHtmlConfig(text);
  const lowerText = text.toLowerCase();
  const lowerKw = filterKw.toLowerCase();
  const kwLen = lowerKw.length;
  if (kwLen === 0) return escapeHtmlConfig(text);

  let result = '';
  let lastIdx = 0;
  let idx = lowerText.indexOf(lowerKw, lastIdx);
  while (idx !== -1) {
    if (idx > lastIdx) {
      result += escapeHtmlConfig(text.slice(lastIdx, idx));
    }
    result += `<mark class="log-filter-mark">${escapeHtmlConfig(text.slice(idx, idx + kwLen))}</mark>`;
    lastIdx = idx + kwLen;
    idx = lowerText.indexOf(lowerKw, lastIdx);
  }
  if (lastIdx < text.length) {
    result += escapeHtmlConfig(text.slice(lastIdx));
  }
  return result;
};

const LOG_TOKEN_REGEX = new RegExp(
  [
    '(?<ts>\\[\\d{4}-\\d{2}-\\d{2}[T\\s]\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?(?:Z|[+-]\\d{2}:?\\d{2})?\\]|\\b\\d{4}-\\d{2}-\\d{2}[T\\s]\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?(?:Z|[+-]\\d{2}:?\\d{2})?\\b|\\[(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\\s+[A-Za-z]{3}\\s+\\d{2}\\s+\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?\\s+\\d{4}\\]|\\[\\d{2}\\/[A-Za-z]{3}\\/\\d{4}:\\d{2}:\\d{2}:\\d{2}\\s+[+-]\\d{4}\\])',
    '(?<httpProto>HTTP\\/[12](?:\\.[01])?"\\s+)(?<httpCode>[1-5]\\d{2})\\b',
    '(?<lvlErr>\\[(?:error|fatal|critical|ERROR|FATAL|CRITICAL)\\]|\\[[\\w_]+:(?:error|crit|emerg|alert)\\]|\\b(?:PHP\\s+Fatal\\s+error|PHP\\s+Parse\\s+error|ERROR|FATAL|CRITICAL|Exception|ParseError)\\b|❌)',
    '(?<lvlWarn>\\[(?:warn|warning|WARN|WARNING)\\]|\\[[\\w_]+:warn\\]|\\b(?:PHP\\s+Warning|WARN|WARNING)\\b|⚠️)',
    '(?<lvlInfo>\\[(?:info|notice|INFO|NOTICE)\\]|\\[[\\w_]+:notice\\]|\\b(?:PHP\\s+Notice|INFO|NOTICE)\\b)',
    '(?<lvlOk>\\[(?:ok|success|OK|SUCCESS)\\]|✅|✓|\\b(?:SUCCESS)\\b)',
    '(?<cls>\\[pid\\s+\\d+(?::tid\\s+\\d+)?\\]|\\bAH\\d{5}:|\\b[A-Z][A-Za-z0-9_]+::[A-Za-z0-9_]+\\b)',
    '(?<ip>\\b(?:\\d{1,3}\\.){3}\\d{1,3}(?::\\d{2,5})?\\b)',
    '(?<httpMethod>"(?:GET|POST|PUT|DELETE|OPTIONS|HEAD|PATCH)\\b|\\b(?:GET|POST|PUT|DELETE|OPTIONS|HEAD|PATCH)(?=\\s+\\/))',
    '(?<path>[A-Za-z]:\\\\[^\\s"\'<>]+|\\/(?:api|assets|js|css|logs|www)\\/[^\\s"\'<>]*)',
  ].join('|'),
  'g'
);

const highlightLogLine = (rawLine, filterKw) => {
  if (!rawLine) {
    return '<div class="log-line">&#8203;</div>';
  }
  if (/^\s*(?:【提示】|【已清空|載入中|無法讀取)/.test(rawLine)) {
    return `<div class="log-line log-ts">${escapeAndMark(rawLine, filterKw)}</div>`;
  }

  let lineClass = 'log-line';
  if (/\b(ERROR|FATAL|CRITICAL|Exception|ParseError|Error)\b|:error\]|❌|失敗|異常|告警/i.test(rawLine)) {
    lineClass += ' log-line-error';
  } else if (/\b(WARN|WARNING)\b|:warn\]|⚠️|警告/i.test(rawLine)) {
    lineClass += ' log-line-warn';
  }

  let html = '';
  let lastIdx = 0;
  LOG_TOKEN_REGEX.lastIndex = 0;
  let m;

  while ((m = LOG_TOKEN_REGEX.exec(rawLine)) !== null) {
    if (m.index > lastIdx) {
      html += escapeAndMark(rawLine.slice(lastIdx, m.index), filterKw);
    }

    const g = m.groups;
    if (g.httpProto && g.httpCode) {
      const codeNum = parseInt(g.httpCode, 10);
      const codeCls = codeNum >= 500 ? 'log-lvl-error' : codeNum >= 400 ? 'log-lvl-warn' : 'log-lvl-ok';
      html += `${escapeAndMark(g.httpProto, filterKw)}<span class="${codeCls}">${escapeAndMark(g.httpCode, filterKw)}</span>`;
    } else {
      let spanCls = '';
      if (g.ts) spanCls = 'log-ts';
      else if (g.lvlErr) spanCls = 'log-lvl-error';
      else if (g.lvlWarn) spanCls = 'log-lvl-warn';
      else if (g.lvlInfo) spanCls = 'log-lvl-info';
      else if (g.lvlOk) spanCls = 'log-lvl-ok';
      else if (g.cls) spanCls = 'log-cls';
      else if (g.ip) spanCls = 'log-ip';
      else if (g.httpMethod) spanCls = 'log-http-method';
      else if (g.path) spanCls = 'log-path';

      html += `<span class="${spanCls}">${escapeAndMark(m[0], filterKw)}</span>`;
    }

    lastIdx = m.index + m[0].length;
  }

  if (lastIdx < rawLine.length) {
    html += escapeAndMark(rawLine.slice(lastIdx), filterKw);
  }

  return `<div class="${lineClass}">${html}</div>`;
};

const renderFilteredLog = (forceScroll = false) => {
  const rawKw = el.txtLogFilter.value.trim();
  const kw = rawKw.toLowerCase();
  let filtered = state.rawLogLines;
  if (kw) {
    filtered = filtered.filter((line) => line.toLowerCase().includes(kw));
  }
  state.filteredLogLines = filtered;

  // 判斷使用者是否正在往上滾動檢視舊內容：
  // 若距底部超過 40px，表示使用者主動向上翻閱，此時不強制跳到底部，避免打斷閱讀
  const isNearBottom =
    el.logViewport.scrollHeight - el.logViewport.scrollTop - el.logViewport.clientHeight < 40;

  el.logContent.innerHTML = filtered.map((line) => highlightLogLine(line, rawKw)).join('\n');

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
      const rxBytes = m.network.rx_bytes_per_sec || 0;
      const txBytes = m.network.tx_bytes_per_sec || 0;
      const totalBytes = rxBytes + txBytes;
      const totalSpeed = formatSpeed(totalBytes);

      if (el.valNetTotal) el.valNetTotal.textContent = totalSpeed.val;
      if (el.unitNetTotal) el.unitNetTotal.textContent = totalSpeed.unit;
      if (el.textNetDetails) {
        el.textNetDetails.innerHTML = `下載 <span class="net-detail-badge rx">${m.network.rx_formatted}</span>　上傳 <span class="net-detail-badge tx">${m.network.tx_formatted}</span>`;
      }

      state.netRxHistory.push(rxBytes);
      state.netTxHistory.push(txBytes);
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

    // 4. LAH 服務群組記憶體占用卡片 (單排直立微型卡不換行)
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

          // 解析名稱與 PID，呈現簡潔直立微型卡
          let displayName = item.name;
          let pidStr = item.pid > 0 ? `PID: ${item.pid}` : '-';
          const pidMatch = item.name.match(/^(.*?)\s*\(PID:\s*(\d+)\)$/);
          if (pidMatch) {
            displayName = pidMatch[1];
            pidStr = `PID: ${pidMatch[2]}`;
          } else if (item.category === 'apache') {
            displayName = 'Apache 2.4';
            pidStr = item.pid > 0 ? `PID: ${item.pid}` : (item.status === 'stopped' ? '-' : '主進程');
          }

          const serviceKey = item.service_key || (item.category === 'apache' ? 'LAH_Web_Service' : displayName);
          const isOnline = item.status === 'online' || (!item.status && item.memory_bytes > 0);
          const isRestarting = state.restartingServices && state.restartingServices.has(serviceKey);

          let statusDotClass = 'online';
          if (isRestarting) {
            statusDotClass = 'restarting';
          } else if (!isOnline) {
            statusDotClass = 'stopped';
          }

          let actionHtml = '';
          const escapedDisplayName = displayName.replace(/'/g, "\\'");
          if (isRestarting) {
            actionHtml = `<span class="micro-spin-label" title="${displayName} 處理中..."><span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span></span>`;
          } else if (!isOnline) {
            actionHtml = `<button class="btn-micro btn-micro-start" title="立即啟動 ${displayName}" onclick="quickControlService(event, 'start', '${item.category}', '${serviceKey}', '${escapedDisplayName}')"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 4l13 8-13 8z"/></svg> 啟動</button>`;
          } else {
            actionHtml = `
              <button class="btn-micro btn-micro-restart" title="重啟 ${displayName}" onclick="quickControlService(event, 'restart', '${item.category}', '${serviceKey}', '${escapedDisplayName}')"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></button>
              <button class="btn-micro btn-micro-stop" title="停止 ${displayName}" onclick="quickControlService(event, 'stop', '${item.category}', '${serviceKey}', '${escapedDisplayName}')"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="6" y="6" width="12" height="12"/></svg></button>
            `;
          }

          const cardClass = `mem-micro-card ${!isOnline ? 'is-stopped' : ''} ${isRestarting ? 'is-restarting' : ''}`;
          const formattedMem = isOnline ? item.memory_formatted : '已停止';

          memHtml += `
            <div class="${cardClass}" data-service-key="${serviceKey}" title="${displayName} - ${isOnline ? item.memory_formatted : '已停止'}">
              <div class="mem-micro-header">
                <div class="mem-micro-tag-group">
                  <span class="status-dot-mini ${statusDotClass}"></span>
                  <span class="mem-item-badge ${badgeClass}">${badgeText}</span>
                </div>
                <span class="mem-micro-name">${displayName}</span>
              </div>
              <div class="mem-micro-val">
                <span class="mem-micro-num">${formattedMem}</span>
              </div>
              <div class="mem-micro-footer">
                <span class="mem-micro-pid">${pidStr}</span>
                <div class="mem-micro-actions">
                  ${actionHtml}
                </div>
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
  if (state.isRefreshingAll && !isManual) return;
  state.isRefreshingAll = true;
  if (isManual) showToast('正在更新服務與系統指標...');

  try {
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

    if (isManual) showToast('狀態更新完成');
  } finally {
    state.isRefreshingAll = false;
  }
};

const togglePolling = () => {
  state.isPolling = !state.isPolling;
  if (state.isPolling) {
    el.pollingStatusIcon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 5v14M16 5v14"/></svg>`;
    el.pollingStatusText.textContent = '暫停輪詢';
    el.footerStatusText.textContent = '系統監控總線運作中';
    startPollingScheduler();
    showToast(`已恢復自動狀態輪詢 (每 ${state.pollingInterval / 1000} 秒)`);
  } else {
    el.pollingStatusIcon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 4l13 8-13 8z"/></svg>`;
    el.pollingStatusText.textContent = '恢復輪詢';
    el.footerStatusText.textContent = '自動輪詢已暫停';
    stopPollingScheduler();
    showToast('已暫停自動輪詢');
  }
};

const startPollingScheduler = () => {
  stopPollingScheduler();
  if (state.isRestarting) return;

  // 主輪詢迴圈：依使用者設定之週期 (預設 5000ms) 探測當前分頁資料
  state.pollingTimer = setInterval(async () => {
    if (!state.isPolling || state.isRestarting) return;
    await refreshAllData();
  }, state.pollingInterval);
};

let burstTimer = null;

const triggerBurstPolling = (totalMs = 8000, intervalMs = 1500) => {
  if (burstTimer) {
    clearTimeout(burstTimer);
    burstTimer = null;
  }
  stopPollingScheduler();

  const startTime = Date.now();
  const runBurstTick = async () => {
    if (state.isRestarting) return;
    await refreshAllData();
    const elapsed = Date.now() - startTime;
    if (elapsed >= totalMs) {
      burstTimer = null;
      if (state.isPolling) {
        startPollingScheduler();
      }
      showToast('服務狀態已全面恢復上線');
    } else {
      burstTimer = setTimeout(runBurstTick, intervalMs);
    }
  };

  burstTimer = setTimeout(runBurstTick, intervalMs);
};

// ==========================================================================
// 全系統服務重啟進度遮罩與步驟檢核管線
// ==========================================================================
const setRestartStepStatus = (stepIndex, status, customText = '') => {
  const item = document.getElementById(`restartStepItem${stepIndex}`);
  const icon = document.getElementById(`stepIcon${stepIndex}`);
  if (!item || !icon) return;

  item.className = `restart-step-item ${status}`;
  icon.setAttribute('aria-label', { pending: '等待中', running: '執行中', success: '完成', error: '失敗' }[status] || status);
  if (status === 'pending') {
    icon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3h12M6 21h12M7 3c0 5 2 6 5 9-3 3-5 4-5 9M17 3c0 5-2 6-5 9 3 3 5 4 5 9"/></svg>`;
  } else if (status === 'running') {
    icon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg>`;
  } else if (status === 'success') {
    icon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 12l5 5L20 5"/></svg>`;
  } else if (status === 'error') {
    icon.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 5l14 14M19 5L5 19"/></svg>`;
  }

  if (customText) {
    const subtitle = document.getElementById('restartModalSubtitle');
    if (subtitle) subtitle.textContent = customText;
  }
};

const showRestartProgressOverlay = () => {
  const overlay = document.getElementById('restartProgressOverlay');
  if (!overlay) return;

  setRestartStepStatus(1, 'pending');
  setRestartStepStatus(2, 'pending');
  setRestartStepStatus(3, 'pending');

  const title = document.getElementById('restartModalTitle');
  const subtitle = document.getElementById('restartModalSubtitle');
  const errorActions = document.getElementById('restartErrorActions');

  if (title) {
    title.textContent = '全系統服務重啟中';
    title.style.color = '';
  }
  if (subtitle) subtitle.textContent = '正在循序重啟底層服務與應用進程，請稍候...';
  if (errorActions) errorActions.classList.add('hidden');

  overlay.classList.remove('fade-out', 'hidden');
};

const hideRestartProgressOverlay = () => {
  state.isRestarting = false;
  const overlay = document.getElementById('restartProgressOverlay');
  if (overlay) {
    overlay.classList.add('fade-out');
    setTimeout(() => {
      overlay.classList.add('hidden');
    }, 380);
  }
  refreshAllData();
  if (state.isPolling) {
    startPollingScheduler();
  }
};

// 步驟 3 密集探測與自動補救 (Auto-Heal)
const pollAndHealUntilAllOnline = async (maxAttempts = 20) => {
  setRestartStepStatus(3, 'running', '步驟 3/3：正在密集探測各項服務與進程在線綠燈狀態...');

  let allOnline = false;
  let lastUnreadyName = '';

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    await new Promise((r) => setTimeout(r, 1200));

    let svcs = [];
    try {
      svcs = await invoke('get_system_services', {}, 15000);
      if (svcs && svcs.length > 0 && state.activeTab === 'tab-services') {
        renderSystemServicesList(svcs);
      }
    } catch (_) {}

    const apacheSvc = (svcs || []).find((s) => s.name === 'LAH_Web_Service');
    const isApacheOk = apacheSvc ? apacheSvc.is_running : false;

    let procs = [];
    try {
      procs = await invoke('get_pm2_processes', {}, 15000);
      if (procs && procs.length > 0 && state.activeTab === 'tab-services') {
        renderPm2ProcessesList(procs);
      }
    } catch (_) {}

    let isPm2Ok = true;
    const unreadyPm2Names = [];
    if (!procs || procs.length === 0) {
      isPm2Ok = false;
      lastUnreadyName = 'PM2 尚未回傳進程列表';
    } else {
      for (const p of procs) {
        if (p.status !== 'online') {
          isPm2Ok = false;
          unreadyPm2Names.push(p.name);
          lastUnreadyName = `${p.name} (${p.status})`;
        }
      }
    }

    if (!isApacheOk) {
      lastUnreadyName = apacheSvc ? `LAH_Web_Service (${apacheSvc.status})` : 'LAH_Web_Service (探測中)';
    }

    if (isApacheOk && isPm2Ok) {
      allOnline = true;
      break;
    }

    setRestartStepStatus(
      3,
      'running',
      `步驟 3/3：等待服務就緒 (${attempt}/${maxAttempts}) — 目前等待：${lastUnreadyName}`
    );

    // 自動補救機制 (Auto-Heal)：若第 4 次或第 10 次探測發現服務呈現 STOPPED，自動補發啟動指令
    if (attempt === 4 || attempt === 10) {
      if (apacheSvc && !apacheSvc.is_running && apacheSvc.status === 'STOPPED') {
        setRestartStepStatus(3, 'running', '步驟 3/3：偵測到 Apache 尚未啟動，正在自動補發啟動指令...');
        try {
          await invoke('start_system_service', { name: 'LAH_Web_Service' }, 35000);
        } catch (_) {}
      }
      if (unreadyPm2Names.length > 0) {
        for (const procName of unreadyPm2Names) {
          setRestartStepStatus(3, 'running', `步驟 3/3：正在自動補救喚醒 PM2 進程 [${procName}]...`);
          try {
            await invoke('restart_pm2_process', { target: procName }, 25000);
          } catch (_) {}
        }
      }
    }
  }

  if (allOnline) {
    state.isRestarting = false;
    await refreshAllData();
    setRestartStepStatus(3, 'success', '全系統服務已順利重啟並全數在線！');
    const title = document.getElementById('restartModalTitle');
    if (title) {
      title.textContent = '全域服務已全數成功上線';
      title.style.color = '#10b981';
    }
    const errorActions = document.getElementById('restartErrorActions');
    if (errorActions) errorActions.classList.add('hidden');
    await new Promise((r) => setTimeout(r, 800));
    hideRestartProgressOverlay();
    showToast('全系統服務已全面恢復在線運作！');
    return true;
  } else {
    setRestartStepStatus(3, 'error', `警告：部分服務尚未就緒：${lastUnreadyName}`);
    handleRestartError(`部分服務尚未就緒 (${lastUnreadyName})，可點擊下方「重試未就緒服務」或「繼續等待探測」。`);
    return false;
  }
};

// 遮罩內按鈕：僅針對尚未就緒之服務執行補救重試
const retryUnreadyServicesInOverlay = async () => {
  state.isRestarting = true;
  stopPollingScheduler();

  const errorActions = document.getElementById('restartErrorActions');
  if (errorActions) errorActions.classList.add('hidden');

  const title = document.getElementById('restartModalTitle');
  if (title) {
    title.textContent = '正在補救重試未就緒服務...';
    title.style.color = '';
  }

  setRestartStepStatus(3, 'running', '正在檢查並重新啟動未就緒的服務項目...');

  try {
    let svcs = [];
    try {
      svcs = await invoke('get_system_services', {}, 15000);
    } catch (_) {}
    const apacheSvc = (svcs || []).find((s) => s.name === 'LAH_Web_Service');
    if (!apacheSvc || !apacheSvc.is_running) {
      setRestartStepStatus(2, 'running', '正在重新啟動 Apache 2.4 系統服務 (LAH_Web_Service)...');
      await invoke('start_system_service', { name: 'LAH_Web_Service' }, 45000);
      setRestartStepStatus(2, 'success');
    } else {
      setRestartStepStatus(2, 'success');
    }

    let procs = [];
    try {
      procs = await invoke('get_pm2_processes', {}, 15000);
    } catch (_) {}
    if (!procs || procs.length === 0) {
      setRestartStepStatus(1, 'running', '正在重新喚醒 PM2 應用進程群...');
      await invoke('restart_pm2_process', { target: 'all' }, 45000);
      setRestartStepStatus(1, 'success');
    } else {
      for (const p of procs) {
        if (p.status !== 'online') {
          setRestartStepStatus(1, 'running', `正在重新啟動 PM2 進程 [${p.name}]...`);
          await invoke('restart_pm2_process', { target: p.name }, 25000);
        }
      }
      setRestartStepStatus(1, 'success');
    }

    await pollAndHealUntilAllOnline(15);
  } catch (err) {
    handleRestartError(`重試過程發生異常：${err}`);
  }
};

// 遮罩內按鈕：不重發重啟指令，直接延長等待探測
const continuePollingServicesInOverlay = async () => {
  state.isRestarting = true;
  stopPollingScheduler();

  const errorActions = document.getElementById('restartErrorActions');
  if (errorActions) errorActions.classList.add('hidden');

  const title = document.getElementById('restartModalTitle');
  if (title) {
    title.textContent = '正在繼續追蹤服務狀態...';
    title.style.color = '';
  }

  await pollAndHealUntilAllOnline(15);
};

const executeRestartAllPipeline = async () => {
  state.isRestarting = true;
  stopPollingScheduler();
  showRestartProgressOverlay();
  setAllServicesRestartingVisual();

  try {
    // 步驟 1：重新啟動 PM2 應用進程層
    setRestartStepStatus(1, 'running', '步驟 1/3：正在重啟 PM2 應用進程群 (FE, WSS, 排程, FastCGI)...');
    try {
      const pm2Res = await invoke('restart_pm2_process', { target: 'all' }, 45000);
      if (pm2Res && !pm2Res.success) {
        throw new Error(pm2Res.message || 'PM2 重啟失敗');
      }
      setRestartStepStatus(1, 'success');
    } catch (err) {
      setRestartStepStatus(1, 'error', `警告：PM2 重啟異常：${err}`);
      handleRestartError(`PM2 進程重啟異常：${err}`);
      return;
    }

    // 步驟 2：重新啟動 Apache 2.4 系統閘道 (LAH_Web_Service)
    setRestartStepStatus(2, 'running', '步驟 2/3：正在重啟 Apache 2.4 系統服務 (LAH_Web_Service)...');
    try {
      const apacheRes = await invoke('restart_system_service', { name: 'LAH_Web_Service' }, 45000);
      if (apacheRes && !apacheRes.success) {
        throw new Error(apacheRes.message || 'Apache 重啟失敗');
      }
      setRestartStepStatus(2, 'success');
    } catch (err) {
      setRestartStepStatus(2, 'error', `警告：Apache 服務重啟異常：${err}`);
      handleRestartError(`Apache 系統服務重啟異常：${err}`);
      return;
    }

    // 步驟 3：追蹤探測所有服務在線就緒狀態（含自動補救）
    await pollAndHealUntilAllOnline(20);
  } catch (globalErr) {
    handleRestartError(`重啟程序發生未預期異常：${globalErr}`);
  }
};

const handleRestartError = (errMsg) => {
  state.isRestarting = false;
  const subtitle = document.getElementById('restartModalSubtitle');
  const errorActions = document.getElementById('restartErrorActions');
  const title = document.getElementById('restartModalTitle');
  if (title) {
    title.textContent = '警告：服務重啟未完全就緒';
    title.style.color = '#ef4444';
  }
  if (subtitle) subtitle.textContent = errMsg;
  if (errorActions) errorActions.classList.remove('hidden');
  showToast(`警告：${errMsg}`);
  refreshAllData();
};

const setAllServicesRestartingVisual = () => {
  if (el.systemServicesTableBody) {
    const svcPills = el.systemServicesTableBody.querySelectorAll('.status-pill');
    svcPills.forEach((p) => {
      p.className = 'status-pill restarting';
      p.innerHTML = '<span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>重啟中...';
    });
  }
  if (el.pm2TableBody) {
    const pm2Pills = el.pm2TableBody.querySelectorAll('.status-pill');
    pm2Pills.forEach((p) => {
      p.className = 'status-pill restarting';
      p.innerHTML = '<span class="spin-icon"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg></span>重啟中...';
    });
  }
};

const stopPollingScheduler = () => {
  if (state.pollingTimer) {
    clearInterval(state.pollingTimer);
    state.pollingTimer = null;
  }
  if (burstTimer) {
    clearTimeout(burstTimer);
    burstTimer = null;
  }
};

// ==========================================================================
// 6. 核心組態編輯器與多格式語法高亮引擎 (分頁四)
// ==========================================================================
const escapeHtmlConfig = (str) => {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
};

const detectConfigLanguage = (filePath) => {
  if (!filePath) return { id: 'plain', label: 'PLAIN' };
  const fileName = String(filePath).split(/[\\/]/).pop() || '';
  const cleanName = fileName.replace(/\.(example|sample|bak|orig|\d{8}_\d{6})$/i, '').toLowerCase();

  if (cleanName === '.env' || cleanName === 'env' || cleanName.endsWith('.env') || cleanName.endsWith('.properties')) {
    return { id: 'dotenv', label: 'DotEnv' };
  }
  if (cleanName.endsWith('.ini')) {
    return { id: 'ini', label: 'PHP INI' };
  }
  if (cleanName.endsWith('.conf') || cleanName.includes('httpd') || cleanName.includes('vhost')) {
    return { id: 'apache', label: 'Apache Conf' };
  }
  if (cleanName.endsWith('.js') || cleanName.endsWith('.cjs') || cleanName.endsWith('.mjs')) {
    return { id: 'javascript', label: 'JavaScript' };
  }
  if (cleanName.endsWith('.json')) {
    return { id: 'json', label: 'JSON' };
  }
  if (cleanName.endsWith('.xml') || cleanName.endsWith('.html')) {
    return { id: 'xml', label: 'XML' };
  }
  if (cleanName.endsWith('.bat') || cleanName.endsWith('.cmd') || cleanName.endsWith('.ps1') || cleanName.endsWith('.sh')) {
    return { id: 'batch', label: 'Shell / Bat' };
  }
  return { id: 'plain', label: 'Plain Text' };
};

const pushSyntaxToken = (tokens, type, text) => {
  if (!text) return;
  const last = tokens[tokens.length - 1];
  if (last && last.type === type) {
    last.text += text;
  } else {
    tokens.push({ type, text });
  }
};

// 針對字串或參數內含之變數 ($1, ${VAR}, %{ENV:VAR}, %PATH%) 細分高亮
const tokenizeWithVariables = (tokens, text, baseType) => {
  const varRegex = /(\$\{[^}]+\}|%\{[^}]+\}|\$[0-9]+|\$[A-Za-z_][A-Za-z0-9_]*|%[A-Za-z0-9_~]+%)/g;
  let lastIdx = 0;
  let m;
  while ((m = varRegex.exec(text)) !== null) {
    if (m.index > lastIdx) {
      pushSyntaxToken(tokens, baseType, text.slice(lastIdx, m.index));
    }
    pushSyntaxToken(tokens, 'hl-variable', m[0]);
    lastIdx = m.index + m[0].length;
  }
  if (lastIdx < text.length) {
    pushSyntaxToken(tokens, baseType, text.slice(lastIdx));
  }
};

const tokenizeDotenvValue = (tokens, rhs) => {
  let i = 0;
  const n = rhs.length;
  while (i < n) {
    const ch = rhs[i];
    if (ch === ' ' || ch === '\t') {
      let j = i + 1;
      while (j < n && (rhs[j] === ' ' || rhs[j] === '\t')) j++;
      pushSyntaxToken(tokens, '', rhs.slice(i, j));
      i = j;
      if (i < n && rhs[i] === '#') {
        pushSyntaxToken(tokens, 'hl-comment', rhs.slice(i));
        break;
      }
      continue;
    }
    if ((ch === '#' || ch === ';') && i === 0) {
      pushSyntaxToken(tokens, 'hl-comment', rhs.slice(i));
      break;
    }
    if (ch === '"' || ch === "'") {
      const quote = ch;
      let j = i + 1;
      while (j < n) {
        if (rhs[j] === '\\') {
          j += 2;
          continue;
        }
        if (rhs[j] === quote) {
          j++;
          break;
        }
        j++;
      }
      tokenizeWithVariables(tokens, rhs.slice(i, j), 'hl-string');
      i = j;
      continue;
    }
    // 非引號區段：讀至空白或引號前
    let j = i;
    while (j < n && rhs[j] !== ' ' && rhs[j] !== '\t' && rhs[j] !== '"' && rhs[j] !== "'") {
      j++;
    }
    const word = rhs.slice(i, j);
    if (/^(true|false|null|yes|no|on|off)$/i.test(word)) {
      pushSyntaxToken(tokens, 'hl-boolean', word);
    } else if (/^-?\d+(\.\d+)*$/.test(word)) {
      pushSyntaxToken(tokens, 'hl-number', word);
    } else {
      tokenizeWithVariables(tokens, word, 'hl-string');
    }
    i = j;
  }
};

const tokenizeIniValue = (tokens, rhs) => {
  let i = 0;
  const n = rhs.length;
  while (i < n) {
    const ch = rhs[i];
    if (ch === ';') {
      pushSyntaxToken(tokens, 'hl-comment', rhs.slice(i));
      break;
    }
    if (ch === ' ' || ch === '\t') {
      let j = i + 1;
      while (j < n && (rhs[j] === ' ' || rhs[j] === '\t')) j++;
      pushSyntaxToken(tokens, '', rhs.slice(i, j));
      i = j;
      continue;
    }
    if (ch === '"' || ch === "'") {
      const quote = ch;
      let j = i + 1;
      while (j < n) {
        if (rhs[j] === '\\') {
          j += 2;
          continue;
        }
        if (rhs[j] === quote) {
          j++;
          break;
        }
        j++;
      }
      tokenizeWithVariables(tokens, rhs.slice(i, j), 'hl-string');
      i = j;
      continue;
    }
    if ('~&|^!()'.includes(ch)) {
      pushSyntaxToken(tokens, 'hl-operator', ch);
      i++;
      continue;
    }
    let j = i;
    while (j < n && !' \t;"\'~&|^!()'.includes(rhs[j])) {
      j++;
    }
    const word = rhs.slice(i, j);
    if (/^(On|Off|True|False|Yes|No|None|null|E_[A-Z_]+)$/i.test(word)) {
      pushSyntaxToken(tokens, 'hl-boolean', word);
    } else if (/^-?\d+(\.\d+)?[KMGkmg]?$/.test(word)) {
      pushSyntaxToken(tokens, 'hl-number', word);
    } else {
      tokenizeWithVariables(tokens, word, 'hl-string');
    }
    i = j;
  }
};

const tokenizeApacheArgs = (tokens, argsText, defaultType = '') => {
  let i = 0;
  const n = argsText.length;
  while (i < n) {
    const ch = argsText[i];
    if (ch === ' ' || ch === '\t') {
      let j = i + 1;
      while (j < n && (argsText[j] === ' ' || argsText[j] === '\t')) j++;
      pushSyntaxToken(tokens, '', argsText.slice(i, j));
      i = j;
      if (i < n && argsText[i] === '#') {
        pushSyntaxToken(tokens, 'hl-comment', argsText.slice(i));
        break;
      }
      continue;
    }
    if (ch === '#' && i === 0) {
      pushSyntaxToken(tokens, 'hl-comment', argsText.slice(i));
      break;
    }
    if (ch === '"' || ch === "'") {
      const quote = ch;
      let j = i + 1;
      while (j < n) {
        if (argsText[j] === '\\') {
          j += 2;
          continue;
        }
        if (argsText[j] === quote) {
          j++;
          break;
        }
        j++;
      }
      tokenizeWithVariables(tokens, argsText.slice(i, j), 'hl-string');
      i = j;
      continue;
    }
    let j = i;
    while (j < n && argsText[j] !== ' ' && argsText[j] !== '\t' && argsText[j] !== '"' && argsText[j] !== "'") {
      j++;
    }
    const word = argsText.slice(i, j);
    if (/^(On|Off|all|none|granted|denied|Permanent|Temp|env|expr|from|allow|deny)$/i.test(word)) {
      pushSyntaxToken(tokens, 'hl-boolean', word);
    } else if (/^(\*:\d+|\d+(\.\d+)*(:\d+)?(\/\d+)?)$/.test(word)) {
      pushSyntaxToken(tokens, 'hl-number', word);
    } else if (/_module$/i.test(word)) {
      pushSyntaxToken(tokens, 'hl-key', word);
    } else if (/^[+\-][A-Za-z]+$/.test(word)) {
      pushSyntaxToken(tokens, 'hl-boolean', word);
    } else {
      tokenizeWithVariables(tokens, word, defaultType || 'hl-string');
    }
    i = j;
  }
};

const JS_KEYWORDS = new Set([
  'module', 'exports', 'const', 'let', 'var', 'function', 'return', 'require',
  'process', 'env', 'async', 'await', 'if', 'else', 'for', 'of', 'in', 'new',
  'try', 'catch', 'finally', 'throw', 'typeof', 'import', 'from', 'export', 'default',
]);

const JS_BOOLEANS = new Set(['true', 'false', 'null', 'undefined', 'NaN', 'Infinity']);

const tokenizeJsOrJsonLine = (tokens, line, parserState) => {
  let i = 0;
  const n = line.length;
  while (i < n) {
    if (parserState.inBlockComment) {
      const endIdx = line.indexOf('*/', i);
      if (endIdx === -1) {
        pushSyntaxToken(tokens, 'hl-comment', line.slice(i));
        return;
      }
      pushSyntaxToken(tokens, 'hl-comment', line.slice(i, endIdx + 2));
      parserState.inBlockComment = false;
      i = endIdx + 2;
      continue;
    }

    const ch = line[i];
    const next = line[i + 1];

    if (ch === '/' && next === '/') {
      pushSyntaxToken(tokens, 'hl-comment', line.slice(i));
      return;
    }
    if (ch === '/' && next === '*') {
      const endIdx = line.indexOf('*/', i + 2);
      if (endIdx === -1) {
        parserState.inBlockComment = true;
        pushSyntaxToken(tokens, 'hl-comment', line.slice(i));
        return;
      }
      pushSyntaxToken(tokens, 'hl-comment', line.slice(i, endIdx + 2));
      i = endIdx + 2;
      continue;
    }
    if (ch === '"' || ch === "'" || ch === '`') {
      const quote = ch;
      let j = i + 1;
      while (j < n) {
        if (line[j] === '\\') {
          j += 2;
          continue;
        }
        if (line[j] === quote) {
          j++;
          break;
        }
        j++;
      }
      const strToken = line.slice(i, j);
      // 檢查是否為 JSON / Object 屬性鍵 (後面緊接著冒號 :)
      const restAfterStr = line.slice(j);
      if (/^\s*:/.test(restAfterStr)) {
        pushSyntaxToken(tokens, 'hl-key', strToken);
      } else {
        tokenizeWithVariables(tokens, strToken, 'hl-string');
      }
      i = j;
      continue;
    }
    if (/[A-Za-z_$]/.test(ch)) {
      let j = i + 1;
      while (j < n && /[A-Za-z0-9_$]/.test(line[j])) j++;
      const ident = line.slice(i, j);
      const restAfterIdent = line.slice(j);
      if (/^\s*:/.test(restAfterIdent)) {
        pushSyntaxToken(tokens, 'hl-key', ident);
      } else if (JS_BOOLEANS.has(ident)) {
        pushSyntaxToken(tokens, 'hl-boolean', ident);
      } else if (JS_KEYWORDS.has(ident)) {
        pushSyntaxToken(tokens, 'hl-keyword', ident);
      } else {
        pushSyntaxToken(tokens, '', ident);
      }
      i = j;
      continue;
    }
    if (/\d/.test(ch) || (ch === '-' && /\d/.test(next))) {
      let j = i + 1;
      while (j < n && /[\d.xXa-fA-F_]/.test(line[j])) j++;
      pushSyntaxToken(tokens, 'hl-number', line.slice(i, j));
      i = j;
      continue;
    }
    if ('=:+-*/&|<>!?,;{}[]().'.includes(ch)) {
      pushSyntaxToken(tokens, 'hl-operator', ch);
      i++;
      continue;
    }
    pushSyntaxToken(tokens, '', ch);
    i++;
  }
};

const tokenizeXmlLine = (tokens, line, parserState) => {
  let i = 0;
  const n = line.length;
  while (i < n) {
    if (parserState.inBlockComment) {
      const endIdx = line.indexOf('-->', i);
      if (endIdx === -1) {
        pushSyntaxToken(tokens, 'hl-comment', line.slice(i));
        return;
      }
      pushSyntaxToken(tokens, 'hl-comment', line.slice(i, endIdx + 3));
      parserState.inBlockComment = false;
      i = endIdx + 3;
      continue;
    }
    if (line.startsWith('<!--', i)) {
      const endIdx = line.indexOf('-->', i + 4);
      if (endIdx === -1) {
        parserState.inBlockComment = true;
        pushSyntaxToken(tokens, 'hl-comment', line.slice(i));
        return;
      }
      pushSyntaxToken(tokens, 'hl-comment', line.slice(i, endIdx + 3));
      i = endIdx + 3;
      continue;
    }
    if (line[i] === '<') {
      const closeIdx = line.indexOf('>', i);
      if (closeIdx !== -1) {
        const tagChunk = line.slice(i, closeIdx + 1);
        const m = tagChunk.match(/^(<\/?[A-Za-z0-9_:-]+)(.*?)(\/?>)$/);
        if (m) {
          pushSyntaxToken(tokens, 'hl-tag', m[1]);
          // 解析屬性名與值
          const attrText = m[2];
          const attrRe = /([A-Za-z0-9_:-]+)(\s*=\s*)("[^"]*"|'[^']*')/g;
          let lastIdx = 0;
          let am;
          while ((am = attrRe.exec(attrText)) !== null) {
            if (am.index > lastIdx) {
              pushSyntaxToken(tokens, '', attrText.slice(lastIdx, am.index));
            }
            pushSyntaxToken(tokens, 'hl-key', am[1]);
            pushSyntaxToken(tokens, 'hl-operator', am[2]);
            tokenizeWithVariables(tokens, am[3], 'hl-string');
            lastIdx = am.index + am[0].length;
          }
          if (lastIdx < attrText.length) {
            pushSyntaxToken(tokens, '', attrText.slice(lastIdx));
          }
          pushSyntaxToken(tokens, 'hl-tag', m[3]);
          i = closeIdx + 1;
          continue;
        }
      }
    }
    let nextTag = line.indexOf('<', i + 1);
    if (nextTag === -1) nextTag = n;
    tokenizeWithVariables(tokens, line.slice(i, nextTag), '');
    i = nextTag;
  }
};

const tokenizeConfigLine = (line, lang, parserState) => {
  if (!line) return [];
  const tokens = [];

  if (lang === 'dotenv') {
    const leadMatch = line.match(/^(\s*)(.*)$/);
    const indent = leadMatch ? leadMatch[1] : '';
    const rest = leadMatch ? leadMatch[2] : line;
    if (indent) pushSyntaxToken(tokens, '', indent);
    if (!rest) return tokens;

    if (rest.startsWith('#') || rest.startsWith(';')) {
      pushSyntaxToken(tokens, 'hl-comment', rest);
      return tokens;
    }

    let body = rest;
    const exportMatch = body.match(/^(export\s+)(.*)$/);
    if (exportMatch) {
      pushSyntaxToken(tokens, 'hl-keyword', exportMatch[1]);
      body = exportMatch[2];
    }

    const kvMatch = body.match(/^([A-Za-z_][A-Za-z0-9_.-]*)(\s*=\s*)(.*)$/);
    if (kvMatch) {
      pushSyntaxToken(tokens, 'hl-key', kvMatch[1]);
      pushSyntaxToken(tokens, 'hl-operator', kvMatch[2]);
      tokenizeDotenvValue(tokens, kvMatch[3]);
    } else {
      tokenizeDotenvValue(tokens, body);
    }
  } else if (lang === 'ini') {
    const leadMatch = line.match(/^(\s*)(.*)$/);
    const indent = leadMatch ? leadMatch[1] : '';
    const rest = leadMatch ? leadMatch[2] : line;
    if (indent) pushSyntaxToken(tokens, '', indent);
    if (!rest) return tokens;

    if (rest.startsWith(';') || rest.startsWith('#')) {
      pushSyntaxToken(tokens, 'hl-comment', rest);
      return tokens;
    }

    const secMatch = rest.match(/^(\[[^\]]+\])(.*)$/);
    if (secMatch) {
      pushSyntaxToken(tokens, 'hl-tag', secMatch[1]);
      tokenizeIniValue(tokens, secMatch[2]);
      return tokens;
    }

    const kvMatch = rest.match(/^([^=;]+?)(\s*=\s*)(.*)$/);
    if (kvMatch) {
      pushSyntaxToken(tokens, 'hl-key', kvMatch[1]);
      pushSyntaxToken(tokens, 'hl-operator', kvMatch[2]);
      tokenizeIniValue(tokens, kvMatch[3]);
    } else {
      tokenizeIniValue(tokens, rest);
    }
  } else if (lang === 'apache') {
    const leadMatch = line.match(/^(\s*)(.*)$/);
    const indent = leadMatch ? leadMatch[1] : '';
    const rest = leadMatch ? leadMatch[2] : line;
    if (indent) pushSyntaxToken(tokens, '', indent);
    if (!rest) return tokens;

    if (rest.startsWith('#')) {
      pushSyntaxToken(tokens, 'hl-comment', rest);
      return tokens;
    }

    const tagMatch = rest.match(/^(<\/?[A-Za-z0-9_:-]+)(.*?)(\/?>)(.*)$/);
    if (tagMatch) {
      pushSyntaxToken(tokens, 'hl-tag', tagMatch[1]);
      tokenizeApacheArgs(tokens, tagMatch[2], 'hl-key');
      pushSyntaxToken(tokens, 'hl-tag', tagMatch[3]);
      if (tagMatch[4]) {
        tokenizeApacheArgs(tokens, tagMatch[4], '');
      }
      return tokens;
    }

    const dirMatch = rest.match(/^([A-Za-z_][A-Za-z0-9_-]*)(.*)$/);
    if (dirMatch) {
      pushSyntaxToken(tokens, 'hl-keyword', dirMatch[1]);
      tokenizeApacheArgs(tokens, dirMatch[2], '');
    } else {
      tokenizeApacheArgs(tokens, rest, '');
    }
  } else if (lang === 'javascript' || lang === 'json') {
    tokenizeJsOrJsonLine(tokens, line, parserState);
  } else if (lang === 'xml') {
    tokenizeXmlLine(tokens, line, parserState);
  } else if (lang === 'batch') {
    const leadMatch = line.match(/^(\s*)(.*)$/);
    const indent = leadMatch ? leadMatch[1] : '';
    const rest = leadMatch ? leadMatch[2] : line;
    if (indent) pushSyntaxToken(tokens, '', indent);
    if (/^(rem\b|::|#)/i.test(rest)) {
      pushSyntaxToken(tokens, 'hl-comment', rest);
    } else {
      const cmdMatch = rest.match(/^(@?[A-Za-z_][A-Za-z0-9_-]*)(.*)$/);
      if (cmdMatch) {
        pushSyntaxToken(tokens, 'hl-keyword', cmdMatch[1]);
        tokenizeApacheArgs(tokens, cmdMatch[2], '');
      } else {
        tokenizeWithVariables(tokens, rest, '');
      }
    }
  } else {
    pushSyntaxToken(tokens, '', line);
  }

  // 安全防禦：確保拆分後之字元總和與原行 100% 一致，絕不遺失或增加任何字元
  const joined = tokens.map((t) => t.text).join('');
  if (joined !== line) {
    return [{ type: '', text: line }];
  }
  return tokens;
};

// 將單行 Token 結合關鍵字搜尋範圍轉換為 HTML
const renderTokensWithSearchMarks = (tokens, lineMatches, currentMatchIdx) => {
  if (!tokens || tokens.length === 0) return '&#8203;';

  if (!lineMatches || lineMatches.length === 0) {
    let html = '';
    for (let i = 0; i < tokens.length; i++) {
      const tk = tokens[i];
      const escaped = escapeHtmlConfig(tk.text);
      html += tk.type ? `<span class="${tk.type}">${escaped}</span>` : escaped;
    }
    return html || '&#8203;';
  }

  let html = '';
  let col = 0;
  for (let i = 0; i < tokens.length; i++) {
    const tk = tokens[i];
    const tkStart = col;
    const tkEnd = col + tk.text.length;
    col = tkEnd;

    let innerHtml = '';
    let pos = tkStart;

    for (let mIdx = 0; mIdx < lineMatches.length; mIdx++) {
      const m = lineMatches[mIdx];
      if (m.endCol <= pos) continue;
      if (m.startCol >= tkEnd) break;

      if (m.startCol > pos) {
        innerHtml += escapeHtmlConfig(tk.text.slice(pos - tkStart, m.startCol - tkStart));
        pos = m.startCol;
      }

      const sliceEnd = Math.min(tkEnd, m.endCol);
      const matchedSlice = escapeHtmlConfig(tk.text.slice(pos - tkStart, sliceEnd - tkStart));
      const isCurrent = m.matchIdx === currentMatchIdx;
      innerHtml += `<mark class="hl-search-match${isCurrent ? ' current' : ''}" data-match-idx="${m.matchIdx}">${matchedSlice}</mark>`;
      pos = sliceEnd;
    }

    if (pos < tkEnd) {
      innerHtml += escapeHtmlConfig(tk.text.slice(pos - tkStart));
    }

    html += tk.type ? `<span class="${tk.type}">${innerHtml}</span>` : innerHtml;
  }

  return html || '&#8203;';
};

const computeSearchMatches = (text, query) => {
  const matches = [];
  if (!query || !text) return matches;

  const qLower = query.toLowerCase();
  const qLen = qLower.length;
  const lines = text.split('\n');
  let globalOffset = 0;

  for (let i = 0; i < lines.length; i++) {
    const lineLower = lines[i].toLowerCase();
    let fromCol = 0;
    while (fromCol <= lineLower.length - qLen) {
      const foundCol = lineLower.indexOf(qLower, fromCol);
      if (foundCol === -1) break;
      matches.push({
        matchIdx: matches.length,
        lineNum: i + 1,
        startCol: foundCol,
        endCol: foundCol + qLen,
        globalStart: globalOffset + foundCol,
        globalEnd: globalOffset + foundCol + qLen,
      });
      fromCol = foundCol + Math.max(1, qLen);
    }
    globalOffset += lines[i].length + 1;
  }
  return matches;
};

const ensureGutterLinesCount = (gutterEl, lineCount, activeLineNum) => {
  if (!gutterEl) return;
  const targetCount = Math.max(1, lineCount);
  if (gutterEl.childElementCount !== targetCount) {
    let html = '';
    for (let i = 1; i <= targetCount; i++) {
      html += `<div class="gutter-line${i === activeLineNum ? ' active' : ''}" data-line="${i}">${i}</div>`;
    }
    gutterEl.innerHTML = html;
  } else {
    const prevActive = gutterEl.querySelector('.gutter-line.active');
    if (prevActive && Number(prevActive.dataset.line) !== activeLineNum) {
      prevActive.classList.remove('active');
    }
    const nextActive = gutterEl.children[activeLineNum - 1];
    if (nextActive && !nextActive.classList.contains('active')) {
      nextActive.classList.add('active');
    }
  }
};

const renderMainConfigHighlight = (preserveMatchIdx = true) => {
  if (!el.configHighlightCode || !el.txtConfigEditor) return;
  const text = el.txtConfigEditor.value || '';
  const langInfo = detectConfigLanguage(state.currentConfigPath);
  state.configSyntaxLang = langInfo.id;
  if (el.configSyntaxBadge) {
    el.configSyntaxBadge.textContent = langInfo.label;
  }

  const query = state.configSearchQuery || '';
  const matches = computeSearchMatches(text, query);
  state.configSearchMatches = matches;

  if (matches.length === 0) {
    state.configSearchCurrentIdx = -1;
  } else if (!preserveMatchIdx || state.configSearchCurrentIdx < 0 || state.configSearchCurrentIdx >= matches.length) {
    state.configSearchCurrentIdx = 0;
  }
  updateSearchCountUI();

  const matchesByLine = new Map();
  for (let i = 0; i < matches.length; i++) {
    const m = matches[i];
    if (!matchesByLine.has(m.lineNum)) {
      matchesByLine.set(m.lineNum, []);
    }
    matchesByLine.get(m.lineNum).push(m);
  }

  const lines = text.split('\n');
  const parserState = { inBlockComment: false };
  const htmlLines = new Array(lines.length);

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const tokens = tokenizeConfigLine(lines[i], langInfo.id, parserState);
    const lineInner = renderTokensWithSearchMarks(
      tokens,
      matchesByLine.get(lineNum),
      state.configSearchCurrentIdx
    );
    htmlLines[i] = `<span class="hl-line" data-line="${lineNum}">${lineInner}</span>`;
  }

  el.configHighlightCode.innerHTML = htmlLines.join('\n');
  ensureGutterLinesCount(el.configLineGutter, lines.length, state.configActiveLineNum);
  scheduleConfigEditorSync();
};

const renderExampleConfigHighlight = () => {
  if (!el.exampleHighlightCode || !el.txtExampleViewer) return;
  const text = el.txtExampleViewer.value || '';
  const langInfo = detectConfigLanguage(state.currentConfigPath || '.env.example');
  const query = state.configSearchQuery || '';
  const matches = computeSearchMatches(text, query);

  const matchesByLine = new Map();
  for (let i = 0; i < matches.length; i++) {
    const m = matches[i];
    if (!matchesByLine.has(m.lineNum)) {
      matchesByLine.set(m.lineNum, []);
    }
    matchesByLine.get(m.lineNum).push(m);
  }

  const lines = text.split('\n');
  const parserState = { inBlockComment: false };
  const htmlLines = new Array(lines.length);

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const tokens = tokenizeConfigLine(lines[i], langInfo.id, parserState);
    const lineInner = renderTokensWithSearchMarks(tokens, matchesByLine.get(lineNum), -1);
    htmlLines[i] = `<span class="hl-line" data-line="${lineNum}">${lineInner}</span>`;
  }

  el.exampleHighlightCode.innerHTML = htmlLines.join('\n');
  ensureGutterLinesCount(el.exampleLineGutter, lines.length, state.exampleActiveLineNum);
  scheduleConfigEditorSync();
};

const syncEditorScroll = (pane = 'main') => {
  if (pane === 'main') {
    if (!el.txtConfigEditor || !el.configBackdrop || !el.configLineGutter) return;
    el.configBackdrop.scrollTop = el.txtConfigEditor.scrollTop;
    el.configBackdrop.scrollLeft = el.txtConfigEditor.scrollLeft;
    el.configLineGutter.scrollTop = el.txtConfigEditor.scrollTop;
  } else {
    if (!el.txtExampleViewer || !el.exampleBackdrop || !el.exampleLineGutter) return;
    el.exampleBackdrop.scrollTop = el.txtExampleViewer.scrollTop;
    el.exampleBackdrop.scrollLeft = el.txtExampleViewer.scrollLeft;
    el.exampleLineGutter.scrollTop = el.txtExampleViewer.scrollTop;
  }
};

const syncSinglePaneMetrics = (codeEl, preEl, gutterEl, activeLineEl, activeLineNum, isWrap) => {
  if (!codeEl || !preEl || !gutterEl) return;
  const lineSpans = codeEl.children;
  const count = lineSpans.length;
  if (count === 0) return;

  ensureGutterLinesCount(gutterEl, count, activeLineNum);
  const gutterLines = gutterEl.children;

  if (isWrap) {
    const tops = new Array(count);
    for (let i = 0; i < count; i++) {
      tops[i] = lineSpans[i].offsetTop;
    }
    const totalPreHeight = preEl.offsetHeight;
    for (let i = 0; i < count; i++) {
      const h = i + 1 < count ? tops[i + 1] - tops[i] : Math.max(totalPreHeight - tops[i], 20);
      if (gutterLines[i] && h > 0) {
        gutterLines[i].style.height = `${h}px`;
      }
    }
  } else {
    for (let i = 0; i < gutterLines.length; i++) {
      if (gutterLines[i].style.height) {
        gutterLines[i].style.height = '';
      }
    }
  }

  if (activeLineEl) {
    const clampedIdx = Math.min(Math.max(0, activeLineNum - 1), count - 1);
    const activeSpan = lineSpans[clampedIdx];
    if (activeSpan) {
      const nextSpan = lineSpans[clampedIdx + 1];
      const top = preEl.offsetTop + activeSpan.offsetTop;
      const height = nextSpan
        ? nextSpan.offsetTop - activeSpan.offsetTop
        : Math.max(preEl.offsetHeight - activeSpan.offsetTop, activeSpan.offsetHeight || 20);
      activeLineEl.style.top = `${top}px`;
      activeLineEl.style.height = `${Math.max(18, height)}px`;
    }
  }
};

let configSyncRafId = null;
const scheduleConfigEditorSync = () => {
  if (configSyncRafId) cancelAnimationFrame(configSyncRafId);
  configSyncRafId = requestAnimationFrame(() => {
    configSyncRafId = null;
    const isWrap = el.chkWrapConfig ? el.chkWrapConfig.checked : true;
    syncSinglePaneMetrics(
      el.configHighlightCode,
      el.configHighlightPre,
      el.configLineGutter,
      el.configActiveLine,
      state.configActiveLineNum,
      isWrap
    );
    syncEditorScroll('main');

    if (state.isExamplePaneOpen) {
      syncSinglePaneMetrics(
        el.exampleHighlightCode,
        el.exampleHighlightPre,
        el.exampleLineGutter,
        el.exampleActiveLine,
        state.exampleActiveLineNum,
        isWrap
      );
      syncEditorScroll('example');
    }
  });
};

const updateConfigCursorAndActiveLine = (pane = 'main') => {
  if (pane === 'main') {
    if (!el.txtConfigEditor) return;
    const pos = el.txtConfigEditor.selectionStart || 0;
    const before = el.txtConfigEditor.value.slice(0, pos);
    const lines = before.split('\n');
    const lineNum = lines.length;
    const colNum = lines[lines.length - 1].length + 1;
    state.configActiveLineNum = lineNum;
    if (el.configCursorInfo) {
      el.configCursorInfo.textContent = `行 ${lineNum}, 列 ${colNum}`;
    }
    ensureGutterLinesCount(
      el.configLineGutter,
      (el.txtConfigEditor.value || '').split('\n').length,
      lineNum
    );
    scheduleConfigEditorSync();
  } else {
    if (!el.txtExampleViewer) return;
    const pos = el.txtExampleViewer.selectionStart || 0;
    const before = el.txtExampleViewer.value.slice(0, pos);
    const lineNum = before.split('\n').length;
    state.exampleActiveLineNum = lineNum;
    ensureGutterLinesCount(
      el.exampleLineGutter,
      (el.txtExampleViewer.value || '').split('\n').length,
      lineNum
    );
    scheduleConfigEditorSync();
  }
};

const updateSearchCountUI = () => {
  if (!el.configSearchCount) return;
  const total = state.configSearchMatches.length;
  const query = state.configSearchQuery || '';
  el.configSearchCount.classList.remove('has-match', 'no-match');

  if (!query) {
    el.configSearchCount.textContent = '0/0';
  } else if (total === 0) {
    el.configSearchCount.textContent = '0/0';
    el.configSearchCount.classList.add('no-match');
  } else {
    el.configSearchCount.textContent = `${state.configSearchCurrentIdx + 1}/${total}`;
    el.configSearchCount.classList.add('has-match');
  }
};

const scrollConfigLineIntoView = (lineNum) => {
  if (!el.txtConfigEditor || !el.configHighlightCode) return;
  const spans = el.configHighlightCode.children;
  const clampedIdx = Math.min(Math.max(0, lineNum - 1), spans.length - 1);
  const targetSpan = spans[clampedIdx];
  if (!targetSpan) return;

  const targetTop = Math.max(0, targetSpan.offsetTop - el.txtConfigEditor.clientHeight / 2 + 24);
  el.txtConfigEditor.scrollTop = targetTop;
  syncEditorScroll('main');
};

const handleConfigSearchInput = () => {
  state.configSearchQuery = el.txtConfigSearch ? el.txtConfigSearch.value : '';
  state.configSearchCurrentIdx = 0;
  renderMainConfigHighlight(false);
  if (state.isExamplePaneOpen) {
    renderExampleConfigHighlight();
  }
  if (state.configSearchMatches.length > 0) {
    const firstMatch = state.configSearchMatches[0];
    state.configActiveLineNum = firstMatch.lineNum;
    if (el.configCursorInfo) {
      el.configCursorInfo.textContent = `行 ${firstMatch.lineNum}, 列 ${firstMatch.startCol + 1}`;
    }
    scrollConfigLineIntoView(firstMatch.lineNum);
    scheduleConfigEditorSync();
  }
};

const navigateConfigSearch = (step = 1) => {
  const total = state.configSearchMatches.length;
  if (total === 0) {
    if (state.configSearchQuery) {
      showToast(`警告：找不到匹配「${state.configSearchQuery}」的內容`);
    }
    return;
  }

  state.configSearchCurrentIdx = (state.configSearchCurrentIdx + step + total) % total;
  updateSearchCountUI();

  // 更新當前匹配項樣式
  if (el.configHighlightCode) {
    const prevMark = el.configHighlightCode.querySelector('mark.hl-search-match.current');
    if (prevMark) prevMark.classList.remove('current');
    const nextMarks = el.configHighlightCode.querySelectorAll(
      `mark.hl-search-match[data-match-idx="${state.configSearchCurrentIdx}"]`
    );
    nextMarks.forEach((mk) => mk.classList.add('current'));
  }

  const currentMatch = state.configSearchMatches[state.configSearchCurrentIdx];
  if (currentMatch) {
    state.configActiveLineNum = currentMatch.lineNum;
    if (el.configCursorInfo) {
      el.configCursorInfo.textContent = `行 ${currentMatch.lineNum}, 列 ${currentMatch.startCol + 1}`;
    }
    scrollConfigLineIntoView(currentMatch.lineNum);
    scheduleConfigEditorSync();
  }
};

const clearConfigSearch = (focusEditor = false) => {
  if (el.txtConfigSearch) el.txtConfigSearch.value = '';
  state.configSearchQuery = '';
  state.configSearchMatches = [];
  state.configSearchCurrentIdx = -1;
  updateSearchCountUI();
  renderMainConfigHighlight(false);
  if (state.isExamplePaneOpen) {
    renderExampleConfigHighlight();
  }
  if (focusEditor && el.txtConfigEditor) {
    el.txtConfigEditor.focus();
  }
};

const jumpToConfigLine = (targetLineNum, focusEditor = true) => {
  if (!el.txtConfigEditor) return;
  const lines = (el.txtConfigEditor.value || '').split('\n');
  const clampedLine = Math.min(Math.max(1, targetLineNum), lines.length);

  let charOffset = 0;
  for (let i = 0; i < clampedLine - 1; i++) {
    charOffset += lines[i].length + 1;
  }

  state.configActiveLineNum = clampedLine;
  if (el.txtConfigGotoLine) {
    el.txtConfigGotoLine.value = String(clampedLine);
  }
  if (el.configCursorInfo) {
    el.configCursorInfo.textContent = `行 ${clampedLine}, 列 1`;
  }

  if (focusEditor && !el.txtConfigEditor.disabled) {
    el.txtConfigEditor.focus();
    el.txtConfigEditor.setSelectionRange(charOffset, charOffset);
  }

  scrollConfigLineIntoView(clampedLine);
  scheduleConfigEditorSync();
};

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
  el.configPathInfo.title = filePath;
  el.txtConfigEditor.value = '正在讀取組態內容...';
  el.txtConfigEditor.disabled = true;
  state.configActiveLineNum = 1;
  renderMainConfigHighlight(false);

  try {
    const content = await invoke('read_config_content', { path: filePath });
    el.txtConfigEditor.value = content;
    el.txtConfigEditor.disabled = false;
    el.txtConfigEditor.scrollTop = 0;
    el.txtConfigEditor.scrollLeft = 0;
    state.originalConfigContent = content;
    state.isConfigDirty = false;
    state.configActiveLineNum = 1;
    el.configDirtyFlag.classList.add('hidden');
    if (el.configCursorInfo) {
      el.configCursorInfo.textContent = '行 1, 列 1';
    }
    updateConfigEditorMeta();
    renderMainConfigHighlight(false);

    // 載入該檔案之歷史備份
    await loadConfigBackupsList(filePath);

    // 檢查是否有同名範本檔 (.env.example 等)
    const normPath = filePath.toLowerCase().replace(/\//g, '\\');
    let cfgInfo = state.availableConfigs.find(
      (c) => c.path && c.path.toLowerCase().replace(/\//g, '\\') === normPath
    );
    let hasExample = cfgInfo ? cfgInfo.has_example : false;

    // 若清單中無 (例如自訂挑選之檔案)，主動探測是否有範本檔
    if (!cfgInfo) {
      try {
        const testExample = await invoke('get_config_example_content', { path: filePath });
        if (testExample !== null && testExample !== undefined) {
          hasExample = true;
          state.availableConfigs.push({
            path: filePath,
            has_example: true,
            example_path: filePath + '.example',
          });
        }
      } catch (_) {
        hasExample = false;
      }
    }

    state.currentConfigHasExample = hasExample;

    if (hasExample) {
      el.btnToggleExampleDiff.classList.remove('hidden');
      // 只要該組態具有範本檔，自動展開對照窗格並載入範本內容，提供即時參考
      await toggleExamplePane(true);
    } else {
      el.btnToggleExampleDiff.classList.add('hidden');
      await toggleExamplePane(false);
    }
  } catch (err) {
    el.txtConfigEditor.value = `無法讀取檔案內容：\n${err}`;
    el.txtConfigEditor.disabled = true;
    state.originalConfigContent = '';
    state.isConfigDirty = false;
    el.configDirtyFlag.classList.add('hidden');
    el.btnToggleExampleDiff.classList.add('hidden');
    await toggleExamplePane(false);
    updateConfigEditorMeta();
    renderMainConfigHighlight(false);
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
  scheduleConfigEditorSync();
};

const loadExampleContent = async (filePath) => {
  if (!filePath) return;
  try {
    el.txtExampleViewer.value = '正在讀取範本內容...';
    renderExampleConfigHighlight();
    const content = await invoke('get_config_example_content', { path: filePath });
    state.currentExampleContent = content || '';
    el.txtExampleViewer.value = content || '(範本內容為空)';
    el.txtExampleViewer.scrollTop = 0;
    el.txtExampleViewer.scrollLeft = 0;
    state.exampleActiveLineNum = 1;
    renderExampleConfigHighlight();

    const normTarget = filePath.toLowerCase().replace(/\//g, '\\');
    const found = state.availableConfigs.find(
      (c) => c.path && c.path.toLowerCase().replace(/\//g, '\\') === normTarget
    );
    if (found && found.example_path) {
      const parts = found.example_path.split('\\');
      el.exampleFileName.textContent = parts[parts.length - 1] || '.env.example';
    } else {
      el.exampleFileName.textContent = '.env.example';
    }
  } catch (err) {
    console.warn('[範本讀取異常]', err);
    el.txtExampleViewer.value = `無法讀取範本檔案：\n${err}`;
    state.currentExampleContent = '';
    renderExampleConfigHighlight();
  }
};

const updateConfigEditorMeta = () => {
  const text = el.txtConfigEditor.value || '';
  const lines = text.split('\n').length;
  const bytes = new Blob([text]).size;
  el.configMetaInfo.textContent = `大小：${formatBytes(bytes)} ｜ 行數：${lines} ｜ 編碼：UTF-8`;
  if (el.txtConfigGotoLine) {
    el.txtConfigGotoLine.max = String(lines);
  }
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
      const backupPath = b.path || b.full_path;
      const backupTime = b.modified || b.created;
      html += `<option value="${backupPath}">${backupTime} (${formatBytes(b.size_bytes)})</option>`;
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
    showToast('警告：請先選擇欲編輯的組態檔案');
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

    // 若語法檢驗失敗，彈窗警告並提供一鍵自動還原 (支援 Apache httpd -t 與 Node.js node -c)
    if (res.syntax_checked && res.syntax_error) {
      const isJs = state.currentConfigPath.toLowerCase().endsWith('.js');
      const syntaxType = isJs ? 'Node.js (node -c)' : 'Apache (httpd -t)';

      // 自動解析報錯行號並跳轉定位 (如 Syntax error on line 524 或 :12)
      if (res.syntax_output) {
        const lineMatch =
          res.syntax_output.match(/on line\s+(\d+)/i) ||
          res.syntax_output.match(/\.js:(\d+)/i) ||
          res.syntax_output.match(/line\s+(\d+)/i);
        if (lineMatch && lineMatch[1]) {
          jumpToConfigLine(parseInt(lineMatch[1], 10), false);
        }
      }

      showConfirmModal(
        `警告：${syntaxType} 組態語法驗證失敗`,
        `檔案已寫入，但執行 ${syntaxType} 檢測未通過！\n\n錯誤細節：\n${res.syntax_output}\n\n為避免服務重啟或運作崩潰，建議立即還原至儲存前自動建立的備份檔。\n是否立即自動還原？`,
        async () => {
          if (res.backup_path) {
            showToast('正在自動還原至備份檔...');
            await invoke('restore_config_backup', {
              targetPath: state.currentConfigPath,
              backupPath: res.backup_path,
            });
            await fetchConfigContent(state.currentConfigPath);
            showToast('已成功復原至儲存前狀態');
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
    } else if (res.related_service === 'pm2:fe') {
      showConfirmModal(
        '組態儲存成功 (語法正確)',
        `${res.message}\n\n此組態隸屬前端與排程服務。\n是否立即重新啟動排程進程 (LAH-SCHEDULER) 以同步更新 runtime-config.js 與套用新設定？`,
        async () => {
          showToast('正在重啟排程進程 (LAH-SCHEDULER)...');
          await invoke('restart_pm2_process', { target: 'LAH-SCHEDULER' });
          triggerBurstPolling(8000, 1500);
          showToast('已送出 LAH-SCHEDULER 重啟與組態同步指令');
        }
      );
    } else if (res.related_service === 'pm2:wss') {
      showConfirmModal(
        '組態儲存成功 (JS 語法正確)',
        `${res.message}\n\n此組態隸屬 PM2 即時通訊服務。\n是否立即重新啟動即時通進程 (LAH-WSS) 以套用新設定？`,
        async () => {
          showToast('正在重啟即時通進程...');
          await invoke('restart_pm2_process', { target: 'LAH-WSS' });
          triggerBurstPolling(8000, 1500);
          showToast('已送出 LAH-WSS 重啟指令');
        }
      );
    } else if (res.related_service === 'pm2:api') {
      showConfirmModal(
        '組態儲存成功 (JS 語法正確)',
        `${res.message}\n\n此組態隸屬 PM2 API FastCGI 核心服務。\n是否立即重新啟動 FastCGI 進程 (lah-api-v85) 以套用新設定？`,
        async () => {
          showToast('正在重啟 FastCGI 進程...');
          await invoke('restart_pm2_process', { target: 'lah-api-v85' });
          triggerBurstPolling(8000, 1500);
          showToast('已送出 lah-api-v85 重啟指令');
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
      showToast(res.message);
    }
  } catch (err) {
    showToast(`儲存失敗：${err}`);
  }
};

const handleRestoreConfigBackup = async () => {
  const selectedBackup = el.configBackupSelect.value;
  if (!selectedBackup || selectedBackup === 'undefined') {
    showToast('警告：請先選擇欲還原的歷史備份檔');
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
          targetPath: state.currentConfigPath,
          backupPath: selectedBackup,
        });

        if (res && res.success) {
          showToast(res.message);
          await fetchConfigContent(state.currentConfigPath);
        } else {
          showToast('警告：' + (res ? res.message : '還原失敗'));
        }
      } catch (err) {
        showToast(`還原失敗：${err}`);
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
      opt.textContent = `自訂：${fileName}`;
      el.configSourceSelect.prepend(opt);
      el.configSourceSelect.selectedIndex = 0;
    }

    await fetchConfigContent(pickedPath);
    showToast(`已載入自訂檔案：${pickedPath}`);
  } catch (err) {
    showToast(`選擇檔案失敗: ${err}`);
  }
};

// ==========================================================================
// 8. 沙盒終端機控制 (分頁五 - xterm.js 專業虛擬終端)
// ==========================================================================
const initXterm = () => {
  if (state.xtermInstance) return;
  if (!window.Terminal) {
    console.warn('xterm.js 尚未就緒');
    return;
  }

  const term = new window.Terminal({
    theme: {
      background: '#030712',
      foreground: '#f8fafc',
      cursor: '#22c55e',
      cursorAccent: '#030712',
      selectionBackground: 'rgba(14, 165, 233, 0.35)',
      black: '#1e293b',
      red: '#ef4444',
      green: '#22c55e',
      yellow: '#eab308',
      blue: '#3b82f6',
      magenta: '#a855f7',
      cyan: '#06b6d4',
      white: '#f8fafc',
      brightBlack: '#64748b',
      brightRed: '#f87171',
      brightGreen: '#4ade80',
      brightYellow: '#fde047',
      brightBlue: '#60a5fa',
      brightMagenta: '#c084fc',
      brightCyan: '#22d3ee',
      brightWhite: '#ffffff',
    },
    fontFamily: 'Consolas, "Cascadia Code", "Courier New", monospace',
    fontSize: 14,
    lineHeight: 1.25,
    cursorBlink: true,
    cursorStyle: 'block',
    scrollback: 5000,
    convertEol: true,
  });

  if (window.FitAddon && window.FitAddon.FitAddon) {
    const fitAddon = new window.FitAddon.FitAddon();
    term.loadAddon(fitAddon);
    state.fitAddonInstance = fitAddon;
  }

  term.open(el.terminalContainer);

  // 鍵盤輸入事件處理 (行編輯模式)
  term.onData((data) => {
    handleXtermInput(data);
  });

  state.xtermInstance = term;

  setTimeout(() => {
    if (state.fitAddonInstance) state.fitAddonInstance.fit();
    term.focus();
  }, 100);
};

const toggleTerminalMaximize = (forceState) => {
  const isMax = forceState !== undefined ? forceState : !state.isTerminalMaximized;
  state.isTerminalMaximized = isMax;

  const container = el.terminalContainerBox || document.querySelector('.terminal-container');
  if (container) {
    container.classList.toggle('maximized', isMax);
  }

  if (el.btnToggleTerminalMaximize) {
    if (isMax) {
      el.btnToggleTerminalMaximize.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="6" y="8" width="12" height="12"/><path d="M9 8V4h11v12h-2"/></svg> 還原視窗`;
      el.btnToggleTerminalMaximize.classList.add('btn-primary');
      el.btnToggleTerminalMaximize.classList.remove('btn-outline');
    } else {
      el.btnToggleTerminalMaximize.innerHTML = `<svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 9V4h5m6 0h5v5M4 15v5h5m6 0h5v-5"/></svg> 放大視窗`;
      el.btnToggleTerminalMaximize.classList.remove('btn-primary');
      el.btnToggleTerminalMaximize.classList.add('btn-outline');
    }
  }

  setTimeout(() => {
    if (state.fitAddonInstance) {
      try {
        state.fitAddonInstance.fit();
      } catch (_) {}
    }
    if (state.xtermInstance) {
      state.xtermInstance.focus();
    }
  }, 60);
};

const handleXtermInput = (data) => {
  const term = state.xtermInstance;
  if (!term) return;

  switch (data) {
    case '\r': // Enter
      term.write('\r\n');
      const cmd = state.terminalInputBuffer;
      state.terminalInputBuffer = '';
      if (cmd.trim().length > 0) {
        state.terminalHistory.push(cmd);
        if (state.terminalHistory.length > 50) state.terminalHistory.shift();
        state.terminalHistoryIndex = -1;
      }
      sendTerminalInput(cmd);
      break;

    case '\u007F': // Backspace (DEL)
    case '\b': // Backspace
      if (state.terminalInputBuffer.length > 0) {
        state.terminalInputBuffer = state.terminalInputBuffer.slice(0, -1);
        term.write('\b \b');
      }
      break;

    case '\u0003': // Ctrl+C
      term.write('^C\r\n');
      state.terminalInputBuffer = '';
      sendTerminalInput('\x03');
      break;

    case '\u001b[A': // 上箭頭 (歷史上一條)
      if (state.terminalHistory.length > 0) {
        if (state.terminalHistoryIndex === -1) {
          state.terminalHistoryIndex = state.terminalHistory.length - 1;
        } else if (state.terminalHistoryIndex > 0) {
          state.terminalHistoryIndex--;
        }
        replaceCurrentInput(state.terminalHistory[state.terminalHistoryIndex] || '');
      }
      break;

    case '\u001b[B': // 下箭頭 (歷史下一條)
      if (state.terminalHistoryIndex >= 0) {
        if (state.terminalHistoryIndex < state.terminalHistory.length - 1) {
          state.terminalHistoryIndex++;
          replaceCurrentInput(state.terminalHistory[state.terminalHistoryIndex]);
        } else {
          state.terminalHistoryIndex = -1;
          replaceCurrentInput('');
        }
      }
      break;

    default:
      // 一般字元鍵入或剪貼簿貼上 (包含中文字元)
      if (data >= ' ' || data === '\t') {
        state.terminalInputBuffer += data;
        term.write(data);
      }
      break;
  }
};

const replaceCurrentInput = (newText) => {
  const term = state.xtermInstance;
  if (!term) return;
  // 退格清空畫面當前行
  while (state.terminalInputBuffer.length > 0) {
    term.write('\b \b');
    state.terminalInputBuffer = state.terminalInputBuffer.slice(0, -1);
  }
  state.terminalInputBuffer = newText;
  term.write(newText);
};

const runQuickCommand = (cmd) => {
  const term = state.xtermInstance;
  if (term) {
    // 若畫面有正在打的字，退格清除
    while (state.terminalInputBuffer.length > 0) {
      term.write('\b \b');
      state.terminalInputBuffer = state.terminalInputBuffer.slice(0, -1);
    }
    term.write(cmd + '\r\n');
  }
  sendTerminalInput(cmd);
};

const startTerminalSession = async () => {
  initXterm();

  if (!state.terminalIsInitialized) {
    try {
      await invoke('terminal_ensure_started');
      state.terminalIsInitialized = true;
    } catch (err) {
      console.warn('啟動終端失敗：', err);
    }
  }

  startTerminalPolling();

  if (state.fitAddonInstance) {
    setTimeout(() => {
      try {
        state.fitAddonInstance.fit();
      } catch (_) {}
    }, 80);
  }

  if (state.xtermInstance) {
    setTimeout(() => state.xtermInstance.focus(), 120);
  }
};

const startTerminalPolling = () => {
  stopTerminalPolling();
  // 首次立即獲取目前緩衝區內容
  pollTerminalOutput();

  // 高頻 100ms 輪詢終端輸出，確保打字回顯即時流暢
  state.terminalTimer = setInterval(async () => {
    if (state.activeTab !== 'tab-terminal') {
      stopTerminalPolling();
      return;
    }
    await pollTerminalOutput();
  }, 100);
};

const stopTerminalPolling = () => {
  if (state.terminalTimer) {
    clearInterval(state.terminalTimer);
    state.terminalTimer = null;
  }
};

const pollTerminalOutput = async () => {
  try {
    const chunk = await invoke('terminal_poll_output');
    if (chunk && chunk.length > 0) {
      if (state.xtermInstance) {
        state.xtermInstance.write(chunk);
      }
    }
  } catch (err) {
    console.warn('輪詢終端輸出失敗：', err);
  }
};

const sendTerminalInput = async (cmd) => {
  try {
    await invoke('terminal_send_command', { command: cmd });
    // 立即排程讀取回顯
    setTimeout(pollTerminalOutput, 30);
    setTimeout(pollTerminalOutput, 100);
  } catch (err) {
    showToast(`發送失敗：${err}`);
  }
};

// 視窗載入即刻啟動
window.addEventListener('DOMContentLoaded', () => {
  initApp();
});
