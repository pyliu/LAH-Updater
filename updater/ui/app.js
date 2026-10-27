const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.plugin?.dialog || window.__TAURI__.dialog;

// 預設 10 大更新包定義 (啟動時立即同步渲染，消除延遲等待)
const DEFAULT_PACKAGES = [
  // 基礎環境 (左欄)
  {
    id: "apache24",
    name: "Apache24.zip",
    title: "Apache 2.4 網頁伺服器 (Apache24.zip)",
    rel_target: "Apache24",
    is_base_env: true,
  },
  {
    id: "php7",
    name: "php7.zip",
    title: "PHP 7.4 執行環境 (php7.zip)",
    rel_target: "php7",
    is_base_env: true,
  },
  {
    id: "php85",
    name: "php85.zip",
    title: "PHP 8.5 執行環境 (php85.zip)",
    rel_target: "php85",
    is_base_env: true,
  },
  // 應用模組 (右欄)
  {
    id: "www",
    name: "www.zip",
    title: "後端 PHP7 更新包 (www.zip)",
    rel_target: "www",
    is_base_env: false,
  },
  {
    id: "lah_api",
    name: "LAH-API.zip",
    title: "後端 PHP8 更新包 (LAH-API.zip)",
    rel_target: "LAH-API",
    is_base_env: false,
  },
  {
    id: "lah_nuxtjs",
    name: "LAH-NUXTJS.zip",
    title: "前端網站 更新包 (LAH-NUXTJS.zip)",
    rel_target: "LAH-NUXTJS",
    is_base_env: false,
  },
  {
    id: "lah_nuxtjs_modules",
    name: "LAH-NUXTJS-node_modules.zip",
    title: "└─ 前端網站 核心套件 (LAH-NUXTJS-node_modules.zip)",
    rel_target: "LAH-NUXTJS\\node_modules",
    is_base_env: false,
  },
  {
    id: "messenger",
    name: "lah-messenger-server.zip",
    title: "即時通訊伺服器 (lah-messenger-server.zip)",
    rel_target: "lah-messenger-server",
    is_base_env: false,
  },
  {
    id: "messenger_modules",
    name: "lah-messenger-server-node_modules.zip",
    title: "└─ 即時通訊 核心套件 (lah-messenger-server-node_modules.zip)",
    rel_target: "lah-messenger-server\\node_modules",
    is_base_env: false,
  },
  {
    id: "node",
    name: "node.zip",
    title: "Node.js 綠色執行環境 (node.zip)",
    rel_target: "node",
    is_base_env: false,
  },
];

let state = {
  packages: DEFAULT_PACKAGES.map((p) => ({
    ...p,
    exists: false,
    checked: true,
    file_path: "",
    file_size: 0,
    target_path: `C:\\AppServ\\${p.rel_target}`,
    target_exists: false,
  })),
  rootDir: "C:\\AppServ",
  backupZip: "",
  isUpdating: false,
  lastSummary: null,
};

function formatSize(bytes) {
  if (!bytes || bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let s = bytes;
  while (s >= 1024 && i < units.length - 1) {
    s /= 1024;
    i++;
  }
  return `${s.toFixed(2)} ${units[i]}`;
}

function appendLog(level, msg) {
  const box = document.getElementById("log_box");
  const div = document.createElement("div");
  div.className = `log-entry ${level}`;
  div.textContent = msg;
  box.appendChild(div);
  box.scrollTop = box.scrollHeight;
}

// 視窗尺寸直接由 tauri.conf.json 精準鎖定 630px 高度 (減去 150px 消除多餘空白)
function autoFitWindow() {
  // no-op
}

// 首次同步渲染初始清單 (0 延遲，無等待感)
function renderInitialCatalog() {
  const baseListEl = document.getElementById("base_pkg_list");
  const appListEl = document.getElementById("app_pkg_list");
  baseListEl.innerHTML = "";
  appListEl.innerHTML = "";

  state.packages.forEach((pkg) => {
    const itemEl = document.createElement("div");
    itemEl.className = "pkg-item";
    itemEl.id = `pkg_card_${pkg.id}`;

    itemEl.innerHTML = `
      <div class="pkg-header">
        <label class="pkg-left checkbox-item" style="padding:0;">
          <input type="checkbox" id="chk_pkg_${pkg.id}" checked>
          <span class="pkg-title">${pkg.title}</span>
        </label>
        <span class="pkg-badge notfound" id="badge_pkg_${pkg.id}">⚪ 偵測中...</span>
      </div>
      <div class="pkg-meta">
        <span id="meta_file_${pkg.id}">檔名：<code>${pkg.name}</code> (掃描中...)</span>
        <span id="meta_target_${pkg.id}" style="margin-left: 10px; color:#94a3b8;">
          ○ 正在檢查目標目錄...
        </span>
      </div>
    `;

    const chk = itemEl.querySelector(`#chk_pkg_${pkg.id}`);
    chk.addEventListener("change", (e) => {
      pkg.checked = e.target.checked;
      updateSummaryText();
    });

    if (pkg.is_base_env) {
      baseListEl.appendChild(itemEl);
    } else {
      appListEl.appendChild(itemEl);
    }
  });

  updateSummaryText();
}

// 背景掃描完成後，原位更新現有 DOM 元素狀態
function updatePackageCards(scannedPkgs) {
  scannedPkgs.forEach((scanned) => {
    const targetState = state.packages.find((p) => p.id === scanned.id);
    if (targetState) {
      Object.assign(targetState, scanned);
    }

    const cardEl = document.getElementById(`pkg_card_${scanned.id}`);
    const badgeEl = document.getElementById(`badge_pkg_${scanned.id}`);
    const metaFileEl = document.getElementById(`meta_file_${scanned.id}`);
    const metaTgtEl = document.getElementById(`meta_target_${scanned.id}`);
    const chkEl = document.getElementById(`chk_pkg_${scanned.id}`);

    if (cardEl && badgeEl && metaFileEl && metaTgtEl && chkEl) {
      if (scanned.exists) {
        cardEl.className = "pkg-item";
        badgeEl.className = "pkg-badge found";
        badgeEl.textContent = "已就緒";
        chkEl.disabled = false;
        chkEl.checked = scanned.checked;
        metaFileEl.innerHTML = `檔名：<code>${scanned.name}</code> (${formatSize(scanned.file_size)})`;
      } else {
        cardEl.className = "pkg-item missing";
        badgeEl.className = "pkg-badge notfound";
        badgeEl.textContent = "未偵測到";
        chkEl.disabled = true;
        chkEl.checked = false;
        metaFileEl.innerHTML = `檔名：<code>${scanned.name}</code> (未找到壓縮包)`;
      }

      if (scanned.target_exists) {
        metaTgtEl.textContent = "✓ 目標目錄已存在";
        metaTgtEl.style.color = "#16a34a";
      } else {
        metaTgtEl.textContent = "○ 目標目錄尚未建立";
        metaTgtEl.style.color = "#94a3b8";
      }
    }
  });

  updateSummaryText();
  autoFitWindow();
}

function updateSummaryText() {
  const selectedCount = state.packages.filter((p) => p.checked && p.exists).length;
  const summaryEl = document.getElementById("status_summary");
  if (selectedCount === 0) {
    summaryEl.textContent = "尚未選取任何已就緒的安裝更新包。";
  } else {
    summaryEl.textContent = `已選取 ${selectedCount} 個系統安裝包準備更新解壓。`;
  }
}

async function init() {
  // 1. 立即同步呈現完整清單與自動自適應尺寸
  renderInitialCatalog();
  autoFitWindow();

  // 2. 權限檢測
  try {
    const isAdmin = await invoke("check_admin");
    const badge = document.getElementById("admin_badge");
    if (isAdmin) {
      badge.textContent = "👑 管理員權限";
      badge.className = "admin-badge is-admin";
    } else {
      badge.textContent = "⚠️ 一般權限 (建議以管理員身分執行)";
      badge.className = "admin-badge not-admin";
    }
  } catch (e) {
    console.error("權限檢測失敗:", e);
  }

  // 3. 註冊即時日誌與進度事件監聽
  await listen("log", (event) => {
    const { level, message } = event.payload;
    appendLog(level, message);
  });

  await listen("progress", (event) => {
    const { percent, message } = event.payload;
    document.getElementById("lbl_progress_status").textContent = message;
    document.getElementById("lbl_progress_percent").textContent = `${percent}%`;
    document.getElementById("bar_progress").style.width = `${percent}%`;
  });

  // 4. 【核心需求】：檢測預設目錄是否存在
  const defaultPath = document.getElementById("txt_root_dir").value.trim() || "C:\\AppServ";
  try {
    const check = await invoke("check_root_dir", { path: defaultPath });
    if (!check.exists) {
      document.getElementById("lbl_missing_dir").textContent = defaultPath;
      document.getElementById("modal_root_missing").classList.add("active");
    }
  } catch (e) {
    console.warn("目錄檢測異常:", e);
  }

  // 5. 背景非同步環境掃描 (完成後原位刷新套件卡片)
  await scanEnvironment();
}

async function scanEnvironment() {
  const rootDir = document.getElementById("txt_root_dir").value.trim() || "C:\\AppServ";
  state.rootDir = rootDir;

  document.getElementById("root_status_badge").textContent = "掃描中...";

  try {
    const res = await invoke("scan_environment", { rootDir });

    // 狀態摘要更新
    const rootBadge = document.getElementById("root_status_badge");
    if (res.root_exists) {
      rootBadge.textContent = "🟢 目錄已就緒";
      rootBadge.style.color = "#16a34a";
    } else {
      rootBadge.textContent = "🔴 目錄尚未建立";
      rootBadge.style.color = "#dc2626";
    }

    // 服務狀態連動
    if (res.apache_service) {
      document.getElementById("chk_restart_apache").checked = res.apache_running;
      document.getElementById("lbl_restart_apache").textContent = `更新後重新啟動 Apache 服務 [${res.apache_service}] (${res.apache_running ? "運行中" : "已停止"})`;
    } else {
      document.getElementById("lbl_restart_apache").textContent = "更新後重新啟動 Apache 服務 (未安裝)";
    }

    if (res.pm2_installed) {
      document.getElementById("chk_restart_pm2").checked = res.pm2_running;
      document.getElementById("lbl_restart_pm2").textContent = `更新後重新啟動 PM2 服務 (${res.pm2_daemon_info})`;
    } else {
      document.getElementById("lbl_restart_pm2").textContent = "更新後重新啟動 PM2 服務 (本機未安裝)";
    }

    // 備份包自動帶入
    if (res.backup_zip_found) {
      document.getElementById("txt_backup_zip").value = res.backup_zip_found;
      state.backupZip = res.backup_zip_found;
    }

    // 原位更新套件狀態
    if (res.packages && res.packages.length > 0) {
      updatePackageCards(res.packages);
    }
  } catch (e) {
    console.error("掃描環境失敗:", e);
    document.getElementById("root_status_badge").textContent = "掃描失敗";
  }
}

// 監聽 DOM 與按鈕事件
document.addEventListener("DOMContentLoaded", () => {
  init();

  // 【新增需求】：立即建立預設目錄按鈕
  document.getElementById("btn_create_root").addEventListener("click", async () => {
    const dir = document.getElementById("txt_root_dir").value.trim() || "C:\\AppServ";
    try {
      await invoke("create_root_dir", { path: dir });
      document.getElementById("modal_root_missing").classList.remove("active");
      await scanEnvironment();
    } catch (e) {
      alert(`建立目錄失敗: ${e}`);
    }
  });

  // 【新增需求】：彈窗內瀏覽其他目錄按鈕
  document.getElementById("btn_modal_browse_root").addEventListener("click", async () => {
    try {
      const selected = await dialog.open({ directory: true, multiple: false });
      if (selected) {
        document.getElementById("txt_root_dir").value = selected;
        document.getElementById("modal_root_missing").classList.remove("active");
        await scanEnvironment();
      }
    } catch (e) {
      console.error(e);
    }
  });

  // 【新增需求】：跳過目錄建立引導
  document.getElementById("btn_modal_skip_root").addEventListener("click", () => {
    document.getElementById("modal_root_missing").classList.remove("active");
  });

  // 主畫面瀏覽根目錄
  document.getElementById("btn_browse_root").addEventListener("click", async () => {
    try {
      const selected = await dialog.open({ directory: true, multiple: false });
      if (selected) {
        document.getElementById("txt_root_dir").value = selected;
        await scanEnvironment();
      }
    } catch (e) {
      console.error(e);
    }
  });

  // 重新整理
  document.getElementById("btn_rescan").addEventListener("click", () => {
    scanEnvironment();
  });

  // 更新說明文件
  document.getElementById("btn_release_notes").addEventListener("click", async () => {
    try {
      const notes = await invoke("read_release_notes");
      if (notes) {
        document.getElementById("release_notes_content").textContent = notes;
      } else {
        document.getElementById("release_notes_content").textContent = "目前未找到更新說明文件 (*.txt)。";
      }
      document.getElementById("modal_release_notes").classList.add("active");
    } catch (e) {
      console.error(e);
    }
  });
  document.getElementById("btn_close_notes").addEventListener("click", () => {
    document.getElementById("modal_release_notes").classList.remove("active");
  });

  // 全選 / 全部取消
  document.getElementById("btn_select_all").addEventListener("click", () => {
    state.packages.forEach((pkg) => {
      if (!pkg.is_base_env && pkg.exists) {
        pkg.checked = true;
        const el = document.getElementById(`chk_pkg_${pkg.id}`);
        if (el) el.checked = true;
      }
    });
    updateSummaryText();
  });
  document.getElementById("btn_deselect_all").addEventListener("click", () => {
    state.packages.forEach((pkg) => {
      if (!pkg.is_base_env) {
        pkg.checked = false;
        const el = document.getElementById(`chk_pkg_${pkg.id}`);
        if (el) el.checked = false;
      }
    });
    updateSummaryText();
  });

  // 備份檔勾選展開
  document.getElementById("chk_restore_backup").addEventListener("change", (e) => {
    document.getElementById("backup_zip_row").style.display = e.target.checked ? "flex" : "none";
    autoFitWindow();
  });
  document.getElementById("btn_browse_backup").addEventListener("click", async () => {
    try {
      const selected = await dialog.open({
        filters: [{ name: "ZIP 備份壓縮檔", extensions: ["zip"] }],
        multiple: false,
      });
      if (selected) {
        document.getElementById("txt_backup_zip").value = selected;
      }
    } catch (e) {
      console.error(e);
    }
  });

  // 關閉離開
  document.getElementById("btn_exit").addEventListener("click", () => {
    invoke("exit_app");
  });
  document.getElementById("btn_close_modal").addEventListener("click", () => {
    invoke("exit_app");
  });
  document.getElementById("btn_modal_back").addEventListener("click", () => {
    document.getElementById("modal_progress").classList.remove("active");
    scanEnvironment();
  });

  // 開始更新作業
  document.getElementById("btn_start_update").addEventListener("click", async () => {
    const selectedPkgs = state.packages.filter((p) => p.checked && p.exists);
    if (selectedPkgs.length === 0) {
      alert("請至少勾選一項已就緒的安裝更新包！");
      return;
    }

    const rootDir = document.getElementById("txt_root_dir").value.trim() || "C:\\AppServ";
    const autoRegisterApache = document.getElementById("chk_auto_apache").checked;
    const autoSecurity = document.getElementById("chk_security_hardening").checked;
    const restartApache = document.getElementById("chk_restart_apache").checked;
    const restartPm2 = document.getElementById("chk_restart_pm2").checked;
    const restoreBackup = document.getElementById("chk_restore_backup").checked;
    const backupZipPath = document.getElementById("txt_backup_zip").value.trim();

    const options = {
      root_dir: rootDir,
      packages: selectedPkgs.map((p) => ({
        id: p.id,
        name: p.name,
        zip_path: p.file_path,
        target_path: p.target_path,
        is_base_env: p.is_base_env,
        is_nodejs: p.is_nodejs,
        is_sub_package: p.is_sub_package,
        expected_folder: p.rel_target.split("\\")[0],
        alt_folders: [],
      })),
      auto_register_apache: autoRegisterApache,
      auto_security_hardening: autoSecurity,
      restart_apache: restartApache,
      restart_pm2: restartPm2,
      restore_backup: restoreBackup,
      backup_zip_path: backupZipPath,
    };

    // 重設日誌與進度視窗
    document.getElementById("log_box").innerHTML = "";
    document.getElementById("bar_progress").style.width = "0%";
    document.getElementById("lbl_progress_percent").textContent = "0%";
    document.getElementById("lbl_progress_status").textContent = "正在準備更新作業...";
    document.getElementById("modal_progress_title").textContent = "系統自動更新進行中...";
    document.getElementById("badge_updating").textContent = "處理中";
    document.getElementById("badge_updating").style.color = "#0284c7";
    document.getElementById("btn_modal_back").style.display = "none";
    document.getElementById("btn_close_modal").style.display = "none";
    document.getElementById("modal_progress").classList.add("active");

    state.isUpdating = true;

    try {
      const summary = await invoke("run_update", { options });
      state.lastSummary = summary;
      state.isUpdating = false;

      if (summary.success) {
        document.getElementById("modal_progress_title").textContent = "🎉 系統更新完成！";
        document.getElementById("badge_updating").textContent = "已完成";
        document.getElementById("badge_updating").style.color = "#16a34a";
        document.getElementById("btn_modal_back").style.display = "inline-block";
        document.getElementById("btn_close_modal").style.display = "inline-block";

        // 若有更新完成的 zip 檔案，彈出清理詢問彈窗
        if (summary.zips_to_clean && summary.zips_to_clean.length > 0) {
          const reportBox = document.getElementById("health_report_box");
          reportBox.innerHTML = `<strong>【系統服務驗證摘要】：</strong><br>${(summary.health_report || []).join("<br>")}`;

          const listEl = document.getElementById("cleanup_zip_list");
          listEl.innerHTML = summary.zips_to_clean.map((z) => `• ${z.split("\\").pop()}`).join("<br>");

          document.getElementById("modal_cleanup").classList.add("active");
        }
      } else {
        document.getElementById("modal_progress_title").textContent = "❌ 更新失敗";
        document.getElementById("badge_updating").textContent = "異常中斷";
        document.getElementById("badge_updating").style.color = "#dc2626";
        document.getElementById("btn_modal_back").style.display = "inline-block";
      }
    } catch (e) {
      state.isUpdating = false;
      document.getElementById("modal_progress_title").textContent = "❌ 更新發生錯誤";
      appendLog("error", `更新過程發生異常錯誤: ${e}`);
      document.getElementById("btn_modal_back").style.display = "inline-block";
    }
  });

  // 清理彈窗按鈕
  document.getElementById("btn_cleanup_keep").addEventListener("click", () => {
    document.getElementById("modal_cleanup").classList.remove("active");
  });

  document.getElementById("btn_cleanup_delete").addEventListener("click", async () => {
    if (state.lastSummary && state.lastSummary.zips_to_clean) {
      try {
        const deletedCount = await invoke("delete_installed_zips", {
          zipPaths: state.lastSummary.zips_to_clean,
        });
        alert(`已成功刪除 ${deletedCount} 個更新壓縮包！`);
      } catch (e) {
        alert(`刪除壓縮包失敗: ${e}`);
      }
    }
    document.getElementById("modal_cleanup").classList.remove("active");
    scanEnvironment();
  });
});
