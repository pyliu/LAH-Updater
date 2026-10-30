const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;

// 項目核取定義與文字對齊清單
const CHECKBOX_ITEMS = [
  { id: "chk_services", key: "do_services", label: "• 🛑 停止並解除安裝 Windows 服務 (系統入口閘道服務, 背景服務總線)" },
  { id: "chk_kill_procs", key: "do_kill_procs", label: "• ⚡ 終止守護行程與背景程序 (pm2 kill, 殘留 node.exe / php-cgi / httpd)" },
  { id: "chk_autostart", key: "do_autostart", label: "• 🧹 清除開機自啟動與環境變數 (啟動資料夾, 排程器, PATH, PM2_HOME)" },
  { id: "chk_remove_fe", key: "do_remove_fe", label: "• 🌐 移除智慧前端網站 (LAH-NUXTJS)" },
  { id: "chk_remove_wss", key: "do_remove_wss", label: "• 💬 移除即時通訊伺服器 (lah-messenger-server)" },
  { id: "chk_remove_api", key: "do_remove_api", label: "• 🔌 移除現代化後端 API 模組 (LAH-API)" },
  { id: "chk_remove_www", key: "do_remove_www", label: "• 🏢 移除舊版後端生產庫 (www)" },
  { id: "chk_remove_apache", key: "do_remove_apache", label: "• 🌐 移除 Apache 網頁伺服器本體 (Apache24)" },
  { id: "chk_remove_node", key: "do_remove_node", label: "• 🟩 移除 Node.js 執行期環境 (node, .pm2, lah-cmd.bat)" },
  { id: "chk_remove_php85", key: "do_remove_php85", label: "• 🐘 移除 PHP 8.5 手動安裝目錄 (php85)" },
  { id: "chk_remove_php7", key: "do_remove_php7", label: "• 🐘 移除 PHP 7 手動安裝目錄 (php7)" },
  { id: "chk_remove_instantclient", key: "do_remove_instantclient", label: "• 🏛️ 移除 Oracle Instant Client (instantclient_11_2)" },
  { id: "chk_remove_logs", key: "do_remove_logs", label: "• 📜 移除系統日誌目錄 (logs)" },
  { id: "chk_pm2_dumps", key: "do_pm2_dumps", label: "• 🔄 重置 PM2 狀態快照 (.pm2\\dump.pm2)" },
  { id: "chk_clean_root", key: "do_clean_root", label: "• 🧹 深度清理安裝根目錄其餘殘留檔案 (僅保留備份檔)" }
];

// 初始化檢測管理員權限
async function initAdminStatus() {
  try {
    const isAdmin = await invoke("check_admin");
    const badge = $("admin_badge");
    if (isAdmin) {
      badge.textContent = "👑 管理員權限";
      badge.className = "admin-badge is-admin";
    } else {
      badge.textContent = "⚠️ 一般使用者權限";
      badge.className = "admin-badge not-admin";
    }
  } catch (e) {
    console.error("Failed to check admin status:", e);
  }
}

// 動態探測並填入預設安裝根目錄
async function initRootDir() {
  try {
    const detected = await invoke("detect_root_dir");
    if (detected && $("root_dir")) {
      $("root_dir").value = detected;
    }
  } catch (e) {
    console.error("Failed to detect root dir:", e);
  }
}

// 快速全選/取消
function setAllCheckboxes(state) {
  for (const item of CHECKBOX_ITEMS) {
    const el = $(item.id);
    if (el) el.checked = state;
  }
}

$("btn_all").onclick = () => setAllCheckboxes(true);
$("btn_none").onclick = () => setAllCheckboxes(false);

// 目錄瀏覽選擇
$("browse_btn").onclick = async () => {
  try {
    const selected = await open({
      directory: true,
      defaultPath: $("root_dir").value.trim() || "C:\\AppServ"
    });
    if (selected) {
      $("root_dir").value = selected;
    }
  } catch (e) {
    console.error("Browse directory failed:", e);
  }
};

// 日誌輸出
function appendLog(message, level = "info") {
  const box = $("log_box");
  const now = new Date();
  const timeStr = `[${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}] `;

  const line = document.createElement("div");
  line.className = `log-line ${level}`;

  const timeSpan = document.createElement("span");
  timeSpan.className = "log-line time";
  timeSpan.textContent = timeStr;

  const msgSpan = document.createElement("span");
  msgSpan.textContent = message;

  line.appendChild(timeSpan);
  line.appendChild(msgSpan);
  box.appendChild(line);
  box.scrollTop = box.scrollHeight;
}

// 監聽後端進度與日誌事件
listen("progress", ({ payload }) => {
  const pct = Math.round(payload.percent);
  const bar = $("progress_bar");
  if (bar) bar.value = payload.percent;
  const pctEl = $("progress_percent") || $("percent_text");
  if (pctEl) pctEl.textContent = `${pct}%`;
  const statusEl = $("status_text");
  if (statusEl) statusEl.textContent = payload.message || "處理中...";
});

listen("log", ({ payload }) => {
  appendLog(payload.message, payload.level);
});

// 收集表單選項
function getOptions() {
  const rootDir = $("root_dir").value.trim();
  const opts = { root_dir: rootDir };
  for (const item of CHECKBOX_ITEMS) {
    opts[item.key] = $(item.id).checked;
  }
  return opts;
}

// 退出應用程式
async function exitApp() {
  try {
    await invoke("exit_app");
  } catch (e) {
    window.close();
  }
}

// 二次確認狀態暫存
let pendingUninstallOptions = null;

function closeConfirmModal() {
  const modal = $("modal_confirm");
  if (modal) modal.classList.remove("active");
  pendingUninstallOptions = null;
}

// 開始移除按鈕：開啟 UI 內置二次確認視窗 (完全不呼叫原生 confirm 以免非同步 Promise 提前穿透)
$("btn_start").onclick = () => {
  const opts = getOptions();
  if (!opts.root_dir) {
    alert("請先指定系統安裝根目錄！");
    $("root_dir").focus();
    return;
  }

  // 計算已勾選項目數量
  const selectedCount = CHECKBOX_ITEMS.filter((item) => opts[item.key]).length;
  if (selectedCount === 0) {
    alert("請至少選取一個欲移除的項目！");
    return;
  }

  pendingUninstallOptions = opts;

  // 填寫確認彈窗資訊
  if ($("confirm_target_dir")) {
    $("confirm_target_dir").textContent = opts.root_dir;
  }
  const listEl = $("confirm_items_list");
  if (listEl) {
    listEl.innerHTML = "";
    for (const item of CHECKBOX_ITEMS) {
      if (opts[item.key]) {
        const row = document.createElement("div");
        row.textContent = item.label;
        listEl.appendChild(row);
      }
    }
  }

  const modal = $("modal_confirm");
  if (modal) modal.classList.add("active");
};

// 取消確認視窗
$("btn_confirm_cancel").onclick = closeConfirmModal;

// 點擊確認彈窗遮罩背景取消
$("modal_confirm").onclick = (e) => {
  if (e.target === $("modal_confirm")) closeConfirmModal();
};

// 鍵盤 Esc 關閉確認視窗
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    if ($("modal_confirm") && $("modal_confirm").classList.contains("active")) {
      closeConfirmModal();
    }
  }
});

// 使用者於二次確認視窗明確點擊「確認並立即執行移除」按鈕後，才真正啟動移除流程！
$("btn_confirm_ok").onclick = async () => {
  if (!pendingUninstallOptions || isBusy) return;
  const opts = pendingUninstallOptions;
  closeConfirmModal();

  isBusy = true;
  $("btn_start").disabled = true;
  $("btn_exit").disabled = true;

  try {
    const btnClose = $("btn_close_modal");
    if (btnClose) {
      btnClose.disabled = true;
      btnClose.textContent = "移除進行中...";
    }
    const btnBack = $("btn_modal_back");
    if (btnBack) btnBack.style.display = "none";
    const logBox = $("log_box");
    if (logBox) logBox.innerHTML = "";
    const pBar = $("progress_bar");
    if (pBar) pBar.value = 0;
    const pctEl = $("progress_percent") || $("percent_text");
    if (pctEl) pctEl.textContent = "0%";
    const statusEl = $("status_text");
    if (statusEl) statusEl.textContent = "正在準備啟動移除工作流程...";

    const modal = $("modal_progress");
    if (modal) modal.classList.add("active");

    const res = await invoke("run_uninstall", { options: opts });
    if (res.success) {
      appendLog("\n==================================================", "success");
      appendLog("🎉 智慧控管與監控系統 已成功解除安裝！", "success");
      if (res.backup_path) {
        appendLog(`📦 刪除前備份壓縮檔存放位置: ${res.backup_path}`, "success");
      }
      appendLog("==================================================", "success");
      if (statusEl) statusEl.textContent = "移除作業已圓滿完成！";
      if (btnClose) {
        btnClose.disabled = false;
        btnClose.textContent = "完成並結束";
      }
    } else {
      if (statusEl) statusEl.textContent = `⚠️ 移除過程發生錯誤：${res.message}`;
      if (btnBack) btnBack.style.display = "inline-block";
      if (btnClose) {
        btnClose.disabled = false;
        btnClose.textContent = "關閉離開";
      }
    }
  } catch (err) {
    appendLog(`❌ 執行移除作業發生嚴重異常: ${err}`, "error");
    const statusEl = $("status_text");
    if (statusEl) statusEl.textContent = "移除過程發生異常！";
    const btnBack = $("btn_modal_back");
    if (btnBack) btnBack.style.display = "inline-block";
    const btnClose = $("btn_close_modal");
    if (btnClose) {
      btnClose.disabled = false;
      btnClose.textContent = "關閉離開";
    }
    const modal = $("modal_progress");
    if (!modal || !modal.classList.contains("active")) {
      alert(`移除作業發生異常: ${err}`);
      $("btn_start").disabled = false;
      $("btn_exit").disabled = false;
    }
  } finally {
    isBusy = false;
  }
};

// 關閉進度視窗 (完成並結束程式)
$("btn_close_modal").onclick = exitApp;

// 返回主畫面按鈕 (關閉日誌浮層，保留主介面)
$("btn_modal_back").onclick = () => {
  if (isBusy) return;
  $("modal_progress").classList.remove("active");
  $("btn_start").disabled = false;
  $("btn_exit").disabled = false;
};

// 主畫面退出按鈕
$("btn_exit").onclick = exitApp;


// 動態自適應視窗高度：依據文檔流真實累積高度精準微調客戶區高度，徹底消除垂直捲軸
let fitTimer = null;
function autoFitWindow() {
  if (fitTimer) cancelAnimationFrame(fitTimer);
  fitTimer = requestAnimationFrame(() => {
    const footer = document.querySelector(".footer-bar");
    if (!footer) return;

    // 計算文檔流到達 footer 底部所需之客戶區完整高度 (+4px 裕度防護浮點進位誤差)
    const requiredHeight = Math.ceil(footer.offsetTop + footer.offsetHeight + 12 + 4);

    invoke("fit_window_to_content", { clientHeight: requiredHeight }).catch((err) => {
      console.warn("自適應視窗高度呼叫失敗:", err);
    });
  });
}

// 啟動檢測
initAdminStatus();
initRootDir();
autoFitWindow();
