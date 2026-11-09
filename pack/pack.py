# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 原始碼自動編譯與打包腳本 (pack.py)

功能流程：
1. 進入 LAH-FE 執行 `yarn build` 進行前端正式環境編譯，並等待其完成（可使用 --skip-build 略過）。
2. 依據 zip_api.* 規則，將 LAH-API (www) 打包為 www.zip（後端 PHP7 更新包）。
3. 將 LAH-API (API) 打包為 LAH-API.zip（後端 PHP8 更新包）。
4. 依據 zip_fe.* 規則，將 LAH-FE 編譯後檔案打包為 LAH-NUXTJS.zip（前端更新包）。
5. 依據參數可選打包即時通訊伺服器 (lah-messenger-server.zip)。
6. 依據參數可選打包前端與即時通核心依賴套件 (LAH-NUXTJS-node_modules.zip, lah-messenger-server-node_modules.zip)。
7. 依據參數可選打包可攜式 Node.js 綠色環境 (node.zip) 與 VC++ 核心程式庫 (vcredist-x64.zip)。
8. 產出壓縮包預設輸出至專案根目錄之 dist/ 目錄，並自動依類型分流存檔：
   - 程式更新包：dist/patch/ (www.zip, LAH-API.zip, LAH-NUXTJS.zip, lah-messenger-server.zip)
   - 核心依賴包：dist/node_modules/ (LAH-NUXTJS-node_modules.zip, lah-messenger-server-node_modules.zip)
   - 基礎環境包：dist/base/ (node.zip, vcredist-x64.zip)
"""
import argparse
import fnmatch
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import time
import zipfile

# 安全容錯：確保 Windows 控制台在 UTF-8 下正確輸出進度條圖示與中文，避免 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


# ==============================================================================
# 路徑與打包清單設定（對齊 d:\DEV\FreeFileSync\zip_api.* 與 zip_fe.*）
# ==============================================================================

# 對應 zip_api.txt（基底目錄：LAH-API/www - PHP7 API）
API_INCLUDES = [
    "api",
    "include",
    "task",
    "json",
    "project",
    "vendor",
    "*.php",
    "*.html",
    "favicon.ico",
    r"assets\js",
    r"assets\sh",
    r"assets\xlsx",
    ".env.example",
    ".htaccess",
]

# 對應 zip_api_exclude.txt（基底目錄：LAH-API/www - PHP7 API）
API_EXCLUDES = [
    r"include\config\AD.php",
    r"include\config\AD.VALID.php",
    r"include\config\AD.INVALID.php",
    r"include\config\AD.ALL.php",
    r"include\config\AD.LOCKED.php",
    r"project\novel",
    r"project\bookmark",
    ".env",
    ".git*",
    ".vscode",
    "*.log",
    "*.tmp",
    "*.bak",
]

# LAH-API (API - PHP8 API) 打包規則
API8_INCLUDES = [
    "*",
    ".env.example",
]

API8_EXCLUDES = [
    ".env",
    ".git*",
    ".vscode",
    "*.log",
    "*.tmp",
    "*.bak",
    "__pycache__",
    "*.zip",
    r"include\config\AD.php",
    r"include\config\AD.VALID.php",
    r"include\config\AD.INVALID.php",
    r"include\config\AD.ALL.php",
    r"include\config\AD.LOCKED.php",
    r"project\novel",
    r"project\bookmark",
    ".phpunit.cache",
    "phpunit.xml",
    "tests",
    "run_tests.bat"
]

# 對應 zip_fe.txt（基底目錄：LAH-FE，靜態化後打包 dist/ 子目錄）
FE_INCLUDES = [
    "dist",
    "content",
    "scheduler",
    "static",
    ".env.example",
    "ecosystem.config.js",
    "nuxt.config.js",
    "package.json",
    "README*",
    "system_block.jpg",
]

FE_EXCLUDES = [
    ".env",
]

# lah-messenger-server (LAH-WSS) 打包規則（架構防呆：萬用字元自動納入所有根目錄 .js 與服務腳本，杜絕新模組漏包）
MESSENGER_INCLUDES = [
    "*.js",
    "dimension",
    ".env.example",
    "AGENTS.md",
    "package.json",
    "README*",
    "*.bat",
]

MESSENGER_EXCLUDES = [
    ".env",
    ".gitignore",
    "yarn.lock",
    "node_modules",
    "db",
    "uploads",
    ".git*",
    ".vscode",
    "*.log",
    "*.tmp",
    "*.bak",
]

# node_modules 附屬套件打包排除規則
NODE_MODULES_EXCLUDES = [
    r"node_modules\.cache",
    r"*\.cache\*",
    "*.log",
    "*.tmp",
    "*.bak",
]

# LAH-FE 前端核心套件專屬排除規則（排除歷史殘留之 SQLite 原生模組與編譯工具鏈）
FE_NODE_MODULES_EXCLUDES = NODE_MODULES_EXCLUDES + [
    r"node_modules\better-sqlite3*",
    r"*\better-sqlite3\*",
    r"node_modules\sqlite3*",
    r"*\sqlite3\*",
    r"node_modules\prebuild-install*",
    r"*\prebuild-install\*",
    r"node_modules\node-pre-gyp*",
    r"*\node-pre-gyp\*",
    r"node_modules\node-abi*",
    r"*\node-abi\*",
    r"node_modules\napi-build-utils*",
    r"*\napi-build-utils\*",
]

# Node.js 綠色環境打包排除規則
NODE_EXCLUDES = [
    r"node\node_modules\.cache",
    r"node\service",
    r"node\service\*",
    r"*\.cache\*",
    "*.log",
    "*.tmp",
    "*.bak",
]


def resolve_project_dirs(script_dir):
    """自動探測 LAH-API/www (PHP7)、LAH-API/API (PHP8)、LAH-FE、lah-messenger-server 與 Node.js 綠色環境真實路徑（相容工作區 Junction、獨立目錄與 pack/ 子目錄）"""
    # 自動探測工作區根目錄（向上尋找包含 LAH-API 與 LAH-FE 的工作區）
    cur = os.path.abspath(script_dir)
    workspace_dir = None
    while cur:
        if os.path.isdir(os.path.join(cur, "LAH-API")) and os.path.isdir(os.path.join(cur, "LAH-FE")):
            workspace_dir = cur
            break
        parent = os.path.dirname(cur)
        if parent == cur:
            break
        cur = parent

    if not workspace_dir:
        candidate_parent2 = os.path.dirname(os.path.dirname(os.path.abspath(script_dir)))
        candidate_parent1 = os.path.dirname(os.path.abspath(script_dir))
        workspace_dir = candidate_parent2 if os.path.isdir(os.path.join(candidate_parent2, "LAH-API")) else candidate_parent1

    api_candidates = [
        os.path.join(workspace_dir, "LAH-API", "www"),
        r"d:\DEV\lah-workspace\LAH-API\www",
        r"d:\DEV\Land-Affairs-Helper\www",
    ]
    api8_candidates = [
        os.path.join(workspace_dir, "LAH-API", "API"),
        r"d:\DEV\lah-workspace\LAH-API\API",
        r"d:\DEV\Land-Affairs-Helper\API",
    ]
    fe_candidates = [
        os.path.join(workspace_dir, "LAH-FE"),
        r"d:\DEV\lah-workspace\LAH-FE",
        r"d:\DEV\LAH-NUXTJS",
    ]
    messenger_candidates = [
        os.path.join(workspace_dir, "lah-messenger-server"),
        r"d:\DEV\lah-workspace\lah-messenger-server",
        r"d:\DEV\lah-messenger-server",
    ]
    api_dir = next((p for p in api_candidates if os.path.isdir(p)), api_candidates[0])
    api8_dir = next((p for p in api8_candidates if os.path.isdir(p)), api8_candidates[0])
    fe_dir = next((p for p in fe_candidates if os.path.isdir(p)), fe_candidates[0])
    messenger_dir = next((p for p in messenger_candidates if os.path.isdir(p)), messenger_candidates[0])

    node_candidates = [
        os.path.join(fe_dir, "node"),
        "C:/AppServ/node",
        "D:/AppServ/node",
        os.path.join(workspace_dir, "node"),
    ]
    node_dir = next((p for p in node_candidates if os.path.isdir(p)), node_candidates[0])
    return (
        os.path.realpath(api_dir),
        os.path.realpath(api8_dir),
        os.path.realpath(fe_dir),
        os.path.realpath(messenger_dir),
        os.path.realpath(node_dir)
    )


def is_excluded(rel_path, exclude_patterns):
    """檢查相對路徑是否符合排除清單（忽略大小寫與斜線方向，支援全路徑、前綴與子路徑比對）"""
    norm_rel = os.path.normpath(rel_path).lower()
    norm_rel_slash = norm_rel.replace("\\", "/")
    parts = norm_rel.split(os.sep)

    for pattern in exclude_patterns:
        norm_pat = os.path.normpath(pattern).lower()
        norm_pat_slash = norm_pat.replace("\\", "/")
        if norm_rel == norm_pat or norm_rel_slash == norm_pat_slash:
            return True
        if norm_rel.startswith(norm_pat + os.sep):
            return True
        if fnmatch.fnmatch(norm_rel_slash, norm_pat_slash) or fnmatch.fnmatch(norm_rel_slash, "*/" + norm_pat_slash):
            return True
        if "/" not in norm_pat_slash and any(fnmatch.fnmatch(p, norm_pat) for p in parts):
            return True
    return False


def collect_package_files(base_dir, include_patterns, exclude_patterns):
    """依據包含與排除規則收集所有待壓縮檔案，回傳 [(實體路徑, zip內相對路徑), ...]"""
    matched_files = []
    seen = set()

    for pattern in include_patterns:
        full_pattern = os.path.join(base_dir, pattern)
        matched_paths = sorted(glob.glob(full_pattern))

        for path in matched_paths:
            rel = os.path.relpath(path, base_dir)
            if is_excluded(rel, exclude_patterns):
                continue

            if os.path.isfile(path):
                norm_rel = os.path.normpath(rel)
                key = norm_rel.lower()
                if key not in seen:
                    seen.add(key)
                    arcname = norm_rel.replace("\\", "/")
                    matched_files.append((path, arcname))

            elif os.path.isdir(path):
                for root, dirs, files in os.walk(path):
                    dirs.sort()
                    files.sort()
                    rel_root = os.path.relpath(root, base_dir)
                    # 過濾被排除的子目錄
                    dirs[:] = [
                        d for d in dirs
                        if not is_excluded(os.path.join(rel_root, d), exclude_patterns)
                    ]
                    for f in files:
                        f_full = os.path.join(root, f)
                        f_rel = os.path.normpath(os.path.relpath(f_full, base_dir))
                        if is_excluded(f_rel, exclude_patterns):
                            continue
                        key = f_rel.lower()
                        if key not in seen:
                            seen.add(key)
                            arcname = f_rel.replace("\\", "/")
                            matched_files.append((f_full, arcname))

    return matched_files


def find_dist_node_zip(script_dir=None, fe_dir=None, workspace_dir=None):
    """在 dist 目錄中尋找已存在的 node.zip（優先尋找 base/node.zip）"""
    candidates = []
    # 1. 透過 pack.py 所在之 LAH-Updater/pack 尋找 LAH-Updater/dist
    base_pack_dir = os.path.abspath(script_dir or os.path.dirname(__file__))
    updater_dir = os.path.dirname(base_pack_dir) if os.path.basename(base_pack_dir).lower() == "pack" else base_pack_dir
    candidates.extend([
        os.path.join(updater_dir, "dist", "base", "node.zip"),
        os.path.join(updater_dir, "dist", "node.zip"),
    ])
    # 2. 透過 fe_dir 向上尋找鄰近的 LAH-Updater/dist 或工作區 dist
    if fe_dir:
        parent_fe = os.path.dirname(os.path.abspath(fe_dir))
        candidates.extend([
            os.path.join(parent_fe, "LAH-Updater", "dist", "base", "node.zip"),
            os.path.join(parent_fe, "LAH-Updater", "dist", "node.zip"),
            os.path.join(parent_fe, "dist", "base", "node.zip"),
            os.path.join(parent_fe, "dist", "node.zip"),
        ])
    # 3. 透過 workspace_dir 尋找
    if workspace_dir:
        candidates.extend([
            os.path.join(workspace_dir, "LAH-Updater", "dist", "base", "node.zip"),
            os.path.join(workspace_dir, "LAH-Updater", "dist", "node.zip"),
            os.path.join(workspace_dir, "dist", "base", "node.zip"),
            os.path.join(workspace_dir, "dist", "node.zip"),
        ])
    for c in candidates:
        if c and os.path.isfile(c):
            return os.path.realpath(c)
    return None


def get_global_node_version():
    """檢查系統全域 PATH 是否存在可用之 node 執行檔並回傳其版本字串（例如 'v26.12.1'），若無或執行異常則回傳 None"""
    try:
        node_path = shutil.which("node")
        if not node_path:
            return None
        ret = subprocess.run([node_path, "-v"], capture_output=True, text=True, timeout=5)
        if ret.returncode == 0:
            ver = ret.stdout.strip()
            if ver.startswith("v"):
                return ver
    except Exception:
        pass
    return None


def resolve_build_node_path(fe_dir=None, script_dir=None, force_portable=False):
    """尋找前端編譯用 Node 執行期目錄（預設優先檢測全域 Node；若未安裝或指定 force_portable 則回退至綠色環境）"""
    # 1. 預設模式：優先檢測系統全域 Node.js
    if not force_portable:
        global_ver = get_global_node_version()
        if global_ver:
            print(f"         [環境偵測] 偵測到全域 Node.js ({global_ver})，優先採用系統環境編譯。")
            return None

    if force_portable:
        print("         [環境切換] 已指定強制使用綠色 Node.js 執行期。")

    # 2. 備援方案或強制綠色環境：檢查原始碼目錄（LAH-FE）下的 node
    if fe_dir:
        fe_node_dir = os.path.join(fe_dir, "node")
        if os.path.isfile(os.path.join(fe_node_dir, "node.exe")):
            return fe_node_dir

        # 3. 若原始碼目錄下無 node.exe，自動自 dist 目錄尋找 node.zip 解壓縮至原始碼目錄作為備援防線
        dist_zip = find_dist_node_zip(script_dir=script_dir, fe_dir=fe_dir)
        if dist_zip and os.path.isfile(dist_zip):
            print(f"         [自動還原] 系統未發現全域 Node，正在自 dist 解壓 {os.path.basename(dist_zip)} 至：{fe_node_dir}")
            try:
                with zipfile.ZipFile(dist_zip, "r") as zf:
                    # 判斷壓縮檔內部第一層是否自帶 node/ 前綴
                    has_node_prefix = any(name.replace("\\", "/").startswith("node/") for name in zf.namelist())
                    target_extract = fe_dir if has_node_prefix else fe_node_dir
                    os.makedirs(fe_node_dir, exist_ok=True)
                    zf.extractall(target_extract)
                if os.path.isfile(os.path.join(fe_node_dir, "node.exe")):
                    print(f"         [自動還原] 成功解壓 Node.js 綠色環境至：{fe_node_dir}")
                    return fe_node_dir
            except Exception as e:
                print(f"         [WARN] 解壓縮 {dist_zip} 失敗：{e}")

    # 4. 備援探測：自 AppServ 尋找綠色 Node
    candidates = []
    candidates.extend([
        "C:/AppServ/node",
        "D:/AppServ/node",
    ])
    for c in candidates:
        if os.path.isfile(os.path.join(c, "node.exe")):
            return c
    return None


def run_fe_build(fe_dir, use_portable_node=False):
    """進入 LAH-FE 執行 yarn generate 產出純靜態網站 (dist/) 並等待完成"""
    print("-" * 60)
    print(f"[Step 1] 正在執行前端靜態編譯 (yarn generate)...")
    print(f"         目標目錄：{fe_dir}")
    print("-" * 60)

    if not os.path.isdir(fe_dir):
        print(f"[ERROR] 找不到 LAH-FE 目錄：{fe_dir}")
        return False

    build_env = os.environ.copy()
    build_env["NODE_ENV"] = "production"

    # Nuxt 2 / Webpack 4 於 Node 17+ 需 OpenSSL legacy provider (md4 雜湊)
    node_opts = build_env.get("NODE_OPTIONS", "")
    if "--openssl-legacy-provider" not in node_opts:
        build_env["NODE_OPTIONS"] = (node_opts + " --openssl-legacy-provider").strip()

    build_node_dir = resolve_build_node_path(fe_dir=fe_dir, force_portable=use_portable_node)
    if build_node_dir:
        print(f"         [環境隔離] 自動切換至綠色 Node 執行期：{build_node_dir}")
        build_env["PATH"] = build_node_dir + os.pathsep + build_env.get("PATH", "")

    start_time = time.time()
    ret = subprocess.run("yarn generate", cwd=fe_dir, env=build_env, shell=True)
    elapsed = time.time() - start_time

    if ret.returncode != 0:
        print(f"[ERROR] LAH-FE `yarn generate` 執行失敗（回傳碼：{ret.returncode}），已中止後續打包！")
        return False

    dist_dir = os.path.join(fe_dir, "dist")
    dist_index = os.path.join(dist_dir, "index.html")
    if not os.path.isdir(dist_dir) or not os.path.isfile(dist_index):
        print(f"[ERROR] 編譯完成後找不到靜態輸出 dist/index.html：{dist_index}")
        return False

    print(f"[OK] 前端 `yarn generate` 靜態編譯完成（耗時 {elapsed:.1f} 秒）。")
    return True


class PackIntegrityError(RuntimeError):
    """打包完整性校驗失敗例外（用於中止不完整的更新包產出）"""
    pass


def extract_nuxt_runtime_dependencies(fe_dir):
    """
    從 nuxt.config.js 與 ecosystem.config.js 動態提取執行期必要依賴路徑（單一真相來源）
    回傳: (required_deps, dynamic_includes)
    """
    config_path = os.path.join(fe_dir, "nuxt.config.js")
    eco_path = os.path.join(fe_dir, "ecosystem.config.js")
    required_deps = []
    dynamic_includes = set()

    # 1. 解析 nuxt.config.js 中之 serverMiddleware
    if os.path.isfile(config_path):
        try:
            with open(config_path, "r", encoding="utf-8") as f:
                content = f.read()
            sm_match = re.search(r"serverMiddleware\s*:\s*\[([\s\S]*?)\]", content)
            if sm_match:
                sm_block = sm_match.group(1)
                # 擷取含有 ~ 或 @ 或 ./ 或路徑字串
                raw_paths = re.findall(r"['\"]([~@./][^'\"]+)['\"]", sm_block)
                for p in raw_paths:
                    if p.startswith(("http://", "https://")):
                        continue
                    cleaned = re.sub(r"^[~@./\\]+", "", p).replace("/", os.sep)
                    full = os.path.join(fe_dir, cleaned)
                    if os.path.exists(full) or os.path.exists(full + ".js"):
                        required_deps.append(cleaned)
                        top_dir = cleaned.split(os.sep)[0]
                        dynamic_includes.add(top_dir)
                    else:
                        top_dir = cleaned.split(os.sep)[0]
                        required_deps.append(cleaned)
                        dynamic_includes.add(top_dir)
        except Exception as e:
            print(f"[WARN] 動態解析 nuxt.config.js 失敗：{e}")

    # 2. 自動掃描 fe_dir 根目錄下所有伺服端中介軟體/腳本目錄
    source_only_dirs = {
        "assets", "components", "constants", "layouts", "middleware",
        "mixins", "pages", "plugins", "store", "node_modules",
        ".git", ".vscode", ".antigravity", ".nuxt-dev"
    }
    try:
        for item in os.listdir(fe_dir):
            item_path = os.path.join(fe_dir, item)
            if os.path.isdir(item_path):
                item_lower = item.lower()
                if item_lower not in source_only_dirs and not item.startswith("."):
                    if any(k in item_lower for k in ("server", "middleware", "scheduler", "script")):
                        dynamic_includes.add(item)
    except Exception as e:
        print(f"[WARN] 掃描前端目錄失敗：{e}")

    # 3. 解析 ecosystem.config.js 中的啟動腳本
    if os.path.isfile(eco_path):
        try:
            with open(eco_path, "r", encoding="utf-8") as f:
                content = f.read()
            scripts = re.findall(r"script\s*:\s*['\"]([^'\"]+)['\"]", content)
            for s in scripts:
                if "node_modules" not in s:
                    cleaned = s.lstrip("./\\").replace("/", os.sep)
                    required_deps.append(cleaned)
                    top_dir = cleaned.split(os.sep)[0]
                    dynamic_includes.add(top_dir)
        except Exception as e:
            print(f"[WARN] 動態解析 ecosystem.config.js 失敗：{e}")

    return sorted(set(required_deps)), sorted(dynamic_includes)


def verify_fe_pack_integrity(fe_dir, files_to_pack):
    """
    打包前校驗：比對待打包檔案清單是否涵蓋 nuxt.config.js 與 ecosystem.config.js 宣告的所有執行期檔案
    若有缺失，拋出 PackIntegrityError 中止打包！
    """
    required_deps, _ = extract_nuxt_runtime_dependencies(fe_dir)
    packed_arcnames = {arcname.replace("\\", "/").lower() for _, arcname in files_to_pack}

    missing_deps = []
    for dep in required_deps:
        norm_dep = dep.replace("\\", "/").lower()
        matched = False
        for arc in packed_arcnames:
            if arc == norm_dep or arc == norm_dep + ".js" or arc.startswith(norm_dep + "/"):
                matched = True
                break
        if not matched:
            missing_deps.append(dep)

    if missing_deps:
        err_msg = (
            f"\n[FATAL] 前端打包完整性守門員攔截到缺失的執行期程式碼！\n"
            f"以下於 nuxt.config.js 或 ecosystem.config.js 宣告的執行期依賴未被納入打包清單：\n"
            + "\n".join(f"  - {m}" for m in missing_deps)
            + f"\n這將導致正式生產環境 Nuxt/PM2 啟動崩潰！請檢查 pack.py 打包規則。\n"
        )
        raise PackIntegrityError(err_msg)
    print(f"[OK] 前端打包完整性事前校驗通過（已涵蓋 nuxt.config.js 中介軟體與生態系腳本）。")
    return True


def verify_zip_integrity(zip_path, package_type, base_dir=None):
    """
    打包後校驗：開啟產出的 ZIP 壓縮包，驗證關鍵核心檔案是否存在
    若驗證失敗，自動銷毀破損 ZIP 並拋出 PackIntegrityError！
    """
    if not os.path.isfile(zip_path):
        raise PackIntegrityError(f"產出壓縮檔不存在：{zip_path}")

    with zipfile.ZipFile(zip_path, "r") as zf:
        namelist = set(zf.namelist())
        lower_names = {n.replace("\\", "/").lower() for n in namelist}

    missing = []
    if package_type == "fe":
        for req in ["dist/index.html", "ecosystem.config.js", "package.json"]:
            if req.lower() not in lower_names:
                missing.append(req)
        has_dist = any(n.startswith("dist/") for n in lower_names)
        if not has_dist:
            missing.append("dist 目錄")
        if base_dir:
            required_deps, _ = extract_nuxt_runtime_dependencies(base_dir)
            for dep in required_deps:
                norm = dep.replace("\\", "/").lower()
                matched = any(n == norm or n == norm + ".js" or n.startswith(norm + "/") for n in lower_names)
                if not matched:
                    missing.append(dep)

    elif package_type == "messenger":
        for req in ["server.js", "ecosystem.config.js", "package.json", "channel-db.js", "message-db.js"]:
            if req.lower() not in lower_names:
                missing.append(req)

    elif package_type == "api":
        has_php = any(n.endswith(".php") for n in lower_names)
        if not has_php:
            missing.append("*.php 核心檔案")

    if missing:
        try:
            os.remove(zip_path)
            print(f"[CLEANUP] 已自動刪除破損不完整之更新包：{zip_path}")
        except Exception:
            pass
        err_msg = (
            f"\n[FATAL] 更新包成品完整性校驗失敗！\n"
            f"壓縮檔 {os.path.basename(zip_path)} 缺少以下關鍵執行期檔案：\n"
            + "\n".join(f"  - {m}" for m in missing)
            + f"\n已中止發布並銷毀不完整產物以防部署事故！\n"
        )
        raise PackIntegrityError(err_msg)

    print(f"[OK] 成品完整性校驗通過：{os.path.basename(zip_path)}（已驗證關鍵執行期檔案與中介軟體）")
    return True


def prepare_server_prod_modules(messenger_dir, stage_name="lah-wss-prod-modules", label="即時通"):
    """於暫存目錄以 yarn 僅安裝正式環境相依 (--production)，排除 devDependencies 以精簡 node_modules。
    better-sqlite3 原生模組 (build/) 由來源 node_modules 複製（避免打包時需連網編譯/下載；專案未使用該套件時自動略過）。
    同時供即時通伺服器與 LAH-FE 使用，以 stage_name / label 區分暫存目錄與訊息。
    成功回傳暫存目錄路徑；任何失敗回傳 None（呼叫端會退回打包完整 node_modules）。"""
    import tempfile
    src_mod = os.path.join(messenger_dir, "node_modules")
    pkg = os.path.join(messenger_dir, "package.json")
    lock = os.path.join(messenger_dir, "yarn.lock")
    if not (os.path.isfile(pkg) and os.path.isfile(lock)):
        print("[WARN] 缺少 package.json 或 yarn.lock，無法精簡 node_modules，退回完整打包。")
        return None
    stage = os.path.join(tempfile.gettempdir(), stage_name)
    try:
        shutil.rmtree(stage, ignore_errors=True)
        os.makedirs(stage, exist_ok=True)
        shutil.copy2(pkg, stage)
        shutil.copy2(lock, stage)
        print("[INFO] 正在以 yarn --production 產生精簡版{label} node_modules ...")
        ret = subprocess.run("yarn install --production --frozen-lockfile --ignore-scripts --silent", cwd=stage, shell=True)
        if ret.returncode != 0:
            print(f"[WARN] yarn 安裝失敗（回傳碼 {ret.returncode}），退回完整打包。")
            return None
        # 複製 better-sqlite3 已編譯或預編譯之原生模組 (build/ 或 prebuilds/)
        bs_src = os.path.join(src_mod, "better-sqlite3")
        bs_dst = os.path.join(stage, "node_modules", "better-sqlite3")
        if os.path.isdir(bs_dst):
            has_build = os.path.isfile(os.path.join(bs_src, "build", "Release", "better_sqlite3.node"))
            has_prebuilds = os.path.isdir(os.path.join(bs_src, "prebuilds"))
            if not has_build and not has_prebuilds:
                print("[WARN] 來源 node_modules 缺少 better-sqlite3 原生二進位檔，退回完整打包。")
                return None
            if has_build:
                shutil.copytree(os.path.join(bs_src, "build"), os.path.join(bs_dst, "build"), dirs_exist_ok=True)
            if has_prebuilds:
                shutil.copytree(os.path.join(bs_src, "prebuilds"), os.path.join(bs_dst, "prebuilds"), dirs_exist_ok=True)
        return stage
    except Exception as e:
        print(f"[WARN] 產生精簡 node_modules 發生例外：{e}，退回完整打包。")
        return None

def build_zip_package(title, base_dir, zip_path, include_patterns, exclude_patterns, pre_pack_validator=None):
    """建立 ZIP 壓縮包（若目標已存在則先刪除重建）"""
    print("-" * 60)
    print(f"[打包] 正在建立 {title}...")
    print(f"       來源目錄：{base_dir}")
    print(f"       輸出檔案：{zip_path}")

    if not os.path.isdir(base_dir):
        print(f"[ERROR] 來源目錄不存在：{base_dir}")
        return False

    output_dir = os.path.dirname(zip_path)
    if output_dir and not os.path.isdir(output_dir):
        os.makedirs(output_dir, exist_ok=True)

    # 對齊原 bat 行為：若舊壓縮檔存在則先移除
    if os.path.exists(zip_path):
        try:
            os.remove(zip_path)
        except OSError as e:
            print(f"[ERROR] 無法刪除既有壓縮檔 {zip_path}：{e}")
            return False

    start_time = time.time()
    files_to_pack = collect_package_files(base_dir, include_patterns, exclude_patterns)
    if not files_to_pack:
        print(f"[ERROR] 未收集到任何可打包檔案，請確認來源目錄內容：{base_dir}")
        return False

    if pre_pack_validator:
        try:
            pre_pack_validator(files_to_pack)
        except PackIntegrityError as pie:
            print(str(pie))
            return False

    total_files = len(files_to_pack)
    bar_width = 20
    last_update_time = 0.0

    try:
        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as zf:
            for idx, (full_path, arcname) in enumerate(files_to_pack, 1):
                zf.write(full_path, arcname)

                now = time.time()
                # 節流即時動態刷新：第 1 檔、最後一檔或間隔 >= 0.08 秒才輸出，避免終端機 I/O 拖慢壓縮效能
                if idx == 1 or idx == total_files or (now - last_update_time >= 0.08):
                    last_update_time = now
                    percent = (idx / total_files) * 100.0
                    elapsed = now - start_time
                    speed = idx / elapsed if elapsed > 0 else 0
                    eta_sec = (total_files - idx) / speed if speed > 0 else 0
                    if eta_sec < 3600:
                        eta_str = f"{int(eta_sec // 60):02d}:{int(eta_sec % 60):02d}"
                    else:
                        eta_str = f"{int(eta_sec // 3600)}h{int((eta_sec % 3600) // 60):02d}m"

                    filled = int(bar_width * idx // total_files)
                    bar = "█" * filled + "░" * (bar_width - filled)

                    # 截斷過長檔名以防終端機自動折行造成殘影
                    max_name_len = 26
                    if len(arcname) > max_name_len:
                        display_name = "..." + arcname[-(max_name_len - 3):]
                    else:
                        display_name = arcname

                    line_content = f"\r       [{bar}] {percent:5.1f}% ({idx}/{total_files}) | {int(speed):3d} 檔/秒 | ETA: {eta_str} | {display_name}"
                    cols = shutil.get_terminal_size(fallback=(100, 24)).columns
                    sys.stdout.write(line_content.ljust(cols - 1)[:cols - 1])
                    sys.stdout.flush()

        # 壓縮完成後顯示 100% 狀態並換行
        filled = bar_width
        bar = "█" * filled
        elapsed = time.time() - start_time
        speed = total_files / elapsed if elapsed > 0 else 0
        final_line = f"\r       [{bar}] 100.0% ({total_files}/{total_files}) | {int(speed):3d} 檔/秒 | 完成"
        cols = shutil.get_terminal_size(fallback=(100, 24)).columns
        sys.stdout.write(final_line.ljust(cols - 1)[:cols - 1] + "\n")
        sys.stdout.flush()
    except Exception as e:
        print(f"\n[ERROR] 壓縮過程發生錯誤：{e}")
        return False

    elapsed = time.time() - start_time
    size_mb = os.path.getsize(zip_path) / (1024 * 1024)
    print(f"[OK] {os.path.basename(zip_path)} 打包完成！共 {total_files} 個檔案，大小 {size_mb:.2f} MB（耗時 {elapsed:.1f} 秒）")
    return True


def build_vcredist_package(base_dir):
    """從 Windows 系統目錄打包 x64 VC++ 2015-2022 核心執行期程式庫至 base/vcredist-x64.zip"""
    dll_names = [
        "msvcp140.dll",
        "msvcp140_1.dll",
        "msvcp140_2.dll",
        "msvcp140_atomic_wait.dll",
        "msvcp140_codecvt_ids.dll",
        "vcruntime140.dll",
        "vcruntime140_1.dll",
        "vcruntime140_threads.dll",
        "vcomp140.dll",
        "concrt140.dll",
    ]
    sys32 = os.path.join(os.environ.get("SystemRoot", r"C:\Windows"), "System32")
    out_path = os.path.join(base_dir, "vcredist-x64.zip")

    found = []
    for name in dll_names:
        p = os.path.join(sys32, name)
        if os.path.isfile(p):
            found.append((p, name))

    if not found:
        return False

    os.makedirs(base_dir, exist_ok=True)
    with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED) as z:
        for full_p, name in found:
            z.write(full_p, name)

    size_kb = os.path.getsize(out_path) / 1024
    print(f"[OK] vcredist-x64.zip 打包完成！共 {len(found)} 個 VC++ 核心程式庫，大小 {size_kb:.1f} KB")
    return True



def get_fe_version(fe_dir):
    """自 LAH-FE 的 package.json 取得版本號，回傳標準格式版號字串（例如 'v1.9.9'）"""
    pkg_path = os.path.join(fe_dir, "package.json")
    if os.path.isfile(pkg_path):
        try:
            with open(pkg_path, "r", encoding="utf-8") as f:
                pkg_data = json.load(f)
                raw_ver = pkg_data.get("version", "").strip()
                if raw_ver:
                    return raw_ver if raw_ver.startswith("v") else f"v{raw_ver}"
        except Exception as e:
            print(f"[WARN] 讀取 LAH-FE package.json 版本失敗：{e}")
    return None


def sync_artifacts_to_release_dir(dist_dir, fe_dir):
    """
    將打包成品（白名單：*.txt, *.pdf, *.exe, *.zip）增量複製至開發機發布資料夾。
    目的端版號子目錄依據 LAH-FE package.json 之版本動態建立（例如 v1.9.9）。
    若本機不存在發布資料夾則優雅略過，不影響打包流程。
    """
    release_base = r"d:\DEV\@各所分享區\@智慧管控系統\@更新程式"
    if not os.path.isdir(release_base):
        print("\n[提示] 本機未偵測到發布基底資料夾 (d:\\DEV\\@各所分享區\\@智慧管控系統\\@更新程式)，已略過發布資料夾複製作業。")
        return None

    version_str = get_fe_version(fe_dir)
    if not version_str:
        print("\n[WARN] 無法自 LAH-FE/package.json 取得有效版號，略過複製至發布資料夾。")
        return None

    target_dir = os.path.join(release_base, version_str)
    os.makedirs(target_dir, exist_ok=True)

    allowed_exts = {".txt", ".pdf", ".exe", ".zip"}

    copied_count = 0
    skipped_count = 0
    copied_files = []

    print("-" * 60)
    print(f"[發布] 正在同步成品至開發機發布資料夾 ({version_str})...")
    print(f"       來源目錄：{dist_dir}")
    print(f"       目的目錄：{target_dir}")
    print(f"       副檔名白名單：{', '.join(sorted(allowed_exts))}")

    for root, dirs, files in os.walk(dist_dir):
        rel_dir = os.path.relpath(root, dist_dir)
        dest_subdir = target_dir if rel_dir == "." else os.path.join(target_dir, rel_dir)

        for filename in files:
            ext = os.path.splitext(filename)[1].lower()
            if ext not in allowed_exts:
                continue

            src_file = os.path.join(root, filename)
            dst_file = os.path.join(dest_subdir, filename)

            need_copy = True
            if os.path.exists(dst_file):
                src_stat = os.stat(src_file)
                dst_stat = os.stat(dst_file)
                if src_stat.st_size == dst_stat.st_size and abs(src_stat.st_mtime - dst_stat.st_mtime) <= 1.0:
                    need_copy = False

            if need_copy:
                os.makedirs(dest_subdir, exist_ok=True)
                shutil.copy2(src_file, dst_file)
                copied_count += 1
                rel_name = os.path.join(rel_dir, filename) if rel_dir != "." else filename
                copied_files.append(rel_name)
            else:
                skipped_count += 1

    print(f"[OK] 發布資料夾同步完成！新複製/覆蓋 {copied_count} 個檔案，略過未變更 {skipped_count} 個檔案。")
    if copied_files:
        for f in copied_files[:8]:
            print(f"     + 已更新：{f}")
        if len(copied_files) > 8:
            print(f"     ... 共 {len(copied_files)} 個更新檔案")
    return target_dir


def prompt_interactive_targets():
    """當未傳入任何參數時，顯示互動選單供使用者選擇欲打包的項目"""
    print("=" * 68)
    print("  智慧控管與監控系統 - 原始碼打包工具 (pack.bat / pack.py)")
    print("=" * 68)
    print()
    print("  【單項更新包】")
    print("    1. LAH-API (PHP7)  後端更新包   (www.zip)")
    print("    2. LAH-API (PHP8)  後端更新包   (LAH-API.zip)")
    print("    3. LAH-FE 前端靜態網站 (含 yarn generate) (LAH-NUXTJS.zip)")
    print("    4. 即時通訊伺服器               (lah-messenger-server.zip)")
    print("    5. [已退役] 前端網站 核心套件 (靜態化後零相依) (LAH-NUXTJS-node_modules.zip)")
    print("    6. 即時通訊 核心套件 (node_modules) (lah-messenger-server-node_modules.zip)")
    print("    7. Node.js  綠色環境 (含 logrotate) (node.zip)")
    print("    8. VC++ 核心程式庫 (x64)          (vcredist-x64.zip)")
    print()
    print("  【常用組合快捷】")
    print("    A. 常規程式碼更新包 (1 + 2 + 3 + 4)")
    print("    B. 僅相依環境套件包 (6 + 7 + 8)")
    print("    C. 全套完整大包     (1 + 2 + 3 + 4 + 6 + 7 + 8)")
    print()
    print("    Q. 離開程式")
    print()
    print("  提示：可輸入單一編號、組合代碼 (A/B/C)，或輸入多個數字以空格或逗號分隔")
    print("        (例如輸入「1 3 5」可同時打包 PHP7、前端與前端套件)")
    print("=" * 68)

    while True:
        try:
            choice = input("請輸入欲打包的選項 (預設 A，直接按 Enter 執行): ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n已取消作業。")
            sys.exit(0)

        if not choice:
            choice = "A"

        if choice.upper() == "Q":
            print("已取消作業。")
            sys.exit(0)

        # 解析輸入 token (以空格或逗號分隔)
        tokens = [t.strip().upper() for t in choice.replace(",", " ").split() if t.strip()]

        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = False
        do_server_modules = False
        do_node = False
        do_vcredist = False

        valid = False
        for tok in tokens:
            if tok == "1":
                do_api = True
                valid = True
            elif tok == "2":
                do_api8 = True
                valid = True
            elif tok == "3":
                do_fe = True
                valid = True
            elif tok == "4":
                do_messenger = True
                valid = True
            elif tok == "5":
                do_fe_modules = True
                valid = True
            elif tok == "6":
                do_server_modules = True
                valid = True
            elif tok == "7":
                do_node = True
                valid = True
            elif tok == "8":
                do_vcredist = True
                valid = True
            elif tok == "A":
                do_api = True
                do_api8 = True
                do_fe = True
                do_messenger = True
                valid = True
            elif tok == "B":
                do_fe_modules = False
                do_server_modules = True
                do_node = True
                do_vcredist = True
                valid = True
            elif tok == "C":
                do_api = True
                do_api8 = True
                do_fe = True
                do_messenger = True
                do_fe_modules = False
                do_server_modules = True
                do_node = True
                do_vcredist = True
                valid = True
            elif tok == "Q":
                print("已取消作業。")
                sys.exit(0)
            else:
                print(f"[提示] 未知的選項「{tok}」，已忽略。")

        if valid:
            selected_names = []
            if do_api: selected_names.append("www.zip (PHP7)")
            if do_api8: selected_names.append("LAH-API.zip (PHP8)")
            if do_fe: selected_names.append("LAH-NUXTJS.zip (前端)")
            if do_messenger: selected_names.append("lah-messenger-server.zip (即時通訊)")
            if do_fe_modules: selected_names.append("LAH-NUXTJS-node_modules.zip (前端套件)")
            if do_server_modules: selected_names.append("lah-messenger-server-node_modules.zip (即時通套件)")
            if do_node: selected_names.append("node.zip (綠色環境)")
            if do_vcredist: selected_names.append("vcredist-x64.zip (VC++ 程式庫)")

            print(f"\n已選擇打包項目：{', '.join(selected_names)}\n")
            return do_api, do_api8, do_fe, do_messenger, do_fe_modules, do_server_modules, do_node, do_vcredist

        print("[錯誤] 無任何有效選項，請重新輸入！\n")


def main():
    parser = argparse.ArgumentParser(description="智慧控管與監控系統 - 原始碼自動編譯與打包工具")
    parser.add_argument("--skip-build", action="store_true", help="略過 LAH-FE 的 yarn build 步驟，直接打包現有檔案")
    parser.add_argument("--skip-copy", action="store_true", help="略過將成品複製至開發機發布資料夾的步驟")
    parser.add_argument("--api-only", action="store_true", help="僅打包 LAH-API PHP7 (www.zip)")
    parser.add_argument("--api8-only", action="store_true", help="僅打包 LAH-API PHP8 (LAH-API.zip)")
    parser.add_argument("--fe-only", action="store_true", help="僅編譯並打包 LAH-FE (LAH-NUXTJS.zip)")
    parser.add_argument("--messenger-only", action="store_true", help="僅打包即時通訊伺服器 (lah-messenger-server.zip)")
    parser.add_argument("--with-messenger", action="store_true", help="常規打包時一併打包即時通訊伺服器 (lah-messenger-server.zip)")
    parser.add_argument("--fe-modules", action="store_true", help="打包前端核心套件 (LAH-NUXTJS-node_modules.zip)")
    parser.add_argument("--server-modules", action="store_true", help="打包即時通伺服器核心套件 (lah-messenger-server-node_modules.zip)")
    parser.add_argument("--with-modules", action="store_true", help="常規打包時一併打包這兩個 node_modules 壓縮包")
    parser.add_argument("--modules-only", action="store_true", help="僅打包這兩個 node_modules 壓縮包，不打包程式碼")
    parser.add_argument("--node-only", action="store_true", help="僅打包可攜式 Node.js 綠色環境 (node.zip)")
    parser.add_argument("--with-node", action="store_true", help="常規打包時一併打包可攜式 Node.js 綠色環境 (node.zip)")
    parser.add_argument("--node", action="store_true", dest="do_node_pkg", help="打包可攜式 Node.js 綠色環境 (node.zip)")
    parser.add_argument("--vcredist-only", action="store_true", help="僅打包 VC++ 核心程式庫 (vcredist-x64.zip)")
    parser.add_argument("--with-vcredist", action="store_true", help="常規打包時一併打包 VC++ 核心程式庫 (vcredist-x64.zip)")
    parser.add_argument("--no-notes", action="store_true", help="略過自動生成系統更新說明文件與發布錨點更新")
    parser.add_argument("--use-portable-node", action="store_true", help="強制使用可攜式 Node.js 綠色環境進行前端編譯（預設優先採用系統全域 Node）")
    parser.add_argument("--output-dir", default=None, help="指定輸出基底目錄（預設為專案根目錄之 dist 目錄，自動分流至 patch/、node_modules/ 與 base/）")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.dirname(script_dir)
    dist_dir = os.path.join(root_dir, "dist")
    api_dir, api8_dir, fe_dir, messenger_dir, node_dir = resolve_project_dirs(script_dir)

    # 決定輸出基底目錄：預設直接指向專案 dist/ 目錄，支援命令列 --output-dir 覆寫
    if args.output_dir:
        output_base_dir = os.path.abspath(args.output_dir)
    else:
        output_base_dir = dist_dir

    # 分流目標子目錄配置
    patch_dir = os.path.join(output_base_dir, "patch")
    node_modules_dir = os.path.join(output_base_dir, "node_modules")
    base_dir = os.path.join(output_base_dir, "base")

    os.makedirs(patch_dir, exist_ok=True)
    os.makedirs(node_modules_dir, exist_ok=True)
    os.makedirs(base_dir, exist_ok=True)

    # 決定打包項目：
    # 1. 若無傳入任何命令列參數，進入互動選單模式供使用者自由勾選
    if len(sys.argv) == 1:
        do_api, do_api8, do_fe, do_messenger, do_fe_modules, do_server_modules, do_node, do_vcredist = prompt_interactive_targets()
    # 2. 若傳入特定參數，則依參數執行（支援無人值守與腳本調用）
    elif args.modules_only:
        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = args.fe_modules
        do_server_modules = True
        do_node = False
        do_vcredist = False
    elif args.node_only:
        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = False
        do_server_modules = False
        do_node = True
        do_vcredist = False
    elif args.vcredist_only:
        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = False
        do_server_modules = False
        do_node = False
        do_vcredist = True
    else:
        has_specific_target = (
            args.api_only or args.api8_only or args.fe_only or
            args.messenger_only or args.fe_modules or args.server_modules or
            args.do_node_pkg or args.vcredist_only
        )
        if has_specific_target:
            do_api = args.api_only
            do_api8 = args.api8_only
            do_fe = args.fe_only
            do_messenger = args.messenger_only
            do_fe_modules = args.fe_modules
            do_server_modules = args.server_modules or args.with_modules
            do_node = args.do_node_pkg or args.with_node
            do_vcredist = args.vcredist_only or args.with_vcredist
        else:
            do_api = True
            do_api8 = True
            do_fe = True
            do_messenger = True
            do_fe_modules = args.fe_modules
            do_server_modules = args.with_modules
            do_node = args.with_node
            vcredist_zip_path = os.path.join(base_dir, "vcredist-x64.zip")
            do_vcredist = args.with_vcredist or not os.path.isfile(vcredist_zip_path) or args.with_node

    print("=" * 60)
    print("智慧控管與監控系統 - 原始碼打包工具 (LAH-API / LAH-FE / lah-messenger-server)")
    print("=" * 60)
    print(f"LAH-API (PHP7)         來源：{api_dir}")
    print(f"LAH-API (PHP8)         來源：{api8_dir}")
    print(f"LAH-FE                 來源：{fe_dir}")
    print(f"lah-messenger-server   來源：{messenger_dir}")
    node_status = node_dir if os.path.isdir(node_dir) else f"{node_dir} (備援)"
    print(f"Node.js (綠色環境)     來源：{node_status}")
    print(f"ZIP 輸出基底目錄            ：{output_base_dir}")
    print(f"  ├─ 程式更新包 (patch)     ：{patch_dir}")
    print(f"  ├─ 核心依賴包 (node_modules)：{node_modules_dir}")
    print(f"  └─ 基礎環境包 (base)      ：{base_dir}")

    overall_start = time.time()

    www_zip_path = None
    api8_zip_path = None
    fe_zip_path = None
    ms_zip_path = None
    fe_mod_zip = None
    server_mod_zip = None
    node_zip_path = None
    vcredist_zip_path = None

    # 1. 前端 yarn build
    if do_fe and not args.skip_build:
        if not run_fe_build(fe_dir, use_portable_node=args.use_portable_node):
            sys.exit(1)

    # 2. 打包 LAH-API (PHP7) -> patch/www.zip
    if do_api:
        www_zip_path = os.path.join(patch_dir, "www.zip")
        if not build_zip_package("LAH-API (PHP7) 後端更新包 (patch/www.zip)", api_dir, www_zip_path, API_INCLUDES, API_EXCLUDES):
            sys.exit(1)
        try:
            verify_zip_integrity(www_zip_path, "api", api_dir)
        except PackIntegrityError as pie:
            print(str(pie))
            sys.exit(1)

    # 3. 打包 LAH-API (PHP8) -> patch/LAH-API.zip
    if do_api8:
        api8_zip_path = os.path.join(patch_dir, "LAH-API.zip")
        if not build_zip_package("LAH-API (PHP8) 後端更新包 (patch/LAH-API.zip)", api8_dir, api8_zip_path, API8_INCLUDES, API8_EXCLUDES):
            sys.exit(1)
        try:
            verify_zip_integrity(api8_zip_path, "api", api8_dir)
        except PackIntegrityError as pie:
            print(str(pie))
            sys.exit(1)

    # 4. 打包 LAH-FE -> patch/LAH-NUXTJS.zip
    if do_fe:
        fe_zip_path = os.path.join(patch_dir, "LAH-NUXTJS.zip")
        _, dynamic_fe_includes = extract_nuxt_runtime_dependencies(fe_dir)
        effective_fe_includes = list(dict.fromkeys(FE_INCLUDES + dynamic_fe_includes))
        if not build_zip_package(
            "LAH-FE 前端更新包 (patch/LAH-NUXTJS.zip)",
            fe_dir,
            fe_zip_path,
            effective_fe_includes,
            FE_EXCLUDES,
            pre_pack_validator=lambda files: verify_fe_pack_integrity(fe_dir, files)
        ):
            sys.exit(1)
        try:
            verify_zip_integrity(fe_zip_path, "fe", fe_dir)
        except PackIntegrityError as pie:
            print(str(pie))
            sys.exit(1)

    # 5. 打包 即時通訊伺服器 -> patch/lah-messenger-server.zip
    if do_messenger:
        ms_zip_path = os.path.join(patch_dir, "lah-messenger-server.zip")
        if not build_zip_package("即時通訊伺服器 (patch/lah-messenger-server.zip)", messenger_dir, ms_zip_path, MESSENGER_INCLUDES, MESSENGER_EXCLUDES):
            sys.exit(1)
        try:
            verify_zip_integrity(ms_zip_path, "messenger", messenger_dir)
        except PackIntegrityError as pie:
            print(str(pie))
            sys.exit(1)

    # 6. 打包 前端核心套件 -> node_modules/LAH-NUXTJS-node_modules.zip
    if do_fe_modules:
        fe_mod_dir = os.path.join(fe_dir, "node_modules")
        if not os.path.isdir(fe_mod_dir):
            print(f"[WARN] 找不到前端 node_modules 目錄：{fe_mod_dir}，跳過打包。")
        else:
            fe_mod_zip = os.path.join(node_modules_dir, "LAH-NUXTJS-node_modules.zip")
            fe_pack_base = prepare_server_prod_modules(fe_dir, "lah-fe-prod-modules", "前端") or fe_dir
            fe_ok = build_zip_package("前端核心套件 (node_modules/LAH-NUXTJS-node_modules.zip)", fe_pack_base, fe_mod_zip, ["node_modules"], FE_NODE_MODULES_EXCLUDES)
            if fe_pack_base != fe_dir:
                shutil.rmtree(fe_pack_base, ignore_errors=True)
            if not fe_ok:
                sys.exit(1)

    # 7. 打包 即時通伺服器核心套件 -> node_modules/lah-messenger-server-node_modules.zip
    if do_server_modules:
        server_mod_dir = os.path.join(messenger_dir, "node_modules")
        if not os.path.isdir(server_mod_dir):
            print(f"[WARN] 找不到即時通伺服器 node_modules 目錄：{server_mod_dir}，跳過打包。")
        else:
            server_mod_zip = os.path.join(node_modules_dir, "lah-messenger-server-node_modules.zip")
            pack_base = prepare_server_prod_modules(messenger_dir) or messenger_dir
            ok = build_zip_package("即時通訊核心套件 (node_modules/lah-messenger-server-node_modules.zip)", pack_base, server_mod_zip, ["node_modules"], NODE_MODULES_EXCLUDES)
            if pack_base != messenger_dir:
                shutil.rmtree(pack_base, ignore_errors=True)
            if not ok:
                sys.exit(1)

    # 8. 打包 可攜式 Node.js 綠色環境 -> base/node.zip
    if do_node:
        if not os.path.isdir(node_dir):
            print(f"[WARN] 找不到 Node.js 綠色環境目錄：{node_dir}，跳過打包。")
        else:
            node_zip_path = os.path.join(base_dir, "node.zip")
            node_parent = os.path.dirname(node_dir)
            node_folder = os.path.basename(node_dir)
            if not build_zip_package("可攜式 Node.js 綠色環境 (base/node.zip)", node_parent, node_zip_path, [node_folder], NODE_EXCLUDES):
                sys.exit(1)

    # 8.5. 檢查並打包 VC++ 獨立執行期程式庫至 base/vcredist-x64.zip
    vcredist_zip_path = os.path.join(base_dir, "vcredist-x64.zip")
    if do_vcredist:
        if not build_vcredist_package(base_dir):
            vcredist_zip_path = None

    # 9. 自動生成歷史版本與標準更新說明文件至 dist/
    if not args.no_notes:
        try:
            import generate_release_notes
            print("-" * 60)
            print("[說明] 正在自動萃取跨專案 Git 歷程並產出系統更新說明...")
            _, gen_files = generate_release_notes.generate_notes(
                dry_run=False,
                output_dir=output_base_dir,
                commit_anchor=False  # 延後至發布同步成功後才正式寫入錨點 (事務性守護)
            )
            for gf in gen_files:
                print(f"[OK] 已產生更新說明：{gf}")
        except Exception as e:
            print(f"[WARN] 自動產生更新說明失敗：{e}")
    else:
        print("\n[提示] 已指定 --no-notes，略過系統更新說明產生。")

    # 10. 將成品同步複製至開發機發布資料夾（依據 LAH-FE 版號）
    published_dir = None
    if not args.skip_copy:
        published_dir = sync_artifacts_to_release_dir(output_base_dir, fe_dir)
    else:
        print("\n[提示] 已指定 --skip-copy，略過發布資料夾複製作業。")

    # 11. 事務性發布錨點更新 (只有在未略過說明產生且打包正常結束時正式落盤)
    if not args.no_notes:
        try:
            import generate_release_notes
            anchor_file = os.path.join(script_dir, ".release_anchor.json")
            version_str = get_fe_version(fe_dir) or "1.0.0"
            _, _, repos_dict = generate_release_notes.resolve_project_dirs(script_dir)
            generate_release_notes.commit_release_anchor(anchor_file, version_str, repos_dict)
            print(f"[OK] 已正式更新並推進發布錨點：{anchor_file}")
        except Exception as e:
            print(f"[WARN] 更新發布錨點失敗：{e}")

    total_elapsed = time.time() - overall_start
    print("=" * 60)
    print(f"[SUCCESS] 全部打包作業順利完成！（總耗時 {total_elapsed:.1f} 秒）")
    print(f"輸出基底目錄：{output_base_dir}")
    if published_dir:
        print(f"發布資料夾  ：{published_dir}")
    if do_api:
        if www_zip_path:
            print(f"  - [patch]        {www_zip_path}")
        else:
            print(f"  - [patch]        (略過，未產生 www.zip)")
    if do_api8:
        if api8_zip_path:
            print(f"  - [patch]        {api8_zip_path}")
        else:
            print(f"  - [patch]        (略過，未產生 LAH-API.zip)")
    if do_fe:
        if fe_zip_path:
            print(f"  - [patch]        {fe_zip_path}")
        else:
            print(f"  - [patch]        (略過，未產生 LAH-NUXTJS.zip)")
    if do_messenger:
        if ms_zip_path:
            print(f"  - [patch]        {ms_zip_path}")
        else:
            print(f"  - [patch]        (略過，未產生 lah-messenger-server.zip)")
    if do_fe_modules:
        if fe_mod_zip:
            print(f"  - [node_modules] {fe_mod_zip}")
        else:
            print(f"  - [node_modules] (略過，找不到前端 node_modules 目錄)")
    if do_server_modules:
        if server_mod_zip:
            print(f"  - [node_modules] {server_mod_zip}")
        else:
            print(f"  - [node_modules] (略過，找不到即時通伺服器 node_modules 目錄)")
    if do_node:
        if node_zip_path:
            print(f"  - [base]         {node_zip_path}")
        else:
            print(f"  - [base]         (略過，找不到 Node.js 綠色環境目錄)")
    if do_vcredist:
        if vcredist_zip_path:
            print(f"  - [base]         {vcredist_zip_path}")
        else:
            print(f"  - [base]         (略過，未找到 VC++ 核心程式庫)")
    print("=" * 60)


if __name__ == "__main__":
    main()
