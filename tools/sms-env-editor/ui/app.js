// 地籍異動簡訊參數設定工具 (SMSWEB_MOI) 前端互動邏輯
const { invoke } = window.__TAURI__.core;

// 狀態快取
let currentConfig = null;
let pendingConfig = null;

// DOM 元素引用
const targetPathInput = document.getElementById("targetPathInput");
const btnBrowse = document.getElementById("btnBrowse");
const btnReload = document.getElementById("btnReload");
const btnOpenFolder = document.getElementById("btnOpenFolder");
const btnFooterReload = document.getElementById("btnFooterReload");
const btnFooterSave = document.getElementById("btnFooterSave");

const alertBanner = document.getElementById("alertBanner");
const alertMessage = document.getElementById("alertMessage");
const btnAlertClose = document.getElementById("btnAlertClose");

const loadStatusText = document.getElementById("loadStatusText");
const backupNoticeText = document.getElementById("backupNoticeText");

// 進階區塊開關
const btnToggleAdvanced = document.getElementById("btnToggleAdvanced");
const advancedContent = document.getElementById("advancedContent");
const advancedArrow = document.getElementById("advancedArrow");

// 差異對照彈窗
const diffModal = document.getElementById("diffModal");
const diffTableBody = document.getElementById("diffTableBody");
const btnModalClose = document.getElementById("btnModalClose");
const btnModalCancel = document.getElementById("btnModalCancel");
const btnModalConfirm = document.getElementById("btnModalConfirm");

// 提示橫幅
function showAlert(message, type = "info") {
  alertBanner.className = `alert-banner show ${type}`;
  alertMessage.textContent = message;
}

function hideAlert() {
  alertBanner.className = "alert-banner";
}

btnAlertClose.addEventListener("click", hideAlert);

// 密碼顯隱切換功能
document.querySelectorAll(".btn-eye").forEach((btn) => {
  btn.addEventListener("click", () => {
    const targetId = btn.getAttribute("data-target");
    const input = document.getElementById(targetId);
    if (!input) return;

    if (input.type === "password") {
      input.type = "text";
      btn.style.color = "#0284c7";
      // 切換為眼睛關閉圖示
      btn.innerHTML = `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 7c2.76 0 5 2.24 5 5 0 .65-.13 1.26-.36 1.83l2.92 2.92c1.51-1.26 2.7-2.89 3.44-4.75-1.73-4.39-6-7.5-11-7.5-1.4 0-2.74.25-3.98.7l2.16 2.16C10.74 7.13 11.35 7 12 7zM2 4.27l2.28 2.28.46.46C3.08 8.3 1.78 10.02 1 12c1.73 4.39 6 7.5 11 7.5 1.55 0 3.03-.3 4.38-.84l.42.42L19.73 22 21 20.73 3.27 3 2 4.27zM7.53 9.8l1.55 1.55c-.05.21-.08.43-.08.65 0 1.66 1.34 3 3 3 .22 0 .44-.03.65-.08l1.55 1.55c-.67.33-1.41.53-2.2.53-2.76 0-5-2.24-5-5 0-.79.2-1.53.53-2.2zm4.31-.78l3.15 3.15.02-.16c0-1.66-1.34-3-3-3l-.17.01z"/></svg>`;
    } else {
      input.type = "password";
      btn.style.color = "#64748b";
      // 切換為眼睛開啟圖示
      btn.innerHTML = `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z"/></svg>`;
    }
  });
});

// 進階參數折疊切換
btnToggleAdvanced.addEventListener("click", () => {
  const isExpanded = advancedContent.classList.toggle("expanded");
  advancedArrow.textContent = isExpanded ? "▲" : "▼";
});

// 收集表單資料
function collectFormData() {
  return {
    // 核心欄位
    ap_url: document.getElementById("inputApUrl").value.trim(),
    sms_user: document.getElementById("inputSmsUser").value.trim(),
    sms_pwd: document.getElementById("inputSmsPwd").value,
    mail_user: document.getElementById("inputMailUser").value.trim(),
    mail_pwd: document.getElementById("inputMailPwd").value,
    mail_sender_name: document.getElementById("inputMailSenderName").value.trim(),
    mail_sender_addr: document.getElementById("inputMailSenderAddr").value.trim(),
    db_ip: document.getElementById("inputDbIp").value.trim(),
    db_port: document.getElementById("inputDbPort").value.trim() || "1521",
    db_sid: document.getElementById("inputDbSid").value.trim(),
    db_user: document.getElementById("inputDbUser").value.trim(),
    db_pwd: document.getElementById("inputDbPwd").value,

    // 進階與排程欄位
    search_time: document.getElementById("inputSearchTime").value.trim() || "1",
    resend_time: document.getElementById("inputResendTime").value.trim() || "3",
    start_hours: document.getElementById("inputStartHours").value.trim() || "08",
    start_min: document.getElementById("inputStartMin").value.trim() || "30",
    end_hours: document.getElementById("inputEndHours").value.trim() || "18",
    end_min: document.getElementById("inputEndMin").value.trim() || "30",

    // 系統底層參數
    odriver_class: document.getElementById("inputOdriverClass").value.trim() || "oracle.jdbc.driver.OracleDriver",
    d_user: currentConfig ? currentConfig.d_user : ": ;%4>",
    d_pwd: currentConfig ? currentConfig.d_pwd : ": ;%4>",
    ap_user: currentConfig ? currentConfig.ap_user : "GM",
    ap_pwd: currentConfig ? currentConfig.ap_pwd : "GM",
    section_header: currentConfig ? currentConfig.section_header : "[LAND]",
    extra_lines: currentConfig ? currentConfig.extra_lines : [],
  };
}

// 填充表單資料
function populateForm(data) {
  currentConfig = data;

  document.getElementById("inputApUrl").value = data.ap_url || "";
  document.getElementById("inputSmsUser").value = data.sms_user || "";
  document.getElementById("inputSmsPwd").value = data.sms_pwd || "";
  document.getElementById("inputMailUser").value = data.mail_user || "";
  document.getElementById("inputMailPwd").value = data.mail_pwd || "";
  document.getElementById("inputMailSenderName").value = data.mail_sender_name || "";
  document.getElementById("inputMailSenderAddr").value = data.mail_sender_addr || "";
  document.getElementById("inputDbIp").value = data.db_ip || "";
  document.getElementById("inputDbPort").value = data.db_port || "1521";
  document.getElementById("inputDbSid").value = data.db_sid || "";
  document.getElementById("inputDbUser").value = data.db_user || "";
  document.getElementById("inputDbPwd").value = data.db_pwd || "";

  document.getElementById("inputSearchTime").value = data.search_time || "1";
  document.getElementById("inputResendTime").value = data.resend_time || "3";
  document.getElementById("inputStartHours").value = data.start_hours || "08";
  document.getElementById("inputStartMin").value = data.start_min || "30";
  document.getElementById("inputEndHours").value = data.end_hours || "18";
  document.getElementById("inputEndMin").value = data.end_min || "30";
  document.getElementById("inputOdriverClass").value = data.odriver_class || "oracle.jdbc.driver.OracleDriver";
}

// 讀取設定檔
async function loadConfig(path) {
  try {
    loadStatusText.textContent = "正在載入設定檔...";
    const data = await invoke("load_config", { path });
    populateForm(data);
    const now = new Date().toLocaleTimeString("zh-TW", { hour12: false });
    loadStatusText.textContent = `載入成功 (最後更新：${now})`;
    showAlert(`成功載入設定檔：${path}`, "success");
  } catch (err) {
    loadStatusText.textContent = "載入失敗";
    showAlert(`讀取失敗：${err}`, "error");
  }
}

// 儲存設定處理
async function handleSaveClick() {
  const path = targetPathInput.value.trim();
  if (!path) {
    showAlert("請先指定目標設定檔路徑！", "error");
    return;
  }

  pendingConfig = collectFormData();

  try {
    const diffs = await invoke("preview_diff", { path, newData: pendingConfig });
    if (!diffs || diffs.length === 0) {
      showAlert("目前設定與目標檔案完全一致，無任何變更需要儲存。", "info");
      return;
    }

    // 渲染差異列表
    diffTableBody.innerHTML = "";
    diffs.forEach((item) => {
      const tr = document.createElement("tr");

      const tdLabel = document.createElement("td");
      tdLabel.textContent = item.label;

      const tdOld = document.createElement("td");
      tdOld.className = "diff-old";
      tdOld.textContent = item.is_secret && item.old_val ? "••••••••" : (item.old_val || "(空)");

      const tdNew = document.createElement("td");
      tdNew.className = "diff-new";
      tdNew.textContent = item.is_secret && item.new_val ? "••••••••" : (item.new_val || "(空)");

      tr.appendChild(tdLabel);
      tr.appendChild(tdOld);
      tr.appendChild(tdNew);
      diffTableBody.appendChild(tr);
    });

    diffModal.classList.add("active");
  } catch (err) {
    showAlert(`比對變更差異時出錯：${err}`, "error");
  }
}

// 確認寫入儲存
async function confirmSave() {
  const path = targetPathInput.value.trim();
  if (!pendingConfig || !path) return;

  btnModalConfirm.disabled = true;
  btnModalConfirm.textContent = "正在寫入...";

  try {
    const res = await invoke("save_config", { path, data: pendingConfig });
    diffModal.classList.remove("active");
    if (res.success) {
      currentConfig = { ...pendingConfig };
      const now = new Date().toLocaleTimeString("zh-TW", { hour12: false });
      loadStatusText.textContent = `儲存成功 (最後寫入：${now})`;
      if (res.backup_path) {
        backupNoticeText.textContent = `(已自動備份)`;
        showAlert(`設定已成功寫入！\n自動備份檔案：${res.backup_path}`, "success");
      } else {
        showAlert(`設定已成功寫入 ${path}！`, "success");
      }
    } else {
      showAlert(`儲存未完成：${res.message}`, "error");
    }
  } catch (err) {
    showAlert(`寫入設定失敗：${err}`, "error");
  } finally {
    btnModalConfirm.disabled = false;
    btnModalConfirm.textContent = "確認並寫入檔案";
  }
}

// 事件綁定
btnBrowse.addEventListener("click", async () => {
  try {
    const selected = await invoke("pick_properties_file");
    if (selected) {
      targetPathInput.value = selected;
      loadConfig(selected);
    }
  } catch (err) {
    showAlert(`開啟檔案選擇視窗失敗：${err}`, "error");
  }
});

btnReload.addEventListener("click", () => {
  const p = targetPathInput.value.trim();
  if (p) loadConfig(p);
});

btnFooterReload.addEventListener("click", () => {
  const p = targetPathInput.value.trim();
  if (p) loadConfig(p);
});

btnOpenFolder.addEventListener("click", async () => {
  const p = targetPathInput.value.trim();
  if (!p) return;
  try {
    await invoke("open_target_folder", { path: p });
  } catch (err) {
    showAlert(`無法開啟目錄：${err}`, "error");
  }
});

btnFooterSave.addEventListener("click", handleSaveClick);
btnModalClose.addEventListener("click", () => diffModal.classList.remove("active"));
btnModalCancel.addEventListener("click", () => diffModal.classList.remove("active"));
btnModalConfirm.addEventListener("click", confirmSave);

// 頁面初始化
window.addEventListener("DOMContentLoaded", async () => {
  try {
    const defaultPath = await invoke("get_default_path");
    if (defaultPath) {
      targetPathInput.value = defaultPath;
      loadConfig(defaultPath);
    }
  } catch (err) {
    loadConfig(targetPathInput.value.trim());
  }
});
