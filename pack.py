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
7. 依據參數可選打包可攜式 Node.js 綠色環境 (node.zip)。
8. 產出壓縮檔預設輸出至分享區：d:\DEV\@各所分享區\@智慧管控系統\@更新程式\
   若該分享區目錄不存在（如於未掛載 D 槽之本機開發環境），自動回退輸出至腳本同層 dist 目錄。
"""
import argparse
import fnmatch
import glob
import os
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

DEFAULT_OUTPUT_DIR = r"d:\DEV\@各所分享區\@智慧管控系統\@更新程式"

# 對應 zip_api.txt（基底目錄：LAH-API/www - PHP7 API）
API_INCLUDES = [
    "api",
    "include",
    "vendor",
    "*.php",
    "*.html",
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
    ".env",
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
    ".phpunit.cache",
    "phpunit.xml",
    "tests",
    "run_tests.bat"
]

# 對應 zip_fe.txt（基底目錄：LAH-FE）
FE_INCLUDES = [
    ".nuxt",
    "content",
    "scheduler",
    "scripts",
    "static",
    ".env.example",
    "ecosystem.config.js",
    "nuxt.config.js",
    "package.json",
    "README*",
    "start.bat",
    "system_block.jpg",
]

FE_EXCLUDES = [
    ".env",
]

# lah-messenger-server (LAH-WSS) 打包規則（嚴格僅打包指定之 14 個檔案與 1 個資料夾）
MESSENGER_INCLUDES = [
    "dimension",
    ".env.example",
    "AGENTS.md",
    "channel-db.js",
    "ecosystem.config.js",
    "message-db.js",
    "message-watcher.js",
    "package.json",
    "README.md",
    "request-handler.js",
    "server.js",
    "start_pm2.bat",
    "start_standalone.bat",
    "upload-router.js",
    "utils.js",
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

# Node.js 綠色環境打包排除規則
NODE_EXCLUDES = [
    r"node\node_modules\.cache",
    r"*\.cache\*",
    "*.log",
    "*.tmp",
    "*.bak",
]


def resolve_project_dirs(script_dir):
    """自動探測 LAH-API/www (PHP7)、LAH-API/API (PHP8)、LAH-FE、lah-messenger-server 與 Node.js 綠色環境真實路徑（相容工作區 Junction 與獨立目錄）"""
    workspace_dir = os.path.dirname(script_dir)

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
    node_candidates = [
        r"C:\AppServ\node",
        r"d:\AppServ\node",
        os.path.join(workspace_dir, "node"),
    ]

    api_dir = next((p for p in api_candidates if os.path.isdir(p)), api_candidates[0])
    api8_dir = next((p for p in api8_candidates if os.path.isdir(p)), api8_candidates[0])
    fe_dir = next((p for p in fe_candidates if os.path.isdir(p)), fe_candidates[0])
    messenger_dir = next((p for p in messenger_candidates if os.path.isdir(p)), messenger_candidates[0])
    node_dir = next((p for p in node_candidates if os.path.isdir(p)), node_candidates[0])
    return (
        os.path.realpath(api_dir),
        os.path.realpath(api8_dir),
        os.path.realpath(fe_dir),
        os.path.realpath(messenger_dir),
        os.path.realpath(node_dir)
    )


def is_excluded(rel_path, exclude_patterns):
    """檢查相對路徑是否符合排除清單（忽略大小寫與斜線方向）"""
    norm_rel = os.path.normpath(rel_path).lower()
    norm_rel_slash = norm_rel.replace("\\", "/")

    for pattern in exclude_patterns:
        norm_pat = os.path.normpath(pattern).lower()
        norm_pat_slash = norm_pat.replace("\\", "/")
        if norm_rel == norm_pat or norm_rel_slash == norm_pat_slash:
            return True
        if norm_rel.startswith(norm_pat + os.sep):
            return True
        if fnmatch.fnmatch(norm_rel_slash, norm_pat_slash):
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


def run_fe_build(fe_dir):
    """進入 LAH-FE 執行 yarn build 並等待完成"""
    print("-" * 60)
    print(f"[Step 1] 正在執行前端編譯 (yarn build)...")
    print(f"         目標目錄：{fe_dir}")
    print("-" * 60)

    if not os.path.isdir(fe_dir):
        print(f"[ERROR] 找不到 LAH-FE 目錄：{fe_dir}")
        return False

    build_env = os.environ.copy()
    build_env["NODE_ENV"] = "production"

    start_time = time.time()
    ret = subprocess.run("yarn build", cwd=fe_dir, env=build_env, shell=True)
    elapsed = time.time() - start_time

    if ret.returncode != 0:
        print(f"[ERROR] LAH-FE `yarn build` 執行失敗（回傳碼：{ret.returncode}），已中止後續打包！")
        return False

    nuxt_dir = os.path.join(fe_dir, ".nuxt")
    if not os.path.isdir(nuxt_dir):
        print(f"[ERROR] 編譯完成後找不到 .nuxt 目錄：{nuxt_dir}")
        return False

    print(f"[OK] 前端 `yarn build` 編譯完成（耗時 {elapsed:.1f} 秒）。")
    return True


def build_zip_package(title, base_dir, zip_path, include_patterns, exclude_patterns):
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


def prompt_interactive_targets():
    """當未傳入任何參數時，顯示互動選單供使用者選擇欲打包的項目"""
    print("=" * 68)
    print("  智慧控管與監控系統 - 原始碼打包工具 (pack.bat / pack.py)")
    print("=" * 68)
    print()
    print("  【單項更新包】")
    print("    1. LAH-API (PHP7)  後端更新包   (www.zip)")
    print("    2. LAH-API (PHP8)  後端更新包   (LAH-API.zip)")
    print("    3. LAH-FE 前端網站 (含 yarn build) (LAH-NUXTJS.zip)")
    print("    4. 即時通訊伺服器               (lah-messenger-server.zip)")
    print("    5. 前端網站 核心套件 (node_modules) (LAH-NUXTJS-node_modules.zip)")
    print("    6. 即時通訊 核心套件 (node_modules) (lah-messenger-server-node_modules.zip)")
    print("    7. Node.js  綠色環境 (含 logrotate) (node.zip)")
    print()
    print("  【常用組合快捷】")
    print("    A. 常規程式碼更新包 (1 + 2 + 3 + 4)")
    print("    B. 僅大型相依套件包 (5 + 6 + 7)")
    print("    C. 全套完整大包     (1 至 7 全部打包)")
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
            elif tok == "A":
                do_api = True
                do_api8 = True
                do_fe = True
                do_messenger = True
                valid = True
            elif tok == "B":
                do_fe_modules = True
                do_server_modules = True
                do_node = True
                valid = True
            elif tok == "C":
                do_api = True
                do_api8 = True
                do_fe = True
                do_messenger = True
                do_fe_modules = True
                do_server_modules = True
                do_node = True
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

            print(f"\n已選擇打包項目：{', '.join(selected_names)}\n")
            return do_api, do_api8, do_fe, do_messenger, do_fe_modules, do_server_modules, do_node

        print("[錯誤] 無任何有效選項，請重新輸入！\n")


def main():
    parser = argparse.ArgumentParser(description="智慧控管與監控系統 - 原始碼自動編譯與打包工具")
    parser.add_argument("--skip-build", action="store_true", help="略過 LAH-FE 的 yarn build 步驟，直接打包現有檔案")
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
    parser.add_argument("--output-dir", default=None, help=f"指定輸出目錄（若未指定且預設分享區不存在，將自動輸出至腳本同層 dist 目錄）")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    dist_dir = os.path.join(script_dir, "dist")
    api_dir, api8_dir, fe_dir, messenger_dir, node_dir = resolve_project_dirs(script_dir)

    # 決定輸出目錄：未指定且預設分享區不存在時自動回退至 dist
    if args.output_dir:
        output_dir = os.path.abspath(args.output_dir)
    else:
        if os.path.isdir(DEFAULT_OUTPUT_DIR):
            output_dir = os.path.abspath(DEFAULT_OUTPUT_DIR)
        else:
            output_dir = dist_dir
            print(f"[提示] 預設分享區不存在（{DEFAULT_OUTPUT_DIR}），自動導向本機輸出目錄：{output_dir}")

    os.makedirs(output_dir, exist_ok=True)

    # 決定打包項目：
    # 1. 若無傳入任何命令列參數，進入互動選單模式供使用者自由勾選
    if len(sys.argv) == 1:
        do_api, do_api8, do_fe, do_messenger, do_fe_modules, do_server_modules, do_node = prompt_interactive_targets()
    # 2. 若傳入特定參數，則依參數執行（支援無人值守與腳本調用）
    elif args.modules_only:
        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = True
        do_server_modules = True
        do_node = False
    elif args.node_only:
        do_api = False
        do_api8 = False
        do_fe = False
        do_messenger = False
        do_fe_modules = False
        do_server_modules = False
        do_node = True
    else:
        has_specific_target = (
            args.api_only or args.api8_only or args.fe_only or
            args.messenger_only or args.fe_modules or args.server_modules or
            args.do_node_pkg
        )
        if has_specific_target:
            do_api = args.api_only
            do_api8 = args.api8_only
            do_fe = args.fe_only
            do_messenger = args.messenger_only
            do_fe_modules = args.fe_modules or args.with_modules
            do_server_modules = args.server_modules or args.with_modules
            do_node = args.do_node_pkg or args.with_node
        else:
            do_api = True
            do_api8 = True
            do_fe = True
            do_messenger = True
            do_fe_modules = args.with_modules
            do_server_modules = args.with_modules
            do_node = args.with_node

    print("=" * 60)
    print("智慧控管與監控系統 - 原始碼打包工具 (LAH-API / LAH-FE / lah-messenger-server)")
    print("=" * 60)
    print(f"LAH-API (PHP7)         來源：{api_dir}")
    print(f"LAH-API (PHP8)         來源：{api8_dir}")
    print(f"LAH-FE                 來源：{fe_dir}")
    print(f"lah-messenger-server   來源：{messenger_dir}")
    print(f"Node.js (綠色環境)     來源：{node_dir}")
    print(f"ZIP 輸出目錄                ：{output_dir}")

    overall_start = time.time()

    # 1. 前端 yarn build
    if do_fe and not args.skip_build:
        if not run_fe_build(fe_dir):
            sys.exit(1)

    # 2. 打包 LAH-API (PHP7) -> www.zip
    if do_api:
        www_zip_path = os.path.join(output_dir, "www.zip")
        if not build_zip_package("LAH-API (PHP7) 後端更新包 (www.zip)", api_dir, www_zip_path, API_INCLUDES, API_EXCLUDES):
            sys.exit(1)

    # 3. 打包 LAH-API (PHP8) -> LAH-API.zip
    if do_api8:
        api8_zip_path = os.path.join(output_dir, "LAH-API.zip")
        if not build_zip_package("LAH-API (PHP8) 後端更新包 (LAH-API.zip)", api8_dir, api8_zip_path, API8_INCLUDES, API8_EXCLUDES):
            sys.exit(1)

    # 4. 打包 LAH-FE -> LAH-NUXTJS.zip
    if do_fe:
        fe_zip_path = os.path.join(output_dir, "LAH-NUXTJS.zip")
        if not build_zip_package("LAH-FE 前端更新包 (LAH-NUXTJS.zip)", fe_dir, fe_zip_path, FE_INCLUDES, FE_EXCLUDES):
            sys.exit(1)

    # 5. 打包 即時通訊伺服器 -> lah-messenger-server.zip
    if do_messenger:
        ms_zip_path = os.path.join(output_dir, "lah-messenger-server.zip")
        if not build_zip_package("即時通訊伺服器 (lah-messenger-server.zip)", messenger_dir, ms_zip_path, MESSENGER_INCLUDES, MESSENGER_EXCLUDES):
            sys.exit(1)

    # 6. 打包 前端核心套件 -> LAH-NUXTJS-node_modules.zip (與別名 LAH-FE-node_modules.zip)
    if do_fe_modules:
        fe_mod_dir = os.path.join(fe_dir, "node_modules")
        if not os.path.isdir(fe_mod_dir):
            print(f"[WARN] 找不到前端 node_modules 目錄：{fe_mod_dir}，跳過打包。")
        else:
            fe_mod_zip = os.path.join(output_dir, "LAH-NUXTJS-node_modules.zip")
            if not build_zip_package("前端核心套件 (LAH-NUXTJS-node_modules.zip)", fe_dir, fe_mod_zip, ["node_modules"], NODE_MODULES_EXCLUDES):
                sys.exit(1)
            # 同步建立別名 LAH-FE-node_modules.zip，相容直接以 LAH-FE 命名的使用場景
            fe_alias_zip = os.path.join(output_dir, "LAH-FE-node_modules.zip")
            try:
                if os.path.exists(fe_alias_zip):
                    os.remove(fe_alias_zip)
                try:
                    os.link(fe_mod_zip, fe_alias_zip)
                except OSError:
                    shutil.copy2(fe_mod_zip, fe_alias_zip)
                print(f"       已同步建立別名檔案：{os.path.basename(fe_alias_zip)}")
            except Exception as e:
                print(f"[WARN] 建立別名檔案失敗：{e}")

    # 7. 打包 即時通伺服器核心套件 -> lah-messenger-server-node_modules.zip
    if do_server_modules:
        server_mod_dir = os.path.join(messenger_dir, "node_modules")
        if not os.path.isdir(server_mod_dir):
            print(f"[WARN] 找不到即時通伺服器 node_modules 目錄：{server_mod_dir}，跳過打包。")
        else:
            server_mod_zip = os.path.join(output_dir, "lah-messenger-server-node_modules.zip")
            if not build_zip_package("即時通訊核心套件 (lah-messenger-server-node_modules.zip)", messenger_dir, server_mod_zip, ["node_modules"], NODE_MODULES_EXCLUDES):
                sys.exit(1)

    # 8. 打包 可攜式 Node.js 綠色環境 -> node.zip
    if do_node:
        if not os.path.isdir(node_dir):
            print(f"[WARN] 找不到 Node.js 綠色環境目錄：{node_dir}，跳過打包。")
        else:
            node_zip_path = os.path.join(output_dir, "node.zip")
            node_parent = os.path.dirname(node_dir)
            node_folder = os.path.basename(node_dir)
            if not build_zip_package("可攜式 Node.js 綠色環境 (node.zip)", node_parent, node_zip_path, [node_folder], NODE_EXCLUDES):
                sys.exit(1)

    total_elapsed = time.time() - overall_start
    print("=" * 60)
    print(f"[SUCCESS] 全部打包作業順利完成！（總耗時 {total_elapsed:.1f} 秒）")
    print(f"輸出位置：{output_dir}")
    if do_api:
        print(f"  - {os.path.join(output_dir, 'www.zip')}")
    if do_api8:
        print(f"  - {os.path.join(output_dir, 'LAH-API.zip')}")
    if do_fe:
        print(f"  - {os.path.join(output_dir, 'LAH-NUXTJS.zip')}")
    if do_messenger:
        print(f"  - {os.path.join(output_dir, 'lah-messenger-server.zip')}")
    if do_fe_modules:
        print(f"  - {os.path.join(output_dir, 'LAH-NUXTJS-node_modules.zip')}")
        print(f"  - {os.path.join(output_dir, 'LAH-FE-node_modules.zip')} (別名)")
    if do_server_modules:
        print(f"  - {os.path.join(output_dir, 'lah-messenger-server-node_modules.zip')}")
    if do_node:
        print(f"  - {os.path.join(output_dir, 'node.zip')}")
    print("=" * 60)


if __name__ == "__main__":
    main()
