use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 跨目錄還原時之檔案分類策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationCategory {
    /// 類別 A：不可直接移植之執行期狀態快照（優先結構化重定位並剔除幽靈行程，若異常則安全旁路清空並由 ecosystem 重建）
    EphemeralSnapshot,
    /// 類別 B：必須進行多重路徑格式正規化置換之設定檔與腳本
    TextConfig,
}

/// 路徑分隔符正規化偏好
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashStyle {
    /// 保留原格式（原生 `\` 轉 `\`、JSON `\\` 轉 `\\`、正斜線 `/` 轉 `/`）
    PreserveFormat,
    /// 強制統一為 Unix 正斜線 `/`（適用於 Apache httpd.conf 與 PHP php.ini）
    ForceForwardSlash,
}

/// 領域模型：定義單一受納管之重定位目標規格
#[derive(Debug, Clone, Copy)]
pub struct RelocationTargetSpec {
    pub rel_path: &'static str,
    pub category: RelocationCategory,
    pub slash_style: SlashStyle,
    pub description: &'static str,
}

/// 跨目錄還原時受納管之關鍵目標註冊表
pub const RELOCATION_TARGETS: &[RelocationTargetSpec] = &[
    RelocationTargetSpec {
        rel_path: ".pm2/dump.pm2",
        category: RelocationCategory::EphemeralSnapshot,
        slash_style: SlashStyle::PreserveFormat,
        description: "PM2 行程持久化狀態快照",
    },
    RelocationTargetSpec {
        rel_path: ".pm2/modules/pm2-logrotate/node_modules/pm2-logrotate/app.js",
        category: RelocationCategory::EphemeralSnapshot,
        slash_style: SlashStyle::PreserveFormat,
        description: "PM2 Logrotate 模組載入入口橋接檔",
    },
    RelocationTargetSpec {
        rel_path: ".pm2/module_conf.json",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "PM2 模組全域設定檔",
    },
    RelocationTargetSpec {
        rel_path: "Apache24/conf/httpd.conf",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::ForceForwardSlash,
        description: "Apache 2.4 主設定檔",
    },
    RelocationTargetSpec {
        rel_path: "php7/php.ini",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::ForceForwardSlash,
        description: "PHP 7.4 主設定檔",
    },
    RelocationTargetSpec {
        rel_path: "php85/php.ini",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::ForceForwardSlash,
        description: "PHP 8.5 FastCGI 主設定檔",
    },
    RelocationTargetSpec {
        rel_path: "www/.env",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "舊版後端 www 環境設定檔",
    },
    RelocationTargetSpec {
        rel_path: "LAH-API/.env",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "新版後端 LAH-API 環境設定檔",
    },
    RelocationTargetSpec {
        rel_path: "LAH-NUXTJS/.env",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "前端網站 LAH-NUXTJS 環境設定檔",
    },
    RelocationTargetSpec {
        rel_path: "lah-messenger-server/.env",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "即時通伺服器環境設定檔",
    },
    RelocationTargetSpec {
        rel_path: "LAH-API/ecosystem.config.js",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "LAH-API PM2 啟動設定檔",
    },
    RelocationTargetSpec {
        rel_path: "LAH-NUXTJS/ecosystem.config.js",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "LAH-NUXTJS PM2 啟動設定檔",
    },
    RelocationTargetSpec {
        rel_path: "lah-messenger-server/ecosystem.config.js",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "lah-messenger-server PM2 啟動設定檔",
    },
    RelocationTargetSpec {
        rel_path: "lah-cmd.bat",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "沙盒環境命令列啟動批次檔",
    },
    RelocationTargetSpec {
        rel_path: "node/pm2-service.xml",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "WinSW PM2 服務註冊設定檔 (pm2-service.xml)",
    },
    RelocationTargetSpec {
        rel_path: "node/winsw.xml",
        category: RelocationCategory::TextConfig,
        slash_style: SlashStyle::PreserveFormat,
        description: "WinSW 服務註冊設定檔 (winsw.xml)",
    },
];

/// 殘留路徑檢查單筆紀錄
#[derive(Debug, Clone)]
pub struct StalePathFinding {
    pub rel_path: String,
    pub line_number: usize,
    pub matched_legacy_root: String,
    pub line_preview: String,
}

/// 路徑重定位與啟動前驗證總結報告
#[derive(Debug, Clone, Default)]
pub struct RelocationReport {
    pub detected_legacy_roots: Vec<String>,
    pub relocated_files: Vec<String>,
    pub bypassed_snapshots: Vec<String>,
    pub remaining_findings: Vec<StalePathFinding>,
    pub verified_clean: bool,
}

/// 正規化根目錄字串為 Windows 原生單反斜線格式（不含結尾斜線，例如 `C:\AppServ`）
pub fn normalize_root_str(path_str: &str) -> String {
    let trimmed = path_str.trim().trim_matches('"').trim_matches('\'');
    let mut s = trimmed.replace("\\\\", "\\").replace('/', "\\");
    while s.ends_with('\\') && s.len() > 3 {
        s.pop();
    }
    s
}

/// 從目標目錄內已解壓或還原之設定檔與快照中，自動探測歷史安裝根目錄
pub fn detect_legacy_roots(root_dir: &Path) -> Vec<String> {
    let new_root_norm = normalize_root_str(&root_dir.to_string_lossy());
    let mut candidates = Vec::new();

    // 預設納入標準歷史根目錄候選
    candidates.push(r"C:\AppServ".to_string());
    candidates.push(r"D:\AppServ".to_string());

    // 1. 從 Apache24/conf/httpd.conf 探測 SRVROOT / DocumentRoot / PHPIniDir
    let httpd_conf = root_dir.join("Apache24").join("conf").join("httpd.conf");
    if let Ok(content) = fs::read_to_string(&httpd_conf) {
        let patterns = [
            r#"(?im)^\s*Define\s+SRVROOT\s+"([^"]+)[/\\]Apache24/?""#,
            r#"(?im)^\s*DocumentRoot\s+"([^"]+)[/\\]www/?""#,
            r#"(?im)^\s*PHPIniDir\s+"([^"]+)[/\\]php7/?""#,
            r#"(?im)^\s*Alias\s+/LAH-API\s+"([^"]+)[/\\]LAH-API/?""#,
        ];
        for pat in patterns {
            if let Ok(re) = Regex::new(pat) {
                for cap in re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        candidates.push(normalize_root_str(m.as_str()));
                    }
                }
            }
        }
    }

    // 2. 從 php7/php.ini 與 php85/php.ini 探測 extension_dir
    for sub in ["php7", "php85"] {
        let ini_p = root_dir.join(sub).join("php.ini");
        if let Ok(content) = fs::read_to_string(&ini_p) {
            let pat = format!(
                r#"(?im)^\s*extension_dir\s*=\s*"([^"]+)(?:[/\\]+){}(?:[/\\]+)ext/?""#,
                sub
            );
            if let Ok(re) = Regex::new(&pat) {
                for cap in re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        candidates.push(normalize_root_str(m.as_str()));
                    }
                }
            }
        }
    }

    // 3. 從 .pm2/dump.pm2 探測 PM2_HOME 與 pm_cwd
    let dump_p = root_dir.join(".pm2").join("dump.pm2");
    if let Ok(content) = fs::read_to_string(&dump_p) {
        let patterns = [
            r#""PM2_HOME"\s*:\s*"([^"]+)(?:\\\\|[/\\])\.pm2""#,
            r#""pm_cwd"\s*:\s*"([^"]+)(?:\\\\|[/\\])(?:LAH-NUXTJS|LAH-API|lah-messenger-server|node|www)""#,
        ];
        for pat in patterns {
            if let Ok(re) = Regex::new(pat) {
                for cap in re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        candidates.push(normalize_root_str(m.as_str()));
                    }
                }
            }
        }
    }

    // 4. 從 pm2-logrotate/app.js 探測 require 路徑
    let lr_app = root_dir
        .join(".pm2")
        .join("modules")
        .join("pm2-logrotate")
        .join("node_modules")
        .join("pm2-logrotate")
        .join("app.js");
    if let Ok(content) = fs::read_to_string(&lr_app) {
        if let Ok(re) = Regex::new(
            r#"require\(\s*"([^"]+)(?:\\\\|[/\\])node(?:\\\\|[/\\])node_modules(?:\\\\|[/\\])pm2-logrotate"#,
        ) {
            for cap in re.captures_iter(&content) {
                if let Some(m) = cap.get(1) {
                    candidates.push(normalize_root_str(m.as_str()));
                }
            }
        }
    }

    // 去重、過濾當前目標目錄與過短之磁碟根目錄（如 C:\）
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for c in candidates {
        let norm = normalize_root_str(&c);
        if norm.len() <= 3 {
            continue;
        }
        if norm.eq_ignore_ascii_case(&new_root_norm) {
            continue;
        }
        let lower = norm.to_lowercase();
        if seen.insert(lower) {
            result.push(norm);
        }
    }

    // 依長度降序排列，確保較長之子路徑優先匹配
    result.sort_by(|a, b| b.len().cmp(&a.len()));
    result
}

/// 處理 Windows 路徑跳脫特性的多重格式置換常式
/// 同時涵蓋：
/// 1. JSON 轉義雙反斜線 (`C:\\AppServ`)
/// 2. Unix 正斜線 (`C:/AppServ`)
/// 3. Windows 原生單反斜線 (`C:\AppServ`)
/// 並具備路徑邊界檢查，避免誤傷非隸屬該根目錄之相近前綴目錄。
pub fn relocate_path_string(
    input: &str,
    old_root: &str,
    new_root: &str,
    slash_style: SlashStyle,
) -> (String, usize) {
    let old_native = normalize_root_str(old_root);
    let new_native = normalize_root_str(new_root);

    if old_native.len() <= 3 || old_native.eq_ignore_ascii_case(&new_native) {
        return (input.to_string(), 0);
    }

    let old_json = old_native.replace('\\', "\\\\");
    let old_fwd = old_native.replace('\\', "/");

    let new_json = match slash_style {
        SlashStyle::PreserveFormat => new_native.replace('\\', "\\\\"),
        SlashStyle::ForceForwardSlash => new_native.replace('\\', "/"),
    };
    let new_fwd = new_native.replace('\\', "/");
    let new_single = match slash_style {
        SlashStyle::PreserveFormat => new_native.clone(),
        SlashStyle::ForceForwardSlash => new_native.replace('\\', "/"),
    };

    let mut current = input.to_string();
    let mut total_replacements = 0;

    // 順序 1：JSON 轉義雙反斜線 (C:\\AppServ)
    let pat_json = format!(
        r#"(?i){}(?P<sep>\\\\|/|"|'|;|\s|\)|\]|\}}|$)"#,
        regex::escape(&old_json)
    );
    if let Ok(re_json) = Regex::new(&pat_json) {
        let matches = re_json.find_iter(&current).count();
        if matches > 0 {
            total_replacements += matches;
            current = re_json
                .replace_all(&current, |caps: &regex::Captures| {
                    let sep = caps.name("sep").map_or("", |m| m.as_str());
                    let norm_sep = if slash_style == SlashStyle::ForceForwardSlash && sep == "\\\\" {
                        "/"
                    } else {
                        sep
                    };
                    format!("{}{}", new_json, norm_sep)
                })
                .to_string();
        }
    }

    // 順序 2：Unix 正斜線 (C:/AppServ)
    let pat_fwd = format!(
        r#"(?i){}(?P<sep>/|\\\\|\\|"|'|;|\s|\)|\]|\}}|$)"#,
        regex::escape(&old_fwd)
    );
    if let Ok(re_fwd) = Regex::new(&pat_fwd) {
        let matches = re_fwd.find_iter(&current).count();
        if matches > 0 {
            total_replacements += matches;
            current = re_fwd
                .replace_all(&current, |caps: &regex::Captures| {
                    let sep = caps.name("sep").map_or("", |m| m.as_str());
                    let norm_sep = if slash_style == SlashStyle::ForceForwardSlash
                        && (sep == "\\" || sep == "\\\\")
                    {
                        "/"
                    } else {
                        sep
                    };
                    format!("{}{}", new_fwd, norm_sep)
                })
                .to_string();
        }
    }

    // 順序 3：Windows 原生單反斜線 (C:\AppServ)
    let pat_native = format!(
        r#"(?i){}(?P<sep>\\|/|"|'|;|\s|\)|\]|\}}|$)"#,
        regex::escape(&old_native)
    );
    if let Ok(re_native) = Regex::new(&pat_native) {
        let matches = re_native.find_iter(&current).count();
        if matches > 0 {
            total_replacements += matches;
            current = re_native
                .replace_all(&current, |caps: &regex::Captures| {
                    let sep = caps.name("sep").map_or("", |m| m.as_str());
                    let norm_sep = if slash_style == SlashStyle::ForceForwardSlash && sep == "\\" {
                        "/"
                    } else {
                        sep
                    };
                    format!("{}{}", new_single, norm_sep)
                })
                .to_string();
        }
    }

    (current, total_replacements)
}

/// 針對多個舊根目錄依序執行多重格式路徑置換
pub fn relocate_text_all_roots(
    input: &str,
    old_roots: &[String],
    new_root: &str,
    slash_style: SlashStyle,
) -> (String, usize) {
    let mut current = input.to_string();
    let mut total = 0;
    for old_r in old_roots {
        let (next, count) = relocate_path_string(&current, old_r, new_root, slash_style);
        current = next;
        total += count;
    }
    (current, total)
}

/// 遞迴重定位 JSON Value 樹中的所有字串欄位
fn relocate_json_value_strings(
    val: &mut serde_json::Value,
    old_roots: &[String],
    new_root: &str,
) -> usize {
    match val {
        serde_json::Value::String(s) => {
            let (replaced, count) =
                relocate_text_all_roots(s, old_roots, new_root, SlashStyle::PreserveFormat);
            if count > 0 {
                *s = replaced;
            }
            count
        }
        serde_json::Value::Array(arr) => arr
            .iter_mut()
            .map(|item| relocate_json_value_strings(item, old_roots, new_root))
            .sum(),
        serde_json::Value::Object(map) => map
            .values_mut()
            .map(|v| relocate_json_value_strings(v, old_roots, new_root))
            .sum(),
        _ => 0,
    }
}

/// 類別 A 處理器：處置 .pm2/dump.pm2 快照（清理殘留 .bak/.pid、深層重定位 JSON 欄位、同名去重、剔除幽靈行程；若損毀則安全旁路重設）
pub fn relocate_or_bypass_pm2_dump(
    root_dir: &Path,
    old_roots: &[String],
    log_fn: &dyn Fn(&str, &str),
) -> (bool, bool) {
    let pm2_dir = root_dir.join(".pm2");
    if !pm2_dir.is_dir() {
        return (false, false);
    }

    // 1. 移除不可移植之 dump.pm2.bak 與舊機殘留之 .pid 檔案
    let dump_bak = pm2_dir.join("dump.pm2.bak");
    if dump_bak.is_file() {
        let _ = fs::remove_file(&dump_bak);
    }
    let pids_dir = pm2_dir.join("pids");
    if let Ok(entries) = fs::read_dir(&pids_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let _ = fs::remove_file(&p);
            }
        }
    }

    let dump_path = pm2_dir.join("dump.pm2");
    if !dump_path.is_file() {
        return (false, false);
    }

    let new_root_str = normalize_root_str(&root_dir.to_string_lossy());
    let Ok(raw) = fs::read_to_string(&dump_path) else {
        return (false, false);
    };

    // 解析 JSON；若快照損毀無法解析，執行安全旁路（Bypass）隔離並重設為空陣列 []
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        let quarantine = pm2_dir.join("dump.pm2.corrupted.bak");
        let _ = fs::copy(&dump_path, &quarantine);
        let _ = fs::write(&dump_path, "[]\n");
        log_fn(
            "  🛡️ [快照旁路] .pm2/dump.pm2 格式損毀無法解析，已隔離備份並重設為空快照，將由 ecosystem.config.js 重新建構服務",
            "warning",
        );
        return (false, true);
    };

    if !parsed.is_array() {
        let _ = fs::write(&dump_path, "[]\n");
        return (false, true);
    }

    // 2. 遞迴置換 JSON 內部所有舊根目錄路徑（含 cwd, pm_cwd, pm_exec_path, args, PATH, log_path 等）
    let replaced_count = relocate_json_value_strings(&mut parsed, old_roots, &new_root_str);

    // 3. 針對陣列進行同名去重（保留最後一筆）並剔除目標腳本不存在之幽靈行程
    let mut deduped_rev = Vec::new();
    let mut seen_names = HashSet::new();
    let mut dropped_missing = Vec::new();
    let mut dropped_dups = 0usize;

    if let Some(items) = parsed.as_array() {
        for item in items.iter().rev() {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let exec_path = obj
                .get("pm_exec_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 檢查執行入口檔案是否實際存在於新目錄（避免未安裝該子套件卻由備份快照強行喚醒而崩潰）
            if !exec_path.is_empty() && exec_path != "none" {
                let exec_p = Path::new(&exec_path);
                if exec_p.is_absolute() && !exec_p.exists() {
                    dropped_missing.push(format!("{} ({})", name, exec_path));
                    continue;
                }
            }

            if !name.is_empty() {
                let key = name.to_lowercase();
                if !seen_names.insert(key) {
                    dropped_dups += 1;
                    continue;
                }
            }
            deduped_rev.push(item.clone());
        }
    }
    deduped_rev.reverse();

    let changed = replaced_count > 0 || dropped_dups > 0 || !dropped_missing.is_empty();
    let final_val = serde_json::Value::Array(deduped_rev);

    if changed {
        if let Ok(pretty) = serde_json::to_string_pretty(&final_val) {
            let _ = fs::write(&dump_path, pretty);
        }
    }

    // 呼叫既有環境變數與 Node 路徑正規化常式
    crate::pm2::sanitize_pm2_dump(&dump_path, root_dir, log_fn);

    if replaced_count > 0 || dropped_dups > 0 || !dropped_missing.is_empty() {
        log_fn(
            &format!(
                "  ✓ [路徑重定位] 已修復 .pm2/dump.pm2 快照（置換 {} 處舊路徑、移除 {} 個重複項目、剔除 {} 個未安裝之幽靈行程）",
                replaced_count,
                dropped_dups,
                dropped_missing.len()
            ),
            "success",
        );
    }

    (changed, false)
}

/// 類別 A 處理器：修復 .pm2/modules/pm2-logrotate 橋接入口
pub fn relocate_pm2_logrotate_wrapper(
    root_dir: &Path,
    old_roots: &[String],
    log_fn: &dyn Fn(&str, &str),
) -> bool {
    let wrapper_path = root_dir
        .join(".pm2")
        .join("modules")
        .join("pm2-logrotate")
        .join("node_modules")
        .join("pm2-logrotate")
        .join("app.js");
    if !wrapper_path.is_file() {
        return false;
    }

    // 若綠色版 node 內已具備 pm2-logrotate，優先由 wire_bundled_pm2_logrotate 重建標準橋接檔
    if crate::pm2::wire_bundled_pm2_logrotate(root_dir, log_fn) {
        return true;
    }

    // 若尚未解壓 node.zip，直接將橋接檔內之舊根目錄重定位至新根目錄
    let Ok(content) = fs::read_to_string(&wrapper_path) else {
        return false;
    };
    let new_root_str = normalize_root_str(&root_dir.to_string_lossy());
    let (patched, count) =
        relocate_text_all_roots(&content, old_roots, &new_root_str, SlashStyle::PreserveFormat);
    if count > 0 && patched != content {
        if fs::write(&wrapper_path, patched).is_ok() {
            log_fn(
                "  ✓ [路徑重定位] 已更新 pm2-logrotate 模組入口橋接路徑",
                "success",
            );
            return true;
        }
    }
    false
}

/// 類別 B 處理器：具備原子備份與失敗回滾機制之設定檔文字重定位
pub fn relocate_text_config_file(
    file_path: &Path,
    old_roots: &[String],
    new_root: &str,
    slash_style: SlashStyle,
    description: &str,
    log_fn: &dyn Fn(&str, &str),
) -> Result<usize, String> {
    if !file_path.is_file() {
        return Ok(0);
    }

    let bytes = fs::read(file_path)
        .map_err(|e| format!("讀取 {} ({}) 失敗: {}", description, file_path.display(), e))?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    let (patched, count) = relocate_text_all_roots(&content, old_roots, new_root, slash_style);
    if count == 0 || patched == content {
        return Ok(0);
    }

    // 寫入前先建立 .reloc.bak 安全備份以供寫入異常時回滾
    let bak_path = PathBuf::from(format!("{}.reloc.bak", file_path.display()));
    let _ = fs::copy(file_path, &bak_path);

    match fs::write(file_path, patched.as_bytes()) {
        Ok(_) => {
            let _ = fs::remove_file(&bak_path);
            log_fn(
                &format!(
                    "  ✓ [路徑重定位] {} ({}) 已置換 {} 處舊目錄參照",
                    description,
                    file_path.display(),
                    count
                ),
                "success",
            );
            Ok(count)
        }
        Err(e) => {
            if bak_path.is_file() {
                let _ = fs::copy(&bak_path, file_path);
                let _ = fs::remove_file(&bak_path);
            }
            Err(format!(
                "寫入重定位設定檔 {} 失敗 (已自動從備份回滾): {}",
                file_path.display(),
                e
            ))
        }
    }
}

/// 檢查單一字串是否仍殘留任何舊根目錄參照（包含 `\`、`\\`、`/` 三種格式）
pub fn find_stale_root_in_line(line: &str, old_roots: &[String]) -> Option<String> {
    for old_r in old_roots {
        let (_, count) = relocate_path_string(line, old_r, "__LAH_PROBE__", SlashStyle::PreserveFormat);
        if count > 0 {
            return Some(old_r.clone());
        }
    }
    None
}

/// 執行全目錄跨路徑重定位與啟動前自動化檢驗（解壓與備份還原完成後、服務啟動前呼叫）
pub fn execute_path_relocation_and_verify(
    root_dir: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> RelocationReport {
    let mut report = RelocationReport::default();
    let new_root_str = normalize_root_str(&root_dir.to_string_lossy());
    let old_roots = detect_legacy_roots(root_dir);
    report.detected_legacy_roots = old_roots.clone();

    if old_roots.is_empty() {
        report.verified_clean = true;
        return report;
    }

    log_fn(
        &format!(
            "🧭 [路徑重定位] 正在掃描並遷移跨目錄設定（歷史根目錄候選: {:?} ➔ 目標根目錄: {}）...",
            old_roots, new_root_str
        ),
        "info",
    );

    // 1. 依註冊表處置所有類別 A（快照）與類別 B（設定檔/腳本）目標
    for spec in RELOCATION_TARGETS {
        let target_path = root_dir.join(spec.rel_path);
        if !target_path.is_file() {
            continue;
        }

        match spec.category {
            RelocationCategory::EphemeralSnapshot => {
                if spec.rel_path == ".pm2/dump.pm2" {
                    let (changed, bypassed) =
                        relocate_or_bypass_pm2_dump(root_dir, &old_roots, log_fn);
                    if bypassed {
                        report.bypassed_snapshots.push(spec.rel_path.to_string());
                    } else if changed {
                        report.relocated_files.push(spec.rel_path.to_string());
                    }
                } else if spec.rel_path.ends_with("pm2-logrotate/app.js") {
                    if relocate_pm2_logrotate_wrapper(root_dir, &old_roots, log_fn) {
                        report.relocated_files.push(spec.rel_path.to_string());
                    }
                }
            }
            RelocationCategory::TextConfig => {
                match relocate_text_config_file(
                    &target_path,
                    &old_roots,
                    &new_root_str,
                    spec.slash_style,
                    spec.description,
                    log_fn,
                ) {
                    Ok(count) if count > 0 => {
                        report.relocated_files.push(spec.rel_path.to_string());
                    }
                    Ok(_) => {}
                    Err(err_msg) => {
                        log_fn(&format!("  ⚠️ {}", err_msg), "warning");
                    }
                }
            }
        }
    }

    // 2. 掃描根目錄與 node/ 下所有自訂 .bat / .cmd / .xml 服務描述檔
    for scan_dir in [root_dir.to_path_buf(), root_dir.join("node")] {
        if let Ok(entries) = fs::read_dir(&scan_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let fname = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if fname.ends_with(".bat") || fname.ends_with(".cmd") || fname.ends_with(".xml") {
                    if let Ok(count) = relocate_text_config_file(
                        &p,
                        &old_roots,
                        &new_root_str,
                        SlashStyle::PreserveFormat,
                        "腳本或服務註冊設定檔",
                        log_fn,
                    ) {
                        if count > 0 {
                            let rel = p
                                .strip_prefix(root_dir)
                                .unwrap_or(&p)
                                .to_string_lossy()
                                .replace('\\', "/");
                            if !report.relocated_files.contains(&rel) {
                                report.relocated_files.push(rel);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. 啟動前外部自動化檢驗：確認所有關鍵檔案不再殘留任何舊根目錄
    let findings = verify_no_stale_root_paths(root_dir, &old_roots, log_fn);
    report.remaining_findings = findings;
    report.verified_clean = report.remaining_findings.is_empty();

    if report.verified_clean {
        log_fn(
            &format!(
                "  ✓ [啟動前檢驗通過] 目標目錄 {} 下所有關鍵設定檔與快照已確認無舊路徑殘留",
                new_root_str
            ),
            "success",
        );
    } else {
        log_fn(
            &format!(
                "  ⚠️ [啟動前檢驗警告] 偵測到 {} 處殘留舊路徑，請檢視日誌詳情",
                report.remaining_findings.len()
            ),
            "warning",
        );
    }

    report
}

/// 啟動前自動化檢驗邏輯：掃描目標目錄下所有納管檔案，若發現殘留舊路徑則執行例外自癒或記錄明細
pub fn verify_no_stale_root_paths(
    root_dir: &Path,
    old_roots: &[String],
    log_fn: &dyn Fn(&str, &str),
) -> Vec<StalePathFinding> {
    let mut findings = Vec::new();
    if old_roots.is_empty() {
        return findings;
    }

    let new_root_str = normalize_root_str(&root_dir.to_string_lossy());

    for spec in RELOCATION_TARGETS {
        let target_path = root_dir.join(spec.rel_path);
        if !target_path.is_file() {
            continue;
        }

        let Ok(bytes) = fs::read(&target_path) else {
            continue;
        };
        let content = String::from_utf8_lossy(&bytes);

        let mut file_findings = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            // 略過純註解行（如 httpd.conf / .env 的 # 註解或 php.ini 的 ; 註解）
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if let Some(matched_root) = find_stale_root_in_line(line, old_roots) {
                file_findings.push(StalePathFinding {
                    rel_path: spec.rel_path.to_string(),
                    line_number: idx + 1,
                    matched_legacy_root: matched_root,
                    line_preview: trimmed.chars().take(120).collect(),
                });
            }
        }

        if !file_findings.is_empty() {
            // 例外處置與自癒策略：
            // 若為類別 A 快照 (.pm2/dump.pm2) 仍有殘留舊路徑，執行安全旁路隔離並清空快照
            if spec.category == RelocationCategory::EphemeralSnapshot
                && spec.rel_path == ".pm2/dump.pm2"
            {
                let quarantine = root_dir.join(".pm2").join("dump.pm2.legacy_quarantine");
                let _ = fs::copy(&target_path, &quarantine);
                let _ = fs::write(&target_path, "[]\n");
                log_fn(
                    "  🛡️ [例外自癒] 檢驗發現 .pm2/dump.pm2 仍含殘留舊路徑，已隔離至 dump.pm2.legacy_quarantine 並重設為空陣列",
                    "warning",
                );
                continue;
            }

            // 若為類別 B 設定檔，嘗試二次強制重定位修補
            if let Ok(patched_count) = relocate_text_config_file(
                &target_path,
                old_roots,
                &new_root_str,
                spec.slash_style,
                spec.description,
                log_fn,
            ) {
                if patched_count > 0 {
                    // 重新驗證二次修補後是否已完全清除
                    if let Ok(re_bytes) = fs::read(&target_path) {
                        let re_content = String::from_utf8_lossy(&re_bytes);
                        file_findings.clear();
                        for (idx, line) in re_content.lines().enumerate() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('#') || trimmed.starts_with(';') {
                                continue;
                            }
                            if let Some(matched_root) = find_stale_root_in_line(line, old_roots) {
                                file_findings.push(StalePathFinding {
                                    rel_path: spec.rel_path.to_string(),
                                    line_number: idx + 1,
                                    matched_legacy_root: matched_root,
                                    line_preview: trimmed.chars().take(120).collect(),
                                });
                            }
                        }
                    }
                }
            }

            for f in file_findings {
                log_fn(
                    &format!(
                        "  ⚠️ [殘留路徑警示] {} 第 {} 行仍含舊路徑 [{}]: {}",
                        f.rel_path, f.line_number, f.matched_legacy_root, f.line_preview
                    ),
                    "warning",
                );
                findings.push(f);
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relocate_path_string_handles_all_three_windows_formats_and_boundaries() {
        let input = r#"
"cwd": "C:\\AppServ\\LAH-NUXTJS",
"PM2_HOME": "C:\\AppServ\\.pm2",
"PATH": "C:\\AppServ\\instantclient_11_2;C:\\AppServ\\php85;C:\\Windows\\System32",
Define SRVROOT "C:/AppServ/Apache24"
SET NODE_DIR=C:\AppServ\node
MONITOR_HOST_SMS_DIR=C:/SMSWEB_MOI
OTHER_DIR=C:\AppServerOther\bin
"#;

        let (output, count) =
            relocate_path_string(input, r"C:\AppServ", r"C:\Temp\WTF", SlashStyle::PreserveFormat);

        assert_eq!(count, 6);
        assert!(output.contains(r#""cwd": "C:\\Temp\\WTF\\LAH-NUXTJS""#));
        assert!(output.contains(r#""PM2_HOME": "C:\\Temp\\WTF\\.pm2""#));
        assert!(output.contains(
            r#""PATH": "C:\\Temp\\WTF\\instantclient_11_2;C:\\Temp\\WTF\\php85;C:\\Windows\\System32""#
        ));
        assert!(output.contains(r#"Define SRVROOT "C:/Temp/WTF/Apache24""#));
        assert!(output.contains(r#"SET NODE_DIR=C:\Temp\WTF\node"#));
        // 外部監控目錄與非隸屬之相近前綴目錄絕不可被誤改
        assert!(output.contains("MONITOR_HOST_SMS_DIR=C:/SMSWEB_MOI"));
        assert!(output.contains(r"OTHER_DIR=C:\AppServerOther\bin"));
        assert!(!output.contains("C:\\AppServ\\"));
        assert!(!output.contains("C:/AppServ/"));
    }

    #[test]
    fn test_relocate_or_bypass_pm2_dump_relocates_deduplicates_and_filters_missing_apps() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_reloc_pm2_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pm2_dir = temp_root.join(".pm2");
        let fe_dir = temp_root.join("LAH-NUXTJS");
        fs::create_dir_all(&pm2_dir).unwrap();
        fs::create_dir_all(&fe_dir).unwrap();

        // 建立實際存在之入口腳本，而故意不建立 lah-messenger-server 以測試幽靈行程剔除
        let nuxt_bin = fe_dir.join("server.js");
        fs::write(&nuxt_bin, "console.log('ok');").unwrap();

        let dump_path = pm2_dir.join("dump.pm2");
        let mock_dump = r#"[
  {
    "name": "LAH-FE",
    "pm_exec_path": "C:\\AppServ\\LAH-NUXTJS\\server.js",
    "pm_cwd": "C:\\AppServ\\LAH-NUXTJS",
    "pm_out_log_path": "C:\\AppServ\\.pm2\\logs\\LAH-FE-out.log",
    "env": {
      "PM2_HOME": "C:\\AppServ\\.pm2",
      "Path": "C:\\AppServ\\instantclient_11_2;C:\\AppServ\\php85;C:\\AppServ\\node"
    }
  },
  {
    "name": "LAH-FE",
    "pm_exec_path": "C:\\AppServ\\LAH-NUXTJS\\server.js",
    "pm_cwd": "C:\\AppServ\\LAH-NUXTJS",
    "pm_out_log_path": "C:\\AppServ\\.pm2\\logs\\LAH-FE-out.log",
    "env": {
      "PM2_HOME": "C:\\AppServ\\.pm2",
      "Path": "C:\\AppServ\\instantclient_11_2;C:\\AppServ\\php85;C:\\AppServ\\node"
    }
  },
  {
    "name": "LAH-WSS",
    "pm_exec_path": "C:\\AppServ\\lah-messenger-server\\server.js",
    "pm_cwd": "C:\\AppServ\\lah-messenger-server"
  }
]"#;
        fs::write(&dump_path, mock_dump).unwrap();

        let old_roots = detect_legacy_roots(&temp_root);
        assert!(old_roots.iter().any(|r| r.eq_ignore_ascii_case(r"C:\AppServ")));

        let (changed, bypassed) = relocate_or_bypass_pm2_dump(&temp_root, &old_roots, &|_, _| {});
        assert!(changed);
        assert!(!bypassed);

        let after_raw = fs::read_to_string(&dump_path).unwrap();
        assert!(!after_raw.contains("AppServ"));
        let parsed: serde_json::Value = serde_json::from_str(&after_raw).unwrap();
        let arr = parsed.as_array().unwrap();
        // LAH-FE 重複兩筆應合併為 1 筆，且未安裝之 LAH-WSS 應被剔除
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "LAH-FE");
        let expected_cwd = temp_root.join("LAH-NUXTJS").to_string_lossy().to_string();
        assert_eq!(arr[0]["pm_cwd"], expected_cwd);

        let findings = verify_no_stale_root_paths(&temp_root, &old_roots, &|_, _| {});
        assert!(findings.is_empty());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_real_backup_zip_cross_directory_relocation_to_custom_wtf_root() {
        let backup_zip = Path::new(r"C:\Temp\_LAH_Backup_20261031_150820.zip");
        if !backup_zip.is_file() {
            return;
        }

        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_real_backup_wtf_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        let (ok, _) = crate::extractor::restore_from_backup_zip(backup_zip, &temp_root, &|_, _| {});
        assert!(ok, "從真實備份檔解壓還原應成功");

        let report = execute_path_relocation_and_verify(&temp_root, &|_, _| {});
        assert!(
            report
                .detected_legacy_roots
                .iter()
                .any(|r| r.eq_ignore_ascii_case(r"C:\AppServ")),
            "應自動偵測出備份包內之 C:\\AppServ 舊根目錄"
        );
        assert!(
            report.verified_clean,
            "重定位完成後所有關鍵檔案不得殘留 C:\\AppServ，殘留項目: {:?}",
            report.remaining_findings
        );

        // 雙重驗證 httpd.conf, php7/php.ini, php85/php.ini, .pm2/dump.pm2, pm2-logrotate/app.js 內容
        for rel in [
            "Apache24/conf/httpd.conf",
            "php7/php.ini",
            "php85/php.ini",
            ".pm2/dump.pm2",
            ".pm2/modules/pm2-logrotate/node_modules/pm2-logrotate/app.js",
            "LAH-NUXTJS/.env",
        ] {
            let p = temp_root.join(rel);
            assert!(p.is_file(), "檔案應存在: {}", rel);
            let text = fs::read_to_string(&p).unwrap();
            for line in text.lines() {
                let t = line.trim();
                if t.starts_with('#') || t.starts_with(';') {
                    continue;
                }
                assert!(
                    !t.to_lowercase().contains("c:/appserv")
                        && !t.to_lowercase().contains(r"c:\appserv"),
                    "{} 不得包含未重定位之 AppServ 路徑，違規行: {}",
                    rel,
                    t
                );
            }
        }

        // 確認外部監控路徑 C:/SMSWEB_MOI 未被誤改
        let nuxt_env = fs::read_to_string(temp_root.join("LAH-NUXTJS/.env")).unwrap();
        assert!(
            nuxt_env.contains("MONITOR_HOST_SMS_DIR=C:/SMSWEB_MOI"),
            "非隸屬舊安裝根目錄之外部路徑 MONITOR_HOST_SMS_DIR 必須保持原樣"
        );

        let _ = fs::remove_dir_all(&temp_root);
    }
}
