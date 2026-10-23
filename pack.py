# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 原始碼自動編譯與打包腳本 (pack.py)

功能流程：
1. 進入 LAH-FE 執行 `yarn build` 進行前端正式環境編譯，並等待其完成（可使用 --skip-build 略過）。
2. 依據 zip_api.* 規則，將 LAH-API (www) 打包為 www.zip（後端 PHP7 更新包）。
3. 將 LAH-API (API) 打包為 LAH-API.zip（後端 PHP8 更新包）。
4. 依據 zip_fe.* 規則，將 LAH-FE 編譯後檔案打包為 LAH-NUXTJS.zip（前端更新包）。
5. 產出壓縮檔預設輸出至分享區：d:\DEV\@各所分享區\@智慧管控系統\@更新程式\
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

# 安全容錯：避免 Windows CMD 在 CP950 編碼下因特殊符號噴 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(errors="replace")


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


def resolve_project_dirs(script_dir):
    """自動探測 LAH-API/www (PHP7)、LAH-API/API (PHP8) 與 LAH-FE 的真實路徑（相容工作區 Junction 與獨立目錄）"""
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

    api_dir = next((p for p in api_candidates if os.path.isdir(p)), api_candidates[0])
    api8_dir = next((p for p in api8_candidates if os.path.isdir(p)), api8_candidates[0])
    fe_dir = next((p for p in fe_candidates if os.path.isdir(p)), fe_candidates[0])
    return os.path.realpath(api_dir), os.path.realpath(api8_dir), os.path.realpath(fe_dir)


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
    try:
        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as zf:
            for idx, (full_path, arcname) in enumerate(files_to_pack, 1):
                zf.write(full_path, arcname)
                if idx % 300 == 0 or idx == total_files:
                    print(f"       進度：{idx}/{total_files} 個檔案...")
    except Exception as e:
        print(f"[ERROR] 壓縮過程發生錯誤：{e}")
        return False

    elapsed = time.time() - start_time
    size_mb = os.path.getsize(zip_path) / (1024 * 1024)
    print(f"[OK] {os.path.basename(zip_path)} 打包完成！共 {total_files} 個檔案，大小 {size_mb:.2f} MB（耗時 {elapsed:.1f} 秒）")
    return True


def main():
    parser = argparse.ArgumentParser(description="智慧控管與監控系統 - 原始碼自動編譯與打包工具")
    parser.add_argument("--skip-build", action="store_true", help="略過 LAH-FE 的 yarn build 步驟，直接打包現有檔案")
    parser.add_argument("--api-only", action="store_true", help="僅打包 LAH-API PHP7 (www.zip)")
    parser.add_argument("--api8-only", action="store_true", help="僅打包 LAH-API PHP8 (LAH-API.zip)")
    parser.add_argument("--fe-only", action="store_true", help="僅編譯並打包 LAH-FE (LAH-NUXTJS.zip)")
    parser.add_argument("--output-dir", default=None, help=f"指定輸出目錄（若未指定且預設分享區不存在，將自動輸出至腳本同層 dist 目錄）")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    dist_dir = os.path.join(script_dir, "dist")
    api_dir, api8_dir, fe_dir = resolve_project_dirs(script_dir)

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

    # 決定打包項目：若未指定特定 only 參數，則預設三者全部打包
    has_specific_target = args.api_only or args.api8_only or args.fe_only
    if has_specific_target:
        do_api = args.api_only
        do_api8 = args.api8_only
        do_fe = args.fe_only
    else:
        do_api = True
        do_api8 = True
        do_fe = True

    print("=" * 60)
    print("智慧控管與監控系統 - 原始碼打包工具 (LAH-API / LAH-FE)")
    print("=" * 60)
    print(f"LAH-API (PHP7) 來源：{api_dir}")
    print(f"LAH-API (PHP8) 來源：{api8_dir}")
    print(f"LAH-FE         來源：{fe_dir}")
    print(f"ZIP 輸出目錄        ：{output_dir}")

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
    print("=" * 60)


if __name__ == "__main__":
    main()
