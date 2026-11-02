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
  const curInput = $("root_dir").value.trim();
  const s = await invoke("scan_environment", { rootDir: curInput });
  if (!curInput && s.root_dir) {
    $("root_dir").value = s.root_dir;
  }
  let lrText = "尚未安裝（安裝時將由 node.zip 便攜版內建啟動）";
  if (s.logrotate_version) {
    lrText = s.logrotate_configured
      ? `${s.logrotate_version}（便攜版內建，已掛載啟用）`
      : `${s.logrotate_version}（便攜版內建，點擊開始安裝即可啟動）`;
  } else if (s.logrotate_configured) {
    lrText = "已掛載啟用";
  }
  $("scan_box").innerHTML = [
    row("node.zip 安裝包", s.zip_found || "未找到（請放於工具旁 base 資料夾、工具同層或 C:\\Temp）", !!s.zip_found),
    row("已安裝 Node.js", s.node_version || "尚未安裝", !!s.node_version),
    row("已安裝 PM2", s.pm2_version || "尚未安裝", !!s.pm2_version),
    row("pm2-logrotate 模組", lrText, s.logrotate_installed),
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

$("btn_audit").onclick = async () => {
  if (isBusy) return;
  const rootDir = $("root_dir").value.trim();
  $("btn_audit").disabled = true;
  $("btn_audit").textContent = "檢測中...";

  try {
    const report = await invoke("run_security_audit", { rootDir });
    const badge = $("audit_badge_box");
    if (report.overall_secure) {
      badge.style.background = "#dcfce7";
      badge.style.color = "#15803d";
      badge.style.border = "1px solid #bbf7d0";
      badge.textContent = "🛡️ 資安自檢評定：【合格安全】（已達成零全域污染沙盒隔離，受管服務無執行期標頭洩漏）";
    } else {
      badge.style.background = "#fee2e2";
      badge.style.color = "#b91c1c";
      badge.style.border = "1px solid #fecaca";
      badge.textContent = "⚠️ 資安自檢評定：【發現潛在風險】（請檢視下方標頭外洩項目或清理 PATH 登錄檔）";
    }

    let detail = `檢測時間：${report.timestamp}\n安裝目錄：${report.root_dir}\n`;
    detail += `全域 PATH 零污染沙盒：${report.sandbox_path_clean ? "✅ 合規 (HKLM/HKCU 無殘留)" : "⚠️ 未合規 (登錄檔存在可攜路徑)"}\n\n`;
    detail += "受管服務網路通訊邊界檢測：\n";
    for (const p of report.port_results) {
      detail += `• Port ${p.port} (${p.service_name})：\n`;
      detail += `  - 監聽狀態：${p.is_listening ? "監聽中" : "未啟動/未監聽"}\n`;
      detail += `  - 判定結果：${p.is_secure ? "✅ 合格 (無敏感技術棧指紋)" : "⚠️ 需改善"}\n`;
      if (p.leaked_headers && p.leaked_headers.length > 0) {
        detail += `  - 洩漏標頭：${p.leaked_headers.join(", ")}\n`;
      }
    }
    if (report.report_file_path) {
      detail += `\n📄 完整佐證報告已匯出：\n${report.report_file_path}\n`;
    }
    $("audit_detail_box").textContent = detail;
    $("modal_audit").classList.add("active");
  } catch (err) {
    alert(`執行資安自檢失敗: ${err}`);
  } finally {
    $("btn_audit").disabled = false;
    $("btn_audit").textContent = "🛡️ 資安自檢";
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
