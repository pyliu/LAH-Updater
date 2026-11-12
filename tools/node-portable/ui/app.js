const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;
let currentTab = "console";
let installTabAvailable = null;
let consolePollTimer = null;
let lastOverview = null;
let consoleRefresh = null;
let uninstallPlan = null;
let operationKind = "install";
let currentInlineProc = null;
let currentInlineLogType = "all";
let logModalMode = "file";
let logFileRequest = 0;
let logStream = null;
let logStreamSequence = 0;
let logStreamCommands = Promise.resolve();

async function initAdminStatus() {
  const badge = $("admin_badge");
  try {
    const isAdmin = await invoke("check_admin");
    badge.textContent = isAdmin ? "管理員權限" : "警告：一般權限（服務控制需管理員）";
    badge.className = "admin-badge " + (isAdmin ? "is-admin" : "not-admin");
  } catch (e) {
    console.error("check_admin failed:", e);
  }
}

function escapeHtml(str) {
  return String(str ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function formatBytes(bytes) {
  const n = Number(bytes) || 0;
  if (n <= 0) return "0 MB";
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatUptime(pmUptimeMs) {
  const start = Number(pmUptimeMs) || 0;
  if (start <= 0) return "-";
  const diffSec = Math.max(0, Math.floor((Date.now() - start) / 1000));
  if (diffSec < 60) return `${diffSec}秒`;
  const mins = Math.floor(diffSec / 60);
  if (mins < 60) return `${mins}分`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}時${mins % 60}分`;
  const days = Math.floor(hours / 24);
  return `${days}天${hours % 24}時`;
}

let toastTimer = null;
function setToast(msg, type = "normal") {
  const bar = $("console_toast");
  const text = $("console_toast_msg");
  clearTimeout(toastTimer);
  text.textContent = msg;
  bar.className = "toast-bar" + (type === "error" ? " error" : type === "success" ? " success" : "");
  if (type === "success") {
    toastTimer = setTimeout(() => {
      text.textContent = "就緒";
      bar.className = "toast-bar";
    }, 6000);
  }
}

function switchTab(tab) {
  if (isBusy) return;
  if (tab === "install" && installTabAvailable !== true) {
    tab = "console";
  }
  currentTab = tab;
  document.body.classList.toggle("install-mode", tab === "install");
  document.body.classList.toggle("console-mode", tab === "console");
  $("tab_btn_install").classList.toggle("active", tab === "install");
  $("tab_btn_console").classList.toggle("active", tab === "console");
  $("pane_install").classList.toggle("active", tab === "install");
  $("pane_console").classList.toggle("active", tab === "console");

  if (consolePollTimer) {
    clearInterval(consolePollTimer);
    consolePollTimer = null;
  }

  if (tab === "console") {
    const refreshed = refreshConsole();
    consolePollTimer = setInterval(() => {
      if ($("chk_auto_refresh").checked && !isBusy && !$("modal_progress").classList.contains("active")) {
        refreshConsole(true);
      }
    }, 4000);
    return refreshed;
  } else {
    stopLogStream("已離開控制台");
    $("modal_logs").classList.remove("active");
    rescan();
  }
}

$("tab_btn_install").onclick = () => switchTab("install");
$("tab_btn_console").onclick = () => switchTab("console");

function setInlineLogExpanded(expanded) {
  $("inline_log_card").classList.toggle("is-collapsed", !expanded);
  $("btn_toggle_inline_log").setAttribute("aria-expanded", String(expanded));
  $("btn_toggle_inline_log").textContent = expanded ? "收合日誌" : "展開日誌";
}
$("btn_toggle_inline_log").onclick = () => {
  setInlineLogExpanded($("inline_log_card").classList.contains("is-collapsed"));
};
setInlineLogExpanded(window.innerHeight >= 600);

$("btn_goto_console").onclick = () => switchTab("console");

$("btn_install_guide").onclick = () => {
  $("install_guide").classList.add("is-open");
  $("install_guide").setAttribute("role", "dialog");
  $("install_guide").setAttribute("aria-modal", "true");
  $("install_guide").setAttribute("aria-label", "沙盒架構與部署說明");
  $("btn_close_install_guide").focus();
};
$("btn_close_install_guide").onclick = () => {
  $("install_guide").classList.remove("is-open");
  $("install_guide").removeAttribute("role");
  $("install_guide").removeAttribute("aria-modal");
  $("btn_install_guide").focus();
};
document.addEventListener("keydown", (event) => {
  if (!$("install_guide").classList.contains("is-open")) return;
  if (event.key === "Escape") $("btn_close_install_guide").click();
  if (event.key === "Tab") {
    event.preventDefault();
    $("btn_close_install_guide").focus();
  }
});


function row(label, value, ok) {
  const mark = ok === undefined ? "" : ok ? "正常：" : "未就緒：";
  return `<div>${mark}<strong>${label}</strong>：${value}</div>`;
}

async function rescan(isInitial = false) {
  const curInput = $("root_dir").value.trim();
  const s = await invoke("scan_environment", { rootDir: curInput });
  if (!curInput && s.root_dir) {
    $("root_dir").value = s.root_dir;
  }
  let lrText = "尚未安裝（安裝時將由 node.zip 便攜版內建啟動）";
  if (s.logrotate_version) {
    lrText = s.logrotate_configured
      ? `v${s.logrotate_version}（便攜版內建，已掛載啟用）`
      : `v${s.logrotate_version}（便攜版內建，點擊開始安裝即可啟動）`;
  } else if (s.logrotate_configured) {
    lrText = "已掛載啟用";
  }
  $("scan_box").innerHTML = [
    row("node.zip 安裝包", s.zip_found || "未找到（請放於工具旁 base 資料夾、工具同層或 C:\\Temp）", !!s.zip_found),
    row("已安裝 Node.js", s.node_version || "尚未安裝", !!s.node_version),
    row("已安裝 PM2", s.pm2_version ? `v${s.pm2_version}` : "尚未安裝", !!s.pm2_version),
    row("pm2-logrotate 模組", lrText, s.logrotate_installed),
    row("PM2_Portable_Service", s.service_installed ? (s.service_running ? "已註冊，執行中" : "已註冊，未執行") : "未註冊", s.service_installed),
  ].join("");
  $("btn_start").disabled = !s.zip_found;

  const isInstalled = !!(s.node_version && s.pm2_version);
  $("btn_goto_console").style.display = isInstalled ? "inline-block" : "none";

  if (isInitial) {
    switchTab(installTabAvailable && !isInstalled ? "install" : "console");
  }
}

function refreshConsole(silent = false) {
  if (consoleRefresh) return consoleRefresh;
  consoleRefresh = loadConsole(silent).finally(() => { consoleRefresh = null; });
  return consoleRefresh;
}

async function loadConsole(silent = false) {
  const rootDir = $("root_dir").value.trim();
  const requestedTab = currentTab;
  if (!silent) {
    setToast("正在同步沙盒 Node / PM2 狀態...");
  }
  try {
    const ov = await invoke("get_console_overview", { rootDir });
    if (rootDir !== $("root_dir").value.trim() || currentTab !== requestedTab) return;
    lastOverview = ov;
    if (!rootDir && ov.root_dir) {
      $("root_dir").value = ov.root_dir;
    }

    $("con_root_dir").textContent = ov.root_dir || "-";
    $("con_node_ver").textContent = ov.node_version || "尚未安裝";
    $("con_pm2_ver").textContent = ov.pm2_version ? `v${ov.pm2_version}` : "尚未安裝";

    const svcBadge = $("con_svc_badge");
    if (ov.service_running) {
      svcBadge.textContent = "服務執行中";
      svcBadge.className = "status-pill online";
    } else if (ov.service_installed) {
      svcBadge.textContent = "服務已停止";
      svcBadge.className = "status-pill stopped";
    } else {
      svcBadge.textContent = "服務未註冊";
      svcBadge.className = "status-pill stopped";
    }

    // 解析內建 pm2-logrotate 模組狀態
    const lrProc = (ov.processes || []).find((p) => p.name === "pm2-logrotate" || p.is_module);
    const lrBadge = $("con_lr_badge");
    if (lrProc && lrProc.status === "online") {
      lrBadge.textContent = "運作中 (online)";
      lrBadge.className = "status-pill online";
    } else if (lrProc) {
      lrBadge.textContent = `${lrProc.status}`;
      lrBadge.className = "status-pill errored";
    } else if (ov.logrotate_configured) {
      lrBadge.textContent = "已掛載 (待喚醒)";
      lrBadge.className = "status-pill stopped";
    } else {
      lrBadge.textContent = "未掛載";
      lrBadge.className = "status-pill stopped";
    }

    $("con_lr_ver").textContent = ov.logrotate_version
      ? `v${ov.logrotate_version}`
      : lrProc && lrProc.version !== "-"
      ? `v${lrProc.version}`
      : "-";
    $("con_lr_compress").textContent = ov.logrotate_compress ? "已啟用 (Gzip)" : "未啟用";
    $("con_lr_pid").textContent = lrProc && lrProc.pid ? String(lrProc.pid) : "-";
    $("con_lr_mem").textContent = lrProc ? formatBytes(lrProc.memory) : "-";

    // 渲染受管業務系統清單（排除 pm2-logrotate 模組）
    const apps = (ov.processes || []).filter((p) => !p.is_module && p.name !== "pm2-logrotate");
    if (!currentInlineProc && apps.length > 0) currentInlineProc = apps[0];
    const onlineApps = apps.filter((p) => p.status === "online").length;
    const countBadge = $("con_app_count");
    countBadge.textContent = `${onlineApps} / ${apps.length} 系統在線`;
    countBadge.className = "status-pill " + (onlineApps === apps.length && apps.length > 0 ? "online" : "stopped");

    const tbody = $("pm2_apps_tbody");
    if (!ov.installed) {
      tbody.innerHTML = `<div class="pm2-empty" style="color:#b45309;">
        警告：此目錄尚未安裝沙盒 Node.js 與 PM2，${installTabAvailable
          ? "請先切換至「沙盒安裝與部署」分頁執行安裝。"
          : "請將 node.zip 放在工具執行檔同目錄的 base 資料夾後重新開啟工具。"}
      </div>`;
    } else if (ov.pm2_error && apps.length === 0) {
      tbody.innerHTML = `<div class="pm2-empty">
        目前尚無活躍進程或 PM2 守護進程待喚醒（可點擊上方「還原快照」或「啟動服務」喚醒）
      </div>`;
    } else if (apps.length === 0) {
      tbody.innerHTML = `<div class="pm2-empty">
        目前沙盒 PM2 尚無受管業務系統（可點擊「還原快照」載入 dump.pm2，或開啟「沙盒終端」啟動系統）
      </div>`;
    } else {
      tbody.innerHTML = apps
        .map((app) => {
          const isOnline = app.status === "online";
          const pillClass = isOnline ? "online" : app.status === "stopped" ? "stopped" : "errored";
          const toggleAction = isOnline ? "stop" : "start";
          const toggleLabel = isOnline ? "停止" : "啟動";
          const safeName = escapeHtml(app.name);
          const selected = currentInlineProc && currentInlineProc.pm_id === app.pm_id;
          return `<div class="pm2-card${selected ? " active-row" : ""}" data-pm2-id="${app.pm_id}">
            <div class="pm2-card-header"><button class="pm2-select pm2-card-name" aria-label="選取 ${safeName} 的日誌" aria-pressed="${!!selected}">${safeName}</button><span class="status-pill ${pillClass}">${escapeHtml(app.status)}</span></div>
            <div class="pm2-card-details"><span>ID ${app.pm_id}</span><span>PID ${app.pid || "-"}</span><span>CPU ${app.cpu.toFixed(1)}%</span><span>記憶體 ${formatBytes(app.memory)}</span><span>運行 ${isOnline ? formatUptime(app.uptime) : "-"}</span><span>重啟 ${app.restarts}</span></div>
            <div class="btn-group">
                <button class="btn-sm" data-pm2-action="${toggleAction}" data-pm2-target="${safeName}">${toggleLabel}</button>
                <button class="btn-sm btn-green" data-pm2-action="restart" data-pm2-target="${safeName}"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg> 重啟</button>
                <button class="btn-sm${app.header_protection_enabled ? " btn-green" : ""}" data-header-protection="${app.pm_id}" ${!app.header_protection_supported || (!isOnline && !(app.header_protection_enabled && ["stopped", "errored"].includes(app.status))) ? "disabled" : ""} title="HTTP/HTTPS 標頭防護設定，HTTP/2 不支援；請另執行資安自檢">${app.header_protection_enabled ? "停用防護" : "啟用防護"}</button>
                <button class="btn-sm primary" data-pm2-log="${app.pm_id}"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 2h8l4 4v16H6zM14 2v5h4M9 11h6M9 15h6"/></svg> 日誌</button>
            </div>
          </div>`;
        })
        .join("");
    }

    const now = new Date();
    const p = (n) => String(n).padStart(2, "0");
    $("console_last_updated").textContent = `更新於 ${p(now.getHours())}:${p(now.getMinutes())}:${p(now.getSeconds())}`;
    if (!silent) {
      setToast("沙盒狀態同步完成");
    }
    // 大螢幕工作台模式：自動同步內嵌日誌串流
    const procs = ov.processes || [];
    currentInlineProc = currentInlineProc
      ? procs.find(p => p.pm_id === currentInlineProc.pm_id) || null
      : procs.find(p => !p.is_module) || null;
    if (!currentInlineProc) stopLogStream("目前無受管系統可供監看");
    if (currentTab === "console") updateInlineLog(currentInlineProc, currentInlineLogType);
  } catch (err) {
    setToast(`同步沙盒狀態失敗：${err}`, "error");
  }
}

// 綁定受管系統卡片內的動態按鈕事件
$("pm2_apps_tbody").onclick = async (e) => {
  const row = e.target.closest(".pm2-card");
  if (row) {
    document.querySelectorAll("#pm2_apps_tbody .pm2-card").forEach(r => { r.classList.remove("active-row"); r.querySelector(".pm2-select").setAttribute("aria-pressed", "false"); });
    row.classList.add("active-row");
    row.querySelector(".pm2-select").setAttribute("aria-pressed", "true");
  }

  const btn = e.target.closest("button");
  if (!btn || btn.classList.contains("pm2-select")) {
    // 點擊非按鈕區域：自動切換下方內嵌日誌
    const targetId = row?.getAttribute("data-pm2-id");
    if (targetId != null) {
      const proc = (lastOverview?.processes || []).find((p) => p.pm_id === Number(targetId));
      if (proc) await updateInlineLog(proc, "all");
    }
    return;
  }
  if (isBusy) return;

  const protectionId = btn.getAttribute("data-header-protection");
  if (protectionId !== null) {
    const proc = (lastOverview?.processes || []).find(p => p.pm_id === Number(protectionId));
    if (proc && !btn.disabled) await setHeaderProtection(proc, btn);
    return;
  }
  const action = btn.getAttribute("data-pm2-action");
  const target = btn.getAttribute("data-pm2-target");
  const logId = btn.getAttribute("data-pm2-log");

  if (action && target) {
    await executePm2Action(action, target, btn);
  } else if (logId !== null) {
    const proc = (lastOverview?.processes || []).find((p) => p.pm_id === Number(logId));
    // 連動內嵌日誌
    if (proc) await updateInlineLog(proc, "all");
    await openLogModal(proc?.out_log_path || "", proc || null);
  }
};

async function setHeaderProtection(proc, button) {
  const enabled = !proc.header_protection_enabled;
  const label = enabled ? "啟用" : "停用";
  if (!confirm(`將${label} ${proc.name}（ID ${proc.pm_id}）的標頭防護，重新啟動此程序，並儲存全部 PM2 程序的目前快照。
防護僅移除 X-Powered-By 與 Server，HTTP/2 不支援。是否繼續？`)) return;
  isBusy = true;
  button.disabled = true;
  try {
    const result = await invoke("set_header_protection", {
      rootDir: $("root_dir").value.trim(), pmId: proc.pm_id, enabled, expectedExecPath: proc.exec_path,
    });
    await refreshConsole(true);
    setToast(result.message, result.success ? "success" : "error");
  } catch (error) {
    await refreshConsole(true);
    setToast(`標頭防護操作失敗：${error}`, "error");
  } finally {
    isBusy = false;
    button.disabled = false;
  }
}

async function executePm2Action(action, target, btnEl = null) {
  if (isBusy) return;
  const rootDir = $("root_dir").value.trim();
  isBusy = true;
  if (btnEl) btnEl.disabled = true;
  setToast(`正在對 [${target}] 執行 PM2 ${action} 指令...`);
  try {
    const res = await invoke("pm2_process_action", { rootDir, action, target });
    setToast(res.message, res.success ? "success" : "error");
    await refreshConsole(true);
  } catch (err) {
    setToast(`執行失敗：${err}`, "error");
  } finally {
    isBusy = false;
    if (btnEl) btnEl.disabled = false;
  }
}

async function executeServiceAction(action, btnEl = null) {
  if (isBusy) return;
  const rootDir = $("root_dir").value.trim();
  isBusy = true;
  if (btnEl) btnEl.disabled = true;
  setToast(`正在對 PM2_Portable_Service 執行 [${action}]...`);
  try {
    const res = await invoke("pm2_service_action", { rootDir, action });
    setToast(res.message, res.success ? "success" : "error");
    await refreshConsole(true);
  } catch (err) {
    setToast(`服務操作失敗：${err}`, "error");
  } finally {
    isBusy = false;
    if (btnEl) btnEl.disabled = false;
  }
}

$("btn_svc_start").onclick = (e) => executeServiceAction("start", e.currentTarget);
$("btn_svc_stop").onclick = (e) => executeServiceAction("stop", e.currentTarget);
$("btn_svc_restart").onclick = (e) => executeServiceAction("restart", e.currentTarget);

$("btn_open_cmd").onclick = async () => {
  const button = $("btn_open_cmd");
  if (isBusy || button.disabled) return;
  button.disabled = true;
  try {
    const rootDir = $("root_dir").value.trim();
    const res = await invoke("open_lah_cmd", { rootDir });
    setToast(res.success ? "已在新視窗開啟 lah-cmd 終端，可關閉該視窗，不影響服務" : res.message, res.success ? "success" : "error");
  } catch (err) {
    setToast(`開啟沙盒終端失敗：${err}`, "error");
  } finally {
    button.disabled = false;
  }
};

$("btn_lr_trigger").onclick = async (e) => {
  if (isBusy) return;
  const btn = e.currentTarget;
  const rootDir = $("root_dir").value.trim();
  isBusy = true;
  btn.disabled = true;
  setToast("正在執行 Gzip 日誌壓縮輪替與 pm2 reloadLogs...");
  try {
    const res = await invoke("trigger_logrotate_now", { rootDir });
    setToast(res.message, res.success ? "success" : "error");
    await refreshConsole(true);
  } catch (err) {
    setToast(`日誌輪替失敗：${err}`, "error");
  } finally {
    isBusy = false;
    btn.disabled = false;
  }
};

$("btn_lr_repair").onclick = async (e) => {
  if (isBusy) return;
  const btn = e.currentTarget;
  const rootDir = $("root_dir").value.trim();
  isBusy = true;
  btn.disabled = true;
  setToast("正在由沙盒內建模組掛載並啟動 pm2-logrotate...");
  try {
    const res = await invoke("repair_logrotate", { rootDir });
    setToast(res.message, res.success ? "success" : "error");
    await refreshConsole(true);
  } catch (err) {
    setToast(`啟動 pm2-logrotate 失敗：${err}`, "error");
  } finally {
    isBusy = false;
    btn.disabled = false;
  }
};

$("btn_lr_logs").onclick = async () => {
  const lrProc = (lastOverview?.processes || []).find((p) => p.name === "pm2-logrotate" || p.is_module);
  await openLogModal(lrProc?.out_log_path || "", lrProc || null);
};

$("btn_pm2_refresh").onclick = () => refreshConsole(false);
$("btn_pm2_restart_all").onclick = (e) => executePm2Action("restart", "all", e.currentTarget);
$("btn_pm2_save").onclick = (e) => executePm2Action("save", "all", e.currentTarget);
$("btn_pm2_resurrect").onclick = (e) => executePm2Action("resurrect", "all", e.currentTarget);

$("btn_open_logs_dir").onclick = async () => {
  const rootDir = $("root_dir").value.trim();
  const res = await invoke("open_logs_folder", { rootDir });
  setToast(res.message, res.success ? "success" : "error");
};

$("btn_console_guide").onclick = () => {
  $("modal_console_guide").classList.add("active");
  $("btn_close_console_guide").focus();
};
$("btn_close_console_guide").onclick = () => {
  $("modal_console_guide").classList.remove("active");
  $("btn_console_guide").focus();
};

$("btn_open_log_modal").onclick = () => openLogModal("", null);

// ==================== 即時日誌預覽彈窗 (Log Tail Modal) ====================
async function openLogModal(preferredPath = "", procContext = null) {
  $("modal_logs").classList.add("active");
  if (procContext) {
    setLogModalMode("live");
    await updateInlineLog(procContext);
    return;
  }
  setLogModalMode("file");
  await loadLogFiles(preferredPath);
}

function setLogModalMode(mode) {
  logModalMode = mode;
  logFileRequest++;
  const live = mode === "live";
  $("sel_log_file").style.display = live ? "none" : "";
  $("proc_quick_logs").style.display = live ? "inline-flex" : "none";
  $("btn_log_files").style.display = live ? "" : "none";
  $("btn_log_live").style.display = !live && currentInlineProc ? "" : "none";
  $("btn_log_Notepad").disabled = live;
  if (live) {
    $("sel_log_lines").value = $("sel_inline_lines").value;
    renderLogStream();
  }
}

async function loadLogFiles(preferredPath = "") {
  const request = ++logFileRequest;
  $("log_tail_box").textContent = "正在載入日誌清單...";
  try {
    const logs = await invoke("get_sandbox_logs", { rootDir: $("root_dir").value.trim() });
    if (request !== logFileRequest || logModalMode !== "file") return;
    const sel = $("sel_log_file");
    sel.innerHTML = logs.length ? logs.map(l =>
      `<option value="${escapeHtml(l.path)}">${escapeHtml(l.label)} (${formatBytes(l.size_bytes)})</option>`
    ).join("") : `<option value="">（目前尚無日誌檔案）</option>`;
    if (logs.some(l => l.path.toLowerCase() === preferredPath.toLowerCase())) sel.value = preferredPath;
    if (!logs.length) {
      $("log_meta_bar").textContent = "尚未產生日誌檔案";
      $("log_tail_box").textContent = "目前沒有可查閱的日誌檔案。即時監看請選取受管系統。";
      return;
    }
    await loadSelectedLog();
  } catch (err) {
    if (request === logFileRequest && logModalMode === "file") $("log_tail_box").textContent = `載入日誌清單失敗：${err}`;
  }
}

async function loadSelectedLog() {
  if (logModalMode === "live") {
    $("sel_inline_lines").value = $("sel_log_lines").value;
    return updateInlineLog(null, null, true);
  }
  const path = $("sel_log_file").value;
  if (!path) return;
  const request = ++logFileRequest;
  $("log_tail_box").textContent = "讀取中...";
  try {
    const res = await invoke("get_log_tail", { path, maxLines: Number($("sel_log_lines").value) || 200 });
    if (request !== logFileRequest || logModalMode !== "file") return;
    $("log_meta_bar").textContent = `檔案路徑：${res.path} ｜ 檔案大小：${formatBytes(res.file_size)}`;
    const box = $("log_tail_box");
    box.textContent = res.exists ? res.lines.join("\n") || "[日誌目前為空]" : "日誌檔案尚未產生。";
    box.scrollTop = box.scrollHeight;
  } catch (err) {
    if (request === logFileRequest && logModalMode === "file") $("log_tail_box").textContent = `讀取日誌失敗：${err}`;
  }
}

$("sel_log_file").onchange = loadSelectedLog;
$("sel_log_lines").onchange = loadSelectedLog;
$("btn_log_reload").onclick = loadSelectedLog;
$("btn_quick_all").onclick = () => updateInlineLog(null, "all");
$("btn_quick_out").onclick = () => updateInlineLog(null, "out");
$("btn_quick_err").onclick = () => updateInlineLog(null, "err");
$("btn_log_files").onclick = () => {
  setLogModalMode("file");
  loadLogFiles();
};
$("btn_log_live").onclick = () => {
  setLogModalMode("live");
  updateInlineLog();
};

$("btn_log_Notepad").onclick = async () => {
  const path = $("sel_log_file").value;
  if (!path || logModalMode !== "file") return;
  const res = await invoke("open_log_in_notepad", { path });
  if (!res.success) alert(res.message);
};

$("btn_log_Folder").onclick = async () => {
  await invoke("open_logs_folder", { rootDir: $("root_dir").value.trim() });
};

$("btn_close_logs").onclick = () => {
  logFileRequest++;
  $("modal_logs").classList.remove("active");
};

// ==================== 安裝與資安自檢邏輯 ====================
function appendLog(message, level = "info") {
  const box = $("log_box");
  const now = new Date();
  const p = (n) => String(n).padStart(2, "0");
  const line = document.createElement("div");
  line.className = `log-line ${level}`;
  const t = document.createElement("span");
  t.className = "log-line time";
  t.textContent = `[${p(now.getHours())}:${p(now.getMinutes())}:${p(now.getSeconds())}] `;
  const m = document.createElement("span");
  m.textContent = message;
  line.append(t, m);
  box.appendChild(line);
  box.scrollTop = box.scrollHeight;

  const inlineBox = $("inline_install_log_box");
  if (inlineBox) {
    if (inlineBox.textContent.startsWith("準備就緒")) {
      inlineBox.innerHTML = "";
    }
    const inlineLine = line.cloneNode(true);
    inlineBox.appendChild(inlineLine);
    inlineBox.scrollTop = inlineBox.scrollHeight;
  }
}

listen("progress", ({ payload }) => {
  const pct = Math.max(0, Math.min(100, Number(payload.percent) || 0));
  $("progress_bar").value = pct;
  $("percent_text").textContent = `${pct.toFixed(1)}%`;
  if (payload.message) $("status_text").textContent = payload.message;
});
listen("log", ({ payload }) => appendLog(payload.message, payload.level || "info"));

$("browse_btn").onclick = async () => {
  const selected = await open({ directory: true, defaultPath: $("root_dir").value.trim() || "C:\\AppServ" });
  if (selected) {
    resetLogRoot();
    $("root_dir").value = selected;
    rescan();
  }
};
$("root_dir").onchange = () => {
  resetLogRoot();
  rescan();
};
$("btn_rescan").onclick = () => rescan();

$("btn_audit").onclick = async () => {
  if (isBusy) return;
  const rootDir = $("root_dir").value.trim();
  $("btn_audit").disabled = true;
  $("btn_audit").textContent = "檢測中...";

  try {
    const report = await invoke("run_security_audit", { rootDir });
    const sysTitle = report.system_title || "Node 沙盒受管系統";
    if ($("audit_modal_title")) {
      $("audit_modal_title").textContent = `${sysTitle} - 資安邊界自檢報告`;
    }
    const badge = $("audit_badge_box");
    if (report.overall_secure) {
      badge.style.background = "#dcfce7";
      badge.style.color = "#15803d";
      badge.style.border = "1px solid #bbf7d0";
      badge.textContent = `資安自檢評定：【合格安全】（${sysTitle} 已達成零全域污染沙盒隔離，受管服務無執行期標頭洩漏）`;
    } else {
      badge.style.background = "#fee2e2";
      badge.style.color = "#b91c1c";
      badge.style.border = "1px solid #fecaca";
      badge.textContent = `警告：資安自檢評定：【發現潛在風險】（${sysTitle} 請檢視下方標頭外洩項目或清理 PATH 登錄檔）`;
    }

    let detail = `檢測系統：${sysTitle}\n檢測時間：${report.timestamp}\n安裝目錄：${report.root_dir}\n`;
    detail += `全域 PATH 零污染沙盒：${report.sandbox_path_clean ? "合規 (HKLM/HKCU 無殘留)" : "警告：未合規 (登錄檔存在可攜路徑)"}\n\n`;
    detail += "受管服務網路通訊邊界檢測：\n";
    for (const p of report.port_results) {
      detail += `• Port ${p.port} (${p.service_name})：\n`;
      detail += `  - 監聽狀態：${p.is_listening ? "監聽中" : "未啟動/未監聽"}\n`;
      detail += `  - 判定結果：${p.is_secure ? "合格 (無敏感技術棧指紋)" : "警告：需改善"}\n`;
      if (p.leaked_headers && p.leaked_headers.length > 0) {
        detail += `  - 洩漏標頭：${p.leaked_headers.join(", ")}\n`;
      }
    }
    if (report.report_file_path) {
      detail += `\n完整佐證報告已匯出：\n${report.report_file_path}\n`;
    }
    $("audit_detail_box").textContent = detail;
    $("modal_audit").classList.add("active");
  } catch (err) {
    alert(`執行資安自檢失敗: ${err}`);
  } finally {
    $("btn_audit").disabled = false;
    $("btn_audit").textContent = "資安自檢";
  }
};

$("btn_close_audit").onclick = () => {
  $("modal_audit").classList.remove("active");
};

async function exitApp() {
  if (isBusy) return;
  await stopLogStream("工具已關閉");
  await invoke("exit_app");
}
$("btn_exit").onclick = exitApp;
$("btn_close_modal").onclick = exitApp;

$("btn_modal_back").onclick = () => {
  if (isBusy) return;
  $("modal_progress").classList.remove("active");
  $("btn_exit").disabled = false;
  if (operationKind === "uninstall") switchTab("console");
  else rescan();
};

$("btn_modal_to_console").onclick = () => {
  if (isBusy) return;
  $("modal_progress").classList.remove("active");
  $("btn_exit").disabled = false;
  switchTab("console");
};

$("btn_start").onclick = async () => {
  if (isBusy) return;
  operationKind = "install";
  $("operation_title").textContent = "安裝進行中...";
  $("btn_modal_back").textContent = "返回安裝設定";
  const rootDir = $("root_dir").value.trim();
  if (!rootDir) {
    alert("請輸入安裝根目錄！");
    return;
  }
  const options = {
    root_dir: rootDir,
    register_service: $("chk_service").checked,
    install_logrotate: $("chk_logrotate").checked,
    resurrect: $("chk_resurrect").checked,
  };

  isBusy = true;
  $("btn_start").disabled = true;
  $("btn_exit").disabled = true;
  $("btn_modal_back").style.display = "none";
  $("btn_modal_to_console").style.display = "none";
  $("btn_close_modal").disabled = true;
  $("log_box").innerHTML = "";
  $("progress_bar").value = 0;
  $("percent_text").textContent = "0.0%";
  $("status_text").textContent = "準備中...";
  $("modal_progress").classList.add("active");

  try {
    const res = await invoke("run_install", { options });
    $("status_text").textContent = res.success
      ? `${res.message}（耗時 ${res.elapsed_secs.toFixed(1)} 秒）`
      : `警告：安裝失敗：${res.message}`;
    if (res.success) {
      $("btn_modal_to_console").style.display = "inline-block";
    }
  } catch (err) {
    appendLog(`安裝發生嚴重異常: ${err}`, "error");
    $("status_text").textContent = "安裝過程發生異常！";
  } finally {
    isBusy = false;
    $("btn_modal_back").style.display = "inline-block";
    $("btn_close_modal").disabled = false;
    $("btn_close_modal").textContent = "關閉離開";
  }
};


$("btn_uninstall").onclick = async () => {
  if (isBusy) return;
  isBusy = true;
  uninstallPlan = null;
  $("btn_confirm_uninstall").disabled = true;
  $("btn_cancel_uninstall").disabled = true;
  $("uninstall_detail").textContent = "正在檢查目錄、服務與受影響程序...";
  $("modal_uninstall").classList.add("active");
  try {
    if (consoleRefresh) await consoleRefresh;
    const plan = await invoke("preview_uninstall", { rootDir: $("root_dir").value.trim() });
    uninstallPlan = plan;
    const list = values => `<ul>${values.map(value => `<li>${escapeHtml(value)}</li>`).join("")}</ul>`;
    $("uninstall_detail").innerHTML = `
      <p><b>目標目錄：</b>${escapeHtml(plan.root_dir)}</p>
      <p>移除後，使用這套 Node／PM2 的應用會停止運作。重裝並還原快照後才能恢復。</p>
      <b>解除服務</b>${list(plan.service_installed ? ["PM2_Portable_Service"] : ["此沙盒未註冊服務"])}
      <b>將停止的程序（含受管應用與子程序）</b>${list(plan.processes.length ? plan.processes.map(p => `${p.name} · PID ${p.pid} · ${p.executable}`) : ["目前沒有使用此沙盒的程序"])}
      <p>程序清單為預覽當下的狀態。確認後會重新盤點，停止屬於此沙盒的程序。</p>
      <b>移除</b>${list(plan.remove)}
      <p>node 目錄會整個刪除；若曾自行放入應用或資料，請先搬出，再重新預覽。</p>
      <b>保留</b>${list(plan.preserve)}
      <p>保留既有快照，不會自動覆寫為目前程序清單。工具執行檔會保留，方便日後重新安裝。</p>
      ${plan.blockers.length ? `<div role="alert" style="color:#b91c1c;"><b>目前無法移除</b>${list(plan.blockers)}</div>` : ""}`;
    $("btn_confirm_uninstall").disabled = plan.blockers.length > 0;
    $(plan.blockers.length ? "btn_cancel_uninstall" : "btn_confirm_uninstall").focus();
  } catch (error) {
    $("uninstall_detail").textContent = `無法預覽移除：${error}`;
  } finally {
    $("btn_cancel_uninstall").disabled = false;
    if ($("btn_confirm_uninstall").disabled) $("btn_cancel_uninstall").focus();
  }
};

$("btn_cancel_uninstall").onclick = () => {
  if ($("btn_cancel_uninstall").disabled) return;
  $("modal_uninstall").classList.remove("active");
  uninstallPlan = null;
  isBusy = false;
  $("btn_uninstall").focus();
};

$("btn_confirm_uninstall").onclick = async () => {
  if (!uninstallPlan || $("btn_confirm_uninstall").disabled) return;
  const plan = uninstallPlan;
  uninstallPlan = null;
  operationKind = "uninstall";
  $("btn_confirm_uninstall").disabled = true;
  $("modal_uninstall").classList.remove("active");
  $("operation_title").textContent = "沙盒移除作業";
  $("btn_modal_back").textContent = "返回控制台";
  $("btn_modal_back").style.display = "none";
  $("btn_modal_to_console").style.display = "none";
  $("btn_close_modal").disabled = true;
  $("log_box").innerHTML = "";
  $("progress_bar").value = 0;
  $("percent_text").textContent = "0.0%";
  $("status_text").textContent = "重新檢查移除範圍...";
  $("modal_progress").classList.add("active");
  try {
    await stopLogStream("沙盒移除中");
    const message = await invoke("run_uninstall", { plan });
    $("status_text").textContent = message;
    currentInlineProc = null;
    lastOverview = null;
    $("inline_log_box").textContent = "沙盒已移除；既有日誌仍保留在原目錄。";
  } catch (error) {
    appendLog(`移除已停止：${error}`, "error");
    $("status_text").textContent = `移除未完成：${error}`;
  } finally {
    isBusy = false;
    $("btn_modal_back").style.display = "inline-block";
    $("btn_close_modal").disabled = false;
  }
};

async function initializeTabs() {
  try {
    installTabAvailable = await invoke("has_adjacent_node_zip");
  } catch (err) {
    installTabAvailable = false;
    console.error("has_adjacent_node_zip failed:", err);
  }
  $("tab_btn_install").style.display = installTabAvailable ? "" : "none";
  try {
    await rescan(true);
  } catch (err) {
    await switchTab("console");
    setToast(`環境偵測失敗：${err}`, "error");
  }
}

initAdminStatus();
initializeTabs();

// ==================== 大螢幕工作台模式：內嵌即時日誌串流邏輯 ====================
function queueLogCommand(command) {
  logStreamCommands = logStreamCommands.catch(() => {}).then(command);
  return logStreamCommands;
}

function stopLogStream(message = "監看已停止") {
  const previous = logStream;
  logStream = null;
  renderLogStream(message);
  if (!previous) return Promise.resolve();
  return queueLogCommand(() => invoke("stop_pm2_log_stream", { sessionId: previous.sessionId }))
    .catch(err => setToast(`停止日誌監看失敗：${err}`, "error"));
}

function resetLogRoot() {
  stopLogStream("沙盒目錄已變更，請重新選取受管系統");
  currentInlineProc = null;
  lastOverview = null;
  logFileRequest++;
  $("modal_logs").classList.remove("active");
  renderLogStream();
}

function renderLogStream(emptyMessage = "目前無受管系統可供監看") {
  const stream = logStream;
  const statusText = {
    connecting: "連線中", running: "等待輸出", streaming: "監看中",
    stopped: "已停止", disconnected: "連線中斷", failed: "監看失敗"
  };
  const status = stream ? statusText[stream.status] || stream.status : "已停止";
  const proc = currentInlineProc;
  $("inline_log_proc_name").textContent = proc ? `${proc.name} (ID: ${proc.pm_id}, PID: ${proc.pid || "-"})` : "尚未選取受管系統";
  $("inline_log_badge").textContent = status;
  $("inline_log_badge").className = "status-pill " + (stream && ["running", "streaming"].includes(stream.status) ? "online" : "");
  for (const [suffix, type] of [["all", "all"], ["out", "out"], ["err", "err"]]) {
    for (const prefix of ["btn_inline_", "btn_quick_"]) {
      const btn = $(prefix + suffix);
      btn.setAttribute("aria-pressed", String(currentInlineLogType === type));
      btn.style.fontWeight = currentInlineLogType === type ? "bold" : "normal";
      btn.style.outline = currentInlineLogType === type ? "2px solid #0284c7" : "";
    }
  }
  const filter = { all: "全部輸出", out: "標準輸出", err: "錯誤日誌" }[currentInlineLogType];
  const meta = stream ? `PM2 logs ${proc?.pm_id} ｜ ${filter} ｜ ${status} ｜ ${[stream.notice, stream.message].filter(Boolean).join(" ")}` : emptyMessage;
  $("inline_log_meta").textContent = meta;
  const text = stream ? stream.chunks.filter(c => currentInlineLogType === "all" || c.stream === currentInlineLogType).map(c => c.text).join("") : "";
  const content = text || (stream ? stream.message || "等待 PM2 輸出..." : emptyMessage);
  for (const id of logModalMode === "live" ? ["inline_log_box", "log_tail_box"] : ["inline_log_box"]) {
    const box = $(id);
    if (box.textContent === content) continue;
    const atBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 40;
    box.textContent = content;
    if (atBottom) box.scrollTop = box.scrollHeight;
  }
  if (logModalMode === "live") $("log_meta_bar").textContent = meta;
}

function receiveLogStream(payload) {
  const stream = logStream;
  if (!stream || payload.session_id !== stream.sessionId) return;
  if (payload.kind === "output") {
    const text = payload.message;
    if (!text) return;
    const record = { stream: payload.stream, text, lines: (text.match(/\n/g) || []).length };
    stream.chunks.push(record);
    stream.characters += text.length;
    stream.lines += record.lines;
    while (stream.chunks.length && (stream.characters > 200000 || stream.lines > 2000 || stream.chunks.length > 2000)) {
      const oldest = stream.chunks[0];
      if (stream.chunks.length > 2000) {
        stream.chunks.shift();
        stream.characters -= oldest.text.length;
        stream.lines -= oldest.lines;
        continue;
      }
      let remove = Math.max(0, stream.characters - 200000);
      let lineEnd = -1;
      for (let n = 0; n < stream.lines - 2000; n++) {
        lineEnd = oldest.text.indexOf("\n", lineEnd + 1);
        if (lineEnd < 0) {
          lineEnd = oldest.text.length - 1;
          break;
        }
      }
      remove = Math.min(oldest.text.length, Math.max(remove, lineEnd + 1));
      const removed = oldest.text.slice(0, remove);
      oldest.text = oldest.text.slice(remove);
      const removedLines = (removed.match(/\n/g) || []).length;
      oldest.lines -= removedLines;
      stream.characters -= removed.length;
      stream.lines -= removedLines;
      if (!oldest.text) stream.chunks.shift();
    }
    stream.status = "streaming";
    stream.message = "正在接收 PM2 輸出";
  } else {
    if (!(payload.status === "running" && stream.status === "streaming")) stream.status = payload.status;
    stream.message = payload.message || (payload.status === "running" ? "監看已啟動，等待 PM2 輸出..." : "日誌監看已中斷，請重新載入。");
  }
  renderLogStream();
}

const logStreamListener = listen("pm2-log-stream", ({ payload }) => receiveLogStream(payload));

async function checkLogHistory(stream, proc) {
  const paths = [...new Set([proc.out_log_path, proc.err_log_path].filter(Boolean))];
  const checks = await Promise.allSettled(paths.map(path => invoke("get_log_tail", { path, maxLines: 1 })));
  if (logStream !== stream) return;
  const missing = checks.filter(result => result.status === "fulfilled" && !result.value.exists).length;
  if (!paths.length || missing) stream.notice = "部分歷史日誌檔不存在，仍持續接收即時輸出。";
  else if (checks.some(result => result.status === "rejected")) stream.notice = "部分歷史日誌無法讀取，仍持續接收即時輸出。";
  renderLogStream();
}

async function updateInlineLog(proc = null, type = null, reload = false) {
  if (proc) currentInlineProc = proc;
  if (type) currentInlineLogType = type;
  if (!currentInlineProc || currentTab !== "console") {
    renderLogStream();
    return;
  }
  const rootDir = $("root_dir").value.trim();
  const maxLines = Number($("sel_inline_lines").value) || 200;
  const key = JSON.stringify([rootDir, currentInlineProc.pm_id, maxLines]);
  if (!reload && logStream?.key === key) {
    renderLogStream();
    return;
  }
  const previous = logStream;
  const stream = {
    sessionId: `log-${Date.now()}-${++logStreamSequence}`, key,
    status: "connecting", message: "正在啟動 PM2 日誌監看...", notice: "",
    chunks: [], characters: 0, lines: 0
  };
  logStream = stream;
  const pmId = currentInlineProc.pm_id;
  checkLogHistory(stream, currentInlineProc);
  renderLogStream();
  return queueLogCommand(async () => {
    if (previous) await invoke("stop_pm2_log_stream", { sessionId: previous.sessionId });
    if (logStream !== stream) return;
    await logStreamListener;
    if (logStream !== stream) return;
    await invoke("start_pm2_log_stream", { rootDir, pmId, sessionId: stream.sessionId, maxLines });
    if (logStream !== stream) await invoke("stop_pm2_log_stream", { sessionId: stream.sessionId });
  }).catch(err => {
    if (logStream !== stream) return;
    stream.status = "failed";
    stream.message = `啟動日誌監看失敗：${err}`;
    renderLogStream();
  });
}

$("btn_inline_all").onclick = () => updateInlineLog(null, "all");
$("btn_inline_out").onclick = () => updateInlineLog(null, "out");
$("btn_inline_err").onclick = () => updateInlineLog(null, "err");
$("sel_inline_lines").onchange = () => {
  if (logModalMode === "live") $("sel_log_lines").value = $("sel_inline_lines").value;
  updateInlineLog(null, null, true);
};
$("btn_inline_reload").onclick = () => updateInlineLog(null, null, true);

$("btn_inline_notepad").onclick = async () => {
  if (!currentInlineProc) return;
  if (currentInlineLogType === "all") return openLogModal("", null);
  const path = currentInlineLogType === "out" ? currentInlineProc.out_log_path : currentInlineProc.err_log_path;
  if (path) {
    const res = await invoke("open_log_in_notepad", { path });
    setToast(res.message, res.success ? "success" : "error");
  }
};

$("btn_inline_folder").onclick = async () => {
  const res = await invoke("open_logs_folder", { rootDir: $("root_dir").value.trim() });
  setToast(res.message, res.success ? "success" : "error");
};

$("btn_inline_modal").onclick = () => {
  if (currentInlineProc) openLogModal("", currentInlineProc);
};

window.addEventListener("beforeunload", () => {
  if (logStream) invoke("stop_pm2_log_stream", { sessionId: logStream.sessionId }).catch(() => {});
});
