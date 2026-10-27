const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let isBusy = false;

function openModal(title) {
  $("modalTitle").textContent = title || "執行進度與訊息";
  $("modalOverlay").classList.add("active");
}

function closeModal() {
  if (isBusy) return;
  $("modalOverlay").classList.remove("active");
}

function log(msg) {
  const el = $("log");
  el.textContent += msg + "\n";
  el.scrollTop = el.scrollHeight;
}

listen("progress", ({ payload }) => {
  const pct = Math.round(payload.percent);
  $("bar").value = payload.percent;
  $("percentText").textContent = `${pct}%`;
  $("statusText").textContent = payload.message || "處理中...";
  log(`[${pct}%] ${payload.message}`);
});

async function pick(inputId) {
  const dir = await open({ directory: true, defaultPath: $(inputId).value });
  if (dir) $(inputId).value = dir;
}

$("pickRoot").onclick = () => pick("root");
$("pickDest").onclick = () => pick("dest");
$("pickZip").onclick = async () => {
  const f = await open({ filters: [{ name: "ZIP 備份檔", extensions: ["zip"] }] });
  if (f) $("zip").value = f;
};

$("showLogBtn").onclick = () => openModal("執行歷程日誌");
$("closeModalBtn").onclick = closeModal;

// 點擊遮罩背景也可關閉（非繁忙時）
$("modalOverlay").onclick = (e) => {
  if (e.target === $("modalOverlay")) closeModal();
};

// 鍵盤 Esc 關閉彈窗
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") closeModal();
});

// 立即備份
$("run").onclick = async () => {
  const root = $("root").value.trim();
  const dest = $("dest").value.trim();
  if (!root) return alert("請輸入系統安裝根目錄！");
  if (!dest) return alert("請輸入備份存放目的地！");

  isBusy = true;
  $("run").disabled = $("restore").disabled = true;
  $("closeModalBtn").disabled = true;
  $("closeModalBtn").textContent = "備份進行中...";
  $("log").textContent = "";
  $("bar").value = 0;
  $("percentText").textContent = "0%";
  $("statusText").textContent = "正在準備備份系統重要資料...";

  openModal("📦 系統重要資料備份進行中");

  try {
    const r = await invoke("run_backup", { root, dest });
    if (r.zip_path) {
      log("\n==================================================");
      log(`🎉 備份完成！`);
      log(`📦 儲存檔案：${r.zip_path}`);
      log(`📄 備份檔案數：${r.files} 檔 (略過 ${r.skipped} 檔)`);
      log(`💾 壓縮包大小：${(r.size_bytes / 1048576).toFixed(2)} MB`);
      log("==================================================");
      $("statusText").textContent = "備份已順利完成！";
    } else {
      log("\nℹ️ 未在目標根目錄偵測到需備份的現存資料，略過建立備份檔。");
      $("statusText").textContent = "未偵測到可備份資料";
    }
  } catch (e) {
    log(`\n❌ 備份失敗：${e}`);
    $("statusText").textContent = "備份過程發生錯誤！";
  } finally {
    isBusy = false;
    $("run").disabled = $("restore").disabled = false;
    $("closeModalBtn").disabled = false;
    $("closeModalBtn").textContent = "完成並關閉視窗";
  }
};

// 從備份還原
$("restore").onclick = async () => {
  const root = $("root").value.trim();
  const zip = $("zip").value.trim();
  if (!root) return alert("請輸入系統安裝根目錄！");
  if (!zip) return alert("請先選擇欲還原的備份 ZIP 壓縮檔案！");

  if (!confirm(`⚠️ 【重要警告：即將執行系統還原】\n\n備份來源：\n${zip}\n\n還原目標：\n${root}\n\n• 還原過程同名檔案將被直接覆蓋！\n• 系統將在覆蓋前自動建立 restore_undo_*.zip 復原點。\n\n您確定要繼續執行還原嗎？`)) {
    return;
  }

  isBusy = true;
  $("run").disabled = $("restore").disabled = true;
  $("closeModalBtn").disabled = true;
  $("closeModalBtn").textContent = "還原進行中...";
  $("log").textContent = "";
  $("bar").value = 0;
  $("percentText").textContent = "0%";
  $("statusText").textContent = "正在檢查服務並準備還原...";

  openModal("🚨 系統重要資料還原進行中");

  try {
    const r = await invoke("run_restore", { root, zip });
    log("\n==================================================");
    log(`🎉 還原完成！`);
    log(`📄 成功還原：${r.restored} 個檔案`);
    if (r.undo_zip) {
      log(`🛡️ 覆蓋前自動復原點存檔 (已備份 ${r.undo_files} 檔)：\n  -> ${r.undo_zip}`);
    } else {
      log(`🛡️ 無需覆蓋現存檔案，未建立復原點。`);
    }
    log("==================================================");
    $("statusText").textContent = "還原已順利完成！";
  } catch (e) {
    log(`\n❌ 還原失敗：${e}`);
    $("statusText").textContent = "還原過程發生錯誤！";
  } finally {
    isBusy = false;
    $("run").disabled = $("restore").disabled = false;
    $("closeModalBtn").disabled = false;
    $("closeModalBtn").textContent = "完成並關閉視窗";
  }
};
