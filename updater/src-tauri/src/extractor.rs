use crate::types::PackageItem;
use regex::Regex;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub struct PackageSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub pattern: Option<&'static str>,
    pub alt_names: &'static [&'static str],
    pub rel_target: &'static str,
    pub expected_folder: &'static str,
    pub alt_folders: &'static [&'static str],
    pub is_base_env: bool,
    pub is_nodejs: bool,
    pub is_sub_package: bool,
}

pub const ALL_PACKAGE_SPECS: &[PackageSpec] = &[
    // 基礎環境 (左欄)
    PackageSpec {
        id: "apache24",
        name: "Apache24.zip",
        title: "Apache 2.4 網頁伺服器 (Apache24.zip)",
        pattern: Some(r"(?i)^(httpd-2\.4|apache24?|apache-2\.4).*?\.zip$"),
        alt_names: &["httpd.zip", "Apache.zip", "Apache-2.4.zip", "httpd-2.4.zip"],
        rel_target: "Apache24",
        expected_folder: "Apache24",
        alt_folders: &["Apache24", "apache24", "Apache-2.4", "httpd-2.4"],
        is_base_env: true,
        is_nodejs: false,
        is_sub_package: false,
    },
    PackageSpec {
        id: "php7",
        name: "php7.zip",
        title: "PHP 7.4 執行環境 (php7.zip)",
        pattern: Some(r"(?i)^(php-7\.4|php74?|php-7).*?\.zip$"),
        alt_names: &["php-7.4.zip", "php74.zip", "php.zip"],
        rel_target: "php7",
        expected_folder: "php7",
        alt_folders: &["php7", "php-7.4", "php-7.4.33-Win32-vc15-x64"],
        is_base_env: true,
        is_nodejs: false,
        is_sub_package: false,
    },
    PackageSpec {
        id: "php85",
        name: "php85.zip",
        title: "PHP 8.5 執行環境 (php85.zip)",
        pattern: Some(r"(?i)^(php-8\.5|php85?|php-8).*?\.zip$"),
        alt_names: &["php-8.5.zip", "php8.zip", "php-8.zip"],
        rel_target: "php85",
        expected_folder: "php85",
        alt_folders: &["php85", "php-8.5", "php-8", "php-8.5.0-Win32-x64"],
        is_base_env: true,
        is_nodejs: false,
        is_sub_package: false,
    },
    // 應用模組 (右欄)
    PackageSpec {
        id: "www",
        name: "www.zip",
        title: "後端 PHP7 更新包 (www.zip)",
        pattern: None,
        alt_names: &[],
        rel_target: "www",
        expected_folder: "www",
        alt_folders: &[],
        is_base_env: false,
        is_nodejs: false,
        is_sub_package: false,
    },
    PackageSpec {
        id: "lah_api",
        name: "LAH-API.zip",
        title: "後端 PHP8 更新包 (LAH-API.zip)",
        pattern: None,
        alt_names: &[],
        rel_target: "LAH-API",
        expected_folder: "LAH-API",
        alt_folders: &["api"],
        is_base_env: false,
        is_nodejs: false,
        is_sub_package: false,
    },
    PackageSpec {
        id: "lah_nuxtjs",
        name: "LAH-NUXTJS.zip",
        title: "前端網站 更新包 (LAH-NUXTJS.zip)",
        pattern: None,
        alt_names: &[],
        rel_target: "LAH-NUXTJS",
        expected_folder: "LAH-NUXTJS",
        alt_folders: &[],
        is_base_env: false,
        is_nodejs: true,
        is_sub_package: false,
    },
    PackageSpec {
        id: "lah_nuxtjs_modules",
        name: "LAH-NUXTJS-node_modules.zip",
        title: "└─ 前端網站 核心套件 (LAH-NUXTJS-node_modules.zip)",
        pattern: None,
        alt_names: &["LAH-FE-node_modules.zip", "lah-fe-node_modules.zip"],
        rel_target: r"LAH-NUXTJS\node_modules",
        expected_folder: "node_modules",
        alt_folders: &["lah-nuxtjs-node_modules", "LAH-NUXTJS-node_modules", "lah-fe-node_modules", "LAH-FE-node_modules"],
        is_base_env: false,
        is_nodejs: true,
        is_sub_package: true,
    },
    PackageSpec {
        id: "messenger",
        name: "lah-messenger-server.zip",
        title: "即時通訊伺服器 (lah-messenger-server.zip)",
        pattern: None,
        alt_names: &[],
        rel_target: "lah-messenger-server",
        expected_folder: "lah-messenger-server",
        alt_folders: &[],
        is_base_env: false,
        is_nodejs: true,
        is_sub_package: false,
    },
    PackageSpec {
        id: "messenger_modules",
        name: "lah-messenger-server-node_modules.zip",
        title: "└─ 即時通訊 核心套件 (lah-messenger-server-node_modules.zip)",
        pattern: None,
        alt_names: &["lah-messenger-server-node-modules.zip"],
        rel_target: r"lah-messenger-server\node_modules",
        expected_folder: "node_modules",
        alt_folders: &["lah-messenger-server-node_modules", "lah-messenger-server-node-modules"],
        is_base_env: false,
        is_nodejs: true,
        is_sub_package: true,
    },
    PackageSpec {
        id: "node",
        name: "node.zip",
        title: "Node.js 綠色執行環境 (node.zip)",
        pattern: None,
        alt_names: &[],
        rel_target: "node",
        expected_folder: "node",
        alt_folders: &[],
        is_base_env: false,
        is_nodejs: true,
        is_sub_package: false,
    },
];

/// 判斷各安裝包所屬之分類專屬優先子目錄
pub fn get_preferred_subfolder(pkg_id: &str) -> Option<&'static str> {
    match pkg_id {
        // 基礎服務類：APACHE24, PHP7, PHP85, node -> base/
        "apache24" | "php7" | "php85" | "node" => Some("base"),
        // 程式更新包：LAH-NUXTJS, LAH-API, www, lah-messenger-server -> patch/
        "lah_nuxtjs" | "lah_api" | "www" | "lah_messenger_server" => Some("patch"),
        // node_modules 依賴類：LAH-NUXTJS-node_modules, lah-messenger-server-node_modules -> node_modules/
        "lah_nuxtjs_node_modules" | "lah_messenger_server_node_modules" => Some("node_modules"),
        _ => None,
    }
}

fn collect_zips_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut zips = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if ext == "zip" {
                    zips.push(p);
                }
            }
        }
    }
    zips
}

fn match_package_zips(zips: &[PathBuf], spec: &PackageSpec) -> Vec<PathBuf> {
    let mut matched = Vec::new();
    let compiled_re = spec.pattern.and_then(|pat| Regex::new(pat).ok());

    for zp in zips {
        let fname = zp.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fname_lower = fname.to_lowercase();

        let mut is_match = fname_lower == spec.name.to_lowercase();
        if !is_match {
            for alt in spec.alt_names {
                if fname_lower == alt.to_lowercase() {
                    is_match = true;
                    break;
                }
            }
        }
        if !is_match {
            if let Some(re) = &compiled_re {
                if re.is_match(fname) {
                    is_match = true;
                }
            }
        }

        if is_match {
            matched.push(zp.clone());
        }
    }
    matched
}

pub fn scan_packages(search_dirs: &[PathBuf], root_dir: &Path) -> Vec<PackageItem> {
    let mut items = Vec::new();

    // 執行檔主目錄通常為 search_dirs 的第一個元素
    let app_dir = search_dirs.first().cloned().unwrap_or_else(|| PathBuf::from("."));

    // 收集所有 search_dirs 下的常規 .zip 檔案 (既有備援路徑)
    let mut available_zips: Vec<PathBuf> = Vec::new();
    for d in search_dirs {
        available_zips.extend(collect_zips_in_dir(d));
    }

    for spec in ALL_PACKAGE_SPECS {
        let mut matched_files: Vec<PathBuf> = Vec::new();

        // 1. 優先策略：先至執行目錄下的分類專屬子資料夾 (base/、patch/、node_modules/) 搜尋
        if let Some(sub) = get_preferred_subfolder(spec.id) {
            let sub_dir = app_dir.join(sub);
            if sub_dir.is_dir() {
                let sub_zips = collect_zips_in_dir(&sub_dir);
                matched_files = match_package_zips(&sub_zips, spec);
            }
        }

        // 2. 回退策略：若專屬子資料夾不存在或未找到匹配檔案，則維持既有 search_dirs 搜尋
        if matched_files.is_empty() {
            matched_files = match_package_zips(&available_zips, spec);
        }

        let exists = !matched_files.is_empty();
        let file_path = if exists {
            matched_files[0].to_string_lossy().to_string()
        } else {
            String::new()
        };

        let file_size = if exists {
            fs::metadata(&matched_files[0]).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let all_paths: Vec<String> = matched_files.iter().map(|p| p.to_string_lossy().to_string()).collect();

        let target_path = root_dir.join(spec.rel_target);
        let target_exists = target_path.exists();

        items.push(PackageItem {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            title: spec.title.to_string(),
            rel_target: spec.rel_target.to_string(),
            is_base_env: spec.is_base_env,
            is_nodejs: spec.is_nodejs,
            is_sub_package: spec.is_sub_package,
            exists,
            file_path,
            all_file_paths: all_paths,
            file_size,
            target_path: target_path.to_string_lossy().to_string(),
            target_exists,
            checked: exists, // 預設若有偵測到壓縮包則自動勾選
            expected_folder: spec.expected_folder.to_string(),
            alt_folders: spec.alt_folders.iter().map(|s| s.to_string()).collect(),
        });
    }

    items
}

pub fn detect_strip_prefix(zip_path: &Path, expected_folder: &str, alt_folders: &[&str]) -> Option<String> {
    let file = File::open(zip_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let mut top_level_dirs = std::collections::HashSet::new();
    let mut has_root_files = false;
    let mut sub_dirs_by_top: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();

    for i in 0..archive.len() {
        let Ok(file_entry) = archive.by_index(i) else { continue; };
        let name = file_entry.name().replace('\\', "/");
        if name.starts_with("__MACOSX/") || name.ends_with(".DS_Store") {
            continue;
        }

        let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }

        if parts.len() == 1 && !file_entry.is_dir() && !file_entry.name().ends_with('/') {
            has_root_files = true;
        } else {
            let top_d = parts[0].to_string();
            top_level_dirs.insert(top_d.clone());
            if parts.len() >= 2 {
                sub_dirs_by_top.entry(top_d).or_default().insert(parts[1].to_lowercase());
            }
        }
    }

    if top_level_dirs.len() == 1 {
        let only_dir = top_level_dirs.into_iter().next().unwrap();
        let lower = only_dir.to_lowercase();
        let mut valid_candidates = vec![expected_folder.to_lowercase()];
        for a in alt_folders {
            valid_candidates.push(a.to_lowercase());
        }

        let is_match = valid_candidates.iter().any(|c| lower == *c || lower.contains(c) || c.contains(&lower))
            || lower.starts_with("httpd")
            || lower.starts_with("apache")
            || lower.starts_with("php")
            || lower == "node_modules";

        if is_match {
            if !has_root_files {
                return Some(format!("{}/", only_dir));
            }
            // 針對 Apache Lounge 等官方壓縮包：根目錄帶有 ReadMe.txt 散落說明檔，但內層具備 bin/conf/ext 核心目錄
            let sub_set = sub_dirs_by_top.get(&only_dir);
            let has_core = sub_set.map_or(false, |s| (s.contains("bin") && s.contains("conf")) || s.contains("ext"));
            if valid_candidates.iter().any(|c| lower == *c) || has_core {
                return Some(format!("{}/", only_dir));
            }
        }
    }

    None
}

pub fn extract_zip(
    zip_path: &Path,
    target_dir: &Path,
    strip_prefix: Option<&str>,
    on_file: &dyn Fn(&str, usize, usize),
) -> Result<usize, String> {
    let file = File::open(zip_path).map_err(|e| format!("開啟壓縮檔失敗: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("讀取 ZIP 格式失敗: {}", e))?;

    fs::create_dir_all(target_dir).map_err(|e| format!("建立目標目錄失敗: {}", e))?;

    let total = archive.len();
    let mut processed = 0;

    for i in 0..total {
        let mut entry = archive.by_index(i).map_err(|e| format!("讀取檔案索引 {} 失敗: {}", i, e))?;
        let raw_name = entry.name().replace('\\', "/");

        if raw_name.starts_with("__MACOSX/") || raw_name.ends_with(".DS_Store") {
            continue;
        }

        let mut rel_path_str = raw_name.clone();
        if let Some(prefix) = strip_prefix {
            if rel_path_str.starts_with(prefix) {
                rel_path_str = rel_path_str[prefix.len()..].trim_start_matches('/').to_string();
            } else if !rel_path_str.contains('/') {
                continue;
            }
        }

        if rel_path_str.is_empty() {
            continue;
        }

        // Zip Slip 防禦
        let dest_path = target_dir.join(&rel_path_str);
        if !dest_path.starts_with(target_dir) {
            return Err(format!("偵測到非法路徑跨越 (Zip Slip): {}", rel_path_str));
        }

        if entry.is_dir() {
            let _ = fs::create_dir_all(&dest_path);
        } else {
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            // 清除唯讀屬性以利覆蓋
            let _ = crate::system::ensure_root_dir(target_dir);
            if dest_path.is_file() {
                let _ = fs::remove_file(&dest_path);
            }

            let mut out_file = File::create(&dest_path).map_err(|e| format!("建立目標檔案失敗 [{}]: {}", dest_path.display(), e))?;
            io::copy(&mut entry, &mut out_file).map_err(|e| format!("寫入檔案失敗 [{}]: {}", dest_path.display(), e))?;
        }

        processed += 1;
        on_file(&rel_path_str, processed, total);
    }

    Ok(processed)
}

pub fn find_latest_backup_zip(search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for d in search_dirs {
        if let Ok(entries) = fs::read_dir(d) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                    if fname.starts_with("_lah_backup_") && fname.ends_with(".zip") {
                        if let Ok(meta) = p.metadata() {
                            if let Ok(mod_time) = meta.modified() {
                                candidates.push((mod_time, p));
                            }
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, p)| p)
}

pub fn restore_from_backup_zip(zip_path: &Path, root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    log_fn(&format!("📦 正在從備份檔案還原配置與資料: {}", zip_path.display()), "info");
    match extract_zip(zip_path, root_dir, None, &|_, _, _| {}) {
        Ok(n) => {
            log_fn(&format!("  ✓ 備份還原完成，共覆蓋還原 {} 個重要設定檔與資料項目", n), "success");
            (true, format!("已還原 {} 個檔案", n))
        }
        Err(e) => {
            log_fn(&format!("  ❌ 備份還原失敗: {}", e), "error");
            (false, e)
        }
    }
}

pub fn flatten_directory(src: &Path, dest: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dest)?;

    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let s_path = entry.path();
            let fname = entry.file_name();
            let d_path = dest.join(&fname);

            if s_path.is_dir() {
                if !d_path.exists() {
                    // 同磁碟分割區可直接秒級搬移目錄
                    if fs::rename(&s_path, &d_path).is_err() {
                        flatten_directory(&s_path, &d_path)?;
                        let _ = fs::remove_dir_all(&s_path);
                    }
                } else {
                    flatten_directory(&s_path, &d_path)?;
                    let _ = fs::remove_dir_all(&s_path);
                }
            } else {
                if d_path.exists() {
                    let _ = fs::remove_file(&d_path);
                }
                if fs::rename(&s_path, &d_path).is_err() {
                    fs::copy(&s_path, &d_path)?;
                    let _ = fs::remove_file(&s_path);
                }
            }
        }
    }
    let _ = fs::remove_dir_all(src);
    Ok(())
}

pub fn auto_flatten_nested_dirs(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    // 1. Apache24\Apache24 -> Apache24
    let apache_dir = root_dir.join("Apache24");
    let inner_apache = apache_dir.join("Apache24");
    if inner_apache.is_dir() && !apache_dir.join("bin").join("httpd.exe").is_file() {
        if inner_apache.join("bin").join("httpd.exe").is_file() {
            log_fn("🛠️ 偵測到 Apache24 雙層巢狀目錄，正在自動攤平校正至目標目錄...", "warning");
            if let Err(e) = flatten_directory(&inner_apache, &apache_dir) {
                log_fn(&format!("  ⚠️ 攤平 Apache24 目錄警示: {}", e), "warning");
            } else {
                log_fn("  ✓ 已成功將雙層 Apache24 目錄攤平校正完畢！", "success");
            }
        }
    }

    // 2. LAH-NUXTJS\node_modules\node_modules -> LAH-NUXTJS\node_modules
    let fe_nm = root_dir.join("LAH-NUXTJS").join("node_modules");
    let inner_fe_nm = fe_nm.join("node_modules");
    if inner_fe_nm.is_dir() {
        log_fn("🛠️ 偵測到 LAH-NUXTJS node_modules 雙層巢狀目錄，正在自動攤平校正...", "warning");
        if let Err(e) = flatten_directory(&inner_fe_nm, &fe_nm) {
            log_fn(&format!("  ⚠️ 攤平 LAH-NUXTJS node_modules 警示: {}", e), "warning");
        } else {
            log_fn("  ✓ 已成功將 LAH-NUXTJS node_modules 雙層目錄攤平校正完畢！", "success");
        }
    }

    // 3. lah-messenger-server\node_modules\node_modules -> lah-messenger-server\node_modules
    let wss_nm = root_dir.join("lah-messenger-server").join("node_modules");
    let inner_wss_nm = wss_nm.join("node_modules");
    if inner_wss_nm.is_dir() {
        log_fn("🛠️ 偵測到 lah-messenger-server node_modules 雙層巢狀目錄，正在自動攤平校正...", "warning");
        if let Err(e) = flatten_directory(&inner_wss_nm, &wss_nm) {
            log_fn(&format!("  ⚠️ 攤平 lah-messenger-server node_modules 警示: {}", e), "warning");
        } else {
            log_fn("  ✓ 已成功將 lah-messenger-server node_modules 雙層目錄攤平校正完畢！", "success");
        }
    }
}

pub fn ensure_default_env_files(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    // 1. LAH-NUXTJS
    let fe_env = root_dir.join("LAH-NUXTJS").join(".env");
    let fe_example = root_dir.join("LAH-NUXTJS").join(".env.example");
    if !fe_env.is_file() && fe_example.is_file() {
        if fs::copy(&fe_example, &fe_env).is_ok() {
            log_fn("  ✓ 已依據 .env.example 初始化建立 LAH-NUXTJS/.env (Port 8080)", "success");
        }
    }

    // 2. lah-messenger-server
    let wss_env = root_dir.join("lah-messenger-server").join(".env");
    let wss_example = root_dir.join("lah-messenger-server").join(".env.example");
    if !wss_env.is_file() && wss_example.is_file() {
        if let Ok(content) = fs::read_to_string(&wss_example) {
            let patched = content.replace("WEBSOCKET_HOST=XXX.XXX.XXX.XXX", "WEBSOCKET_HOST=0.0.0.0");
            if fs::write(&wss_env, patched).is_ok() {
                log_fn("  ✓ 已依據 .env.example 初始化建立 lah-messenger-server/.env (Port 8081/8082)", "success");
            }
        }
    }

    // 3. www
    let www_env = root_dir.join("www").join(".env");
    let www_example = root_dir.join("www").join(".env.example");
    if !www_env.is_file() && www_example.is_file() {
        if fs::copy(&www_example, &www_env).is_ok() {
            log_fn("  ✓ 已依據 .env.example 初始化建立 www/.env", "success");
        }
    }

    // 4. LAH-API
    let api_env = root_dir.join("LAH-API").join(".env");
    let api_example = root_dir.join("LAH-API").join(".env.example");
    if !api_env.is_file() && api_example.is_file() {
        if fs::copy(&api_example, &api_env).is_ok() {
            log_fn("  ✓ 已依據 .env.example 初始化建立 LAH-API/.env", "success");
        }
    }
}
