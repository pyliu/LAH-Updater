# -*- coding: utf-8 -*-
"""
智慧控管與監控系統 - 自動更新工具
Automated Update Utility for System Services
"""

import os
import re
import sys
import time
import shutil
import zipfile
import threading
import subprocess
import ctypes
from ctypes import wintypes
import json
import winreg
from datetime import datetime

import tkinter as tk
from tkinter import ttk, filedialog, messagebox
from tkinter import scrolledtext
from tkinter import font as tkfont

# 定義 3 種基礎環境安裝包規格 (左欄)
BASE_ENV_CONFIGS = [
    {
        "id": "apache24",
        "name": "Apache24.zip",
        "pattern": r"(?i)^(httpd-2\.4|apache24?|apache-2\.4).*?\.zip$",
        "alt_names": ["httpd.zip", "Apache.zip", "Apache-2.4.zip", "httpd-2.4.zip"],
        "rel_target": "Apache24",
        "title": "Apache 2.4 網頁伺服器 (Apache24.zip)",
        "expected_folder": "Apache24",
        "alt_folders": ["Apache24", "apache24", "Apache-2.4", "httpd-2.4"],
        "is_base_env": True,
        "is_nodejs": False
    },
    {
        "id": "php7",
        "name": "php7.zip",
        "pattern": r"(?i)^(php-7\.4|php74?|php-7).*?\.zip$",
        "alt_names": ["php-7.4.zip", "php74.zip", "php.zip"],
        "rel_target": "php7",
        "title": "PHP 7.4 執行環境 (php7.zip)",
        "expected_folder": "php7",
        "alt_folders": ["php7", "php-7.4", "php-7.4.33-Win32-vc15-x64"],
        "is_base_env": True,
        "is_nodejs": False
    },
    {
        "id": "php85",
        "name": "php85.zip",
        "pattern": r"(?i)^(php-8\.5|php85?|php-8).*?\.zip$",
        "alt_names": ["php-8.5.zip", "php8.zip", "php-8.zip"],
        "rel_target": "php85",
        "title": "PHP 8.5 執行環境 (php85.zip)",
        "expected_folder": "php85",
        "alt_folders": ["php85", "php-8.5", "php-8", "php-8.5.0-Win32-x64"],
        "is_base_env": True,
        "is_nodejs": False
    }
]

# 定義 7 種應用與系統模組更新包規格 (右欄)
APP_PACKAGE_CONFIGS = [
    {
        "id": "www",
        "name": "www.zip",
        "rel_target": "www",
        "title": "後端 PHP7 更新包 (www.zip)",
        "expected_folder": "www",
        "is_nodejs": False
    },
    {
        "id": "lah_api",
        "name": "LAH-API.zip",
        "rel_target": "LAH-API",
        "title": "後端 PHP8 更新包 (LAH-API.zip)",
        "expected_folder": "LAH-API",
        "alt_folders": ["api"],
        "is_nodejs": False
    },
    {
        "id": "lah_nuxtjs",
        "name": "LAH-NUXTJS.zip",
        "rel_target": "LAH-NUXTJS",
        "title": "前端網站 更新包 (LAH-NUXTJS.zip)",
        "expected_folder": "LAH-NUXTJS",
        "is_nodejs": True
    },
    {
        "id": "lah_nuxtjs_modules",
        "name": "LAH-NUXTJS-node_modules.zip",
        "alt_names": ["LAH-FE-node_modules.zip", "lah-fe-node_modules.zip"],
        "rel_target": os.path.join("LAH-NUXTJS", "node_modules"),
        "title": "└─ 前端網站 核心套件 (LAH-NUXTJS-node_modules.zip)",
        "expected_folder": "node_modules",
        "alt_folders": ["lah-nuxtjs-node_modules", "LAH-NUXTJS-node_modules", "lah-fe-node_modules", "LAH-FE-node_modules"],
        "is_nodejs": True,
        "is_sub_package": True
    },
    {
        "id": "messenger",
        "name": "lah-messenger-server.zip",
        "rel_target": "lah-messenger-server",
        "title": "即時通訊伺服器 (lah-messenger-server.zip)",
        "expected_folder": "lah-messenger-server",
        "is_nodejs": True
    },
    {
        "id": "messenger_modules",
        "name": "lah-messenger-server-node_modules.zip",
        "rel_target": os.path.join("lah-messenger-server", "node_modules"),
        "title": "└─ 即時通訊 核心套件 (lah-messenger-server-node_modules.zip)",
        "expected_folder": "node_modules",
        "alt_folders": ["lah-messenger-server-node_modules", "lah-messenger-server-node-modules"],
        "is_nodejs": True,
        "is_sub_package": True
    },
    {
        "id": "node",
        "name": "node.zip",
        "rel_target": "node",
        "title": "Node.js 綠色執行環境 (node.zip)",
        "expected_folder": "node",
        "is_nodejs": True
    }
]

# 合併所有更新包規格以相容既有迭代處理
PACKAGE_CONFIGS = BASE_ENV_CONFIGS + APP_PACKAGE_CONFIGS


def is_admin():
    """檢查目前行程是否具備系統管理員權限"""
    try:
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except Exception:
        return False


def get_app_dir():
    """取得程式或 .exe 實際所在的目錄路徑（相容 PyInstaller 打包）"""
    if getattr(sys, 'frozen', False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(os.path.abspath(__file__))


def format_file_size(size_bytes):
    """格式化檔案大小為人類易讀格式"""
    if size_bytes < 1024:
        return f"{size_bytes} B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f} KB"
    elif size_bytes < 1024 * 1024 * 1024:
        return f"{size_bytes / (1024 * 1024):.2f} MB"
    else:
        return f"{size_bytes / (1024 * 1024 * 1024):.2f} GB"


def query_apache_service():
    """偵測系統中的 Apache 服務名稱（優先測試常見名稱，再搜尋所有服務）"""
    candidates = ["Apache2.4", "Apache24", "Apache2.2", "Apache2", "Apache", "httpd"]
    for name in candidates:
        try:
            res = subprocess.run(
                ["sc.exe", "query", name],
                capture_output=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            if res.returncode == 0:
                return name
        except Exception:
            pass

    # 備用方案：透過 PowerShell 查詢服務包含 apache 的名稱
    try:
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command",
             "Get-Service *apache* -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name -First 1"],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        val = _decode_console_bytes(res.stdout).strip()
        if val:
            return val
    except Exception:
        pass

    return None


def is_service_running(svc_name):
    """檢查指定 Windows 服務是否正在執行"""
    try:
        res = subprocess.run(
            ["sc.exe", "query", svc_name],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_str = _decode_console_bytes(res.stdout)
        return "STATE" in out_str and "RUNNING" in out_str
    except Exception:
        return False


def stop_apache_service(svc_name, log_func):
    """停止指定的 Apache 服務"""
    if not svc_name:
        log_func("ℹ️ 未指定 Apache 服務名稱，跳過停止操作。", "info")
        return True

    if not is_service_running(svc_name):
        log_func(f"ℹ️ Apache 服務 [{svc_name}] 目前未在執行中，無需停止。", "info")
        return True

    log_func(f"⏳ 正在停止 Apache 服務 [{svc_name}]...", "info")
    try:
        subprocess.run(
            ["net.exe", "stop", svc_name],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
    except Exception as e:
        log_func(f"⚠️ 執行停止服務指令時發生異常: {e}", "warning")

    # 等待服務完全停止 (最長等候 15 秒)
    for _ in range(15):
        time.sleep(1)
        if not is_service_running(svc_name):
            log_func(f"✅ Apache 服務 [{svc_name}] 已成功停止。", "success")
            return True

    log_func(f"⚠️ 警告：停止 Apache 服務 [{svc_name}] 等待逾時，將嘗試繼續進行更新。", "warning")
    return False


def start_apache_service(svc_name, log_func):
    """啟動指定的 Apache 服務"""
    if not svc_name:
        log_func("ℹ️ 未指定 Apache 服務名稱，跳過啟動操作。", "info")
        return True

    log_func(f"⏳ 正在啟動 Apache 服務 [{svc_name}]...", "info")
    try:
        subprocess.run(
            ["net.exe", "start", svc_name],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
    except Exception as e:
        log_func(f"⚠️ 執行啟動服務指令時發生異常: {e}", "warning")

    # 等待服務轉為 RUNNING (最長等候 15 秒)
    for _ in range(15):
        time.sleep(1)
        if is_service_running(svc_name):
            log_func(f"✅ Apache 服務 [{svc_name}] 已成功啟動！", "success")
            return True

    log_func(f"⚠️ 警告：啟動 Apache 服務 [{svc_name}] 等待逾時，請手動確認服務狀態。", "warning")
    return False


def apply_apache_security_hardening(conf_path, log_func=None):
    """
    資安弱點防護：隱藏 Apache 伺服器版本資訊
    1. ServerTokens Prod: 隱藏詳細版本與模組清單，HTTP Server 標頭僅回應 'Server: Apache'
    2. ServerSignature Off: 關閉伺服器產生的錯誤頁面（如 404, 500）底部的 Apache 版本資訊註記
    """
    if not os.path.isfile(conf_path):
        if log_func:
            log_func(f"⚠️ 未找到 Apache 設定檔：{conf_path}", "warning")
        return False, "找不到 httpd.conf"

    try:
        with open(conf_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        orig_content = content

        # 1. 處理 ServerTokens
        if re.search(r"(?m)^\s*#?\s*ServerTokens\b", content):
            content = re.sub(
                r"(?m)^\s*#?\s*ServerTokens\s+.*$",
                "ServerTokens Prod",
                content
            )
        else:
            content += "\n# 資安弱點防護：隱藏 Apache 版本\nServerTokens Prod\n"

        # 2. 處理 ServerSignature
        if re.search(r"(?m)^\s*#?\s*ServerSignature\b", content):
            content = re.sub(
                r"(?m)^\s*#?\s*ServerSignature\s+.*$",
                "ServerSignature Off",
                content
            )
        else:
            content += "\n# 資安弱點防護：關閉錯誤頁面版本簽名\nServerSignature Off\n"

        if content != orig_content:
            bak_path = conf_path + f".bak_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            try:
                shutil.copyfile(conf_path, bak_path)
            except Exception:
                pass
            with open(conf_path, "w", encoding="utf-8") as f:
                f.write(content)
            if log_func:
                log_func("  ✓ Apache httpd.conf 已套用資安強化 (ServerTokens Prod, ServerSignature Off)", "success")
            return True, "已成功套用 Apache 資安防護"
        else:
            if log_func:
                log_func("  ℹ️ Apache httpd.conf 已具備資安防護設定 (ServerTokens Prod, ServerSignature Off)，無需變更", "info")
            return True, "設定已是最新狀態"
    except Exception as e:
        msg = f"套用 Apache 資安防護發生異常: {e}"
        if log_func:
            log_func(f"  ❌ {msg}", "error")
        return False, msg


def apply_php_security_hardening(ini_path, log_func=None):
    """
    資安弱點防護：隱藏 PHP 版本資訊 (expose_php = Off)
    移除 HTTP 回應標頭中的 X-Powered-By: PHP/... 洩漏
    """
    if not os.path.isfile(ini_path):
        if log_func:
            log_func(f"⚠️ 未找到 PHP 設定檔：{ini_path}", "warning")
        return False, "找不到 php.ini"

    try:
        with open(ini_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        orig_content = content

        if re.search(r"(?m)^\s*#?\s*;?\s*expose_php\b", content):
            content = re.sub(
                r"(?m)^\s*#?\s*;?\s*expose_php\s*=.*$",
                "expose_php = Off",
                content
            )
        else:
            if "[PHP]" in content:
                content = content.replace("[PHP]\n", "[PHP]\n# 資安弱點防護：隱藏 PHP 版本\nexpose_php = Off\n", 1)
            else:
                content += "\n# 資安弱點防護：隱藏 PHP 版本\nexpose_php = Off\n"

        if content != orig_content:
            bak_path = ini_path + f".bak_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            try:
                shutil.copyfile(ini_path, bak_path)
            except Exception:
                pass
            with open(ini_path, "w", encoding="utf-8") as f:
                f.write(content)
            if log_func:
                log_func(f"  ✓ {os.path.basename(os.path.dirname(ini_path))}/php.ini 已套用資安強化 (expose_php = Off)", "success")
            return True, "已成功套用 PHP 資安防護"
        else:
            if log_func:
                log_func(f"  ℹ️ {os.path.basename(os.path.dirname(ini_path))}/php.ini 已設定 expose_php = Off，無需變更", "info")
            return True, "設定已是最新狀態"
    except Exception as e:
        msg = f"套用 PHP 資安防護發生異常: {e}"
        if log_func:
            log_func(f"  ❌ {msg}", "error")
        return False, msg


def check_apache_syntax(apache_dir, log_func=None):
    """檢驗 Apache httpd.conf 設定檔語法 (httpd.exe -t)"""
    httpd_exe = os.path.join(apache_dir, "bin", "httpd.exe")
    if not os.path.isfile(httpd_exe):
        return True, "找不到 httpd.exe"

    try:
        res = subprocess.run(
            [httpd_exe, "-t"],
            capture_output=True,
            text=True,
            timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        combined = (res.stderr + " " + res.stdout).strip()
        if "Syntax OK" in combined:
            if log_func:
                log_func("  ✓ Apache 設定檔語法檢驗通過 (Syntax OK)！", "success")
            return True, "Syntax OK"
        else:
            if log_func:
                log_func(f"  ⚠️ Apache 設定檔語法檢驗回傳警告/錯誤: {combined}", "warning")
            return False, combined
    except Exception as e:
        if log_func:
            log_func(f"  ⚠️ 檢驗 Apache 語法時發生例外: {e}", "warning")
        return False, str(e)


def install_apache_service(apache_dir, svc_name="Apache2.4", log_func=None):
    """在 Windows 系統註冊安裝 Apache 服務 (httpd.exe -k install)"""
    httpd_exe = os.path.join(apache_dir, "bin", "httpd.exe")
    if not os.path.isfile(httpd_exe):
        if log_func:
            log_func(f"⚠️ 找不到 httpd.exe，無法註冊服務：{httpd_exe}", "warning")
        return False

    existing_svc = query_apache_service()
    if existing_svc:
        if log_func:
            log_func(f"ℹ️ 系統已註冊 Apache 服務 [{existing_svc}]，無需重複安裝。", "info")
        return True

    if log_func:
        log_func(f"⏳ 正在註冊 Windows 服務 [{svc_name}]...", "info")
    try:
        res = subprocess.run(
            [httpd_exe, "-k", "install", "-n", svc_name],
            capture_output=True,
            text=True,
            timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        combined = (res.stdout + " " + res.stderr).strip()
        if res.returncode == 0 or "successfully installed" in combined.lower():
            if log_func:
                log_func(f"✅ Apache 服務 [{svc_name}] 註冊安裝成功！", "success")
            return True
        else:
            if log_func:
                log_func(f"⚠️ 註冊 Apache 服務回傳: {combined}", "warning")
            return False
    except Exception as e:
        if log_func:
            log_func(f"❌ 註冊 Apache 服務失敗: {e}", "error")
        return False


def configure_apache_environment(apache_dir, root_dir, php7_dir=None, php85_dir=None, enable_security=True, log_func=None):
    """
    自動校正 Apache httpd.conf 設定檔：
    1. 校正 Define SRVROOT 為目標 Apache24 目錄 (正斜線)
    2. 若啟用資安強化，確保 ServerTokens Prod 與 ServerSignature Off
    3. 若目標存在 PHP 7 (php7apache2_4.dll)，自動配置或更新 PHP 7 模組載入
    4. 若目標存在 PHP 8.5 與 LAH-API，自動配置 mod_proxy_fcgi 轉發規則
    5. 執行 httpd.exe -t 語法檢驗
    """
    conf_path = os.path.join(apache_dir, "conf", "httpd.conf")
    if not os.path.isfile(conf_path):
        if log_func:
            log_func(f"⚠️ 未找到 httpd.conf：{conf_path}", "warning")
        return False

    try:
        with open(conf_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        orig_content = content
        apache_fwd = apache_dir.replace('\\', '/')

        # 1. 校正 SRVROOT
        if re.search(r'(?m)^\s*Define\s+SRVROOT\b', content):
            content = re.sub(
                r'(?m)^\s*Define\s+SRVROOT\s+.*$',
                f'Define SRVROOT "{apache_fwd}"',
                content
            )
        else:
            content = f'Define SRVROOT "{apache_fwd}"\nServerRoot "${{SRVROOT}}"\n' + content

        # 2. 資安強化
        if enable_security:
            if re.search(r"(?m)^\s*#?\s*ServerTokens\b", content):
                content = re.sub(r"(?m)^\s*#?\s*ServerTokens\s+.*$", "ServerTokens Prod", content)
            else:
                content += "\nServerTokens Prod\n"

            if re.search(r"(?m)^\s*#?\s*ServerSignature\b", content):
                content = re.sub(r"(?m)^\s*#?\s*ServerSignature\s+.*$", "ServerSignature Off", content)
            else:
                content += "\nServerSignature Off\n"

        # 3. 檢查並配置 PHP 7.4 (mod_php7)
        if php7_dir and os.path.isdir(php7_dir):
            p7_fwd = php7_dir.replace('\\', '/')
            p7_dll = os.path.join(php7_dir, "php7apache2_4.dll")
            if os.path.isfile(p7_dll):
                if not re.search(r"(?m)^\s*LoadModule\s+php7_module\b", content):
                    php7_block = (
                        f'\n# =====================================================================\n'
                        f'# PHP 7.4 (mod_php7) 自動掛載設定\n'
                        f'# =====================================================================\n'
                        f'LoadFile "{p7_fwd}/php7ts.dll"\n'
                        f'LoadModule php7_module "{p7_fwd}/php7apache2_4.dll"\n'
                        f'PHPIniDir "{p7_fwd}"\n'
                        f'<FilesMatch \\.php$>\n'
                        f'    SetHandler application/x-httpd-php\n'
                        f'</FilesMatch>\n'
                    )
                    content += php7_block
                    if log_func:
                        log_func("  ✓ 已向 Apache httpd.conf 注入 PHP 7.4 模組載入規則", "info")
                else:
                    content = re.sub(
                        r'(?m)^\s*LoadModule\s+php7_module\s+.*$',
                        f'LoadModule php7_module "{p7_fwd}/php7apache2_4.dll"',
                        content
                    )
                    content = re.sub(
                        r'(?m)^\s*PHPIniDir\s+.*$',
                        f'PHPIniDir "{p7_fwd}"',
                        content
                    )

        # 4. 檢查並配置 PHP 8.5 (LAH-API FastCGI)
        if php85_dir and os.path.isdir(php85_dir):
            api_dir = os.path.join(root_dir, "LAH-API")
            if os.path.isdir(api_dir):
                api_fwd = api_dir.replace('\\', '/')
                content = re.sub(r'(?m)^\s*#\s*(LoadModule\s+proxy_module\s+modules/mod_proxy\.so)', r'\1', content)
                content = re.sub(r'(?m)^\s*#\s*(LoadModule\s+proxy_fcgi_module\s+modules/mod_proxy_fcgi\.so)', r'\1', content)

                fcgi_block = (
                    f'\n# =====================================================================\n'
                    f'# LAH-API (PHP 8.5 FastCGI via PM2 on 127.0.0.1:8085)\n'
                    f'# =====================================================================\n'
                    f'Alias /LAH-API/api "{api_fwd}/v1"\n'
                    f'Alias /lah-api/api "{api_fwd}/v1"\n'
                    f'Alias /LAH-API "{api_fwd}"\n'
                    f'Alias /lah-api "{api_fwd}"\n'
                    f'<Directory "{api_fwd}">\n'
                    f'    Header set Access-Control-Allow-Origin "*"\n'
                    f'    Options -Indexes +FollowSymLinks\n'
                    f'    AllowOverride None\n'
                    f'    Require all granted\n'
                    f'    <FilesMatch "\\.php$">\n'
                    f'        <If "-f %{{REQUEST_FILENAME}}">\n'
                    f'            SetHandler "proxy:fcgi://127.0.0.1:8085/"\n'
                    f'            ProxyFCGISetEnvIf "reqenv(\'SCRIPT_FILENAME\') =~ m#^proxy:fcgi://127\\.0\\.0\\.1:8085/(.*)$#" SCRIPT_FILENAME "$1"\n'
                    f'        </If>\n'
                    f'    </FilesMatch>\n'
                    f'</Directory>\n'
                    f'<Proxy "fcgi://127.0.0.1:8085/" enablereuse=on max=16>\n'
                    f'    ProxySet timeout=300\n'
                    f'</Proxy>\n'
                )
                if "proxy:fcgi://127.0.0.1:8085/" not in content:
                    content += fcgi_block
                    if log_func:
                        log_func("  ✓ 已向 Apache httpd.conf 注入 LAH-API PHP 8.5 FastCGI 轉發規則", "info")

        if content != orig_content:
            bak_path = conf_path + f".bak_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            try:
                shutil.copyfile(conf_path, bak_path)
            except Exception:
                pass
            with open(conf_path, "w", encoding="utf-8") as f:
                f.write(content)
            if log_func:
                log_func("  ✓ Apache httpd.conf 設定已自動更新！", "success")

        check_apache_syntax(apache_dir, log_func)
        return True
    except Exception as e:
        if log_func:
            log_func(f"  ❌ 設定 Apache 發生異常: {e}", "error")
        return False


def _decode_console_bytes(raw_bytes):
    """將子程序輸出的位元組安全解碼為字串（相容 UTF-8 與 Windows 繁中 CP950）"""
    if not raw_bytes:
        return ""
    for enc in ["utf-8", "cp950", "big5"]:
        try:
            text = raw_bytes.decode(enc)
            break
        except UnicodeDecodeError:
            continue
    else:
        text = raw_bytes.decode("utf-8", errors="replace")
    # 移除 ANSI 色彩控制碼，保持 GUI 日誌整潔
    return re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", text)


def resolve_pm2_command(root_dir=None):
    r"""
    解析 Windows 環境下的 pm2 執行檔路徑。
    優先使用綠色便攜環境路徑 (如 C:\AppServ\node\pm2.cmd 或自訂安裝根目錄下的 node\pm2.cmd)，
    其次使用 shutil.which 尋找 pm2.cmd / pm2，若因 UAC 提權導致環境變數遺失，
    則依序檢查常見安裝路徑。
    """
    candidates = []
    if root_dir:
        candidates.append(os.path.join(root_dir, "node", "pm2.cmd"))
    candidates.append(r"C:\AppServ\node\pm2.cmd")

    for path in candidates:
        if path and os.path.isfile(path):
            return path

    found = shutil.which("pm2.cmd") or shutil.which("pm2")
    if found:
        return found

    appdata = os.environ.get("APPDATA", "")
    localappdata = os.environ.get("LOCALAPPDATA", "")
    userprofile = os.environ.get("USERPROFILE", "")
    prog_files = os.environ.get("ProgramFiles", r"C:\Program Files")

    if appdata:
        candidates.append(os.path.join(appdata, "npm", "pm2.cmd"))
    if localappdata:
        candidates.append(os.path.join(localappdata, "Volta", "bin", "pm2.cmd"))
    if userprofile:
        candidates.append(os.path.join(userprofile, "AppData", "Roaming", "npm", "pm2.cmd"))
        candidates.append(os.path.join(userprofile, "AppData", "Local", "Volta", "bin", "pm2.cmd"))
    candidates.append(os.path.join(prog_files, "nodejs", "pm2.cmd"))
    candidates.append(os.path.join(prog_files, "Volta", "pm2.cmd"))
    candidates.append(r"C:\nvm4w\nodejs\pm2.cmd")

    for path in candidates:
        if path and os.path.isfile(path):
            return path

    return None


def register_path_to_user_environment(target_dir, log_func=None):
    """
    將指定目錄自動登錄至 Windows 目前使用者 (HKCU) 的 PATH 環境變數最前端。
    使用 Python 標準庫 winreg 讀寫註冊表，並透過 ctypes 廣播 WM_SETTINGCHANGE。
    """
    if not target_dir or not os.path.isdir(target_dir):
        return False, f"目錄不存在: {target_dir}"

    target_dir_norm = os.path.normpath(target_dir).rstrip("\\/")

    try:
        # 開啟 HKCU\\Environment
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment", 0, winreg.KEY_READ | winreg.KEY_WRITE) as key:
            try:
                cur_val, val_type = winreg.QueryValueEx(key, "Path")
            except FileNotFoundError:
                cur_val = ""
                val_type = winreg.REG_EXPAND_SZ

            parts = [p.strip() for p in cur_val.split(";") if p.strip()]
            parts_norm = [os.path.normpath(p).rstrip("\\/").lower() for p in parts]

            if target_dir_norm.lower() in parts_norm:
                if log_func:
                    log_func(f"ℹ️ 使用者環境變數 PATH 已包含 [{target_dir_norm}]，略過重複寫入。", "info")
                return True, "已存在於 PATH"

            # 插入至最前端以確保優先權
            new_parts = [target_dir_norm] + parts
            new_val = ";".join(new_parts)

            winreg.SetValueEx(key, "Path", 0, val_type, new_val)

        # 廣播 WM_SETTINGCHANGE 通知系統環境變數已更新
        HWND_BROADCAST = 0xFFFF
        WM_SETTINGCHANGE = 0x001A
        SMTO_ABORTIFHUNG = 0x0002
        result = wintypes.DWORD()

        ctypes.windll.user32.SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            "Environment",
            SMTO_ABORTIFHUNG,
            3000,
            ctypes.byref(result)
        )

        # 同步更新當前行程的 os.environ["PATH"]
        cur_sys_path = os.environ.get("PATH", "")
        cur_sys_parts = [os.path.normpath(p).rstrip("\\/").lower() for p in cur_sys_path.split(";") if p.strip()]
        if target_dir_norm.lower() not in cur_sys_parts:
            os.environ["PATH"] = target_dir_norm + ";" + cur_sys_path

        if log_func:
            log_func(f"✅ 已成功將 [{target_dir_norm}] 自動註冊至使用者環境變數 PATH 最前端！", "success")
        return True, "成功登錄 PATH"
    except Exception as e:
        msg = f"寫入使用者環境變數 PATH 失敗: {e}"
        if log_func:
            log_func(f"⚠️ {msg}", "warning")
        return False, msg


def stop_pm2_services(log_func, root_dir=None):
    """執行 pm2 stop all 暫停所有由 PM2 託管的 Node.js 服務（釋放 node.exe 檔案鎖定）"""
    log_func("-" * 50, "info")
    log_func("⏳ 正在執行 [pm2 stop all] 暫停 Node.js 相關服務...", "info")

    pm2_bin = resolve_pm2_command(root_dir)
    if pm2_bin:
        cmd = [pm2_bin, "stop", "all", "--no-color"]
    else:
        cmd = ["cmd.exe", "/d", "/s", "/c", "pm2 stop all --no-color"]

    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            timeout=30,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_text = _decode_console_bytes(res.stdout).strip()
        err_text = _decode_console_bytes(res.stderr).strip()

        if out_text:
            for line in out_text.splitlines():
                if line.strip():
                    log_func(f"  {line.rstrip()}", "info")

        if res.returncode == 0:
            log_func("✅ 已成功執行 [pm2 stop all] 暫停 Node.js 專案服務！", "success")
            return True, ""
        else:
            err_msg = err_text or out_text or f"返回碼 {res.returncode}"
            log_func(f"ℹ️ [pm2 stop all] 回傳狀態 ({res.returncode}): {err_msg}", "info")
            return False, err_msg
    except Exception as e:
        log_func(f"ℹ️ 呼叫 pm2 stop all 略過或異常: {e}", "info")
        return False, str(e)


def save_pm2_services(log_func, root_dir=None):
    """執行 pm2 save 保存當前受控服務清單至備份 dump.pm2"""
    pm2_bin = resolve_pm2_command(root_dir)
    cmd = [pm2_bin, "save", "--force", "--no-color"] if pm2_bin else ["cmd.exe", "/d", "/s", "/c", "pm2 save --force --no-color"]
    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            timeout=30,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_text = _decode_console_bytes(res.stdout).strip()
        if res.returncode == 0:
            log_func("✅ 已成功執行 [pm2 save] 保存當前受控服務清單至備份！", "success")
            return True, ""
        else:
            return False, out_text
    except Exception as e:
        log_func(f"ℹ️ pm2 save 執行提示: {e}", "info")
        return False, str(e)


def kill_pm2_daemon(log_func, root_dir=None):
    """執行 pm2 kill 徹底終止 PM2 守護程序以完全釋放 node.exe"""
    log_func("⏳ 正在執行 [pm2 kill] 終止 PM2 守護程序以完全釋放檔案鎖定...", "warning")

    pm2_bin = resolve_pm2_command(root_dir)
    if pm2_bin:
        cmd = [pm2_bin, "kill", "--no-color"]
    else:
        cmd = ["cmd.exe", "/d", "/s", "/c", "pm2 kill --no-color"]

    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            timeout=30,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_text = _decode_console_bytes(res.stdout).strip()
        if res.returncode == 0:
            log_func("✅ 已成功執行 [pm2 kill] 完全釋放 PM2 行程！", "success")
            time.sleep(1)
            return True, ""
        else:
            return False, out_text
    except Exception as e:
        log_func(f"ℹ️ 呼叫 pm2 kill 異常: {e}", "info")
        return False, str(e)


def resurrect_pm2_services(log_func, root_dir=None):
    """執行 pm2 resurrect 自 dump.pm2 喚醒守護程序並還原所有受控服務清單"""
    log_func("-" * 50, "info")
    log_func("⏳ 正在執行 [pm2 resurrect] 喚醒 PM2 守護程序並還原受管服務...", "info")

    pm2_bin = resolve_pm2_command(root_dir)
    cmd = [pm2_bin, "resurrect", "--no-color"] if pm2_bin else ["cmd.exe", "/d", "/s", "/c", "pm2 resurrect --no-color"]

    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            timeout=60,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_text = _decode_console_bytes(res.stdout).strip()
        err_text = _decode_console_bytes(res.stderr).strip()

        if out_text:
            for line in out_text.splitlines():
                if line.strip():
                    log_func(f"  {line.rstrip()}", "info")

        if res.returncode == 0:
            log_func("✅ 已成功執行 [pm2 resurrect] 喚醒並還原所有 Node.js 專案服務！", "success")
            # 還原後執行全方位核心巡檢補啟動，確保 LAH-WSS、pm2-logrotate 等服務皆完整在線
            return audit_and_ensure_pm2_services(log_func, root_dir)
        else:
            err_msg = err_text or out_text or f"返回碼 {res.returncode}"
            log_func(f"⚠️ [pm2 resurrect] 執行回傳非零狀態 ({res.returncode}): {err_msg}", "warning")
            return False, err_msg
    except Exception as e:
        msg = f"呼叫 pm2 resurrect 時發生異常: {e}"
        log_func(f"❌ {msg}", "error")
        return False, msg


def is_file_locked(filepath):
    """檢測檔案是否已被其他行程開啟鎖定（例如執行中的 .exe）"""
    if not os.path.exists(filepath):
        return False
    try:
        with open(filepath, "a+b") as f:
            pass
        return False
    except OSError:
        return True


def restart_pm2_services(log_func, root_dir=None):
    """執行 pm2 restart all 重啟所有由 PM2 託管的 Node.js 服務（具備 No process found 自動 resurrect 與全方位核心巡檢補啟動機制）"""
    log_func("-" * 50, "info")
    log_func("⏳ 正在執行 [pm2 restart all] 重啟 Node.js 相關服務...", "info")

    pm2_bin = resolve_pm2_command(root_dir)
    if pm2_bin:
        cmd = [pm2_bin, "restart", "all", "--no-color"]
    else:
        cmd = ["cmd.exe", "/d", "/s", "/c", "pm2 restart all --no-color"]

    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            timeout=60,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out_text = _decode_console_bytes(res.stdout).strip()
        err_text = _decode_console_bytes(res.stderr).strip()

        if out_text:
            for line in out_text.splitlines():
                if line.strip():
                    log_func(f"  {line.rstrip()}", "info")

        if res.returncode == 0:
            log_func("✅ 已成功執行 [pm2 restart all] 重啟所有 Node.js 專案服務！", "success")
        else:
            combined_msg = (out_text + " " + err_text).lower()
            if "no process found" in combined_msg:
                log_func("ℹ️ 偵測到 PM2 目前無執行中進程清單 (No process found)，自動切換執行 [pm2 resurrect] 進行還原...", "info")
                resurrect_pm2_services(log_func, root_dir)
            else:
                err_msg = err_text or out_text or f"返回碼 {res.returncode}"
                log_func(f"⚠️ [pm2 restart all] 執行回傳非零狀態 ({res.returncode}): {err_msg}", "warning")
    except subprocess.TimeoutExpired:
        msg = "執行 pm2 restart all 超過 60 秒逾時，請手動確認 PM2 服務狀態。"
        log_func(f"⚠️ {msg}", "warning")
    except Exception as e:
        msg = f"呼叫 pm2 restart all 時發生異常: {e}"
        log_func(f"❌ {msg}", "error")

    # 無論重啟結果為何，執行全方位核心巡檢，自動補齊並拉起遺漏之 LAH-WSS、LAH-FE、LAH-SCHEDULER、lah-api-v85、pm2-logrotate
    audit_ok, audit_msg = audit_and_ensure_pm2_services(log_func, root_dir)
    return audit_ok, audit_msg


def audit_and_ensure_pm2_services(log_func, root_dir=None):
    """
    全方位巡檢 PM2 核心服務與模組（LAH-WSS、LAH-FE、LAH-SCHEDULER、lah-api-v85、pm2-logrotate）。
    1. 自動建立並確保 logs 目錄存在，杜絕日誌路徑缺失導致啟動失敗。
    2. 透過 pm2 jlist 取得目前所有執行中服務清單。
    3. 若發現核心服務遺漏或離線，自動由目標目錄 (如 C:\\AppServ) 之 ecosystem.config.js 補啟動 (pm2 start)。
    4. 若發現 pm2-logrotate 日誌輪替模組遺漏或離線，自動執行 restart/install 補啟動。
    5. 若有補啟動任何服務或模組，自動執行 pm2 save 保存快照至 dump.pm2。
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    logs_dir = os.path.normpath(os.path.join(target_root, "logs"))
    try:
        os.makedirs(logs_dir, exist_ok=True)
    except Exception as e:
        log_func(f"⚠️ 建立日誌目錄 [{logs_dir}] 時發生警示: {e}", "warning")

    pm2_bin = resolve_pm2_command(target_root)
    if not pm2_bin:
        msg = "系統未偵測到有效的 pm2.cmd，跳過 PM2 服務巡檢。"
        log_func(f"⚠️ {msg}", "warning")
        return False, msg

    log_func("-" * 50, "info")
    log_func("🔍 正在執行 PM2 核心服務全方位狀態巡檢...", "info")

    # 1. 取得現有 PM2 程序清單
    managed_apps = {}
    try:
        res = subprocess.run(
            [pm2_bin, "jlist"],
            capture_output=True,
            timeout=20,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if res.returncode == 0:
            out_json = _decode_console_bytes(res.stdout).strip()
            if out_json:
                data = json.loads(out_json)
                for item in data:
                    app_name = item.get("name")
                    status = item.get("pm2_env", {}).get("status", "unknown")
                    if app_name not in managed_apps or status == "online":
                        managed_apps[app_name] = status
    except Exception as e:
        log_func(f"⚠️ 查詢 PM2 進程狀態發生異常: {e}，將嘗試直接補啟動缺漏設定檔...", "warning")

    # 2. 定義 3 大生態系設定檔與所屬服務
    core_ecosystems = [
        {
            "title": "即時通訊伺服器 (LAH-WSS)",
            "apps": ["LAH-WSS"],
            "candidates": [
                os.path.join(target_root, "lah-messenger-server", "ecosystem.config.js"),
                r"c:\Temp\DEV\LAH-Workspace\lah-messenger-server\ecosystem.config.js",
            ]
        },
        {
            "title": "智慧管控前端網站 (LAH-FE / LAH-SCHEDULER)",
            "apps": ["LAH-FE", "LAH-SCHEDULER"],
            "candidates": [
                os.path.join(target_root, "LAH-NUXTJS", "ecosystem.config.js"),
                r"c:\Temp\DEV\LAH-Workspace\LAH-FE\ecosystem.config.js",
            ]
        },
        {
            "title": "現代化後端 API (lah-api-v85)",
            "apps": ["lah-api-v85"],
            "candidates": [
                os.path.join(target_root, "LAH-API", "ecosystem.config.js"),
                r"c:\Temp\DEV\LAH-Workspace\LAH-API\API\ecosystem.config.js",
            ]
        }
    ]

    need_save = False

    for eco in core_ecosystems:
        eco_path = next((p for p in eco["candidates"] if os.path.isfile(p)), None)
        if not eco_path:
            continue

        missing_apps = [a for a in eco["apps"] if managed_apps.get(a) != "online"]
        if missing_apps:
            log_func(f"⚡ 偵測到 [{eco['title']}] 之服務 {missing_apps} 尚未在線，正在啟動 [{eco_path}]...", "warning")
            try:
                start_res = subprocess.run(
                    [pm2_bin, "start", eco_path, "--no-color"],
                    capture_output=True,
                    timeout=30,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
                )
                start_out = _decode_console_bytes(start_res.stdout).strip()
                if start_res.returncode == 0:
                    log_func(f"  ✅ 成功啟動 [{eco['title']}]！", "success")
                    need_save = True
                else:
                    start_err = _decode_console_bytes(start_res.stderr).strip() or start_out
                    log_func(f"  ⚠️ 啟動 [{eco_path}] 回傳警示: {start_err}", "warning")
            except Exception as e:
                log_func(f"  ❌ 啟動 [{eco_path}] 失敗: {e}", "error")

    # 2.1 檢查並確保日誌輪替模組 (pm2-logrotate) 運作
    if managed_apps.get("pm2-logrotate") != "online":
        log_func("⚡ 偵測到日誌輪替模組 [pm2-logrotate] 尚未在線，正在嘗試啟動...", "warning")
        try:
            start_mod = subprocess.run(
                [pm2_bin, "restart", "pm2-logrotate", "--no-color"],
                capture_output=True,
                timeout=30,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            if start_mod.returncode == 0:
                log_func("  ✅ 成功啟動 [pm2-logrotate] 模組！", "success")
                need_save = True
            else:
                # 優先使用綠色目錄中之本機套件進行離線安裝，避免公務內網無外網環境無法自 npm 下載
                local_mod_path = os.path.normpath(os.path.join(target_root, "node", "node_modules", "pm2-logrotate"))
                target_pkg = local_mod_path if os.path.isdir(local_mod_path) else "pm2-logrotate"
                ins_res = subprocess.run(
                    [pm2_bin, "install", target_pkg, "--no-color"],
                    capture_output=True,
                    timeout=60,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
                )
                if ins_res.returncode == 0:
                    log_func("  ✅ 成功註冊並啟動 [pm2-logrotate] 模組！", "success")
                    subprocess.run(
                        [pm2_bin, "set", "pm2-logrotate:compress", "true"],
                        capture_output=True,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
                    )
                    need_save = True
                else:
                    mod_err = _decode_console_bytes(ins_res.stderr).strip() or _decode_console_bytes(ins_res.stdout).strip()
                    log_func(f"  ⚠️ 啟動 [pm2-logrotate] 模組回傳警示: {mod_err}", "warning")
        except Exception as e:
            log_func(f"  ❌ 啟動 [pm2-logrotate] 失敗: {e}", "error")

    # 3. 若有補啟動任何項目，自動持久化保存狀態
    if need_save:
        log_func("💾 偵測到服務配置異動，正在執行 [pm2 save] 更新持久化存檔...", "info")
        save_pm2_services(log_func, target_root)

    # 4. 重新查詢一次最新狀態印出摘要報告
    final_apps = {}
    try:
        res = subprocess.run(
            [pm2_bin, "jlist"],
            capture_output=True,
            timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if res.returncode == 0:
            out_json = _decode_console_bytes(res.stdout).strip()
            if out_json:
                data = json.loads(out_json)
                for item in data:
                    app_name = item.get("name")
                    status = item.get("pm2_env", {}).get("status", "unknown")
                    if app_name not in final_apps or status == "online":
                        final_apps[app_name] = status
    except Exception:
        pass

    log_func("📋 PM2 核心服務現況總覽：", "info")
    all_healthy = True
    for app_name in ["LAH-WSS", "LAH-FE", "LAH-SCHEDULER", "lah-api-v85", "pm2-logrotate"]:
        st = final_apps.get(app_name, "未註冊 / 離線")
        if st == "online":
            log_func(f"  • {app_name:<16} : ✅ {st}", "success")
        else:
            all_healthy = False
            log_func(f"  • {app_name:<16} : ⚠️ {st}", "warning")

    if all_healthy:
        log_func("🎉 全部 5 大核心服務與模組皆已成功在線運作！", "success")
    else:
        log_func("⚠️ 部分核心服務未能順利在線，請檢視上方日誌輸出或終端機 pm2 status。", "warning")

    return True, ""


def query_pm2_daemon_info(root_dir=None):
    """
    精準偵測目前運行中的 PM2 God Daemon 核心進程資訊。
    回傳字典：
    {
        "is_running": bool,
        "pid": int or None,
        "exe_path": str,
        "is_target_node": bool,
        "target_node_path": str
    }
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    target_node = os.path.normpath(os.path.join(target_root, "node", "node.exe"))

    res_dict = {
        "is_running": False,
        "pid": None,
        "exe_path": "",
        "is_target_node": False,
        "target_node_path": target_node
    }

    try:
        ps_cmd = (
            'Get-CimInstance Win32_Process | '
            'Where-Object { $_.Name -like "*node*" -and $_.CommandLine -like "*Daemon.js*" } | '
            'Select-Object -First 1 -Property ProcessId, ExecutablePath | '
            'ConvertTo-Json'
        )
        cmd = ['powershell', '-NoProfile', '-Command', ps_cmd]
        res = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out = res.stdout.strip()
        if out:
            data = json.loads(out)
            if data and isinstance(data, dict):
                pid = data.get("ProcessId")
                exe = data.get("ExecutablePath", "")
                if pid and exe:
                    norm_exe = os.path.normpath(exe)
                    res_dict["is_running"] = True
                    res_dict["pid"] = pid
                    res_dict["exe_path"] = norm_exe
                    res_dict["is_target_node"] = (norm_exe.lower() == target_node.lower())
    except Exception:
        pass

    return res_dict


def check_pm2_autostart_config():
    """
    檢查系統中是否已設定 PM2 開機自啟動（Windows 服務、工作排程器、啟動捷徑）
    """
    results = {
        "services": [],
        "tasks": [],
        "startup_shortcuts": []
    }

    # 1. Windows 服務
    try:
        ps_cmd = (
            'Get-CimInstance Win32_Service | '
            'Where-Object { $_.Name -like "*pm2*" -or $_.PathName -like "*pm2*" } | '
            'Select-Object Name, State, StartMode, PathName | '
            'ConvertTo-Json'
        )
        res = subprocess.run(
            ['powershell', '-NoProfile', '-Command', ps_cmd],
            capture_output=True, text=True, timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out = res.stdout.strip()
        if out:
            data = json.loads(out)
            if isinstance(data, dict):
                results["services"].append(data)
            elif isinstance(data, list):
                results["services"].extend(data)
    except Exception:
        pass

    # 2. 工作排程器
    try:
        ps_cmd = (
            'Get-ScheduledTask | '
            'Where-Object { $_.TaskName -like "*pm2*" } | '
            'Select-Object TaskName, State | '
            'ConvertTo-Json'
        )
        res = subprocess.run(
            ['powershell', '-NoProfile', '-Command', ps_cmd],
            capture_output=True, text=True, timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out = res.stdout.strip()
        if out:
            data = json.loads(out)
            if isinstance(data, dict):
                results["tasks"].append(data)
            elif isinstance(data, list):
                results["tasks"].extend(data)
    except Exception:
        pass

    # 3. 啟動資料夾
    startup_dirs = [
        os.path.join(os.environ.get("ProgramData", r"C:\ProgramData"), r"Microsoft\Windows\Start Menu\Programs\Startup"),
        os.path.join(os.environ.get("APPDATA", ""), r"Microsoft\Windows\Start Menu\Programs\Startup")
    ]
    for sdir in startup_dirs:
        if os.path.isdir(sdir):
            for fname in os.listdir(sdir):
                if "pm2" in fname.lower():
                    results["startup_shortcuts"].append(os.path.join(sdir, fname))

    return results


def align_pm2_daemon_to_portable(log_func, root_dir=None):
    r"""
    將運行中的 PM2 God Daemon 核心進程無痛切換綁定至目標綠色版 Node (如 C:\AppServ\node\node.exe)。
    標準流程：
    0. 註冊 <root_dir>\node 至 Windows 使用者環境變數 PATH 最前端
    1. pm2 save (儲存當前受管程序清單至 dump.pm2)
    2. pm2 kill (結束目前運行於舊 Node 的 PM2 Daemon)
    3. 等候 1.5 秒讓 Windows 完全釋放控制代碼
    4. <root_dir>\node\pm2.cmd resurrect (透過綠色 node 重新喚醒全新 PM2 Daemon 並拉起服務)
    5. <root_dir>\node\pm2.cmd save (儲存最新狀態)
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    target_node_dir = os.path.normpath(os.path.join(target_root, "node"))
    target_pm2 = os.path.normpath(os.path.join(target_node_dir, "pm2.cmd"))
    current_pm2 = resolve_pm2_command(target_root) or "pm2.cmd"

    # 先確保綠色 node 目錄登錄於使用者 PATH 最前端
    register_path_to_user_environment(target_node_dir, log_func)

    log_func("-" * 50, "info")
    log_func("🔄 正在執行 PM2 核心守護進程切換流程 (save ➔ kill ➔ resurrect ➔ save)...", "info")

    # 步驟 1: pm2 save
    log_func("  [1/4] 正在儲存當前 PM2 受控服務清單 (pm2 save)...", "info")
    save_pm2_services(log_func, target_root)

    # 步驟 2: pm2 kill
    log_func("  [2/4] 正在終止舊版 PM2 核心守護進程 (pm2 kill)...", "warning")
    kill_pm2_daemon(log_func, target_root)

    time.sleep(1.5)

    # 步驟 3: target_pm2 resurrect
    log_func("  [3/4] 正在以綠色環境喚醒全新 PM2 核心守護進程 (pm2 resurrect)...", "info")
    res_ok, res_err = resurrect_pm2_services(log_func, target_root)
    if not res_ok:
        log_func(f"  ❌ pm2 resurrect 執行失敗: {res_err}", "error")
        return False, res_err

    # 步驟 4: target_pm2 save
    log_func("  [4/4] 正在儲存新核心狀態 (pm2 save)...", "info")
    save_pm2_services(log_func, target_root)

    # 檢測切換後成果
    info = query_pm2_daemon_info(target_root)
    if info["is_running"] and info["is_target_node"]:
        log_func(f"✅ PM2 核心守護進程已成功綁定至：{info['exe_path']} (PID: {info['pid']})！", "success")
        return True, ""
    elif info["is_running"]:
        msg = f"切換後 PM2 核心仍為: {info['exe_path']}"
        log_func(f"⚠️ {msg}", "warning")
        return False, msg
    else:
        msg = "切換後未偵測到 PM2 Daemon 運行"
        log_func(f"ℹ️ {msg}", "info")
        return True, msg


def ensure_winsw_service_xml(target_root, xml_path):
    r"""
    動態生成或調和 WinSW 服務組態檔 (lah-service.xml)，確保其絕對路徑與 target_root 完全契合。
    """
    node_exe = os.path.normpath(os.path.join(target_root, "node", "node.exe"))
    runner_js = os.path.normpath(os.path.join(target_root, "node", "service", "service_runner.js"))
    node_dir = os.path.normpath(os.path.join(target_root, "node"))
    oci_dir = os.path.normpath(os.path.join(target_root, "instantclient_11_2"))
    log_path = os.path.normpath(os.path.join(target_root, "logs", "service"))
    norm_root = os.path.normpath(target_root)

    path_value = f"{node_dir};{oci_dir};%PATH%" if os.path.isdir(oci_dir) else f"{node_dir};%PATH%"

    xml_content = f"""<service>
  <id>LAH_PM2_Service</id>
  <name>LAH 智慧管控 - PM2 背景服務與守護進程</name>
  <description>開機自動以 {node_exe} 喚醒並守護 LAH 5 大核心服務（LAH-FE、LAH-SCHEDULER、LAH-WSS、lah-api-v85、pm2-logrotate），具備開機免登入自啟與健康巡檢能力。</description>
  <executable>{node_exe}</executable>
  <arguments>{runner_js}</arguments>
  <workingdirectory>{norm_root}</workingdirectory>

  <env name="PATH" value="{path_value}" />

  <logmode>roll-by-size</logmode>
  <logpath>{log_path}</logpath>
  <sizeThreshold>10240</sizeThreshold>
  <keepFiles>10</keepFiles>

  <onfailure action="restart" delay="10 sec"/>
  <onfailure action="restart" delay="30 sec"/>
  <resetfailure>1 hour</resetfailure>
</service>
"""
    need_write = True
    if os.path.isfile(xml_path):
        try:
            with open(xml_path, "r", encoding="utf-8") as f:
                cur_text = f.read()
            if node_exe.lower() in cur_text.lower() and norm_root.lower() in cur_text.lower():
                need_write = False
        except Exception:
            need_write = True

    if need_write:
        try:
            os.makedirs(os.path.dirname(xml_path), exist_ok=True)
            with open(xml_path, "w", encoding="utf-8") as f:
                f.write(xml_content.strip() + "\n")
        except Exception:
            pass


def query_installed_service_info(svc_name="LAH_PM2_Service"):
    r"""
    查詢指定服務在 Windows SCM 中的註冊狀態與 ImagePath 實體路徑
    """
    try:
        ps_cmd = (
            f"Get-CimInstance Win32_Service -Filter \"Name = '{svc_name}'\" -ErrorAction SilentlyContinue | "
            f"Select-Object Name, State, StartMode, PathName | ConvertTo-Json -Compress"
        )
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps_cmd],
            capture_output=True, text=True, timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if res.returncode == 0 and res.stdout.strip():
            raw = json.loads(res.stdout.strip())
            return {
                "exists": True,
                "name": raw.get("Name", svc_name),
                "state": raw.get("State", "Unknown"),
                "start_mode": raw.get("StartMode", "Unknown"),
                "pathname": (raw.get("PathName") or "").strip().strip('"')
            }
    except Exception:
        pass

    try:
        res = subprocess.run(
            ["sc.exe", "qc", svc_name],
            capture_output=True, text=True, timeout=6,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if res.returncode == 0:
            pathname = ""
            for line in res.stdout.splitlines():
                if "BINARY_PATH_NAME" in line:
                    parts = line.split(":", 1)
                    if len(parts) > 1:
                        pathname = parts[1].strip().strip('"')
            return {
                "exists": True,
                "name": svc_name,
                "state": "Unknown",
                "start_mode": "Unknown",
                "pathname": pathname
            }
    except Exception:
        pass

    return {"exists": False, "name": svc_name, "state": "Unknown", "start_mode": "Unknown", "pathname": ""}


def resolve_winsw_service_files(target_root):
    r"""
    解析 WinSW 服務檔案路徑（lah-service.exe, lah-service.xml, service_runner.js）
    若目標目錄缺失，自動嘗試自 C:\AppServ\node\service 或 updater/service 複製模板補齊，
    並自動依據 target_root 調和生成專屬的 lah-service.xml 組態檔。
    """
    node_dir = os.path.normpath(os.path.join(target_root, "node"))
    service_dir = os.path.normpath(os.path.join(node_dir, "service"))
    exe_path = os.path.join(service_dir, "lah-service.exe")
    xml_path = os.path.join(service_dir, "lah-service.xml")
    js_path = os.path.join(service_dir, "service_runner.js")

    # 確保 template 目錄來源
    template_dir = os.path.join(os.path.dirname(__file__), "service")

    # 嘗試尋找並補全 lah-service.exe
    if not os.path.isfile(exe_path):
        candidates = [
            r"C:\AppServ\node\service\lah-service.exe",
            os.path.join(template_dir, "lah-service.exe"),
            os.path.join(template_dir, "WinSW.NET461.exe"),
        ]
        found_exe = next((c for c in candidates if os.path.isfile(c)), None)
        if found_exe:
            try:
                os.makedirs(service_dir, exist_ok=True)
                shutil.copyfile(found_exe, exe_path)
            except Exception:
                pass

    # 確保 js 腳本存在
    try:
        os.makedirs(service_dir, exist_ok=True)
        if not os.path.isfile(js_path) and os.path.isfile(os.path.join(template_dir, "service_runner.js")):
            shutil.copyfile(os.path.join(template_dir, "service_runner.js"), js_path)
    except Exception:
        pass

    # 動態產生/調和 lah-service.xml，確保其路徑 100% 綁定當前 target_root
    ensure_winsw_service_xml(target_root, xml_path)

    return {
        "service_dir": service_dir,
        "exe_path": exe_path,
        "xml_path": xml_path,
        "js_path": js_path,
        "is_available": os.path.isfile(exe_path)
    }


def install_lah_pm2_service(log_func, root_dir=None):
    r"""
    安裝並啟動 LAH_PM2_Service Windows 原生系統服務 (WinSW - 開機免登入自啟)
    具備跨目錄安全移轉重定位 (Service Relocation & Reconciliation) 機制。
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    sf = resolve_winsw_service_files(target_root)
    if not sf["is_available"]:
        return False, "找不到 lah-service.exe 服務包裝器，略過 Windows 原生服務安裝。"

    exe_path = os.path.normpath(sf["exe_path"])
    xml_path = sf["xml_path"]
    ensure_winsw_service_xml(target_root, xml_path)

    log_func("📦 正在配置 Windows 原生系統服務 (LAH_PM2_Service)...", "info")

    # 1. 偵測 SCM 中是否已存在註冊之服務，並比對其實體路徑是否相符
    svc_info = query_installed_service_info("LAH_PM2_Service")
    if svc_info["exists"] and svc_info["pathname"]:
        installed_exe = os.path.normpath(svc_info["pathname"])
        if installed_exe.lower() != exe_path.lower():
            log_func(f"  🔄 偵測到既有系統服務路徑為 [{installed_exe}]，與目標安裝目錄 [{exe_path}] 不符，正在執行安全移轉重定位...", "info")
            # 停止並刪除舊服務以釋放 SCM 命名空間
            subprocess.run(["sc.exe", "stop", "LAH_PM2_Service"], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            time.sleep(1)
            subprocess.run(["sc.exe", "delete", "LAH_PM2_Service"], capture_output=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            time.sleep(1.5)

    try:
        stat_res = subprocess.run(
            [exe_path, "status"],
            capture_output=True, text=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        status = stat_res.stdout.strip()
        if status in ["Started", "Running"]:
            log_func("  ℹ️ 系統服務已在運行中，正在重新啟動以套用最新設定...", "info")
            subprocess.run([exe_path, "restart"], capture_output=True, timeout=20, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            return True, "Windows 原生系統服務已在運行並成功重新啟動！"
        elif status == "Stopped":
            log_func("  ℹ️ 系統服務已註冊，正在啟動服務...", "info")
            subprocess.run([exe_path, "start"], capture_output=True, timeout=20, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            return True, "Windows 原生系統服務已啟動！"

        # 若未安裝則執行 install
        ins_res = subprocess.run(
            [exe_path, "install"],
            capture_output=True, text=True, timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if ins_res.returncode == 0 or "successfully" in ins_res.stdout.lower():
            log_func("  ✅ 成功註冊 Windows 原生系統服務: LAH_PM2_Service！", "success")
            time.sleep(1)
            subprocess.run([exe_path, "start"], capture_output=True, timeout=20, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            log_func("  ✅ 成功啟動系統服務 (開機免登入自動拉起)！", "success")
            return True, "Windows 原生系統服務 (LAH_PM2_Service) 已安裝並啟動"
        else:
            err_msg = ins_res.stderr.strip() or ins_res.stdout.strip()
            log_func(f"  ⚠️ 安裝 Windows 服務回傳警示: {err_msg}", "warning")
            return False, err_msg
    except Exception as e:
        log_func(f"  ❌ 安裝 Windows 服務失敗: {e}", "error")
        return False, str(e)


def uninstall_lah_pm2_service(log_func, root_dir=None):
    r"""
    停止並卸載 LAH_PM2_Service Windows 原生系統服務
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    sf = resolve_winsw_service_files(target_root)
    exe_path = sf["exe_path"]

    if not os.path.isfile(exe_path):
        try:
            res = subprocess.run(["sc.exe", "delete", "LAH_PM2_Service"], capture_output=True, text=True, timeout=8, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            if res.returncode == 0:
                log_func("  ✓ 已透過 sc.exe 移除系統服務 LAH_PM2_Service", "info")
                return True, "已移除 Windows 系統服務"
        except Exception:
            pass
        return True, "未安裝 Windows 系統服務"

    try:
        log_func("🛑 正在停止並卸載 Windows 系統服務 (LAH_PM2_Service)...", "info")
        subprocess.run([exe_path, "stop"], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        un_res = subprocess.run([exe_path, "uninstall"], capture_output=True, text=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        if un_res.returncode == 0 or "successfully" in un_res.stdout.lower():
            log_func("  ✓ 已成功卸載 Windows 原生系統服務！", "info")
            return True, "已成功卸載 Windows 原生系統服務 (LAH_PM2_Service)"
        else:
            subprocess.run(["sc.exe", "delete", "LAH_PM2_Service"], capture_output=True, timeout=8, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            return True, "已卸載 Windows 系統服務"
    except Exception as e:
        log_func(f"  ⚠️ 卸載服務發生例外: {e}", "warning")
        return False, str(e)


def clean_legacy_autostart_entries(log_func, target_root=None):
    r"""
    清理舊版開機啟動資料夾中之 VBS 腳本與工作排程，避免原生服務與使用者登入重複觸發。
    """
    cleaned_items = []

    # 1. 刪除舊版工作排程
    try:
        res = subprocess.run(
            ["schtasks", "/Delete", "/TN", "LAH_PM2_Autostart", "/F"],
            capture_output=True, text=True, timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
        )
        if res.returncode == 0:
            log_func("  ✓ 已清理舊版 Windows 工作排程器: LAH_PM2_Autostart", "info")
            cleaned_items.append("工作排程器")
    except Exception:
        pass

    # 2. 清理各啟動資料夾中的 pm2 相關腳本
    startup_dirs = [
        os.path.join(os.environ.get("APPDATA", ""), r"Microsoft\Windows\Start Menu\Programs\Startup"),
        os.path.join(os.environ.get("ProgramData", r"C:\ProgramData"), r"Microsoft\Windows\Start Menu\Programs\Startup")
    ]
    for sdir in startup_dirs:
        if os.path.isdir(sdir):
            for fname in os.listdir(sdir):
                if "pm2" in fname.lower() and fname.lower().endswith((".vbs", ".bat", ".cmd", ".lnk")):
                    target_file = os.path.join(sdir, fname)
                    try:
                        os.remove(target_file)
                        log_func(f"  ✓ 已自啟動資料夾清理舊版腳本: {target_file}", "info")
                        cleaned_items.append(f"啟動資料夾 ({fname})")
                    except Exception as e:
                        log_func(f"  ⚠️ 清理 {target_file} 失敗: {e}", "warning")

    # 3. 清理 node 目錄下的舊版 pm2-autostart.vbs
    if target_root:
        node_vbs = os.path.join(target_root, "node", "pm2-autostart.vbs")
        if os.path.isfile(node_vbs):
            try:
                os.remove(node_vbs)
                log_func(f"  ✓ 已清理便攜目錄舊版腳本: {node_vbs}", "info")
                cleaned_items.append("便攜目錄腳本")
            except Exception:
                pass

    return cleaned_items


def setup_pm2_autostart(log_func, root_dir=None):
    r"""
    設定 Windows 開機時自動喚醒並守護 PM2 服務。
    優先順序：
    1. 安裝並啟動 Windows 原生系統服務 (LAH_PM2_Service - 開機免登入自啟)
       若成功安裝原生系統服務，自動清除使用者啟動資料夾與排程中的舊版 VBS 避免重複觸發。
    2. 回退模式 (若無原生服務)：建立無黑視窗靜默喚醒備用腳本 (pm2-autostart.vbs)
    3. 部署至使用者啟動資料夾 (%APPDATA%\...\Startup\LAH-PM2-Autostart.vbs)
    4. 註冊 Windows 工作排程器 (ONLOGON) 作為雙重保障
    5. 執行 pm2 save 確保程序清單最新
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    pm2_cmd = resolve_pm2_command(target_root) or os.path.normpath(os.path.join(target_root, "node", "pm2.cmd"))
    node_dir = os.path.dirname(pm2_cmd)

    success_items = []

    # 1. 優先嘗試安裝 Windows 原生系統服務 (WinSW - 開機免登入自啟)
    svc_ok, svc_msg = install_lah_pm2_service(log_func, target_root)
    if svc_ok:
        success_items.append("Windows 原生系統服務 (開機免登入自啟)")
        # 原生服務已就緒，開機無需使用者登入即可在系統層自動啟動
        # 自動清理舊版啟動資料夾中的 VBS 與工作排程，避免使用者登入時重複觸發
        clean_legacy_autostart_entries(log_func, target_root)
    else:
        # 2. 回退模式：若無 WinSW 原生服務，建立無黑視窗 VBScript 腳本
        os.makedirs(node_dir, exist_ok=True)
        vbs_path = os.path.normpath(os.path.join(node_dir, "pm2-autostart.vbs"))
        vbs_content = (
            "' ==============================================================\r\n"
            "' LAH PM2 Background Autostart Script (Silent/Hidden Window)\r\n"
            "' ==============================================================\r\n"
            "Set WshShell = CreateObject(\"WScript.Shell\")\r\n"
            f'WshShell.Run "cmd.exe /c ""{pm2_cmd}"" resurrect", 0, False\r\n'
        )

        try:
            with open(vbs_path, "w", encoding="ascii") as f:
                f.write(vbs_content)
            log_func(f"  ✓ 已建立 PM2 備用靜默喚醒腳本: {vbs_path}", "info")
        except Exception as e:
            log_func(f"  ⚠️ 寫入 {vbs_path} 失敗 ({e})，嘗試寫入至 %USERPROFILE%\\.pm2...", "warning")
            user_pm2_dir = os.path.join(os.environ.get("USERPROFILE", ""), ".pm2")
            os.makedirs(user_pm2_dir, exist_ok=True)
            vbs_path = os.path.normpath(os.path.join(user_pm2_dir, "pm2-autostart.vbs"))
            with open(vbs_path, "w", encoding="ascii") as f:
                f.write(vbs_content)
            log_func(f"  ✓ 已建立備用 PM2 靜默喚醒腳本: {vbs_path}", "info")

        # 3. 部署至使用者啟動資料夾 (Startup Folder)
        appdata = os.environ.get("APPDATA", "")
        if appdata:
            startup_dir = os.path.join(appdata, r"Microsoft\Windows\Start Menu\Programs\Startup")
            if os.path.isdir(startup_dir):
                user_startup_vbs = os.path.join(startup_dir, "LAH-PM2-Autostart.vbs")
                try:
                    shutil.copyfile(vbs_path, user_startup_vbs)
                    log_func(f"  ✓ 已部署啟動資料夾捷徑: {user_startup_vbs}", "success")
                    success_items.append("使用者啟動資料夾")
                except Exception as e:
                    log_func(f"  ⚠️ 寫入使用者啟動資料夾失敗: {e}", "warning")

        # 4. 註冊 Windows 工作排程器 (Task Scheduler) 作為雙重保障
        try:
            task_cmd = [
                "schtasks", "/Create", "/TN", "LAH_PM2_Autostart",
                "/TR", f'wscript.exe "{vbs_path}"',
                "/SC", "ONLOGON", "/RL", "HIGHEST", "/F"
            ]
            res = subprocess.run(
                task_cmd, capture_output=True, text=True, timeout=10,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
            )
            if res.returncode == 0:
                log_func("  ✓ 已註冊 Windows 工作排程器 (最高權限 ONLOGON): LAH_PM2_Autostart", "success")
                success_items.append("工作排程器")
            else:
                task_cmd_low = [
                    "schtasks", "/Create", "/TN", "LAH_PM2_Autostart",
                    "/TR", f'wscript.exe "{vbs_path}"',
                    "/SC", "ONLOGON", "/F"
                ]
                res_low = subprocess.run(
                    task_cmd_low, capture_output=True, text=True, timeout=10,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
                )
                if res_low.returncode == 0:
                    log_func("  ✓ 已註冊 Windows 工作排程器: LAH_PM2_Autostart", "success")
                    success_items.append("工作排程器")
        except Exception as e:
            log_func(f"  ℹ️ 工作排程器註冊略過: {e}", "info")

    # 5. 確保最新程序清單存檔 (pm2 save)
    log_func("  正在儲存當前 PM2 程序清單至 dump.pm2...", "info")
    save_pm2_services(log_func, target_root)

    if success_items:
        msg = f"已成功啟用開機自啟動 ({' + '.join(success_items)})"
        return True, msg
    else:
        return False, "未能建立啟動項目，請確認磁碟寫入與系統權限！"


def remove_pm2_autostart(log_func, root_dir=None):
    r"""
    清除 Windows 開機/登入時之 PM2 自啟動配置（原生系統服務、工作排程器與啟動資料夾）。
    """
    removed_items = []

    # 1. 卸載 Windows 原生系統服務
    svc_ok, svc_msg = uninstall_lah_pm2_service(log_func, root_dir)
    if svc_ok and "未安裝" not in svc_msg:
        removed_items.append("Windows 原生系統服務")

    # 2. 清理舊版啟動捷徑與工作排程
    cleaned = clean_legacy_autostart_entries(log_func, root_dir)
    removed_items.extend(cleaned)

    if removed_items:
        return True, f"已順利停用並清除自啟動項目 ({', '.join(removed_items)})。"
    else:
        return True, "目前系統中無待清除的自啟動項目。"


def test_pm2_autostart_execution(log_func, root_dir=None):
    r"""
    立即測試執行開機喚醒與守護流程，驗證各服務是否在線且各監聽埠正常運作。
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"

    # 1. 若安裝了原生系統服務，先確認服務狀態
    sf = resolve_winsw_service_files(target_root)
    if sf["is_available"]:
        try:
            stat_res = subprocess.run([sf["exe_path"], "status"], capture_output=True, text=True, timeout=8, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            st = stat_res.stdout.strip()
            log_func(f"ℹ️ Windows 原生系統服務狀態: {st}", "info")
            if st not in ["Started", "Running"]:
                log_func("  正在啟動系統服務...", "info")
                subprocess.run([sf["exe_path"], "start"], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        except Exception as e:
            log_func(f"  ⚠️ 查詢服務狀態警示: {e}", "warning")
    else:
        # 備用執行 VBS
        pm2_cmd = resolve_pm2_command(target_root) or os.path.normpath(os.path.join(target_root, "node", "pm2.cmd"))
        node_dir = os.path.dirname(pm2_cmd)
        vbs_path = os.path.normpath(os.path.join(node_dir, "pm2-autostart.vbs"))
        if os.path.exists(vbs_path):
            log_func(f"🚀 正在測試執行靜默喚醒腳本: wscript.exe \"{vbs_path}\"...", "info")
            try:
                subprocess.run(["cscript.exe", "//nologo", vbs_path], check=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            except Exception as e:
                log_func(f"  ⚠️ 執行喚醒腳本警示: {e}", "warning")

    # 2. 執行全方位 5 大核心服務巡檢補啟動
    audit_ok, audit_msg = audit_and_ensure_pm2_services(log_func, target_root)

    # 3. 檢測核心連接埠
    time.sleep(1)
    ports_status = []
    for port in [8080, 8081, 8082, 8085]:
        ps_cmd = f"Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1"
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps_cmd],
            capture_output=True, text=True, timeout=5,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
        )
        if res.stdout.strip():
            ports_status.append(f"Port {port} [正常聆聽中]")
        else:
            ports_status.append(f"Port {port} [未聆聽]")

    log_func(f"✅ 開機與巡檢測試完成！連接埠狀態：{', '.join(ports_status)}", "success")
    return True, f"開機與巡檢測試完成！\n" + "\n".join([f"• {p}" for p in ports_status])


def get_zip_member_path(info):
    """
    取得 ZipInfo 成員的檔名，並處理繁體中文 Windows 舊版壓縮常見的編碼問題 (CP437/CP950/UTF-8)
    """
    if getattr(info, 'flag_bits', 0) & 0x800:
        return info.filename
    try:
        raw_bytes = info.filename.encode('cp437')
        for enc in ['utf-8', 'cp950', 'gbk']:
            try:
                return raw_bytes.decode(enc)
            except UnicodeDecodeError:
                continue
    except Exception:
        pass
    return info.filename


def detect_strip_prefix(zf, expected_folder_name, zip_stem, target_leaf_name="", alt_folders=None):
    """
    【關鍵防呆機制】：
    檢查 zip 壓縮包內部的頂層結構。
    1. 如果壓縮檔頂層已經包含與預期名稱、壓縮檔名或目標目錄同名的單一資料夾，
       且無散落於根目錄的單獨檔案，則傳回該資料夾前綴 (例如 'www/')。
    2. 針對 Apache Lounge 官方壓縮包 (httpd-2.4.*-x64-*.zip)，其頂層通常包含
       'Apache24/' 資料夾與散落的 'ReadMe.txt', '-- Win64 VS18 --' 等說明檔。
       若頂層同時僅有 1 個與預期名稱匹配的資料夾，且該資料夾內具備核心目錄 (如 bin/, conf/)，
       則同樣傳回 'Apache24/' 並於解壓縮迴圈自動略過外層散落說明檔，徹底杜絕 Apache24\\Apache24 雙層巢狀！
    """
    top_dirs = set()
    has_root_files = False
    sub_dirs_by_top = {}

    candidates = {
        expected_folder_name.strip("/\\").lower(),
        zip_stem.strip("/\\").lower()
    }
    if target_leaf_name:
        candidates.add(target_leaf_name.strip("/\\").lower())
    if alt_folders:
        for af in alt_folders:
            if af:
                candidates.add(af.strip("/\\").lower())

    for info in zf.infolist():
        fname = get_zip_member_path(info).replace('\\', '/').strip('/')
        if not fname:
            continue
        # 排除 macOS 特殊隱藏檔
        if fname.startswith('__MACOSX') or fname.endswith('.DS_Store'):
            continue

        parts = fname.split('/')
        # 如果第一層直接是檔案（不是目錄）
        if len(parts) == 1 and not (info.is_dir() or info.filename.endswith('/')):
            has_root_files = True
            continue
        top_d = parts[0]
        top_dirs.add(top_d)
        if len(parts) >= 2:
            sub_dirs_by_top.setdefault(top_d, set()).add(parts[1].lower())

    if len(top_dirs) == 1:
        single_dir = list(top_dirs)[0]
        s_lower = single_dir.lower()
        is_match = (
            s_lower in candidates or
            any(s_lower.startswith(c) or c.startswith(s_lower) for c in candidates) or
            s_lower.startswith("httpd") or s_lower.startswith("apache") or s_lower.startswith("php")
        )
        if is_match:
            if not has_root_files:
                return single_dir + '/'
            # 即使根目錄有 ReadMe.txt 等散落說明檔，只要該唯一資料夾名稱完全吻合預期資料夾 (如 Apache24)
            # 或內部包含 bin/conf/ext 等核心子目錄，即判定為官方封裝外層目錄並予以剝除
            sub_set = sub_dirs_by_top.get(single_dir, set())
            if s_lower in candidates or ("bin" in sub_set and "conf" in sub_set) or ("ext" in sub_set):
                return single_dir + '/'

    return ""


def flatten_nested_apache_dir(apache_dir, log_func=None):
    """
    自動修復 Apache24\\Apache24 雙層巢狀結構：
    若目標 apache_dir 下找不到 bin/httpd.exe，但 apache_dir\\Apache24\\bin\\httpd.exe 存在，
    自動將內層 Apache24 的所有內容提升合併至外層 apache_dir。
    """
    outer_httpd = os.path.join(apache_dir, "bin", "httpd.exe")
    inner_apache = os.path.join(apache_dir, "Apache24")
    inner_httpd = os.path.join(inner_apache, "bin", "httpd.exe")

    if not os.path.isfile(outer_httpd) and os.path.isfile(inner_httpd):
        if log_func:
            log_func(f"🛠️ 偵測到雙層巢狀目錄 [{inner_apache}]，正在自動攤平校正至 [{apache_dir}]...", "warning")
        try:
            for item in os.listdir(inner_apache):
                s_item = os.path.join(inner_apache, item)
                d_item = os.path.join(apache_dir, item)
                if os.path.isdir(s_item):
                    shutil.copytree(s_item, d_item, dirs_exist_ok=True)
                    shutil.rmtree(s_item, ignore_errors=True)
                else:
                    shutil.copy2(s_item, d_item)
                    os.remove(s_item)
            try:
                os.rmdir(inner_apache)
            except Exception:
                shutil.rmtree(inner_apache, ignore_errors=True)
            if log_func:
                log_func(f"  ✓ 已成功將雙層目錄攤平至：{apache_dir}", "success")
            return True
        except Exception as e:
            if log_func:
                log_func(f"  ⚠️ 攤平 Apache24 目錄時發生例外: {e}", "warning")
    return False


def find_latest_backup_zip(search_dirs):
    """在指定目錄清單中搜尋最新的 _LAH_Backup_*.zip 備份壓縮檔"""
    candidates = []
    for d in search_dirs:
        if not d or not os.path.isdir(d):
            continue
        try:
            for fname in os.listdir(d):
                if fname.lower().startswith("_lah_backup_") and fname.lower().endswith(".zip"):
                    full_p = os.path.join(d, fname)
                    if os.path.isfile(full_p):
                        candidates.append((os.path.getmtime(full_p), full_p))
        except Exception:
            pass
    if not candidates:
        return None
    candidates.sort(key=lambda x: x[0], reverse=True)
    return candidates[0][1]


def restore_from_backup_zip(zip_path, root_dir, log_func=None):
    """
    自 _LAH_Backup_*.zip 備份壓縮包還原原始設定檔與資料庫 (.env, AD.php, db/, uploads/, php.ini, httpd.conf)。
    還原後若包含 Apache24/conf/httpd.conf 或 php.ini，會確保路徑與目標 root_dir 對齊。
    """
    if not zip_path or not os.path.isfile(zip_path):
        msg = f"找不到指定的備份壓縮檔：{zip_path}"
        if log_func:
            log_func(f"❌ {msg}", "error")
        return False, msg, 0

    if log_func:
        log_func("-" * 50, "info")
        log_func(f"📦 正在自備份壓縮檔還原系統設定與資料：{os.path.basename(zip_path)} ➔ {root_dir}", "info")

    restored_count = 0
    restored_categories = set()
    norm_root = os.path.normpath(root_dir)

    try:
        with zipfile.ZipFile(zip_path, "r") as zf:
            for m in zf.infolist():
                raw_name = get_zip_member_path(m).replace("\\", "/").lstrip("/")
                if not raw_name or raw_name.startswith("__MACOSX") or raw_name.endswith(".DS_Store"):
                    continue

                dest_path = os.path.normpath(os.path.join(norm_root, raw_name.replace("/", os.sep)))
                # Zip Slip 防護
                if not (dest_path == norm_root or dest_path.startswith(norm_root + os.sep)):
                    continue

                if m.is_dir() or m.filename.endswith("/"):
                    os.makedirs(dest_path, exist_ok=True)
                    continue

                os.makedirs(os.path.dirname(dest_path), exist_ok=True)
                with zf.open(m) as src, open(dest_path, "wb") as dst:
                    shutil.copyfileobj(src, dst)

                try:
                    dt = time.mktime(m.date_time + (0, 0, -1))
                    os.utime(dest_path, (dt, dt))
                except Exception:
                    pass

                restored_count += 1
                top_part = raw_name.split("/")[0]
                if raw_name.endswith(".env") or raw_name.endswith("AD.php") or raw_name.endswith("php.ini") or raw_name.endswith("httpd.conf"):
                    restored_categories.add(raw_name)
                else:
                    restored_categories.add(f"{top_part}/...")

        if log_func:
            for cat in sorted(restored_categories):
                log_func(f"  ✓ 已還原項目: {cat}", "info")
            log_func(f"✅ 備份還原完成！共成功還原 {restored_count} 個設定與資料檔案。", "success")
        return True, f"成功還原 {restored_count} 個檔案", restored_count
    except Exception as e:
        msg = f"還原備份壓縮檔發生異常: {e}"
        if log_func:
            log_func(f"❌ {msg}", "error")
        return False, msg, restored_count


def find_release_notes(app_dir):
    """
    搜尋同目錄下檔名包含「更新說明」的文字檔。
    若有多個符合檔案（例如歷史發布殘留），優先載入最後修改時間 (mtime) 最新的檔案。
    """
    try:
        candidates = []
        for fname in os.listdir(app_dir):
            if fname.lower().endswith('.txt') and '更新說明' in fname:
                full_path = os.path.join(app_dir, fname)
                if os.path.isfile(full_path):
                    mtime = os.path.getmtime(full_path)
                    candidates.append((mtime, fname, full_path))

        if not candidates:
            return None, None

        # 依最後修改時間降冪排序（最新修改的排在最前面）
        candidates.sort(key=lambda x: x[0], reverse=True)

        for _, fname, full_path in candidates:
            for enc in ['utf-8-sig', 'utf-8', 'cp950', 'big5', 'gbk', 'latin-1']:
                try:
                    with open(full_path, 'r', encoding=enc) as f:
                        return fname, f.read()
                except UnicodeDecodeError:
                    continue
            try:
                with open(full_path, 'r', encoding='utf-8', errors='replace') as f:
                    return fname, f.read()
            except Exception:
                pass
    except Exception:
        pass
    return None, None


class UpdaterApp:
    def __init__(self, root):
        self.root = root
        self.root.title("智慧控管與監控系統 - 自動更新工具")
        self.root.geometry("1260x760")
        self.root.minsize(1100, 740)

        # 基礎路徑
        self.app_dir = get_app_dir()
        self.is_updating = False
        self.apache_service_name = None
        self.pm2_path = None
        self.release_note_name = None
        self.release_note_content = None

        # 文字放大 25%：縮放 Tkinter 內建 Named Fonts
        for name in ("TkDefaultFont", "TkTextFont", "TkFixedFont", "TkMenuFont", "TkHeadingFont", "TkCaptionFont", "TkSmallCaptionFont", "TkIconFont", "TkTooltipFont"):
            try:
                f = tkfont.nametofont(name)
                sz = f.cget("size")
                if sz != 0:
                    sign = 1 if sz > 0 else -1
                    f.configure(size=int(round(abs(sz) * 1.25)) * sign)
            except Exception:
                pass

        # 樣式設定
        self.style = ttk.Style()
        try:
            self.style.theme_use("vista")
        except Exception:
            pass

        self.style.configure(".", font=("Microsoft JhengHei UI", 11))
        self.style.configure("TLabelframe.Label", font=("Microsoft JhengHei UI", 11, "bold"))
        self.style.configure("TButton", font=("Microsoft JhengHei UI", 11))
        self.style.configure("TCheckbutton", font=("Microsoft JhengHei UI", 11))

        self._init_variables()
        self._build_ui()
        self._detect_apache_service()
        self._detect_pm2_service()
        self._auto_detect_backup_zip()
        self.rescan()
        # 啟動時偵測預設安裝目錄是否存在，若無則提示使用者選擇
        self._check_default_root_dir()

    def _init_variables(self):
        # 根目錄路徑
        self.root_dir_var = tk.StringVar(value=r"C:\AppServ")
        self.root_dir_var.trace_add("write", self._on_root_dir_changed)

        # 備份還原選項與路徑變數
        self.restore_backup_var = tk.BooleanVar(value=False)
        self.backup_zip_path_var = tk.StringVar(value="")

        # 各壓縮包狀態變數
        self.pkg_vars = {}
        for pkg in PACKAGE_CONFIGS:
            self.pkg_vars[pkg["id"]] = {
                "checked": tk.BooleanVar(value=False),
                "exists": False,
                "filepath": "",
                "all_filepaths": [],
                "filesize": 0,
                "status_var": tk.StringVar(value="⚪ 偵測中..."),
                "target_path_var": tk.StringVar(value=""),
                "is_custom_path": False
            }

        # 左側基礎環境選項變數
        self.auto_register_apache_var = tk.BooleanVar(value=True)
        self.auto_security_hardening_var = tk.BooleanVar(value=True)

        # Apache 自動重啟勾選（預設留白，偵測到本機服務時自動勾選）
        self.restart_apache_var = tk.BooleanVar(value=False)

        # PM2 自動重啟勾選（預設留白，偵測到本機 PM2 套件時自動勾選）
        self.restart_pm2_var = tk.BooleanVar(value=False)
        self.pm2_daemon_status_var = tk.StringVar(value="⚪ 偵測中...")

        # 顯示全部安裝包（含未偵測到項目）核取變數（預設 False：未偵測到則隱藏選項）
        self.show_all_packages_var = tk.BooleanVar(value=False)

        # 進度百分比與狀態文字
        self.progress_percent_var = tk.StringVar(value="0%")
        self.progress_status_var = tk.StringVar(value="準備就緒，請確認更新項目後點選「開始更新」")

        # 執行日誌歷史與彈窗元件參照
        self.log_history = []
        self.progress_dlg = None
        self.progress_log_box = None
        self.progress_bar = None
        self.btn_close_progress = None

    def _build_ui(self):
        # 主容器
        main_frame = ttk.Frame(self.root, padding="10")
        main_frame.pack(fill=tk.BOTH, expand=True)

        # 1. 頂部標題區塊 (side=tk.TOP)
        header_frame = ttk.Frame(main_frame)
        header_frame.pack(side=tk.TOP, fill=tk.X, pady=(0, 4))

        title_label = ttk.Label(
            header_frame,
            text="🚀 智慧控管與監控系統 - 自動更新工具",
            font=("Microsoft JhengHei UI", 19, "bold")
        )
        title_label.pack(side=tk.LEFT)

        admin_badge = "👑 管理員權限" if is_admin() else "⚠️ 一般使用者權限"
        badge_fg = "#2e7d32" if is_admin() else "#d32f2f"
        admin_label = tk.Label(
            header_frame,
            text=admin_badge,
            fg=badge_fg,
            font=("Microsoft JhengHei UI", 11, "bold")
        )
        admin_label.pack(side=tk.RIGHT, padx=5)

        # 2. 安裝根目錄與備份設定還原區塊 (side=tk.TOP)
        dir_frame = ttk.LabelFrame(main_frame, text=" 📂 安裝根目錄與備份設定還原 ", padding="6")
        dir_frame.pack(side=tk.TOP, fill=tk.X, pady=(0, 4))

        dir_inner = ttk.Frame(dir_frame)
        dir_inner.pack(fill=tk.X, pady=(0, 3))

        ttk.Label(dir_inner, text="系統安裝根目錄：", font=("Microsoft JhengHei UI", 11, "bold")).pack(side=tk.LEFT, padx=(0, 6))
        self.root_entry = ttk.Entry(dir_inner, textvariable=self.root_dir_var, font=("Consolas", 12))
        self.root_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 8))

        self.btn_browse = ttk.Button(dir_inner, text="瀏覽...", command=self._browse_root_dir, width=10)
        self.btn_browse.pack(side=tk.RIGHT)

        # 第 2 列：從備份 ZIP 包還原原始設定與資料
        backup_inner = ttk.Frame(dir_frame)
        backup_inner.pack(fill=tk.X)

        self.chk_restore_backup = ttk.Checkbutton(
            backup_inner,
            text="📦 安裝/更新後自動從備份包回復原始設定：",
            variable=self.restore_backup_var
        )
        self.chk_restore_backup.pack(side=tk.LEFT, padx=(0, 4))

        self.backup_entry = ttk.Entry(backup_inner, textvariable=self.backup_zip_path_var, font=("Consolas", 11))
        self.backup_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 6))

        self.btn_restore_now = ttk.Button(
            backup_inner,
            text="⚡ 立即單獨還原備份",
            command=self.manual_restore_backup,
            width=18
        )
        self.btn_restore_now.pack(side=tk.RIGHT)

        self.btn_browse_backup = ttk.Button(
            backup_inner,
            text="選擇備份檔...",
            command=self._browse_backup_zip,
            width=12
        )
        self.btn_browse_backup.pack(side=tk.RIGHT, padx=(0, 6))

        # 3. 雙欄卡片式更新清單區塊 (左右雙欄格局)
        pkg_frame = ttk.Frame(main_frame)
        pkg_frame.pack(side=tk.TOP, fill=tk.BOTH, expand=True, pady=(0, 4))

        self.chk_widgets = {}
        self.path_entries = {}
        self.browse_buttons = {}
        self.reset_buttons = {}

        # =============================================================
        # 左側欄位：基礎環境與 Web 伺服器 (Apache / PHP)
        # =============================================================
        left_col = ttk.LabelFrame(pkg_frame, text=" 🌐 基礎環境與 Web 伺服器 (Apache / PHP) ", padding="6")
        left_col.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(0, 4))

        left_hint = ttk.Label(
            left_col,
            text=f"來源：程式目錄下的 x64 安裝包 (支援 httpd-2.4.*.zip, php-7.4.*.zip, php-8.*.zip)",
            foreground="#555555",
            font=("Microsoft JhengHei UI", 10)
        )
        left_hint.pack(anchor=tk.W, pady=(0, 2))

        # 1. 左欄底部全域控制列 (優先 pack 於 BOTTOM，固定於左欄最底端)
        # 採用雙列平衡佈局，垂直高度與右欄 PM2 控制項完美對齊
        left_action_frame = ttk.Frame(left_col, padding="2")
        left_action_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(4, 0))

        # 輔助列 (上列)：顯示全部安裝包核取框 (靠左) + 檢視日誌與查看說明按鈕 (靠右)
        action_row_top = ttk.Frame(left_action_frame)
        action_row_top.pack(fill=tk.X, pady=(1, 2))

        self.chk_show_all = ttk.Checkbutton(
            action_row_top,
            text="👁️ 顯示全部安裝包 (含未偵測到)",
            variable=self.show_all_packages_var,
            command=self._update_package_cards_visibility
        )
        self.chk_show_all.pack(side=tk.LEFT, padx=(2, 0))

        self.btn_view_log = ttk.Button(
            action_row_top,
            text="📊 檢視執行日誌",
            command=self._open_progress_dialog,
            width=14
        )
        self.btn_view_log.pack(side=tk.RIGHT, padx=(4, 0))

        self.btn_view_notes = ttk.Button(
            action_row_top,
            text="📋 查看更新說明",
            command=self._show_release_notes_dialog,
            state="disabled",
            width=14
        )
        self.btn_view_notes.pack(side=tk.RIGHT)

        # 主行動列 (下列)：關閉離開按鈕 (靠左) + 🚀 開始更新主按鈕 (靠右)
        action_row_bottom = ttk.Frame(left_action_frame)
        action_row_bottom.pack(fill=tk.X, pady=(2, 0))

        self.btn_exit = ttk.Button(
            action_row_bottom,
            text="關閉離開",
            command=self._on_close,
            width=12
        )
        self.btn_exit.pack(side=tk.LEFT)

        self.btn_start = tk.Button(
            action_row_bottom,
            text="🚀 開始更新",
            command=self.start_update_thread,
            font=("Microsoft JhengHei UI", 12, "bold"),
            bg="#1976d2",
            fg="#ffffff",
            activebackground="#1565c0",
            activeforeground="#ffffff",
            relief=tk.FLAT,
            padx=16,
            pady=3,
            cursor="hand2"
        )
        self.btn_start.pack(side=tk.RIGHT)

        # 2. 左欄卡片容器 (依序 pack 基礎環境卡片)
        self.left_cards_frame = ttk.Frame(left_col)
        self.left_cards_frame.pack(side=tk.TOP, fill=tk.X)

        self.lbl_no_pkg_left = tk.Label(
            self.left_cards_frame,
            text="ℹ️ 目前目錄未偵測到基礎環境安裝包\n(如需檢視請勾選下方「👁️ 顯示全部安裝包」)",
            font=("Microsoft JhengHei UI", 10),
            fg="#777777",
            justify=tk.CENTER
        )

        for pkg in BASE_ENV_CONFIGS:
            self._create_pkg_card(self.left_cards_frame, pkg)

        # 3. Apache 服務與資安防護設定區塊 (上移緊接在 PHP 8.5 卡片正下方)
        left_sec_frame = ttk.LabelFrame(left_col, text=" 🛡️ Apache 服務與資安防護設定 ", padding="4")
        left_sec_frame.pack(side=tk.TOP, fill=tk.X, pady=(6, 0))

        # 第 1 列：Apache 服務重啟選項 + 重新掃描按鈕
        ctrl_row_apache = ttk.Frame(left_sec_frame, padding="1")
        ctrl_row_apache.pack(fill=tk.X, pady=(1, 2))

        self.chk_apache = ttk.Checkbutton(
            ctrl_row_apache,
            text="更新前後自動重啟 Apache 服務 (解壓縮前停止、完成後啟動)",
            variable=self.restart_apache_var,
            onvalue=True,
            offvalue=False
        )
        self.chk_apache.pack(side=tk.LEFT)

        self.btn_rescan = ttk.Button(ctrl_row_apache, text="🔄 重新掃描", command=self.rescan, width=12)
        self.btn_rescan.pack(side=tk.RIGHT)

        chk_reg_apache = ttk.Checkbutton(
            left_sec_frame,
            text="若未安裝 Apache 服務則自動註冊 (Apache2.4)",
            variable=self.auto_register_apache_var
        )
        chk_reg_apache.pack(anchor=tk.W, pady=(1, 2))

        chk_sec = ttk.Checkbutton(
            left_sec_frame,
            text="安裝/更新時自動套用資安防護 (ServerTokens Prod / expose_php = Off)",
            variable=self.auto_security_hardening_var
        )
        chk_sec.pack(anchor=tk.W, pady=(1, 2))

        # 獨立按鈕：手動一鍵套用資安弱點防護
        self.btn_sec_harden = ttk.Button(
            left_sec_frame,
            text="🛡️ 一鍵套用資安弱點防護 (隱藏 Apache / PHP 版本資訊)",
            command=self.manual_apply_security_hardening
        )
        self.btn_sec_harden.pack(fill=tk.X, pady=(3, 2))

        # =============================================================
        # 右側欄位：智慧控管系統模組更新 (LAH 應用與套件)
        # =============================================================
        right_col = ttk.LabelFrame(pkg_frame, text=" 📦 智慧控管系統模組更新 (LAH 應用與套件) ", padding="6")
        right_col.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True, padx=(4, 0))

        right_hint = ttk.Label(
            right_col,
            text=f"來源更新目錄：{self.app_dir}  (各模組路徑預設跟隨根目錄，亦可單獨點擊「瀏覽...」自訂)",
            foreground="#555555",
            font=("Microsoft JhengHei UI", 10)
        )
        right_hint.pack(anchor=tk.W, pady=(0, 2))

        # 右欄底部控制選項 (優先 pack 於 BOTTOM)
        right_ctrl_frame = ttk.Frame(right_col, padding="2")
        right_ctrl_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(4, 0))

        # 第 1 列：PM2 管理套件重啟服務選項 + 開機自啟動設定按鈕
        ctrl_row_pm2_top = ttk.Frame(right_ctrl_frame, padding="1")
        ctrl_row_pm2_top.pack(fill=tk.X, pady=(1, 2))

        self.chk_pm2 = ttk.Checkbutton(
            ctrl_row_pm2_top,
            text="更新後自動重啟 PM2 服務",
            variable=self.restart_pm2_var,
            onvalue=True,
            offvalue=False
        )
        self.chk_pm2.pack(side=tk.LEFT)

        self.btn_autostart_pm2 = ttk.Button(
            ctrl_row_pm2_top,
            text="⚙️ 開機自啟動設定",
            command=self.manage_pm2_autostart,
            width=16
        )
        self.btn_autostart_pm2.pack(side=tk.RIGHT)

        # 第 2 列：PM2 核心守護進程狀態標籤 + 立即校正核心按鈕
        ctrl_row_pm2_bottom = ttk.Frame(right_ctrl_frame, padding="1")
        ctrl_row_pm2_bottom.pack(fill=tk.X, pady=(1, 2))

        self.lbl_pm2_status = tk.Label(
            ctrl_row_pm2_bottom,
            textvariable=self.pm2_daemon_status_var,
            font=("Microsoft JhengHei UI", 10, "bold"),
            fg="#555555"
        )
        self.lbl_pm2_status.pack(side=tk.LEFT, padx=(2, 4))

        self.btn_align_pm2 = ttk.Button(
            ctrl_row_pm2_bottom,
            text="⚡ 立即校正 PM2 核心",
            command=self.manual_align_pm2_daemon,
            width=18
        )
        self.btn_align_pm2.pack(side=tk.RIGHT)

        # 右欄卡片容器與未偵測到提示
        self.right_cards_frame = ttk.Frame(right_col)
        self.right_cards_frame.pack(side=tk.TOP, fill=tk.BOTH, expand=True)

        self.lbl_no_pkg_right = tk.Label(
            self.right_cards_frame,
            text="ℹ️ 目前目錄未偵測到系統模組更新包\n(如需檢視請勾選下方「👁️ 顯示全部安裝包」)",
            font=("Microsoft JhengHei UI", 10),
            fg="#777777",
            justify=tk.CENTER
        )

        for pkg in APP_PACKAGE_CONFIGS:
            self._create_pkg_card(self.right_cards_frame, pkg)

        # 初始依狀態切換卡片可見度
        self._update_package_cards_visibility()

        self.root.protocol("WM_DELETE_WINDOW", self._on_close)

    def _create_pkg_card(self, parent_frame, pkg):
        """建立單一更新包項目元件卡片（支援目標路徑自訂與附屬套件縮排）"""
        p_id = pkg["id"]
        v = self.pkg_vars[p_id]
        is_sub = pkg.get("is_sub_package", False)

        card_padding = (22, 0, 0, 1) if is_sub else (0, 0, 0, 1)
        item_card = ttk.Frame(parent_frame, padding=card_padding)
        v["card_frame"] = item_card

        top_line = ttk.Frame(item_card)
        top_line.pack(fill=tk.X)

        chk = ttk.Checkbutton(
            top_line,
            text=pkg["title"],
            variable=v["checked"]
        )
        chk.pack(side=tk.LEFT)
        self.chk_widgets[p_id] = chk

        status_lbl = tk.Label(
            top_line,
            textvariable=v["status_var"],
            font=("Microsoft JhengHei UI", 11, "bold"),
            anchor="e"
        )
        status_lbl.pack(side=tk.RIGHT, padx=4)
        v["status_label_widget"] = status_lbl

        bottom_line = ttk.Frame(item_card, padding=(24, 1, 0, 0))
        bottom_line.pack(fill=tk.X)

        ttk.Label(
            bottom_line,
            text="目標路徑：",
            font=("Microsoft JhengHei UI", 10),
            foreground="#555555"
        ).pack(side=tk.LEFT)

        path_entry = ttk.Entry(
            bottom_line,
            textvariable=v["target_path_var"],
            font=("Consolas", 11)
        )
        path_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(2, 6))
        path_entry.bind("<KeyRelease>", lambda e, pid=p_id: self._on_module_path_manual_edit(pid))
        self.path_entries[p_id] = path_entry

        btn_browse_pkg = ttk.Button(
            bottom_line,
            text="瀏覽...",
            width=7,
            command=lambda pid=p_id: self._browse_pkg_target_dir(pid)
        )
        btn_browse_pkg.pack(side=tk.LEFT, padx=(0, 4))
        self.browse_buttons[p_id] = btn_browse_pkg

        btn_reset_pkg = ttk.Button(
            bottom_line,
            text="↺ 重設",
            width=6,
            command=lambda pid=p_id: self._reset_pkg_target_dir(pid)
        )
        btn_reset_pkg.pack(side=tk.LEFT)
        self.reset_buttons[p_id] = btn_reset_pkg

    def _update_package_cards_visibility(self):
        """根據 show_all_packages_var 與各模組 exists 狀態切換更新卡片顯示/隱藏"""
        show_all = self.show_all_packages_var.get()

        # 處理左欄 (基礎環境)
        for pkg in BASE_ENV_CONFIGS:
            v = self.pkg_vars.get(pkg["id"])
            if v and "card_frame" in v:
                v["card_frame"].pack_forget()

        left_visible_count = 0
        for pkg in BASE_ENV_CONFIGS:
            v = self.pkg_vars.get(pkg["id"])
            if v and "card_frame" in v:
                if show_all or v.get("exists", False):
                    v["card_frame"].pack(fill=tk.X, pady=(0, 1))
                    left_visible_count += 1

        if hasattr(self, 'lbl_no_pkg_left'):
            if left_visible_count == 0:
                self.lbl_no_pkg_left.pack(fill=tk.X, expand=True, pady=40)
            else:
                self.lbl_no_pkg_left.pack_forget()

        # 處理右欄 (系統模組)
        for pkg in APP_PACKAGE_CONFIGS:
            v = self.pkg_vars.get(pkg["id"])
            if v and "card_frame" in v:
                v["card_frame"].pack_forget()

        right_visible_count = 0
        for pkg in APP_PACKAGE_CONFIGS:
            v = self.pkg_vars.get(pkg["id"])
            if v and "card_frame" in v:
                if show_all or v.get("exists", False):
                    v["card_frame"].pack(fill=tk.X, pady=(0, 1))
                    right_visible_count += 1

        if hasattr(self, 'lbl_no_pkg_right'):
            if right_visible_count == 0:
                self.lbl_no_pkg_right.pack(fill=tk.X, expand=True, pady=40)
            else:
                self.lbl_no_pkg_right.pack_forget()

    def _show_release_notes_dialog(self):
        """彈出獨立視窗顯示版本更新說明內容"""
        if not self.release_note_content:
            messagebox.showinfo("版本更新說明", "目前同目錄下未偵測到任何版本更新說明文字檔。")
            return

        dlg = tk.Toplevel(self.root)
        dlg.title(f"📋 版本更新說明 - {self.release_note_name}")
        dlg.geometry("720x560")
        dlg.minsize(600, 420)
        dlg.transient(self.root)

        # 嘗試置中於主視窗上方
        try:
            self.root.update_idletasks()
            rx = self.root.winfo_rootx()
            ry = self.root.winfo_rooty()
            rw = self.root.winfo_width()
            rh = self.root.winfo_height()
            dx = max(30, rx + (rw - 720) // 2)
            dy = max(30, ry + (rh - 560) // 2)
            dlg.geometry(f"720x560+{dx}+{dy}")
        except Exception:
            pass

        content_frame = ttk.Frame(dlg, padding="14")
        content_frame.pack(fill=tk.BOTH, expand=True)

        # 頂部檔案提示列
        top_frame = ttk.Frame(content_frame)
        top_frame.pack(fill=tk.X, pady=(0, 8))
        ttk.Label(
            top_frame,
            text=f"📄 更新說明檔案：{self.release_note_name}",
            font=("Microsoft JhengHei UI", 12, "bold")
        ).pack(side=tk.LEFT)

        # 說明文字滾動顯示區
        txt = scrolledtext.ScrolledText(
            content_frame,
            wrap=tk.WORD,
            font=("Microsoft JhengHei UI", 12),
            bg="#fbfbfb",
            fg="#222222",
            relief=tk.SOLID,
            borderwidth=1
        )
        txt.pack(fill=tk.BOTH, expand=True, pady=(0, 10))
        txt.insert(tk.END, self.release_note_content.strip())
        txt.configure(state="disabled")

        # 底部按鈕列
        btn_frame = ttk.Frame(content_frame)
        btn_frame.pack(fill=tk.X)
        btn_close = ttk.Button(btn_frame, text="關閉 (Esc)", command=dlg.destroy, width=12)
        btn_close.pack(side=tk.RIGHT)

        dlg.bind("<Escape>", lambda e: dlg.destroy())
        dlg.focus_set()

    def _open_progress_dialog(self):
        """開啟獨立的更新進度與執行日誌視窗（較大空間顯示即時進度與文字 LOG）"""
        if self.progress_dlg and self.progress_dlg.winfo_exists():
            self.progress_dlg.lift()
            self.progress_dlg.focus_set()
            return

        self.progress_dlg = tk.Toplevel(self.root)
        self.progress_dlg.title("📊 更新進度與執行日誌")
        self.progress_dlg.geometry("860x600")
        self.progress_dlg.minsize(720, 480)
        self.progress_dlg.transient(self.root)

        # 嘗試置中於主視窗上方
        try:
            self.root.update_idletasks()
            rx = self.root.winfo_rootx()
            ry = self.root.winfo_rooty()
            rw = self.root.winfo_width()
            rh = self.root.winfo_height()
            dx = max(20, rx + (rw - 860) // 2)
            dy = max(20, ry + (rh - 600) // 2)
            self.progress_dlg.geometry(f"860x600+{dx}+{dy}")
        except Exception:
            pass

        dlg_frame = ttk.Frame(self.progress_dlg, padding="14")
        dlg_frame.pack(fill=tk.BOTH, expand=True)

        # 1. 頂部狀態列與進度百分比
        info_frame = ttk.Frame(dlg_frame)
        info_frame.pack(fill=tk.X, pady=(0, 6))

        status_lbl = ttk.Label(
            info_frame,
            textvariable=self.progress_status_var,
            font=("Microsoft JhengHei UI", 12, "bold")
        )
        status_lbl.pack(side=tk.LEFT, fill=tk.X, expand=True)

        percent_lbl = ttk.Label(
            info_frame,
            textvariable=self.progress_percent_var,
            font=("Consolas", 15, "bold"),
            foreground="#1976d2"
        )
        percent_lbl.pack(side=tk.RIGHT)

        # 2. 進度條
        self.progress_bar = ttk.Progressbar(dlg_frame, mode="determinate")
        self.progress_bar.pack(fill=tk.X, pady=(0, 8))

        # 嘗試套用目前進度數值
        try:
            cur_pct = float(self.progress_percent_var.get().replace('%', ''))
            self.progress_bar["value"] = cur_pct
        except Exception:
            pass

        # 3. 獨立大尺寸即時日誌視窗 (ScrolledText)
        log_frame = ttk.LabelFrame(dlg_frame, text=" 📜 即時執行日誌 ", padding="6")
        log_frame.pack(fill=tk.BOTH, expand=True, pady=(0, 8))

        self.progress_log_box = scrolledtext.ScrolledText(
            log_frame,
            wrap=tk.WORD,
            font=("Consolas", 11),
            bg="#1e1e1e",
            fg="#d4d4d4",
            insertbackground="#ffffff"
        )
        self.progress_log_box.pack(fill=tk.BOTH, expand=True)

        # 日誌標籤樣式
        self.progress_log_box.tag_config("info", foreground="#9cdcfe")
        self.progress_log_box.tag_config("success", foreground="#4ec9b0")
        self.progress_log_box.tag_config("warning", foreground="#ce9178")
        self.progress_log_box.tag_config("error", foreground="#f44747")
        self.progress_log_box.tag_config("time", foreground="#6a9955")

        # 填入自啟動以來所記錄的歷史日誌
        for now_str, msg, level in self.log_history:
            self.progress_log_box.insert(tk.END, now_str, "time")
            self.progress_log_box.insert(tk.END, msg + "\n", level)
        self.progress_log_box.see(tk.END)

        # 4. 底部控制按鈕列
        dlg_bottom = ttk.Frame(dlg_frame)
        dlg_bottom.pack(fill=tk.X)

        if self.is_updating:
            btn_text = "更新進行中，請稍候..."
            btn_state = "disabled"
        else:
            btn_text = "關閉視窗 (Esc)"
            btn_state = "normal"
            self.progress_dlg.bind("<Escape>", lambda e: self._close_progress_dialog())

        self.btn_close_progress = ttk.Button(
            dlg_bottom,
            text=btn_text,
            command=self._close_progress_dialog,
            state=btn_state,
            width=22
        )
        self.btn_close_progress.pack(side=tk.RIGHT)

        self.progress_dlg.protocol("WM_DELETE_WINDOW", self._on_progress_dlg_close)
        self.progress_dlg.focus_set()

    def _on_progress_dlg_close(self):
        """處理進度視窗點選右上角關閉"""
        if self.is_updating:
            messagebox.showwarning(
                "更新進行中",
                "更新作業正在背景執行中，請稍候完成。\n完成後即可點選關閉視窗。",
                parent=self.progress_dlg
            )
            return
        self._close_progress_dialog()

    def _close_progress_dialog(self):
        """關閉進度視窗並清除元件參照"""
        if self.is_updating:
            return
        if self.progress_dlg and self.progress_dlg.winfo_exists():
            self.progress_dlg.destroy()
        self.progress_dlg = None
        self.progress_log_box = None
        self.progress_bar = None
        self.btn_close_progress = None

    def _on_update_completed_ui(self):
        """更新完成或出錯時，恢復主視窗狀態並啟用進度彈窗關閉按鈕"""
        self.set_ui_updating_state(False)
        if self.btn_close_progress and self.btn_close_progress.winfo_exists():
            self.btn_close_progress.configure(
                text="✅ 完成並關閉 (Esc)",
                state="normal"
            )
        if self.progress_dlg and self.progress_dlg.winfo_exists():
            self.progress_dlg.bind("<Escape>", lambda e: self._close_progress_dialog())
            self.progress_dlg.lift()

    def _check_default_root_dir(self):
        r"""
        程式啟動時偵測預設根目錄 (如 C:\AppServ) 是否存在。
        若找不到，在視窗顯示後彈出提示對話框，引導使用者立即選取系統安裝根目錄。
        """
        default_dir = self.root_dir_var.get().strip()
        if not default_dir or not os.path.exists(default_dir):
            def _prompt():
                msg = (
                    f"⚠️ 偵測到預設安裝目錄不存在：\n【{default_dir}】\n\n"
                    "若您的系統安裝於其他磁碟機（例如 D:\\AppServ 或自訂目錄），\n"
                    "是否立即瀏覽並選取正確的安裝根目錄？\n\n"
                    "（點選「是」開啟瀏覽對話框；點選「否」亦可稍後於介面手動輸入，或為各模組個別自訂路徑）"
                )
                self.log(f"⚠️ 預設安裝目錄 [{default_dir}] 不存在，請確認安裝路徑。", "warning")
                if messagebox.askyesno("安裝目錄提示", msg, icon=messagebox.WARNING):
                    self._browse_root_dir()

            self.root.after(250, _prompt)

    def _browse_root_dir(self):
        """開啟資料夾瀏覽選擇器設定安裝根目錄"""
        current = self.root_dir_var.get().strip()
        init_dir = current if (current and os.path.exists(current)) else "C:\\"
        selected = filedialog.askdirectory(initialdir=init_dir, title="請選擇系統安裝根目錄")
        if selected:
            norm = os.path.normpath(selected)
            self.root_dir_var.set(norm)
            self.log(f"已更新安裝根目錄為：{norm}", "info")

    def _auto_detect_backup_zip(self):
        """啟動時自動在常用目錄搜尋最新的 _LAH_Backup_*.zip 備份檔並帶入"""
        root_dir = self.root_dir_var.get().strip()
        parent_root = os.path.dirname(root_dir) if root_dir else ""
        search_dirs = [
            self.app_dir,
            r"C:\Temp",
            parent_root,
            os.path.join(os.environ.get("USERPROFILE", ""), "Desktop"),
            os.path.join(os.environ.get("USERPROFILE", ""), "Downloads"),
        ]
        found = find_latest_backup_zip(search_dirs)
        if found:
            self.backup_zip_path_var.set(os.path.normpath(found))
            self.log(f"📦 已自動偵測到系統備份包：{found} (如需還原請勾選上方備份還原選項或點擊立即還原)", "info")

    def _browse_backup_zip(self):
        """瀏覽並選擇 _LAH_Backup_*.zip 備份壓縮檔"""
        cur = self.backup_zip_path_var.get().strip()
        init_dir = os.path.dirname(cur) if (cur and os.path.exists(os.path.dirname(cur))) else (r"C:\Temp" if os.path.exists(r"C:\Temp") else self.app_dir)
        selected = filedialog.askopenfilename(
            initialdir=init_dir,
            title="請選擇系統備份壓縮檔 (_LAH_Backup_*.zip)",
            filetypes=[("ZIP 備份壓縮檔", "*.zip"), ("所有檔案", "*.*")]
        )
        if selected:
            norm = os.path.normpath(selected)
            self.backup_zip_path_var.set(norm)
            self.restore_backup_var.set(True)
            self.log(f"已選擇備份壓縮檔：{norm}", "info")

    def manual_restore_backup(self):
        """手動獨立執行：從指定的備份 ZIP 檔還原原始設定與資料庫至安裝根目錄"""
        if self.is_updating:
            messagebox.showwarning("提示", "系統正在執行更新作業，請稍候再進行備份還原！", parent=self.root)
            return

        zip_path = self.backup_zip_path_var.get().strip()
        if not zip_path or not os.path.isfile(zip_path):
            self._browse_backup_zip()
            zip_path = self.backup_zip_path_var.get().strip()
            if not zip_path or not os.path.isfile(zip_path):
                return

        root_dir = self.root_dir_var.get().strip()
        if not root_dir:
            messagebox.showwarning("提示", "請先設定系統安裝根目錄！", parent=self.root)
            return

        confirm = messagebox.askyesno(
            "確認還原備份設定與資料",
            f"即將從以下備份壓縮包還原原始設定與資料庫：\n"
            f"📦 備份檔案：{zip_path}\n"
            f"📂 目標根目錄：{root_dir}\n\n"
            f"還原範圍包含：\n"
            f"• 各專案環境設定檔 (.env / AD.php)\n"
            f"• 即時通資料庫與上傳附件 (lah-messenger-server/db & uploads)\n"
            f"• 伺服器設定檔 (Apache httpd.conf / PHP php.ini)\n"
            f"• 並自動校正設定檔路徑與資安防護\n\n"
            f"是否確認立即執行還原？",
            parent=self.root
        )
        if not confirm:
            return

        self.set_ui_updating_state(True)
        self._open_progress_dialog()

        def _worker():
            try:
                self.log("=" * 60, "info")
                self.log("📦 開始執行備份設定與資料還原作業...", "info")
                self.set_progress(15, "正在解壓縮並還原備份檔案...")

                ok, msg, count = restore_from_backup_zip(zip_path, root_dir, self.log)

                # 還原後自動校正 Apache / PHP 設定路徑與資安防護
                apache_tgt = os.path.normpath(self.pkg_vars["apache24"]["target_path_var"].get().strip()) if "apache24" in self.pkg_vars else os.path.join(root_dir, "Apache24")
                php7_tgt = os.path.normpath(self.pkg_vars["php7"]["target_path_var"].get().strip()) if "php7" in self.pkg_vars else os.path.join(root_dir, "php7")
                php85_tgt = os.path.normpath(self.pkg_vars["php85"]["target_path_var"].get().strip()) if "php85" in self.pkg_vars else os.path.join(root_dir, "php85")

                if os.path.isdir(apache_tgt):
                    flatten_nested_apache_dir(apache_tgt, self.log)
                    self.set_progress(75, "正在校正 Apache 與 PHP 設定檔路徑...")
                    configure_apache_environment(
                        apache_dir=apache_tgt,
                        root_dir=root_dir,
                        php7_dir=php7_tgt if os.path.isdir(php7_tgt) else None,
                        php85_dir=php85_tgt if os.path.isdir(php85_tgt) else None,
                        enable_security=self.auto_security_hardening_var.get(),
                        log_func=self.log
                    )
                if self.auto_security_hardening_var.get():
                    for p_dir in (php7_tgt, php85_tgt):
                        ini_f = os.path.join(p_dir, "php.ini")
                        if os.path.isfile(ini_f):
                            apply_php_security_hardening(ini_f, self.log)

                self.set_progress(100, f"✅ 備份還原完成！(共還原 {count} 個檔案)")
                self.log("=" * 60, "success")

                def _done():
                    self._on_update_completed_ui()
                    parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                    if ok:
                        messagebox.showinfo(
                            "備份還原成功",
                            f"🎉 已成功從備份檔還原 {count} 個設定與資料檔案至：\n{root_dir}\n\n"
                            "若 Apache 或 PM2 服務正在運行中，建議重新啟動服務以載入最新設定與資料庫。",
                            parent=parent_w
                        )
                    else:
                        messagebox.showerror("備份還原失敗", f"還原過程發生錯誤：\n{msg}", parent=parent_w)

                self.root.after(0, _done)
            except Exception as ex:
                self.log(f"❌ 備份還原發生未預期錯誤: {ex}", "error")
                self.root.after(0, self._on_update_completed_ui)

        threading.Thread(target=_worker, daemon=True).start()

    def _on_root_dir_changed(self, *args):
        """當安裝根目錄變更時，即時連動更新未個別自訂路徑的壓縮包目標路徑"""
        base = self.root_dir_var.get().strip()
        for pkg in PACKAGE_CONFIGS:
            v = self.pkg_vars[pkg["id"]]
            # 若使用者未單獨指定此模組的路徑，才隨根目錄連動
            if not v.get("is_custom_path", False):
                rel = pkg["rel_target"]
                full_target = os.path.normpath(os.path.join(base, rel)) if base else rel
                v["target_path_var"].set(full_target)

    def _browse_pkg_target_dir(self, pkg_id):
        """為單一模組開啟瀏覽視窗自訂目標安裝路徑"""
        v = self.pkg_vars[pkg_id]
        pkg = next((p for p in PACKAGE_CONFIGS if p["id"] == pkg_id), None)
        title = pkg["title"] if pkg else pkg_id
        display_title = title.replace("└─ ", "")

        current = v["target_path_var"].get().strip()
        init_dir = current if (current and os.path.exists(current)) else self.root_dir_var.get().strip()
        if not os.path.exists(init_dir):
            init_dir = "C:\\"

        selected = filedialog.askdirectory(initialdir=init_dir, title=f"請選擇 [{display_title}] 的目標解壓縮路徑")
        if selected:
            norm = os.path.normpath(selected)
            v["target_path_var"].set(norm)
            v["is_custom_path"] = True
            self.log(f"已為模組 [{pkg['name']}] 設定自訂安裝路徑：{norm}", "info")

    def _reset_pkg_target_dir(self, pkg_id):
        """重設單一模組路徑，恢復與安裝根目錄連動"""
        v = self.pkg_vars[pkg_id]
        v["is_custom_path"] = False
        pkg = next((p for p in PACKAGE_CONFIGS if p["id"] == pkg_id), None)
        if pkg:
            base = self.root_dir_var.get().strip()
            rel = pkg["rel_target"]
            norm = os.path.normpath(os.path.join(base, rel)) if base else rel
            v["target_path_var"].set(norm)
            self.log(f"已重設模組 [{pkg['name']}] 目標路徑為跟隨根目錄：{norm}", "info")

    def _on_module_path_manual_edit(self, pkg_id):
        """當使用者手動在文字框修改路徑時，將該模組標記為自訂路徑"""
        self.pkg_vars[pkg_id]["is_custom_path"] = True

    def _detect_apache_service(self):
        """偵測 Apache 服務名稱（有偵測到服務預設選取，沒有則留白）"""
        self.apache_service_name = query_apache_service()
        if self.apache_service_name:
            self.chk_apache.configure(
                text=f"更新前後自動重啟 Apache 服務 (已偵測到服務：{self.apache_service_name})"
            )
            self.restart_apache_var.set(True)
            self.log(f"已偵測到本機 Apache 服務：[{self.apache_service_name}]", "info")
        else:
            self.chk_apache.configure(
                text="更新前後自動重啟 Apache 服務 (系統未偵測到常見 Apache 服務)"
            )
            self.restart_apache_var.set(False)
            self.log("未在系統中偵測到常見 Apache 服務 (如 Apache2.4)，若有需求請確認服務名稱。", "warning")

    def _detect_pm2_service(self):
        """偵測本機 PM2 管理套件與指令路徑，並同步更新 PM2 Daemon 核心綁定狀態"""
        root_dir = self.root_dir_var.get().strip() if hasattr(self, 'root_dir_var') else None
        node_dir = os.path.join(root_dir, "node") if root_dir else r"C:\AppServ\node"
        if os.path.isdir(node_dir):
            register_path_to_user_environment(node_dir, None)

        oci_dir = os.path.join(root_dir, "instantclient_11_2") if root_dir else r"C:\AppServ\instantclient_11_2"
        if os.path.isdir(oci_dir):
            register_path_to_user_environment(oci_dir, None)

        self.pm2_path = resolve_pm2_command(root_dir)
        if self.pm2_path:
            self.chk_pm2.configure(
                text=f"更新後自動重啟 PM2 服務 (套件：{os.path.basename(self.pm2_path)})"
            )
            self.restart_pm2_var.set(True)
            self.log(f"已偵測到本機 PM2 管理套件：[{self.pm2_path}]", "info")
        else:
            self.chk_pm2.configure(
                text="更新後自動重啟 PM2 服務 (未偵測到套件)"
            )
            self.restart_pm2_var.set(False)
            self.log("未在系統中偵測到常見 PM2 管理套件 (pm2.cmd)，若有需求請確認安裝與 PATH 環境變數。", "warning")

        self.update_pm2_daemon_status()

    def update_pm2_daemon_status(self):
        """查詢並更新主介面上 PM2 核心守護進程的即時綁定狀態標籤"""
        root_dir = self.root_dir_var.get().strip() if hasattr(self, 'root_dir_var') else None

        def _query_and_set():
            info = query_pm2_daemon_info(root_dir)

            def _apply_ui():
                if not hasattr(self, 'lbl_pm2_status') or not self.lbl_pm2_status.winfo_exists():
                    return
                if info["is_running"]:
                    if info["is_target_node"]:
                        self.pm2_daemon_status_var.set(f"🟢 核心：{info['exe_path']} (已綁定綠色版)")
                        self.lbl_pm2_status.config(fg="#2e7d32")
                        self.btn_align_pm2.configure(state="normal", text="⚡ 重新校正 PM2 核心")
                    else:
                        self.pm2_daemon_status_var.set(f"🟡 核心：{info['exe_path']} (非綠色版，點擊校正)")
                        self.lbl_pm2_status.config(fg="#e65100")
                        self.btn_align_pm2.configure(state="normal", text="⚡ 立即校正 PM2 核心")
                else:
                    self.pm2_daemon_status_var.set("⚪ 核心：PM2 守護程序未運行 (啟動將以綠色版喚醒)")
                    self.lbl_pm2_status.config(fg="#666666")
                    self.btn_align_pm2.configure(state="normal", text="⚡ 喚醒綠色 PM2 核心")

            try:
                self.root.after(0, _apply_ui)
            except Exception:
                pass

        threading.Thread(target=_query_and_set, daemon=True).start()

    def manage_pm2_autostart(self):
        r"""開啟 PM2 開機自啟動管理對話框"""
        if self.is_updating:
            messagebox.showwarning("提示", "系統正在執行更新作業，請稍候再進行自啟動設定！")
            return

        root_dir = self.root_dir_var.get().strip()

        dlg = tk.Toplevel(self.root)
        dlg.title("⚙️ PM2 開機自啟動管理")
        dlg.geometry("680x520")
        dlg.minsize(580, 420)
        dlg.transient(self.root)

        # 置中於主視窗
        try:
            self.root.update_idletasks()
            rx = self.root.winfo_rootx()
            ry = self.root.winfo_rooty()
            rw = self.root.winfo_width()
            rh = self.root.winfo_height()
            dx = max(30, rx + (rw - 720) // 2)
            dy = max(30, ry + (rh - 540) // 2)
            dlg.geometry(f"720x540+{dx}+{dy}")
        except Exception:
            pass

        content = ttk.Frame(dlg, padding="14")
        content.pack(fill=tk.BOTH, expand=True)

        # 頂部狀態列
        top_frame = ttk.Frame(content)
        top_frame.pack(fill=tk.X, pady=(0, 10))

        ttk.Label(
            top_frame,
            text="⚙️ PM2 伺服器開機自啟動配置",
            font=("Microsoft JhengHei UI", 13, "bold")
        ).pack(side=tk.LEFT)

        status_var = tk.StringVar(value="正在偵測狀態...")
        lbl_status = tk.Label(
            top_frame,
            textvariable=status_var,
            font=("Microsoft JhengHei UI", 11, "bold"),
            fg="#555555"
        )
        lbl_status.pack(side=tk.RIGHT)

        # 說明區
        hint_frame = ttk.LabelFrame(content, text="功能說明與機制", padding="8")
        hint_frame.pack(fill=tk.X, pady=(0, 10))

        hint_text = (
            "• 原生系統服務：透過 WinSW 註冊 Windows 服務 (LAH_PM2_Service)，開機無需使用者登入即可在背景啟動。\n"
            "• 環境鎖定保障：強制使用 C:\\AppServ\\node\\node.exe 運行守護進程，避免受全域 PATH 污染。\n"
            "• 自動修復守護：背景常駐 Watchdog 每 60 秒自動巡檢，若 LAH-FE / WSS / PHP8 API 離線自動重啟。\n"
            "• 多重備援防線：同時配置 pm2-autostart 登入腳本與排程工作，確保任意環境下皆能自動復原。"
        )
        ttk.Label(
            hint_frame,
            text=hint_text,
            font=("Microsoft JhengHei UI", 9),
            foreground="#333333",
            justify=tk.LEFT
        ).pack(anchor="w")

        # 目前偵測詳細資訊區
        detail_frame = ttk.LabelFrame(content, text="目前系統自啟動偵測詳情", padding="8")
        detail_frame.pack(fill=tk.BOTH, expand=True, pady=(0, 12))

        txt_detail = scrolledtext.ScrolledText(
            detail_frame,
            wrap=tk.WORD,
            font=("Consolas", 10),
            bg="#fbfbfb",
            fg="#222222",
            height=7,
            relief=tk.SOLID,
            borderwidth=1
        )
        txt_detail.pack(fill=tk.BOTH, expand=True)

        # 底部按鈕區
        btn_row = ttk.Frame(content)
        btn_row.pack(fill=tk.X, pady=(4, 0))

        def _refresh_status():
            cfg = check_pm2_autostart_config()
            has_svc = bool(cfg["services"])
            has_task = bool(cfg["tasks"])
            has_sc = bool(cfg["startup_shortcuts"])
            is_enabled = has_svc or has_task or has_sc

            if has_svc:
                status_var.set("🟢 狀態：Windows 原生系統服務【運行中】(開機免登入自啟)")
                lbl_status.config(fg="#2e7d32")
            elif is_enabled:
                status_var.set("🟡 狀態：開機自啟動【部分啟用 / 傳統腳本模式】")
                lbl_status.config(fg="#e65100")
            else:
                status_var.set("⚪ 狀態：開機自啟動【未設定】")
                lbl_status.config(fg="#777777")

            txt_detail.configure(state="normal")
            txt_detail.delete("1.0", tk.END)

            lines = []
            if has_svc:
                lines.append(f"[✓] Windows 原生系統服務 (開機免登入自啟) ({len(cfg['services'])} 項):")
                for s in cfg["services"]:
                    lines.append(f"    • {s.get('Name', 'PM2')} (狀態: {s.get('State', '未知')} / 啟動模式: {s.get('StartMode', '未知')})")
                    if s.get("PathName"):
                        lines.append(f"      執行路徑: {s.get('PathName')}")
                lines.append("      特色: 開機免登入由 LocalSystem 啟動，鎖定 C:\\AppServ\\node，每 60 秒巡檢自動修復")
            else:
                lines.append("[-] Windows 原生系統服務 (LAH_PM2_Service)：未安裝")

            if has_sc:
                lines.append(f"\n[✓] 傳統啟動資料夾腳本 ({len(cfg['startup_shortcuts'])} 項):")
                for sc in cfg["startup_shortcuts"]:
                    lines.append(f"    • {sc}")
            else:
                lines.append("\n[-] 傳統啟動資料夾 (Startup)：未設定")

            if has_task:
                lines.append(f"\n[✓] Windows 工作排程器 ({len(cfg['tasks'])} 項):")
                for t in cfg["tasks"]:
                    lines.append(f"    • {t.get('TaskName', 'PM2')} (狀態: {t.get('State', '未知')})")
            else:
                lines.append("\n[-] Windows 工作排程器：未設定")

            txt_detail.insert(tk.END, "\n".join(lines))
            txt_detail.configure(state="disabled")

        def _on_enable():
            confirm = messagebox.askyesno(
                "啟用確認",
                "即將設定 Windows 開機時自動於系統層級 (LocalSystem) 啟動並監控 PM2 服務。\n\n"
                "操作項目：\n"
                "1. 儲存當前受管服務狀態 (pm2 save) 並同步設定檔至系統設定目錄\n"
                "2. 部署 WinSW 原生服務封裝檔 (lah-service.exe / xml / service_runner.js)\n"
                "3. 註冊並啟動 Windows 原生系統服務 (LAH_PM2_Service)\n"
                "4. 建立輔助啟動腳本與工作排程器作為備援防線\n\n"
                "優勢：開機無須使用者登入 Windows 桌面即可自動拉起服務，並鎖定 C:\\AppServ\\node 環境。\n\n"
                "是否立即啟用？",
                parent=dlg
            )
            if not confirm:
                return

            self.log("=" * 50, "info")
            self.log("⚙️ 正在啟用 PM2 原生系統服務與開機自啟動設定...", "info")
            ok, msg = setup_pm2_autostart(self.log, root_dir)
            _refresh_status()
            if ok:
                messagebox.showinfo("啟用成功", f"🎉 {msg}\n\n電腦重新開機時，系統無須登入即可自動啟動 PM2 並確保各服務正常運作！", parent=dlg)
            else:
                messagebox.showerror("啟用失敗", f"設定過程發生錯誤：\n{msg}", parent=dlg)

        def _on_disable():
            confirm = messagebox.askyesno(
                "停用確認",
                "確定要停用並移除 PM2 原生系統服務與開機自啟動設定嗎？\n\n"
                "操作項目：\n"
                "1. 停止並解除安裝 LAH_PM2_Service Windows 系統服務\n"
                "2. 刪除 Windows 工作排程器中的 LAH_PM2_Autostart\n"
                "3. 清除啟動資料夾中的 pm2-autostart 腳本\n\n"
                "停用後，伺服器重新開機將不會自動拉起 PM2 服務。",
                parent=dlg
            )
            if not confirm:
                return

            self.log("=" * 50, "info")
            self.log("🛑 正在停用並移除 PM2 系統服務與開機自啟動設定...", "info")
            ok, msg = remove_pm2_autostart(self.log)
            _refresh_status()
            messagebox.showinfo("自啟動設定", msg, parent=dlg)

        def _on_test():
            self.log("=" * 50, "info")
            self.log("⚡ 正在測試執行 PM2 原生服務與全核心巡檢...", "info")
            ok, msg = test_pm2_autostart_execution(self.log, root_dir)
            _refresh_status()
            if ok:
                messagebox.showinfo("測試結果", f"🎉 {msg}", parent=dlg)
            else:
                messagebox.showwarning("測試結果", f"測試過程回傳：\n{msg}", parent=dlg)

        btn_enable = ttk.Button(btn_row, text="🚀 啟用原生系統服務 (開機免登入)", command=_on_enable, width=28)
        btn_enable.pack(side=tk.LEFT, padx=(0, 6))

        btn_disable = ttk.Button(btn_row, text="🛑 停用 / 移除系統服務", command=_on_disable, width=20)
        btn_disable.pack(side=tk.LEFT, padx=(0, 6))

        btn_test = ttk.Button(btn_row, text="⚡ 測試服務與全核心巡檢", command=_on_test, width=22)
        btn_test.pack(side=tk.LEFT, padx=(0, 6))

        btn_close = ttk.Button(btn_row, text="關閉", command=dlg.destroy, width=10)
        btn_close.pack(side=tk.RIGHT)

        dlg.bind("<Escape>", lambda e: dlg.destroy())
        _refresh_status()

    def manual_align_pm2_daemon(self):
        r"""手動觸發一鍵校正切換 PM2 核心至 C:\AppServ\node"""
        if self.is_updating:
            messagebox.showwarning("提示", "系統正在執行更新作業，請稍候再進行 PM2 核心校正！")
            return

        root_dir = self.root_dir_var.get().strip()
        target_node = os.path.normpath(os.path.join(root_dir, "node", "node.exe"))

        confirm = messagebox.askyesno(
            "PM2 核心進程校正確認",
            f"即將切換 PM2 核心守護進程至專屬綠色環境：\n{target_node}\n\n"
            "操作流程：\n"
            "1. 儲存當前受控服務清單 (pm2 save)\n"
            "2. 結束當前運行之 PM2 Daemon (pm2 kill)\n"
            "3. 調用綠色 Node 重新喚醒守護進程並恢復服務 (pm2 resurrect)\n"
            "4. 儲存新核心狀態 (pm2 save)\n\n"
            "是否立即執行校正？"
        )
        if not confirm:
            return

        self._open_progress_dialog()
        self.set_ui_updating_state(True)

        def _worker():
            self.log("=" * 60, "info")
            self.log("⚡ 開始執行 PM2 核心守護進程切換與校正作業...", "info")
            self.set_progress(15, "正在執行 PM2 核心切換流程...")

            ok, msg = align_pm2_daemon_to_portable(self.log, root_dir)

            self.set_progress(80, "正在檢查伺服器開機自啟動設定...")
            autostart = check_pm2_autostart_config()

            self.set_progress(100, "PM2 核心校正作業完成！")
            self.log("=" * 60, "success")

            # 彙整開機自啟動提示
            svc_info = [s.get("Name", "PM2") for s in autostart["services"]]
            task_info = [t.get("TaskName", "PM2") for t in autostart["tasks"]]
            sc_info = [os.path.basename(p) for p in autostart["startup_shortcuts"]]

            auto_lines = []
            if svc_info:
                auto_lines.append(f"• 註冊之 Windows 服務：{', '.join(svc_info)}")
            if task_info:
                auto_lines.append(f"• 登記之工作排程：{', '.join(task_info)}")
            if sc_info:
                auto_lines.append(f"• 啟動資料夾捷徑：{', '.join(sc_info)}")

            if not auto_lines:
                autostart_hint = "\n\n💡 提示：目前伺服器尚未註冊開機自啟動 Windows 服務或排程。\n若有開機自動啟動需求，可至各專案目錄執行 start.bat 或建立開機捷徑。"
            else:
                autostart_hint = "\n\n📌 伺服器開機自啟動設定偵測：\n" + "\n".join(auto_lines)

            def _done():
                self.set_ui_updating_state(False)
                self.update_pm2_daemon_status()
                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                if ok:
                    messagebox.showinfo(
                        "PM2 核心校正成功",
                        f"🎉 PM2 核心守護進程已順利綁定至綠色便攜環境！\n\n"
                        f"• 核心執行檔：{target_node}\n"
                        f"• 所有受管服務已透過 pm2 resurrect 恢復運作。{autostart_hint}",
                        parent=parent_w
                    )
                else:
                    messagebox.showwarning(
                        "PM2 核心校正提示",
                        f"校正過程回傳狀態：\n{msg}{autostart_hint}",
                        parent=parent_w
                    )

            self.root.after(0, _done)

        threading.Thread(target=_worker, daemon=True).start()

    def manual_apply_security_hardening(self):
        """獨立功能按鈕：一鍵手動套用資安弱點防護（隱藏 Web 伺服器與 PHP 版本）"""
        if self.is_updating:
            messagebox.showwarning("提示", "系統正在執行更新作業，請稍候再進行資安防護設定！", parent=self.root)
            return

        root_dir = self.root_dir_var.get().strip()
        apache_dir = os.path.normpath(self.pkg_vars["apache24"]["target_path_var"].get().strip()) if "apache24" in self.pkg_vars else os.path.join(root_dir, "Apache24")
        php7_dir = os.path.normpath(self.pkg_vars["php7"]["target_path_var"].get().strip()) if "php7" in self.pkg_vars else os.path.join(root_dir, "php7")
        php85_dir = os.path.normpath(self.pkg_vars["php85"]["target_path_var"].get().strip()) if "php85" in self.pkg_vars else os.path.join(root_dir, "php85")

        httpd_conf = os.path.join(apache_dir, "conf", "httpd.conf")
        php7_ini = os.path.join(php7_dir, "php.ini")
        php85_ini = os.path.join(php85_dir, "php.ini")

        found_any = os.path.isfile(httpd_conf) or os.path.isfile(php7_ini) or os.path.isfile(php85_ini)
        if not found_any:
            messagebox.showwarning(
                "找不到設定檔",
                f"在指定的安裝根目錄下未找到任何 Apache 或 PHP 設定檔：\n"
                f"• {httpd_conf}\n"
                f"• {php7_ini}\n"
                f"• {php85_ini}\n\n請確認安裝根目錄路徑是否正確！",
                parent=self.root
            )
            return

        self._open_progress_dialog()
        self.log("=" * 60, "info")
        self.log("🛡️ 開始手動套用資安弱點防護（隱藏 Web 伺服器與 PHP 版本資訊）...", "info")

        results = []

        # 1. 處理 Apache httpd.conf
        if os.path.isfile(httpd_conf):
            self.log(f"正在檢查並設定 Apache: {httpd_conf}", "info")
            ok, msg = apply_apache_security_hardening(httpd_conf, self.log)
            if ok:
                results.append("• Apache httpd.conf: 已設定 ServerTokens Prod 與 ServerSignature Off")
                check_apache_syntax(apache_dir, self.log)
            else:
                results.append(f"• Apache httpd.conf: 套用失敗 ({msg})")
        else:
            self.log(f"ℹ️ 未偵測到 Apache 設定檔: {httpd_conf}（略過）", "info")

        # 2. 處理 PHP 7 php.ini
        if os.path.isfile(php7_ini):
            self.log(f"正在檢查並設定 PHP 7: {php7_ini}", "info")
            ok, msg = apply_php_security_hardening(php7_ini, self.log)
            if ok:
                results.append("• PHP 7 php.ini: 已設定 expose_php = Off")
            else:
                results.append(f"• PHP 7 php.ini: 套用失敗 ({msg})")
        else:
            self.log(f"ℹ️ 未偵測到 PHP 7 設定檔: {php7_ini}（略過）", "info")

        # 3. 處理 PHP 8.5 php.ini
        if os.path.isfile(php85_ini):
            self.log(f"正在檢查並設定 PHP 8.5: {php85_ini}", "info")
            ok, msg = apply_php_security_hardening(php85_ini, self.log)
            if ok:
                results.append("• PHP 8.5 php.ini: 已設定 expose_php = Off")
            else:
                results.append(f"• PHP 8.5 php.ini: 套用失敗 ({msg})")
        else:
            self.log(f"ℹ️ 未偵測到 PHP 8.5 設定檔: {php85_ini}（略過）", "info")

        self.log("=" * 60, "success")
        self.log("🎉 資安弱點防護設定套用完成！", "success")

        # 若 Apache 正在運行，詢問是否重啟以生效
        svc_name = self.apache_service_name or query_apache_service()
        restarted_svc = False
        if svc_name and is_service_running(svc_name):
            do_restart = messagebox.askyesno(
                "重啟服務以套用新設定",
                f"資安弱點防護設定已更新完成！\n\n"
                f"偵測到 Apache 服務 [{svc_name}] 正在運行中。\n"
                f"是否立即重新啟動 Apache 服務，使隱藏版本設定立即生效？",
                parent=self.root
            )
            if do_restart:
                self.log(f"正在重啟 Apache 服務 [{svc_name}]...", "info")
                stop_apache_service(svc_name, self.log)
                start_apache_service(svc_name, self.log)
                restarted_svc = True
                self.log(f"✅ Apache 服務 [{svc_name}] 已成功重啟並套用資安設定！", "success")

        res_text = "\n".join(results)
        restart_hint = f"\n\n已成功重啟 Apache 服務 [{svc_name}]！" if restarted_svc else ("\n\n提示：若服務正在運行，請重啟 Apache 與 PM2 使設定生效。" if svc_name else "")
        messagebox.showinfo(
            "資安弱點防護套用完成",
            f"🛡️ 資安弱點防護處理完成！\n\n{res_text}{restart_hint}",
            parent=self.root
        )

    def rescan(self):
        """重新掃描更新包與更新說明檔案"""
        self.log(f"正在掃描更新目錄：{self.app_dir} ...", "info")
        self._detect_apache_service()
        self._detect_pm2_service()
        detected_count = 0

        # 更新各壓縮包目標路徑
        self._on_root_dir_changed()

        for pkg in PACKAGE_CONFIGS:
            p_id = pkg["id"]
            p_name = pkg["name"]
            v = self.pkg_vars[p_id]
            candidates = [p_name] + pkg.get("alt_names", [])
            matched_paths = []
            seen_norm = set()

            for name in candidates:
                cp = os.path.join(self.app_dir, name)
                if os.path.isfile(cp):
                    np = os.path.normpath(cp).lower()
                    if np not in seen_norm:
                        seen_norm.add(np)
                        matched_paths.append(cp)

            # 支援 pattern 正則表達式模糊/萬用字元辨識
            if pkg.get("pattern"):
                try:
                    pat = re.compile(pkg["pattern"])
                    for fname in os.listdir(self.app_dir):
                        if pat.match(fname):
                            full_f = os.path.join(self.app_dir, fname)
                            if os.path.isfile(full_f):
                                np = os.path.normpath(full_f).lower()
                                if np not in seen_norm:
                                    seen_norm.add(np)
                                    matched_paths.append(full_f)
                except Exception:
                    pass

            found_path = matched_paths[0] if matched_paths else None
            zip_path = found_path if found_path else os.path.join(self.app_dir, p_name)

            if os.path.isfile(zip_path):
                v["exists"] = True
                v["filepath"] = zip_path
                v["all_filepaths"] = matched_paths
                size = os.path.getsize(zip_path)
                v["filesize"] = size
                v["checked"].set(True)
                v["status_var"].set(f"✅ 已偵測到檔案 ({format_file_size(size)})")
                v["status_label_widget"].config(fg="#2e7d32")
                self.chk_widgets[p_id].configure(state="normal")
                detected_count += 1
            else:
                v["exists"] = False
                v["filepath"] = ""
                v["all_filepaths"] = []
                v["filesize"] = 0
                v["checked"].set(False)
                v["status_var"].set("⚪ 未提供（本次略過）")
                v["status_label_widget"].config(fg="#888888")
                self.chk_widgets[p_id].configure(state="disabled")

        if detected_count > 0:
            self.log(f"掃描完成：共偵測到 {detected_count} 個待更新壓縮包。", "success")
        else:
            self.log("掃描完成：同目錄下未偵測到任何指定更新壓縮包 (.zip)。", "warning")

        # 讀取版本說明並切換彈窗按鈕狀態
        self.release_note_name, self.release_note_content = find_release_notes(self.app_dir)
        if self.release_note_name and self.release_note_content:
            self.btn_view_notes.configure(state="normal", text="📋 查看更新說明")
            self.log(f"已成功載入更新說明檔：{self.release_note_name}", "info")
        else:
            self.btn_view_notes.configure(state="disabled", text="📋 無更新說明")

        # 依目前偵測結果與顯示設定即時更新卡片可見度
        self._update_package_cards_visibility()

    def log(self, message, level="info"):
        """執行緒安全的日誌輸出與歷史記錄"""
        now_str = datetime.now().strftime("[%H:%M:%S] ")
        self.log_history.append((now_str, message, level))

        def _append():
            if self.progress_log_box and self.progress_log_box.winfo_exists():
                self.progress_log_box.insert(tk.END, now_str, "time")
                self.progress_log_box.insert(tk.END, message + "\n", level)
                self.progress_log_box.see(tk.END)

        if threading.current_thread() is threading.main_thread():
            _append()
        else:
            self.root.after(0, _append)

    def set_progress(self, percent, status_text=None):
        """設定進度條與狀態文字"""
        def _update():
            p_val = max(0.0, min(100.0, float(percent)))
            self.progress_percent_var.set(f"{p_val:.1f}%")
            if status_text:
                self.progress_status_var.set(status_text)
            if self.progress_bar and self.progress_bar.winfo_exists():
                self.progress_bar["value"] = p_val

        if threading.current_thread() is threading.main_thread():
            _update()
        else:
            self.root.after(0, _update)

    def set_ui_updating_state(self, is_updating):
        """更新中鎖定或解鎖 UI 元件"""
        self.is_updating = is_updating
        state = "disabled" if is_updating else "normal"
        self.root_entry.configure(state=state)
        self.btn_browse.configure(state=state)
        self.btn_rescan.configure(state=state)
        self.chk_apache.configure(state=state)
        self.chk_pm2.configure(state=state)
        if hasattr(self, 'btn_align_pm2'):
            self.btn_align_pm2.configure(state=state)
        if hasattr(self, 'btn_autostart_pm2'):
            self.btn_autostart_pm2.configure(state=state)
        if hasattr(self, 'btn_sec_harden'):
            self.btn_sec_harden.configure(state=state)
        if hasattr(self, 'chk_show_all'):
            self.chk_show_all.configure(state=state)
        if hasattr(self, 'chk_restore_backup'):
            self.chk_restore_backup.configure(state=state)
        if hasattr(self, 'backup_entry'):
            self.backup_entry.configure(state=state)
        if hasattr(self, 'btn_browse_backup'):
            self.btn_browse_backup.configure(state=state)
        if hasattr(self, 'btn_restore_now'):
            self.btn_restore_now.configure(state=state)

        for p_id in self.pkg_vars:
            exists = self.pkg_vars[p_id]["exists"]
            chk_state = state if exists else "disabled"
            self.chk_widgets[p_id].configure(state=chk_state)
            self.path_entries[p_id].configure(state=state)
            self.browse_buttons[p_id].configure(state=state)
            self.reset_buttons[p_id].configure(state=state)

        if is_updating:
            self.btn_start.configure(state="disabled", bg="#9e9e9e", cursor="wait")
        else:
            self.btn_start.configure(state="normal", bg="#1976d2", cursor="hand2")

    def start_update_thread(self):
        """點擊開始更新按鈕後觸發"""
        if self.is_updating:
            return

        selected_pkgs = [
            pkg for pkg in PACKAGE_CONFIGS
            if self.pkg_vars[pkg["id"]]["exists"] and self.pkg_vars[pkg["id"]]["checked"].get()
        ]

        if not selected_pkgs:
            messagebox.showwarning("提示", "未勾選任何要更新的壓縮包項目！")
            return

        # 驗證每個選取的模組都有指定目標路徑
        for p in selected_pkgs:
            tgt = self.pkg_vars[p["id"]]["target_path_var"].get().strip()
            if not tgt:
                messagebox.showwarning("提示", f"模組 [{p['name']}] 未設定目標路徑，請先輸入或選擇目標路徑！")
                return

        summary_env = []
        summary_app = []
        for p in selected_pkgs:
            tgt = self.pkg_vars[p["id"]]["target_path_var"].get().strip()
            fname = os.path.basename(self.pkg_vars[p["id"]]["filepath"]) or p["name"]
            line = f"• {p['title']} [{fname}] ➔ {tgt}"
            if p.get("is_base_env"):
                summary_env.append(line)
            else:
                summary_app.append(line)

        summary_parts = []
        if summary_env:
            summary_parts.append("【基礎環境與 Web 伺服器】：\n" + "\n".join(summary_env))
        if summary_app:
            summary_parts.append("【智慧控管系統模組】：\n" + "\n".join(summary_app))

        summary = "\n\n".join(summary_parts)
        backup_zip = self.backup_zip_path_var.get().strip() if self.restore_backup_var.get() else ""
        if self.restore_backup_var.get() and (not backup_zip or not os.path.isfile(backup_zip)):
            messagebox.showwarning("提示", "您已勾選「從備份包回復原始設定」，但尚未指定有效的備份壓縮檔 (_LAH_Backup_*.zip)！", parent=self.root)
            return

        backup_note = f"\n• 安裝後將自動從備份包回復原始設定與資料 ({os.path.basename(backup_zip)})" if backup_zip else ""
        apache_note = f"\n• 更新前後將自動重啟或啟動 Apache 服務 ({self.apache_service_name or 'Apache2.4'})" if (self.restart_apache_var.get() or any(p["id"] == "apache24" for p in selected_pkgs)) else ""
        pm2_note = "\n• 更新完成後將自動綁定綠色版 Node、拉起 PM2 核心服務並設定開機自動化" if (self.restart_pm2_var.get() or any(p.get("is_nodejs") for p in selected_pkgs)) else ""
        node_note = "\n• 包含 Node.js 綠色執行環境或核心依賴套件更新，解壓前將安全暫停 Node/PM2 服務以釋放檔案鎖定" if any(p["id"] == "node" or p.get("is_sub_package", False) for p in selected_pkgs) else ""
        sec_note = "\n• 將自動套用資安弱點防護 (隱藏 Apache / PHP 版本資訊)" if self.auto_security_hardening_var.get() else ""
        apache_svc_note = "\n• 將自動檢查並註冊啟動 Apache2.4 Windows 服務" if self.auto_register_apache_var.get() else ""

        confirm = messagebox.askyesno(
            "確認開始更新",
            f"即將開始更新以下項目：\n\n{summary}{backup_note}{apache_note}{pm2_note}{node_note}{sec_note}{apache_svc_note}\n\n是否確認立即執行解壓縮覆蓋？"
        )
        if not confirm:
            return

        self.set_ui_updating_state(True)
        self._open_progress_dialog()
        worker = threading.Thread(target=self._run_update_process, args=(selected_pkgs,), daemon=True)
        worker.start()

    def _run_update_process(self, selected_pkgs):
        """背景更新執行緒主要流程"""
        start_time = time.time()
        self.log("=" * 60, "info")
        self.log("🚀 開始執行自動更新作業...", "info")
        self.set_progress(0, "正在準備更新作業...")

        # 優先將選取的更新包排序：is_base_env 基礎環境優先解壓縮
        selected_pkgs.sort(key=lambda p: (0 if p.get("is_base_env") else 1))

        svc_name = self.apache_service_name or query_apache_service()
        has_base_env_update = any(p.get("is_base_env") for p in selected_pkgs)
        has_apache_app_update = any(p["id"] in ("www", "lah_api") for p in selected_pkgs)
        should_stop_apache = self.restart_apache_var.get() or has_base_env_update or has_apache_app_update
        apache_was_stopped = False
        apache_restarted = False

        try:
            # 步驟 1：若需要，先停止 Apache 服務以防止 DLL 與執行檔鎖定
            if should_stop_apache:
                if svc_name and is_service_running(svc_name):
                    self.set_progress(2, f"正在停止 Apache 服務 [{svc_name}]...")
                    apache_was_stopped = stop_apache_service(svc_name, self.log)
                else:
                    self.log("ℹ️ 準備更新相關服務，系統未偵測到運行中的 Apache 服務，跳過停止步驟。", "info")

            # 步驟 1.5：若有更新 node.zip 或 node_modules，執行階梯式安全防護釋放鎖定
            root_dir = self.root_dir_var.get().strip()
            has_node_runtime = any(p["id"] == "node" for p in selected_pkgs)
            has_node_modules = any(p.get("is_sub_package", False) for p in selected_pkgs)
            is_updating_node = has_node_runtime or has_node_modules
            pm2_was_killed = False
            if is_updating_node:
                self.set_progress(3, "正在檢查 Node.js 執行狀態並釋放檔案鎖定...")
                if has_node_runtime:
                    self.log("ℹ️ 偵測到更新項目包含 Node.js 綠色執行環境，執行安全停機防護...", "info")
                else:
                    self.log("ℹ️ 偵測到更新項目包含 Node.js 核心依賴套件 (node_modules)，暫停 PM2 服務以釋放檔案鎖定...", "info")
                node_tgt = os.path.normpath(self.pkg_vars["node"]["target_path_var"].get().strip()) if "node" in self.pkg_vars else os.path.join(root_dir, "node")
                target_node_exe = os.path.join(node_tgt, "node.exe")

                # 第一階段：執行 pm2 stop all 暫停工作服務
                stop_ok, _ = stop_pm2_services(self.log, root_dir)

                # 檢測 node.exe 是否仍被占用（如 PM2 God 守護進程本身使用該 node.exe）
                if is_file_locked(target_node_exe):
                    self.log("⚠️ 偵測到 node.exe 仍被背景守護進程鎖定，先保存受控服務 (pm2 save) 後執行 [pm2 kill] 徹底釋放...", "warning")
                    save_pm2_services(self.log, root_dir)
                    kill_ok, _ = kill_pm2_daemon(self.log, root_dir)
                    pm2_was_killed = True
                    if is_file_locked(target_node_exe):
                        self.log(f"⚠️ 警告：{target_node_exe} 仍處於鎖定狀態，若解壓存取被拒請手動結束 node.exe 行程。", "warning")
                    else:
                        self.log("✅ node.exe 檔案鎖定已成功完全解除！", "success")
                else:
                    self.log("✅ node.exe 檔案未受鎖定，可安全執行覆蓋解壓。", "info")

            # 步驟 2：統計所有選取壓縮包的總檔案數（用於進度計算）
            self.set_progress(5, "正在計算待解壓縮檔案數量...")
            total_files_all = 0
            pkg_file_counts = {}

            for pkg in selected_pkgs:
                zpath = self.pkg_vars[pkg["id"]]["filepath"]
                try:
                    with zipfile.ZipFile(zpath, 'r') as zf:
                        # 排除 Mac 雜項
                        valid_items = [
                            m for m in zf.infolist()
                            if not (get_zip_member_path(m).startswith('__MACOSX') or get_zip_member_path(m).endswith('.DS_Store'))
                        ]
                        count = len(valid_items)
                        pkg_file_counts[pkg["id"]] = (valid_items, count)
                        total_files_all += count
                except Exception as e:
                    self.log(f"❌ 讀取壓縮檔 [{pkg['name']}] 失敗: {e}", "error")
                    raise

            self.log(f"總計需更新解壓 {total_files_all} 個項目檔案。", "info")

            if total_files_all == 0:
                total_files_all = 1

            # 步驟 3：逐一解壓縮各壓縮包至個別指定的目標路徑
            processed_files = 0

            for idx, pkg in enumerate(selected_pkgs, start=1):
                p_name = pkg["name"]
                # 取得該模組個別設定的目標解壓縮路徑
                target_folder = os.path.normpath(self.pkg_vars[pkg["id"]]["target_path_var"].get().strip())
                zpath = self.pkg_vars[pkg["id"]]["filepath"]
                valid_items, pkg_count = pkg_file_counts[pkg["id"]]

                self.log("-" * 50, "info")
                self.log(f"[{idx}/{len(selected_pkgs)}] 正在解壓縮：{p_name} ➔ {target_folder}", "info")
                os.makedirs(target_folder, exist_ok=True)

                with zipfile.ZipFile(zpath, 'r') as zf:
                    # 執行【關鍵防呆機制】：檢測頂層結構
                    zip_stem = os.path.splitext(p_name)[0]
                    target_leaf = os.path.basename(target_folder)
                    strip_prefix = detect_strip_prefix(zf, pkg["expected_folder"], zip_stem, target_leaf, pkg.get("alt_folders", []))

                    if strip_prefix:
                        self.log(f"🛡️ [防呆機制觸發] 偵測到頂層包含 '{strip_prefix}' 單一目錄，已自動對齊解壓至目標路徑！", "warning")
                    else:
                        self.log("ℹ️ 壓縮包頂層為直接檔案/結構，依照目標目錄直接解壓縮覆蓋。", "info")

                    # 解壓縮個別項目
                    for m in valid_items:
                        raw_name = get_zip_member_path(m).replace('\\', '/')

                        # 剔除防呆前綴
                        if strip_prefix:
                            if raw_name.startswith(strip_prefix):
                                rel_sub = raw_name[len(strip_prefix):].lstrip('/')
                            elif '/' not in raw_name.strip('/'):
                                # 若啟用了單一資料夾剝除 (如 Apache24/)，自動略過壓縮包最外層散落的 ReadMe.txt 等說明檔
                                processed_files += 1
                                continue
                            else:
                                rel_sub = raw_name.lstrip('/')
                        else:
                            rel_sub = raw_name.lstrip('/')

                        if not rel_sub:
                            # 頂層目錄本身已存在，跳過
                            processed_files += 1
                            continue

                        # 計算實際完整目標路徑
                        dest_file_path = os.path.normpath(os.path.join(target_folder, rel_sub.replace('/', os.sep)))

                        # 安全性路徑檢查 (防止 Zip Slip 跨目錄逃逸漏洞)
                        if not (dest_file_path == target_folder or dest_file_path.startswith(target_folder + os.sep)):
                            self.log(f"⚠️ 跳過潛在不安全檔案路徑: {raw_name}", "warning")
                            processed_files += 1
                            continue

                        if m.is_dir() or m.filename.endswith('/'):
                            os.makedirs(dest_file_path, exist_ok=True)
                        else:
                            os.makedirs(os.path.dirname(dest_file_path), exist_ok=True)
                            # 解壓縮檔案內容並覆蓋
                            with zf.open(m) as src, open(dest_file_path, 'wb') as dst:
                                shutil.copyfileobj(src, dst)

                            # 嘗試保留原檔案時間戳記
                            try:
                                dt = time.mktime(m.date_time + (0, 0, -1))
                                os.utime(dest_file_path, (dt, dt))
                            except Exception:
                                pass

                        processed_files += 1
                        pct = 5.0 + (processed_files / total_files_all) * 85.0
                        self.set_progress(pct, f"[{idx}/{len(selected_pkgs)}] 解壓中: {p_name} ({processed_files}/{total_files_all})")

                self.log(f"✅ {p_name} 解壓縮覆蓋完成 (共處理 {pkg_count} 個項目)。", "success")

            # 步驟 3.2：若有勾選「從備份包回復原始設定」，立即執行備份還原
            if self.restore_backup_var.get():
                b_zip = self.backup_zip_path_var.get().strip()
                if b_zip and os.path.isfile(b_zip):
                    self.set_progress(90, f"正在從備份包還原原始設定與資料 ({os.path.basename(b_zip)})...")
                    restore_from_backup_zip(b_zip, root_dir, self.log)
                else:
                    self.log(f"⚠️ 找不到指定的備份檔 [{b_zip}]，略過備份還原步驟。", "warning")

            # 步驟 3.5：檢查並自動校正 Apache 與 PHP 設定檔、資安弱點防護與服務註冊
            has_apache = any(p["id"] == "apache24" for p in selected_pkgs)
            has_php7 = any(p["id"] == "php7" for p in selected_pkgs)
            has_php85 = any(p["id"] == "php85" for p in selected_pkgs)
            has_nodejs_app = any(p.get("is_nodejs", False) for p in selected_pkgs)

            apache_tgt = os.path.normpath(self.pkg_vars["apache24"]["target_path_var"].get().strip()) if "apache24" in self.pkg_vars else os.path.join(root_dir, "Apache24")
            php7_tgt = os.path.normpath(self.pkg_vars["php7"]["target_path_var"].get().strip()) if "php7" in self.pkg_vars else os.path.join(root_dir, "php7")
            php85_tgt = os.path.normpath(self.pkg_vars["php85"]["target_path_var"].get().strip()) if "php85" in self.pkg_vars else os.path.join(root_dir, "php85")
            node_tgt = os.path.normpath(self.pkg_vars["node"]["target_path_var"].get().strip()) if "node" in self.pkg_vars else os.path.join(root_dir, "node")

            # 防呆：若目標 Apache24 目錄存在雙層 Apache24\Apache24 巢狀結構，自動攤平修復
            if os.path.isdir(apache_tgt):
                flatten_nested_apache_dir(apache_tgt, self.log)

            # 3.5a：處理 PHP 7 與 PHP 8.5 設定檔 (php.ini) 與資安弱點防護
            if os.path.isdir(php7_tgt) and (has_php7 or self.restore_backup_var.get() or self.auto_security_hardening_var.get()):
                php7_ini = os.path.join(php7_tgt, "php.ini")
                if not os.path.isfile(php7_ini):
                    for cand in ("php.ini-production", "php.ini-development"):
                        cand_path = os.path.join(php7_tgt, cand)
                        if os.path.isfile(cand_path):
                            shutil.copyfile(cand_path, php7_ini)
                            self.log(f"  ✓ 已依據 {cand} 產生 PHP 7 設定檔 (php.ini)", "info")
                            break
                if os.path.isfile(php7_ini) and self.auto_security_hardening_var.get():
                    apply_php_security_hardening(php7_ini, self.log)

            if os.path.isdir(php85_tgt) and (has_php85 or self.restore_backup_var.get() or self.auto_security_hardening_var.get()):
                php85_ini = os.path.join(php85_tgt, "php.ini")
                if not os.path.isfile(php85_ini):
                    for cand in ("php.ini-production", "php.ini-development"):
                        cand_path = os.path.join(php85_tgt, cand)
                        if os.path.isfile(cand_path):
                            shutil.copyfile(cand_path, php85_ini)
                            self.log(f"  ✓ 已依據 {cand} 產生 PHP 8.5 設定檔 (php.ini)", "info")
                            break
                if os.path.isfile(php85_ini) and self.auto_security_hardening_var.get():
                    apply_php_security_hardening(php85_ini, self.log)

            # 3.5b：處理 Apache 設定檔 (httpd.conf)、資安強化與 Apache2.4 Windows 服務註冊
            if os.path.isdir(apache_tgt) and (has_apache or has_php7 or has_php85 or self.restore_backup_var.get() or self.auto_security_hardening_var.get() or self.auto_register_apache_var.get()):
                self.set_progress(91, "正在自動校正 Apache 與 PHP 整合設定檔...")
                self.log("⏳ 正在檢查並自動校正 Apache 與 PHP 整合設定...", "info")

                conf_file = os.path.join(apache_tgt, "conf", "httpd.conf")
                if not os.path.isfile(conf_file):
                    ref_conf = os.path.join(root_dir, "LAH-API", "httpd.conf-ref")
                    if not os.path.isfile(ref_conf):
                        ref_conf = os.path.join(self.app_dir, "httpd.conf-ref")
                    if os.path.isfile(ref_conf):
                        os.makedirs(os.path.join(apache_tgt, "conf"), exist_ok=True)
                        shutil.copyfile(ref_conf, conf_file)
                        self.log(f"  ✓ 已套用專案標準 httpd.conf 參考範本至 [{conf_file}]", "info")

                configure_apache_environment(
                    apache_dir=apache_tgt,
                    root_dir=root_dir,
                    php7_dir=php7_tgt if os.path.isdir(php7_tgt) else None,
                    php85_dir=php85_tgt if os.path.isdir(php85_tgt) else None,
                    enable_security=self.auto_security_hardening_var.get(),
                    log_func=self.log
                )

                if self.auto_register_apache_var.get():
                    self.set_progress(92, "正在檢查並註冊 Apache Windows 服務...")
                    install_apache_service(apache_tgt, "Apache2.4", self.log)
                    self.apache_service_name = query_apache_service()
                    if self.apache_service_name:
                        svc_name = self.apache_service_name
                        self.root.after(0, self._detect_apache_service)

            # 若目標目錄存在 node，自動將目標 node 目錄註冊至使用者 PATH 最前端
            if os.path.isdir(node_tgt):
                self.set_progress(93, "正在註冊綠色 Node.js 環境變數 PATH...")
                register_path_to_user_environment(node_tgt, self.log)

            # 若目標目錄存在 instantclient_11_2，自動註冊至使用者 PATH
            oci_dir = os.path.join(root_dir, "instantclient_11_2")
            if os.path.isdir(oci_dir):
                register_path_to_user_environment(oci_dir, self.log)

            # 步驟 4：啟動或重啟 Apache 服務（若更新前曾停止、或勾選了重啟、或全新安裝註冊了 Apache 服務）
            target_svc = self.apache_service_name or svc_name or query_apache_service()
            should_start_apache = (
                apache_was_stopped or
                self.restart_apache_var.get() or
                has_apache or
                has_base_env_update or
                has_apache_app_update
            )
            if target_svc and should_start_apache and not apache_restarted:
                if not is_service_running(target_svc):
                    self.set_progress(95, f"正在啟動 Apache 服務 [{target_svc}]...")
                    start_apache_service(target_svc, self.log)
                apache_restarted = True

            # 步驟 5：確認 PM2 使用本機綠色版 Node、拉起 5 大核心服務並配置 PM2 自動化服務
            target_pm2_cmd = resolve_pm2_command(root_dir)
            should_run_pm2 = self.restart_pm2_var.get() or has_node_runtime or has_nodejs_app or self.restore_backup_var.get()

            if should_run_pm2 and target_pm2_cmd:
                self.root.after(0, self._detect_pm2_service)
                self.set_progress(96, "正在確認 PM2 核心守護進程綁定本機綠色版 Node...")
                daemon_info = query_pm2_daemon_info(root_dir)
                target_node = os.path.normpath(os.path.join(root_dir, "node", "node.exe"))

                if pm2_was_killed or not daemon_info["is_running"]:
                    self.set_progress(97, "正在透過綠色版 Node 喚醒 PM2 守護程序並拉起核心服務...")
                    self.log(f"🚀 正在透過本機綠色版 Node ({target_node}) 喚醒 PM2 並巡檢啟動 5 大核心服務...", "info")
                    res_ok, res_err = resurrect_pm2_services(self.log, root_dir)
                    if not res_ok:
                        self.log(f"⚠️ PM2 resurrect 提示 ({res_err})，正在執行全方位生態系補啟動...", "warning")
                        audit_and_ensure_pm2_services(self.log, root_dir)
                elif not daemon_info["is_target_node"]:
                    self.log(f"⚠️ 偵測到現有 PM2 核心由外部 Node 運行 [{daemon_info['exe_path']}]", "warning")
                    self.log(f"🚀 正在自動將 PM2 核心無痛切換綁定至本機綠色版 Node ({target_node})...", "info")
                    align_ok, align_msg = align_pm2_daemon_to_portable(self.log, root_dir)
                    if not align_ok:
                        self.log(f"⚠️ PM2 核心切換回傳警示: {align_msg}", "warning")
                    audit_and_ensure_pm2_services(self.log, root_dir)
                else:
                    self.set_progress(97, "正在執行 pm2 restart all 重啟並巡檢 PM2 服務...")
                    restart_pm2_services(self.log, root_dir)

                # 確認並自動配置 PM2 開機自動化守護服務 (LAH_PM2_Service / 排程)
                self.set_progress(99, "正在設定 PM2 開機自動化服務 (LAH_PM2_Service)...")
                self.log("⚙️ 正在設定並確認 PM2 開機自動化服務與守護進程...", "info")
                auto_ok, auto_msg = setup_pm2_autostart(self.log, root_dir)
                if auto_ok:
                    self.log(f"✅ PM2 自動化配置完成：{auto_msg}", "success")
                else:
                    self.log(f"⚠️ PM2 自動化配置提示：{auto_msg}", "warning")

                # 即時更新主視窗 PM2 狀態標籤
                self.root.after(0, self.update_pm2_daemon_status)
            elif should_run_pm2 and not target_pm2_cmd:
                self.log("ℹ️ 系統尚未安裝 Node.js / PM2 執行環境，略過 PM2 服務啟動與自動化設定。", "info")
            else:
                self.log("ℹ️ 未勾選「更新後重啟 PM2 服務」，略過 PM2 重啟作業。", "info")

            elapsed = time.time() - start_time
            self.set_progress(100, f"✅ 全部安裝與自動化設定作業完成！(耗時 {elapsed:.1f} 秒)")
            self.log("=" * 60, "success")
            self.log(f"🎉 全部選取的安裝包與系統自動化配置已順利完成！(總耗時: {elapsed:.1f} 秒)", "success")

            # 先解除更新鎖定並更新進度視窗按鈕，再彈出壓縮包清理詢問對話框（預設不刪除）
            def _post_update_dialog():
                self._on_update_completed_ui()
                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root

                # 彙整本次所有選取模組對應之全部壓縮檔路徑（包含別名壓縮檔如 LAH-FE-node_modules.zip）
                zips_to_clean = []
                seen_z = set()
                for pkg in selected_pkgs:
                    v = self.pkg_vars[pkg["id"]]
                    paths = v.get("all_filepaths") or ([v["filepath"]] if v.get("filepath") else [])
                    for zp in paths:
                        if zp and os.path.isfile(zp):
                            nz = os.path.normpath(zp).lower()
                            if nz not in seen_z:
                                seen_z.add(nz)
                                zips_to_clean.append(zp)

                if not zips_to_clean:
                    return

                zip_names_preview = "\n".join([f"  • {os.path.basename(p)}" for p in zips_to_clean[:8]])
                if len(zips_to_clean) > 8:
                    zip_names_preview += f"\n  ... 等共 {len(zips_to_clean)} 個壓縮檔"

                ask_cleanup = messagebox.askyesno(
                    "安裝/更新完成 - 是否刪除安裝壓縮包？",
                    f"🎉 所有選取的系統模組與自動化配置已順利完成！(耗時 {elapsed:.1f} 秒)\n\n"
                    f"請問是否要刪除本次已安裝完成的壓縮包 (.zip) 以釋放磁碟空間？\n"
                    f"{zip_names_preview}\n\n"
                    f"• 點選「否」(預設建議)：完整保留所有安裝壓縮包。\n"
                    f"• 點選「是」：刪除上述已安裝完成的 zip 檔（說明檔與備份檔仍會保留）。",
                    default=messagebox.NO,
                    parent=parent_w
                )
                if ask_cleanup:
                    del_count = 0
                    for zpath in zips_to_clean:
                        zname = os.path.basename(zpath)
                        if os.path.isfile(zpath):
                            try:
                                os.remove(zpath)
                                self.log(f"🗑️ 已刪除壓縮包: {zname}", "info")
                                del_count += 1
                            except Exception as de:
                                self.log(f"⚠️ 刪除 {zname} 失敗: {de}", "warning")
                    if del_count > 0:
                        self.log(f"✅ 已清理 {del_count} 個更新壓縮包，釋放磁碟空間。", "success")
                        self.rescan()
                else:
                    self.log("ℹ️ 使用者選擇保留安裝壓縮包 (.zip)，未執行刪除。", "info")

            self.root.after(0, _post_update_dialog)

        except PermissionError as pe:
            self.log(f"❌ 更新失敗 (存取權限被拒絕): {pe}", "error")
            self.log("💡 建議：請確認是否有其他程式或 PM2 服務正在鎖定該檔案（若更新 node_modules，建議先執行 pm2 stop all），或確認管理員權限。", "warning")
            def _show_perm_err():
                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                messagebox.showerror(
                    "更新失敗",
                    f"存取權限被拒絕！\n{pe}\n\n"
                    "請以系統管理員權限執行，或關閉正在使用該檔案的軟體。\n"
                    "（若解壓縮 node_modules 等模組失敗，請先於終端機執行 pm2 stop all 釋放檔案鎖定後再試）",
                    parent=parent_w
                )
            self.root.after(0, _show_perm_err)
        except Exception as ex:
            self.log(f"❌ 更新過程發生未預期錯誤: {ex}", "error")
            def _show_unexp_err():
                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                messagebox.showerror("更新錯誤", f"更新過程發生錯誤：\n{ex}", parent=parent_w)
            self.root.after(0, _show_unexp_err)
        finally:
            # 確保若先前停止過 Apache 服務，發生異常中斷時也一定嘗試重啟，維持系統服務運作
            try:
                if should_stop_apache and svc_name and apache_was_stopped and not apache_restarted:
                    self.log(f"⚠️ 偵測到更新中斷或異常，緊急嘗試恢復 Apache 服務 [{svc_name}]...", "warning")
                    start_apache_service(svc_name, self.log)
                    apache_restarted = True
            except Exception as fin_ex:
                self.log(f"⚠️ 恢復 Apache 服務時發生例外: {fin_ex}", "warning")
            self.root.after(0, self._on_update_completed_ui)

    def _on_close(self):
        """視窗關閉防呆"""
        if self.is_updating:
            parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
            if not messagebox.askyesno("警告", "更新作業正在背景進行中，確定要強制中斷離開嗎？", parent=parent_w):
                return
        self.root.destroy()


def main():
    root = tk.Tk()
    app = UpdaterApp(root)
    root.mainloop()


if __name__ == "__main__":
    main()
