const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;
let currentTab = "install";
let initialTabDecided = false;
let consolePollTimer = null;
let lastOverview = null;
let currentLogProcess = null;

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

function setToast(msg, type = "normal") {
  const bar = $("console_toast");
  const text = $("console_toast_msg");
  text.textContent = msg;
  bar.className = "toast-bar" + (type === "error" ? " error" : type === "success" ? " success" : "");
}

function switchTab(tab) {
  currentTab = tab;
  $("tab_btn_install").classList.toggle("active", tab === "install");
  $("tab_btn_console").classList.toggle("active", tab === "console");
  $("pane_install").classList.toggle("active", tab === "install");
  $("pane_console").classList.toggle("active", tab === "console");

  if (consolePollTimer) {
    clearInterval(consolePollTimer);
    consolePollTimer = null;
  }

  if (tab === "console") {
    refreshConsole();
    consolePollTimer = setInterval(() => {
      if ($("chk_auto_refresh").checked && !isBusy) {
        refreshConsole(true);
      }
    }, 4000);
  } else {
    rescan();
  }
}

$("tab_btn_install").onclick = () => switchTab("install");
$("tab_btn_console").onclick = () => switchTab("console");
$("btn_goto_console").onclick = () => switchTab("console");

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

  // 啟動預設畫面：若偵測到沙盒 Node 與 PM2 已經安裝，直接預設打開控制台分頁
  if (isInitial && !initialTabDecided) {
    initialTabDecided = true;
    if (isInstalled) {
      switchTab("console");
    }
  }
}

async function refreshConsole(silent = false) {
  const rootDir = $("root_dir").value.trim();
  if (!silent) {
    setToast("正在同步沙盒 Node / PM2 狀態...");
  }
  try {
    const ov = await invoke("get_console_overview", { rootDir });
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
    const onlineApps = apps.filter((p) => p.status === "online").length;
    const countBadge = $("con_app_count");
    countBadge.textContent = `${onlineApps} / ${apps.length} 系統在線`;
    countBadge.className = "status-pill " + (onlineApps === apps.length && apps.length > 0 ? "online" : "stopped");

    const tbody = $("pm2_apps_tbody");
    if (!ov.installed) {
      tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; padding: 26px; color:#b45309;">
        警告：此目錄尚未安裝沙盒 Node.js 與 PM2，請先切換至「沙盒安裝與部署」分頁執行安裝。
      </td></tr>`;
    } else if (ov.pm2_error && apps.length === 0) {
      tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; padding: 24px; color:#64748b;">
        目前尚無活躍進程或 PM2 守護進程待喚醒（可點擊上方「還原快照」或「啟動服務」喚醒）
      </td></tr>`;
    } else if (apps.length === 0) {
      tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; padding: 24px; color:#64748b;">
        目前沙盒 PM2 尚無受管業務系統（可點擊「還原快照」載入 dump.pm2，或開啟「沙盒終端」啟動系統）
      </td></tr>`;
    } else {
      tbody.innerHTML = apps
        .map((app) => {
          const isOnline = app.status === "online";
          const pillClass = isOnline ? "online" : app.status === "stopped" ? "stopped" : "errored";
          const toggleAction = isOnline ? "stop" : "start";
          const toggleLabel = isOnline ? "停止" : "啟動";
          const safeName = escapeHtml(app.name);
          return `<tr>
            <td class="mono">${app.pm_id}</td>
            <td><strong>${safeName}</strong></td>
            <td><span class="status-pill ${pillClass}">${escapeHtml(app.status)}</span></td>
            <td class="mono">${app.pid || "-"}</td>
            <td class="mono">${app.cpu.toFixed(1)}%</td>
            <td class="mono">${formatBytes(app.memory)}</td>
            <td class="mono">${isOnline ? formatUptime(app.uptime) : "-"}</td>
            <td class="mono">${app.restarts}</td>
            <td style="text-align:right;">
              <div class="btn-group" style="justify-content:flex-end; gap:4px;">
                <button class="btn-sm" data-pm2-action="${toggleAction}" data-pm2-target="${safeName}">${toggleLabel}</button>
                <button class="btn-sm btn-green" data-pm2-action="restart" data-pm2-target="${safeName}"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 9a7 7 0 0112-2l3 5M19 15a7 7 0 01-12 2l-3-5"/></svg> 重啟</button>
                <button class="btn-sm primary" data-pm2-log="${safeName}"><svg class="ui-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 2h8l4 4v16H6zM14 2v5h4M9 11h6M9 15h6"/></svg> 日誌</button>
              </div>
            </td>
          </tr>`;
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
    if (ov.processes && ov.processes.length > 0) {
      if (!currentInlineProc) {
        currentInlineProc = ov.processes.find(p => !p.is_module) || ov.processes[0];
      } else {
        const found = ov.processes.find(p => p.name === currentInlineProc.name);
        if (found) currentInlineProc = found;
      }
      updateInlineLog(currentInlineProc, currentInlineLogType);
    }
  } catch (err) {
    setToast(`同步沙盒狀態失敗：${err}`, "error");
  }
}

// 綁定受管系統表格內的動態按鈕事件
$("pm2_apps_tbody").onclick = async (e) => {
  const row = e.target.closest("tr");
  if (row) {
    document.querySelectorAll("#pm2_apps_tbody tr").forEach(r => r.classList.remove("active-row"));
    row.classList.add("active-row");
  }

  const btn = e.target.closest("button");
  if (!btn) {
    // 點擊非按鈕區域：自動切換下方內嵌日誌
    const targetName = row?.querySelector("[data-pm2-target]")?.getAttribute("data-pm2-target")
                    || row?.querySelector("[data-pm2-log]")?.getAttribute("data-pm2-log");
    if (targetName) {
      const proc = (lastOverview?.processes || []).find((p) => p.name === targetName);
      if (proc) await updateInlineLog(proc, "out");
    }
    return;
  }
  if (isBusy) return;

  const action = btn.getAttribute("data-pm2-action");
  const target = btn.getAttribute("data-pm2-target");
  const logName = btn.getAttribute("data-pm2-log");

  if (action && target) {
    await executePm2Action(action, target, btn);
  } else if (logName) {
    const proc = (lastOverview?.processes || []).find((p) => p.name === logName);
    // 連動內嵌日誌
    if (proc) await updateInlineLog(proc, "out");
    await openLogModal(proc?.out_log_path || "", proc || null);
  }
};

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
  const rootDir = $("root_dir").value.trim();
  const res = await invoke("open_lah_cmd", { rootDir });
  setToast(res.message, res.success ? "success" : "error");
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

$("btn_open_log_modal").onclick = () => openLogModal("", null);

// ==================== 即時日誌預覽彈窗 (Log Tail Modal) ====================
async function openLogModal(preferredPath = "", procContext = null) {
  currentLogProcess = procContext;
  const quickWrap = $("proc_quick_logs");
  if (procContext && (procContext.out_log_path || procContext.err_log_path)) {
    quickWrap.style.display = "inline-flex";
  } else {
    quickWrap.style.display = "none";
  }

  $("modal_logs").classList.add("active");
  $("log_tail_box").textContent = "正在載入日誌清單...";

  const rootDir = $("root_dir").value.trim();
  try {
    const logs = await invoke("get_sandbox_logs", { rootDir });
    const sel = $("sel_log_file");
    if (!logs || logs.length === 0) {
      sel.innerHTML = `<option value="">（目前尚無日誌檔案）</option>`;
      $("log_meta_bar").textContent = "檔案路徑：尚未產生任何 .log 檔案";
      $("log_tail_box").textContent = "目前沙盒 .pm2\\logs 與 logs 目錄下尚無日誌檔案。";
      return;
    }

    sel.innerHTML = logs
      .map(
        (l) =>
          `<option value="${escapeHtml(l.path)}">${escapeHtml(l.label)} (${formatBytes(l.size_bytes)})</option>`
      )
      .join("");

    if (preferredPath) {
      const match = logs.find((l) => l.path.toLowerCase() === preferredPath.toLowerCase());
      if (match) {
        sel.value = match.path;
      }
    }

    await loadSelectedLog();
  } catch (err) {
    $("log_tail_box").textContent = `載入日誌清單失敗：${err}`;
  }
}

async function loadSelectedLog() {
  const path = $("sel_log_file").value;
  if (!path) return;
  const maxLines = Number($("sel_log_lines").value) || 200;
  $("log_tail_box").textContent = "讀取中...";
  try {
    const res = await invoke("get_log_tail", { path, maxLines });
    $("log_meta_bar").textContent = `檔案路徑：${res.path} ｜ 檔案大小：${formatBytes(res.file_size)}`;
    const box = $("log_tail_box");
    box.textContent = (res.lines || []).join("\n");
    box.scrollTop = box.scrollHeight;
  } catch (err) {
    $("log_tail_box").textContent = `讀取日誌失敗：${err}`;
  }
}

$("sel_log_file").onchange = loadSelectedLog;
$("sel_log_lines").onchange = loadSelectedLog;
$("btn_log_reload").onclick = loadSelectedLog;

$("btn_quick_out").onclick = async () => {
  if (currentLogProcess?.out_log_path) {
    $("sel_log_file").value = currentLogProcess.out_log_path;
    await loadSelectedLog();
  }
};
$("btn_quick_err").onclick = async () => {
  if (currentLogProcess?.err_log_path) {
    $("sel_log_file").value = currentLogProcess.err_log_path;
    await loadSelectedLog();
  }
};

$("btn_log_Notepad").onclick = async () => {
  const path = $("sel_log_file").value;
  if (!path) return;
  const res = await invoke("open_log_in_notepad", { path });
  if (!res.success) alert(res.message);
};

$("btn_log_Folder").onclick = async () => {
  const rootDir = $("root_dir").value.trim();
  await invoke("open_logs_folder", { rootDir });
};

$("btn_close_logs").onclick = () => {
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
    $("root_dir").value = selected;
    rescan();
  }
};
$("root_dir").onchange = () => rescan();
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
  await invoke("exit_app");
}
$("btn_exit").onclick = exitApp;
$("btn_close_modal").onclick = exitApp;

$("btn_modal_back").onclick = () => {
  if (isBusy) return;
  $("modal_progress").classList.remove("active");
  $("btn_exit").disabled = false;
  rescan();
};

$("btn_modal_to_console").onclick = () => {
  if (isBusy) return;
  $("modal_progress").classList.remove("active");
  $("btn_exit").disabled = false;
  switchTab("console");
};

$("btn_start").onclick = async () => {
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

initAdminStatus();
rescan(true);

// ==================== 大螢幕工作台模式：內嵌即時日誌串流邏輯 ====================
let currentInlineProc = null;
let currentInlineLogType = "out";

async function updateInlineLog(proc = null, type = null) {
  if (proc) currentInlineProc = proc;
  if (type) currentInlineLogType = type;

  if (!currentInlineProc) {
    const procs = (lastOverview?.processes || []).filter(p => !p.is_module);
    if (procs.length > 0) currentInlineProc = procs[0];
  }

  const nameEl = $("inline_log_proc_name");
  const metaEl = $("inline_log_meta");
  const boxEl = $("inline_log_box");
  if (!nameEl || !boxEl) return;

  if (!currentInlineProc) {
    nameEl.textContent = "尚未受管業務系統在線";
    metaEl.textContent = "檔案路徑：-";
    boxEl.textContent = "目前無受管系統運行，啟動服務後將自動串流最新日誌...";
    return;
  }

  nameEl.textContent = `${currentInlineProc.name} (PID: ${currentInlineProc.pid || "-"})`;

  const btnOut = $("btn_inline_out");
  const btnErr = $("btn_inline_err");
  if (btnOut && btnErr) {
    if (currentInlineLogType === "out") {
      btnOut.style.fontWeight = "bold";
      btnOut.style.border = "2px solid #16a34a";
      btnErr.style.fontWeight = "normal";
      btnErr.style.border = "1px solid #fde68a";
    } else {
      btnErr.style.fontWeight = "bold";
      btnErr.style.border = "2px solid #ea580c";
      btnOut.style.fontWeight = "normal";
      btnOut.style.border = "1px solid #86efac";
    }
  }

  const isOut = currentInlineLogType === "out";
  const path = isOut ? currentInlineProc.out_log_path : currentInlineProc.err_log_path;
  metaEl.textContent = `檔案路徑：${path || "未指定"} | 串流類型：${isOut ? "標準輸出 (stdout)" : "錯誤日誌 (stderr)"}`;

  if (!path) {
    boxEl.textContent = "此進程尚未設定日誌路徑。";
    return;
  }

  const maxLines = parseInt($("sel_inline_lines")?.value || "200", 10);
  try {
    const res = await invoke("get_log_tail", { path, maxLines });
    if (!res.exists) {
      boxEl.textContent = `日誌檔案尚未產生：${path}`;
      return;
    }
    if (!res.lines || res.lines.length === 0) {
      boxEl.textContent = `[日誌目前為空] (${path})`;
      return;
    }
    boxEl.textContent = res.lines.join("\n");
    boxEl.scrollTop = boxEl.scrollHeight;
  } catch (err) {
    boxEl.textContent = `讀取即時日誌失敗：${err}`;
  }
}

if ($("btn_inline_out")) {
  $("btn_inline_out").onclick = () => updateInlineLog(null, "out");
  $("btn_inline_err").onclick = () => updateInlineLog(null, "err");
  $("sel_inline_lines").onchange = () => updateInlineLog(null, null);
  $("btn_inline_reload").onclick = () => updateInlineLog(null, null);

  $("btn_inline_notepad").onclick = async () => {
    if (!currentInlineProc) return;
    const path = currentInlineLogType === "out" ? currentInlineProc.out_log_path : currentInlineProc.err_log_path;
    if (path) {
      const res = await invoke("open_log_in_notepad", { path });
      setToast(res.message, res.success ? "success" : "error");
    }
  };

  $("btn_inline_folder").onclick = async () => {
    const rootDir = $("root_dir").value.trim();
    const res = await invoke("open_logs_folder", { rootDir });
    setToast(res.message, res.success ? "success" : "error");
  };

  $("btn_inline_modal").onclick = () => {
    if (!currentInlineProc) return;
    const path = currentInlineLogType === "out" ? currentInlineProc.out_log_path : currentInlineProc.err_log_path;
    openLogModal(path || "", currentInlineProc);
  };
}
