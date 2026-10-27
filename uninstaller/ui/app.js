const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;

// 項目核取定義與文字對齊清單
const CHECKBOX_ITEMS = [
  { id: "chk_services", key: "do_services", label: "• 🛑 停止並解除安裝 Windows 服務 (LAH_PM2_Service, Apache2.4)" },
  { id: "chk_kill_procs", key: "do_kill_procs", label: "• ⚡ 終止守護行程與背景程序 (pm2 kill, 殘留 node.exe / php-cgi)" },
  { id: "chk_autostart", key: "do_autostart", label: "• 🧹 清除開機自啟動與環境變數 (啟動資料夾, 排程器, PATH)" },
  { id: "chk_remove_fe", key: "do_remove_fe", label: "• 🌐 移除智慧前端網站 (LAH-NUXTJS)" },
  { id: "chk_remove_wss", key: "do_remove_wss", label: "• 💬 移除即時通訊伺服器 (lah-messenger-server)" },
  { id: "chk_remove_api", key: "do_remove_api", label: "• 🔌 移除現代化後端 API 模組 (LAH-API)" },
  { id: "chk_remove_www", key: "do_remove_www", label: "• 🏢 移除舊版後端生產庫 (www)" },
  { id: "chk_remove_apache", key: "do_remove_apache", label: "• 🌐 移除 Apache 網頁伺服器本體 (Apache24)" },
  { id: "chk_remove_node", key: "do_remove_node", label: "• 🟩 移除 Node.js 執行期環境 (node)" },
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

// 事件監聽：progress & log
listen("progress", ({ payload }) => {
  const pct = Math.max(0, Math.min(100, Number(payload.percent) || 0));
  $("progress_bar").value = pct;
  $("percent_text").textContent = `${pct.toFixed(1)}%`;
  if (payload.message) {
    $("status_text").textContent = payload.message;
  }
});

listen("log", ({ payload }) => {
  appendLog(payload.message, payload.level || "info");
});

// 確認視窗操作
function openConfirmModal() {
  const rootDir = $("root_dir").value.trim();
  if (!rootDir) {
    alert("請輸入系統安裝根目錄！");
    return;
  }

  const selectedItems = [];
  for (const item of CHECKBOX_ITEMS) {
    const el = $(item.id);
    if (el && el.checked) {
      selectedItems.push(item.label);
    }
  }

  if (selectedItems.length === 0) {
    alert("您尚未選取任何欲移除的項目！");
    return;
  }

  $("confirm_target_dir").textContent = rootDir;
  const listEl = $("confirm_items_list");
  listEl.innerHTML = selectedItems.map(s => `<div>${s}</div>`).join("");
  $("modal_confirm").classList.add("active");
}

function closeConfirmModal() {
  $("modal_confirm").classList.remove("active");
}

$("btn_confirm_cancel").onclick = closeConfirmModal;
$("btn_start").onclick = openConfirmModal;

// 退出應用程式 (調用 Rust 原生 exit_app 以避免 WebView2 window.close 白畫面問題)
async function exitApp() {
  if (isBusy) return;
  try {
    await invoke("exit_app");
  } catch (err) {
    console.error("Failed to exit app:", err);
  }
}

// 執行移除
$("btn_confirm_ok").onclick = async () => {
  closeConfirmModal();

  const rootDir = $("root_dir").value.trim();
  const options = { root_dir: rootDir };
  for (const item of CHECKBOX_ITEMS) {
    const el = $(item.id);
    options[item.key] = el ? el.checked : false;
  }

  isBusy = true;
  $("btn_start").disabled = true;
  $("btn_exit").disabled = true;
  $("btn_modal_back").style.display = "none";
  $("btn_close_modal").disabled = true;
  $("btn_close_modal").textContent = "移除進行中...";
  $("log_box").innerHTML = "";
  $("progress_bar").value = 0;
  $("percent_text").textContent = "0.0%";
  $("status_text").textContent = "正在準備移除作業...";
  $("modal_progress").classList.add("active");

  try {
    const res = await invoke("run_uninstall", { options });
    if (res.success) {
      $("status_text").textContent = `✅ 全部選取項目已成功移除！(耗時 ${res.elapsed_secs.toFixed(1)} 秒)`;
      $("btn_modal_back").style.display = "inline-block";
      $("btn_close_modal").disabled = false;
      $("btn_close_modal").textContent = "完成並關閉程式";
    } else {
      $("status_text").textContent = `⚠️ 移除過程發生錯誤：${res.message}`;
      $("btn_modal_back").style.display = "inline-block";
      $("btn_close_modal").disabled = false;
      $("btn_close_modal").textContent = "關閉離開";
    }
  } catch (err) {
    appendLog(`❌ 執行移除作業發生嚴重異常: ${err}`, "error");
    $("status_text").textContent = "移除過程發生異常！";
    $("btn_modal_back").style.display = "inline-block";
    $("btn_close_modal").disabled = false;
    $("btn_close_modal").textContent = "關閉離開";
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

// 啟動檢測
initAdminStatus();
