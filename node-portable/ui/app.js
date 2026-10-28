const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;

async function initAdminStatus() {
  const badge = $("admin_badge");
  try {
    const isAdmin = await invoke("check_admin");
    badge.textContent = isAdmin ? "👑 管理員權限" : "⚠️ 一般使用者權限（註冊服務需管理員）";
    badge.className = "admin-badge " + (isAdmin ? "is-admin" : "not-admin");
  } catch (e) {
    console.error("check_admin failed:", e);
  }
}

function row(label, value, ok) {
  const mark = ok === undefined ? "" : ok ? "✅ " : "⚪ ";
  return `<div>${mark}<strong>${label}</strong>：${value}</div>`;
}

async function rescan() {
  const s = await invoke("scan_environment", { rootDir: $("root_dir").value.trim() });
  $("scan_box").innerHTML = [
    row("node.zip 安裝包", s.zip_found || "未找到（請放於工具旁 base 資料夾、工具同層或 C:\\Temp）", !!s.zip_found),
    row("已安裝 Node.js", s.node_version || "尚未安裝", !!s.node_version),
    row("已安裝 PM2", s.pm2_version || "尚未安裝", !!s.pm2_version),
    row("PM2_Portable_Service", s.service_installed ? (s.service_running ? "已註冊，執行中" : "已註冊，未執行") : "未註冊", s.service_installed),
  ].join("");
  $("btn_start").disabled = !s.zip_found;
}

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
$("root_dir").onchange = rescan;
$("btn_rescan").onclick = rescan;

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

$("btn_start").onclick = async () => {
  const rootDir = $("root_dir").value.trim();
  if (!rootDir) {
    alert("請輸入安裝根目錄！");
    return;
  }
  const options = {
    root_dir: rootDir,
    register_path: $("chk_path").checked,
    set_pm2_home: $("chk_home").checked,
    register_service: $("chk_service").checked,
    resurrect: $("chk_resurrect").checked,
  };

  isBusy = true;
  $("btn_start").disabled = true;
  $("btn_exit").disabled = true;
  $("btn_modal_back").style.display = "none";
  $("btn_close_modal").disabled = true;
  $("log_box").innerHTML = "";
  $("progress_bar").value = 0;
  $("percent_text").textContent = "0.0%";
  $("status_text").textContent = "準備中...";
  $("modal_progress").classList.add("active");

  try {
    const res = await invoke("run_install", { options });
    $("status_text").textContent = res.success
      ? `✅ ${res.message}（耗時 ${res.elapsed_secs.toFixed(1)} 秒）`
      : `⚠️ 安裝失敗：${res.message}`;
  } catch (err) {
    appendLog(`❌ 安裝發生嚴重異常: ${err}`, "error");
    $("status_text").textContent = "安裝過程發生異常！";
  } finally {
    isBusy = false;
    $("btn_modal_back").style.display = "inline-block";
    $("btn_close_modal").disabled = false;
    $("btn_close_modal").textContent = "關閉離開";
  }
};

initAdminStatus();
rescan();
