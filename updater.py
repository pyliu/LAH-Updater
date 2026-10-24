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

# 定義 7 種更新包規格
PACKAGE_CONFIGS = [
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
        "rel_target": os.path.join("LAH-NUXTJS", "node_modules"),
        "title": "└─ 前端網站 核心套件 (LAH-NUXTJS-node_modules.zip)",
        "expected_folder": "node_modules",
        "alt_folders": ["lah-nuxtjs-node_modules", "LAH-NUXTJS-node_modules"],
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
                text=True,
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
            text=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        val = res.stdout.strip()
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
            text=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        return "STATE" in res.stdout and "RUNNING" in res.stdout
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
            text=True,
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
            text=True,
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
            # 還原後執行全方位核心巡檢補啟動，確保 LAH-WSS 等服務皆完整在線
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

    # 無論重啟結果為何，執行全方位核心巡檢，自動補齊並拉起遺漏之 LAH-WSS、LAH-FE、LAH-SCHEDULER、lah-api-v85
    audit_ok, audit_msg = audit_and_ensure_pm2_services(log_func, root_dir)
    return audit_ok, audit_msg


def audit_and_ensure_pm2_services(log_func, root_dir=None):
    """
    全方位巡檢 PM2 核心服務（LAH-WSS、LAH-FE、LAH-SCHEDULER、lah-api-v85）。
    1. 自動建立並確保 logs 目錄存在，杜絕日誌路徑缺失導致啟動失敗。
    2. 透過 pm2 jlist 取得目前所有執行中服務清單。
    3. 若發現核心服務遺漏或離線，自動由目標目錄 (如 C:\\AppServ) 之 ecosystem.config.js 補啟動 (pm2 start)。
    4. 若有補啟動任何服務，自動執行 pm2 save 保存快照至 dump.pm2。
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
        log_func(f"⚠️ 查詢 PM2 進程狀態 (pm2 jlist) 發生異常: {e}，將嘗試直接補啟動缺漏設定檔...", "warning")

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
    for app_name in ["LAH-WSS", "LAH-FE", "LAH-SCHEDULER", "lah-api-v85"]:
        st = final_apps.get(app_name, "未註冊 / 離線")
        if st == "online":
            log_func(f"  • {app_name:<16} : ✅ {st}", "success")
        else:
            all_healthy = False
            log_func(f"  • {app_name:<16} : ⚠️ {st}", "warning")

    if all_healthy:
        log_func("🎉 全部 4 大核心服務（含 LAH-WSS）皆已成功在線運作！", "success")
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


def setup_pm2_autostart(log_func, root_dir=None):
    r"""
    設定 Windows 開機/登入時自動喚醒 PM2 守護程序 (pm2 resurrect)。
    1. 產生無黑視窗背景執行腳本 (pm2-autostart.vbs)
    2. 部署至使用者啟動資料夾 (%APPDATA%\...\Startup\LAH-PM2-Autostart.vbs)
    3. 嘗試於 Windows 工作排程器註冊 LAH_PM2_Autostart (ONLOGON)
    4. 執行 pm2 save 確保程序清單最新
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    pm2_cmd = resolve_pm2_command(target_root) or os.path.normpath(os.path.join(target_root, "node", "pm2.cmd"))
    node_dir = os.path.dirname(pm2_cmd)

    os.makedirs(node_dir, exist_ok=True)
    vbs_path = os.path.normpath(os.path.join(node_dir, "pm2-autostart.vbs"))

    # 建立無黑視窗 VBScript 腳本 (0 = SW_HIDE, False = 不阻擋)
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
        log_func(f"  ✓ 已建立 PM2 靜默喚醒腳本: {vbs_path}", "info")
    except Exception as e:
        log_func(f"  ⚠️ 寫入 {vbs_path} 失敗 ({e})，嘗試寫入至 %USERPROFILE%\\.pm2...", "warning")
        user_pm2_dir = os.path.join(os.environ.get("USERPROFILE", ""), ".pm2")
        os.makedirs(user_pm2_dir, exist_ok=True)
        vbs_path = os.path.normpath(os.path.join(user_pm2_dir, "pm2-autostart.vbs"))
        with open(vbs_path, "w", encoding="ascii") as f:
            f.write(vbs_content)
        log_func(f"  ✓ 已建立備用 PM2 靜默喚醒腳本: {vbs_path}", "info")

    success_items = []

    # 1. 部署至目前使用者啟動資料夾 (Startup Folder)
    appdata = os.environ.get("APPDATA", "")
    if appdata:
        startup_dir = os.path.join(appdata, r"Microsoft\Windows\Start Menu\Programs\Startup")
        if os.path.isdir(startup_dir):
            user_startup_vbs = os.path.join(startup_dir, "LAH-PM2-Autostart.vbs")
            try:
                shutil.copyfile(vbs_path, user_startup_vbs)
                log_func(f"  ✓ 已部署開機自啟動腳本至使用者啟動資料夾: {user_startup_vbs}", "success")
                success_items.append("使用者啟動資料夾")
            except Exception as e:
                log_func(f"  ⚠️ 寫入使用者啟動資料夾失敗: {e}", "warning")

    # 2. 嘗試註冊 Windows 工作排程器 (Task Scheduler)
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
            log_func("  ✓ 已成功註冊 Windows 工作排程器 (最高權限 ONLOGON): LAH_PM2_Autostart", "success")
            success_items.append("工作排程器 (最高權限)")
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
            else:
                log_func(f"  ℹ️ 工作排程器略過 (由啟動資料夾完全接管): {res_low.stderr.strip() or res.stderr.strip()}", "info")
    except Exception as e:
        log_func(f"  ℹ️ 工作排程器註冊略過: {e}", "info")

    # 3. 確保最新程序清單存檔 (pm2 save)
    log_func("  正在儲存當前 PM2 程序清單至 dump.pm2...", "info")
    save_pm2_services(log_func, target_root)

    if success_items:
        msg = f"已成功啟用 PM2 開機自啟動 ({' + '.join(success_items)})"
        return True, msg
    else:
        return False, "未能建立啟動項目，請確認磁碟寫入與系統權限！"


def remove_pm2_autostart(log_func):
    r"""
    清除 Windows 開機/登入時之 PM2 自啟動配置（工作排程器與啟動資料夾）。
    """
    removed_items = []

    # 1. 刪除工作排程
    try:
        res = subprocess.run(
            ["schtasks", "/Delete", "/TN", "LAH_PM2_Autostart", "/F"],
            capture_output=True, text=True, timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
        )
        if res.returncode == 0:
            log_func("  ✓ 已自 Windows 工作排程器移除: LAH_PM2_Autostart", "info")
            removed_items.append("工作排程器")
    except Exception as e:
        log_func(f"  ℹ️ 工作排程移除略過: {e}", "info")

    # 2. 清除啟動資料夾
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
                        log_func(f"  ✓ 已自啟動資料夾移除: {target_file}", "info")
                        removed_items.append(f"啟動資料夾 ({fname})")
                    except Exception as e:
                        log_func(f"  ⚠️ 刪除 {target_file} 失敗: {e}", "warning")

    if removed_items:
        return True, f"已順利停用並清除 PM2 開機自啟動項目 ({', '.join(removed_items)})。"
    else:
        return True, "目前系統中無待清除的 PM2 自啟動項目。"


def test_pm2_autostart_execution(log_func, root_dir=None):
    r"""
    立即測試執行 PM2 開機喚醒腳本 (pm2-autostart.vbs)，驗證其是否能在背景順利拉起服務。
    """
    target_root = root_dir if (root_dir and os.path.exists(root_dir)) else r"C:\AppServ"
    pm2_cmd = resolve_pm2_command(target_root) or os.path.normpath(os.path.join(target_root, "node", "pm2.cmd"))
    node_dir = os.path.dirname(pm2_cmd)
    vbs_path = os.path.normpath(os.path.join(node_dir, "pm2-autostart.vbs"))

    if not os.path.exists(vbs_path):
        alt_vbs = os.path.normpath(os.path.join(os.environ.get("USERPROFILE", ""), ".pm2", "pm2-autostart.vbs"))
        if os.path.exists(alt_vbs):
            vbs_path = alt_vbs
        else:
            return False, f"找不到喚醒腳本 {vbs_path}，請先點擊「一鍵啟用」進行配置。"

    log_func(f"🚀 正在測試執行靜默喚醒腳本: wscript.exe \"{vbs_path}\"...", "info")
    try:
        subprocess.run(
            ["cscript.exe", "//nologo", vbs_path],
            check=True, timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
        )
        time.sleep(2)

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

        log_func(f"✅ 測試執行完成！連接埠狀態：{', '.join(ports_status)}", "success")
        return True, f"測試喚醒成功！\n" + "\n".join([f"• {p}" for p in ports_status])
    except Exception as e:
        log_func(f"❌ 測試執行失敗: {e}", "error")
        return False, str(e)


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
    如果壓縮檔頂層已經包含與預期名稱、壓縮檔名或目標目錄同名的單一資料夾，
    且無散落於根目錄的單獨檔案，則傳回該資料夾前綴 (例如 'www/')，
    後續解壓縮時自動對齊消除外層，避免變成 www\\www 的雙層目錄！
    若頂層直接散落各檔案或有多個不同頂層資料夾，則傳回空字串 ""。
    """
    top_dirs = set()
    has_root_files = False

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
            break
        top_dirs.add(parts[0])

    if not has_root_files and len(top_dirs) == 1:
        single_dir = list(top_dirs)[0]
        if single_dir.lower() in candidates:
            return single_dir + '/'

    return ""


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
        self.root.geometry("980x750")
        self.root.minsize(860, 640)

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
        self.rescan()
        # 啟動時偵測預設安裝目錄是否存在，若無則提示使用者選擇
        self._check_default_root_dir()

    def _init_variables(self):
        # 根目錄路徑
        self.root_dir_var = tk.StringVar(value=r"C:\AppServ")
        self.root_dir_var.trace_add("write", self._on_root_dir_changed)

        # 各壓縮包狀態變數
        self.pkg_vars = {}
        for pkg in PACKAGE_CONFIGS:
            self.pkg_vars[pkg["id"]] = {
                "checked": tk.BooleanVar(value=False),
                "exists": False,
                "filepath": "",
                "filesize": 0,
                "status_var": tk.StringVar(value="⚪ 偵測中..."),
                "target_path_var": tk.StringVar(value=""),
                "is_custom_path": False
            }

        # Apache 自動重啟勾選（預設留白，偵測到本機服務時自動勾選）
        self.restart_apache_var = tk.BooleanVar(value=False)

        # PM2 自動重啟勾選（預設留白，偵測到本機 PM2 套件時自動勾選）
        self.restart_pm2_var = tk.BooleanVar(value=False)
        self.pm2_daemon_status_var = tk.StringVar(value="⚪ 偵測中...")

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
        header_frame.pack(side=tk.TOP, fill=tk.X, pady=(0, 6))

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

        # 【關鍵佈局順序修復】：底部按鈕列必須在中央展開區塊之前優先向 side=tk.BOTTOM pack！
        # 如此無論視窗解析度或 DPI 縮放為何，底部控制按鈕永遠 100% 完整保留，絕對不會被擠壓或裁切！
        bottom_frame = ttk.Frame(main_frame)
        bottom_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(6, 0))

        self.btn_exit = ttk.Button(bottom_frame, text="關閉離開", command=self._on_close, width=12)
        self.btn_exit.pack(side=tk.RIGHT, padx=(8, 0))

        self.btn_start = tk.Button(
            bottom_frame,
            text="🚀 開始更新",
            command=self.start_update_thread,
            font=("Microsoft JhengHei UI", 14, "bold"),
            bg="#1976d2",
            fg="#ffffff",
            activebackground="#1565c0",
            activeforeground="#ffffff",
            relief=tk.FLAT,
            padx=18,
            pady=4,
            cursor="hand2"
        )
        self.btn_start.pack(side=tk.RIGHT, padx=(8, 0))

        self.btn_view_log = ttk.Button(
            bottom_frame,
            text="📊 檢視執行日誌",
            command=self._open_progress_dialog,
            width=16
        )
        self.btn_view_log.pack(side=tk.RIGHT, padx=(8, 0))

        self.btn_view_notes = ttk.Button(
            bottom_frame,
            text="📋 查看更新說明",
            command=self._show_release_notes_dialog,
            state="disabled",
            width=16
        )
        self.btn_view_notes.pack(side=tk.RIGHT)

        # 2. 安裝根目錄選擇區塊 (side=tk.TOP)
        dir_frame = ttk.LabelFrame(main_frame, text=" 📂 安裝根目錄設定 (預設連動基底) ", padding="6")
        dir_frame.pack(side=tk.TOP, fill=tk.X, pady=(0, 6))

        dir_inner = ttk.Frame(dir_frame)
        dir_inner.pack(fill=tk.X)

        ttk.Label(dir_inner, text="系統安裝根目錄：", font=("Microsoft JhengHei UI", 11, "bold")).pack(side=tk.LEFT, padx=(0, 6))
        self.root_entry = ttk.Entry(dir_inner, textvariable=self.root_dir_var, font=("Consolas", 12))
        self.root_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 8))

        self.btn_browse = ttk.Button(dir_inner, text="瀏覽...", command=self._browse_root_dir, width=10)
        self.btn_browse.pack(side=tk.RIGHT)

        # 3. 更新包偵測清單區塊 (side=tk.TOP, 自適應剩餘中央空間)
        pkg_frame = ttk.LabelFrame(main_frame, text=" 📦 更新包偵測與各模組安裝路徑 ", padding="6")
        pkg_frame.pack(side=tk.TOP, fill=tk.BOTH, expand=True, pady=(0, 6))

        # 表頭提示
        header_hint = ttk.Label(
            pkg_frame,
            text=f"來源更新目錄：{self.app_dir}  (各模組路徑預設跟隨根目錄，亦可單獨點擊「瀏覽...」自訂)",
            foreground="#555555",
            font=("Microsoft JhengHei UI", 10)
        )
        header_hint.pack(anchor=tk.W, pady=(0, 4))

        # 建立 7 個更新包項目元件（兩列卡片式佈局，支援個別自訂目標路徑與附屬套件縮排）
        self.chk_widgets = {}
        self.path_entries = {}
        self.browse_buttons = {}
        self.reset_buttons = {}

        for pkg in PACKAGE_CONFIGS:
            p_id = pkg["id"]
            v = self.pkg_vars[p_id]
            is_sub = pkg.get("is_sub_package", False)

            # 附屬子套件卡片整體向右縮排 (left=26px)
            card_padding = (26, 1, 0, 1) if is_sub else (0, 1, 0, 1)
            item_card = ttk.Frame(pkg_frame, padding=card_padding)
            item_card.pack(fill=tk.X, pady=1)

            # 第 1 列：核取方塊 + 狀態標籤
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

            # 第 2 列：縮排的個別目標解壓縮路徑輸入框 + 瀏覽按鈕 + 重設按鈕
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

        # 服務控制與功能按鈕列
        # 第 1 列：Apache 服務偵測重啟選項 + 重新掃描按鈕
        ctrl_row_apache = ttk.Frame(pkg_frame, padding="2")
        ctrl_row_apache.pack(fill=tk.X, pady=(6, 2))

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

        # 第 2 列：PM2 管理套件偵測與更新後重啟服務選項（置於 Apache 下方）
        ctrl_row_pm2 = ttk.Frame(pkg_frame, padding="2")
        ctrl_row_pm2.pack(fill=tk.X, pady=(1, 2))

        self.chk_pm2 = ttk.Checkbutton(
            ctrl_row_pm2,
            text="更新後自動重啟 PM2 服務",
            variable=self.restart_pm2_var,
            onvalue=True,
            offvalue=False
        )
        self.chk_pm2.pack(side=tk.LEFT)

        # PM2 核心守護進程綁定狀態標籤
        self.lbl_pm2_status = tk.Label(
            ctrl_row_pm2,
            textvariable=self.pm2_daemon_status_var,
            font=("Microsoft JhengHei UI", 10, "bold"),
            fg="#555555"
        )
        self.lbl_pm2_status.pack(side=tk.LEFT, padx=(10, 6))

        # 手動一鍵校正 PM2 核心按鈕
        self.btn_align_pm2 = ttk.Button(
            ctrl_row_pm2,
            text="⚡ 立即校正 PM2 核心",
            command=self.manual_align_pm2_daemon,
            width=18
        )
        self.btn_align_pm2.pack(side=tk.RIGHT, padx=(4, 0))

        # PM2 開機自啟動管理按鈕
        self.btn_autostart_pm2 = ttk.Button(
            ctrl_row_pm2,
            text="⚙️ 開機自啟動設定",
            command=self.manage_pm2_autostart,
            width=16
        )
        self.btn_autostart_pm2.pack(side=tk.RIGHT, padx=(4, 0))

        self.root.protocol("WM_DELETE_WINDOW", self._on_close)

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
                text="關閉視窗 (Esc)",
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

            self.root.after(0, _apply_ui)

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
            dx = max(30, rx + (rw - 680) // 2)
            dy = max(30, ry + (rh - 520) // 2)
            dlg.geometry(f"680x520+{dx}+{dy}")
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
            "• 在 Windows 環境下，PM2 預設不會在重新開機後自動啟動背景守護進程。\n"
            "• 啟用開機自啟動後，系統登入時會自動透過便攜版 Node 執行 pm2 resurrect 喚醒服務。\n"
            "• 採用專屬 VBScript 隱藏視窗技術，背景靜默啟動，保證不彈出任何黑色 CMD 視窗。\n"
            "• 開機自啟動將自動恢復即時通 WSS (8081/8082) 與 PHP8 API (8085) 等所有受管服務。"
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

            if is_enabled:
                status_var.set("🟢 狀態：開機自啟動【已啟用】")
                lbl_status.config(fg="#2e7d32")
            else:
                status_var.set("⚪ 狀態：開機自啟動【未設定】")
                lbl_status.config(fg="#777777")

            txt_detail.configure(state="normal")
            txt_detail.delete("1.0", tk.END)

            lines = []
            if has_sc:
                lines.append(f"[✓] 啟動資料夾捷徑/腳本 ({len(cfg['startup_shortcuts'])} 項):")
                for sc in cfg["startup_shortcuts"]:
                    lines.append(f"    • {sc}")
            else:
                lines.append("[-] 啟動資料夾 (Startup)：未設定")

            if has_task:
                lines.append(f"\n[✓] Windows 工作排程器 ({len(cfg['tasks'])} 項):")
                for t in cfg["tasks"]:
                    lines.append(f"    • {t.get('TaskName', 'PM2')} (狀態: {t.get('State', '未知')})")
            else:
                lines.append("\n[-] Windows 工作排程器：未設定")

            if has_svc:
                lines.append(f"\n[✓] Windows 服務 ({len(cfg['services'])} 項):")
                for s in cfg["services"]:
                    lines.append(f"    • {s.get('Name', 'PM2')} (狀態: {s.get('State', '未知')})")
            else:
                lines.append("\n[-] Windows 服務：未設定")

            txt_detail.insert(tk.END, "\n".join(lines))
            txt_detail.configure(state="disabled")

        def _on_enable():
            confirm = messagebox.askyesno(
                "啟用確認",
                "即將設定 Windows 開機/登入時自動於背景靜默喚醒 PM2 服務。\n\n"
                "操作項目：\n"
                "1. 建立靜默背景腳本 (pm2-autostart.vbs)\n"
                "2. 部署至 Windows 啟動資料夾\n"
                "3. 註冊 Windows 工作排程器 (LAH_PM2_Autostart)\n"
                "4. 儲存當前受管服務清單 (pm2 save)\n\n"
                "是否立即啟用？",
                parent=dlg
            )
            if not confirm:
                return

            self.log("=" * 50, "info")
            self.log("⚙️ 正在啟用 PM2 開機自啟動設定...", "info")
            ok, msg = setup_pm2_autostart(self.log, root_dir)
            _refresh_status()
            if ok:
                messagebox.showinfo("啟用成功", f"🎉 {msg}\n\n電腦重新開機或使用者登入時，系統將自動於背景喚醒 PM2 並拉起服務！", parent=dlg)
            else:
                messagebox.showerror("啟用失敗", f"設定過程發生錯誤：\n{msg}", parent=dlg)

        def _on_disable():
            confirm = messagebox.askyesno(
                "停用確認",
                "確定要停用並清除 PM2 開機自啟動設定嗎？\n\n停用後，重新開機將不會自動啟動 PM2 服務。",
                parent=dlg
            )
            if not confirm:
                return

            self.log("=" * 50, "info")
            self.log("🛑 正在停用並清除 PM2 開機自啟動設定...", "info")
            ok, msg = remove_pm2_autostart(self.log)
            _refresh_status()
            messagebox.showinfo("自啟動設定", msg, parent=dlg)

        def _on_test():
            self.log("=" * 50, "info")
            self.log("⚡ 正在測試執行 PM2 開機喚醒流程...", "info")
            ok, msg = test_pm2_autostart_execution(self.log, root_dir)
            _refresh_status()
            if ok:
                messagebox.showinfo("測試結果", f"🎉 {msg}", parent=dlg)
            else:
                messagebox.showwarning("測試結果", f"測試過程回傳：\n{msg}", parent=dlg)

        btn_enable = ttk.Button(btn_row, text="🚀 一鍵啟用 / 重新配置", command=_on_enable, width=22)
        btn_enable.pack(side=tk.LEFT, padx=(0, 6))

        btn_disable = ttk.Button(btn_row, text="🛑 停用 / 清除設定", command=_on_disable, width=16)
        btn_disable.pack(side=tk.LEFT, padx=(0, 6))

        btn_test = ttk.Button(btn_row, text="⚡ 立即測試喚醒", command=_on_test, width=16)
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
            zip_path = os.path.join(self.app_dir, p_name)

            if os.path.isfile(zip_path):
                v["exists"] = True
                v["filepath"] = zip_path
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

        summary = "\n".join([f"• {p['name']} ➔ {self.pkg_vars[p['id']]['target_path_var'].get().strip()}" for p in selected_pkgs])
        apache_note = f"\n• 更新前後將自動重啟 Apache 服務 ({self.apache_service_name or '自動搜尋'})" if self.restart_apache_var.get() else ""
        pm2_note = "\n• 更新完成後將自動重啟 PM2 服務 (pm2 restart all)" if self.restart_pm2_var.get() else ""
        node_note = "\n• 包含 Node.js 綠色執行環境或核心依賴套件更新，解壓前將安全暫停 Node/PM2 服務以釋放檔案鎖定" if any(p["id"] == "node" or p.get("is_sub_package", False) for p in selected_pkgs) else ""

        confirm = messagebox.askyesno(
            "確認開始更新",
            f"即將開始更新以下項目：\n{summary}{apache_note}{pm2_note}{node_note}\n\n是否確認立即執行解壓縮覆蓋？"
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

        svc_name = self.apache_service_name or query_apache_service()
        should_restart_apache = self.restart_apache_var.get()
        apache_was_stopped = False
        apache_restarted = False

        try:
            # 步驟 1：若有勾選，先停止 Apache 服務
            if should_restart_apache:
                if svc_name:
                    self.set_progress(2, f"正在停止 Apache 服務 [{svc_name}]...")
                    apache_was_stopped = stop_apache_service(svc_name, self.log)
                else:
                    self.log("ℹ️ 勾選了重啟 Apache，但系統未偵測到 Apache 服務，跳過停止步驟。", "warning")

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
                        if strip_prefix and raw_name.startswith(strip_prefix):
                            rel_sub = raw_name[len(strip_prefix):].lstrip('/')
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

            # 若本次有更新 node.zip，自動將目標 node 目錄註冊至使用者 PATH 最前端
            if is_updating_node:
                self.set_progress(92, "正在註冊綠色 Node.js 環境變數 PATH...")
                register_path_to_user_environment(node_tgt, self.log)

            # 步驟 4：若更新前有停止 Apache，或勾選了重啟，在此重新啟動服務
            if should_restart_apache and svc_name and apache_was_stopped and not apache_restarted:
                self.set_progress(95, f"正在啟動 Apache 服務 [{svc_name}]...")
                start_apache_service(svc_name, self.log)
                apache_restarted = True

            elapsed = time.time() - start_time
            self.set_progress(100, f"✅ 全部更新作業完成！(耗時 {elapsed:.1f} 秒)")
            self.log("=" * 60, "success")
            self.log(f"🎉 全部選取的更新包已順利更新完成！(總耗時: {elapsed:.1f} 秒)", "success")

            # 步驟 5：若有勾選「更新後自動重啟 PM2 服務」，自動執行重啟、還原或核心校正
            if self.restart_pm2_var.get():
                self.set_progress(97, "正在檢查 PM2 核心守護進程狀態...")
                daemon_info = query_pm2_daemon_info(root_dir)
                target_node = os.path.normpath(os.path.join(root_dir, "node", "node.exe"))

                if pm2_was_killed or not daemon_info["is_running"]:
                    self.set_progress(98, "正在透過綠色環境喚醒 PM2 守護程序並還原服務 (pm2 resurrect)...")
                    self.log("🚀 偵測到 PM2 守護程序未運行或先前已停止，正在執行 [pm2 resurrect] 自備份還原服務...", "info")
                    res_ok, res_err = resurrect_pm2_services(self.log, root_dir)
                    elapsed = time.time() - start_time
                    self.set_progress(100, f"✅ 全部更新與 PM2 服務還原完成！(耗時 {elapsed:.1f} 秒)")
                    if not res_ok:
                        warn_event = threading.Event()

                        def _warn_res():
                            try:
                                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                                messagebox.showwarning(
                                    "PM2 還原提醒",
                                    f"執行 PM2 服務還原時回傳非預期結果：\n{res_err}\n\n"
                                    "請稍後於終端機手動執行 pm2 resurrect 或檢查 PM2 服務狀態。",
                                    parent=parent_w
                                )
                            finally:
                                warn_event.set()

                        self.root.after(0, _warn_res)
                        warn_event.wait()
                elif not daemon_info["is_target_node"]:
                    self.log(f"⚠️ 偵測到現有 PM2 核心由外部 Node 運行 [{daemon_info['exe_path']}]", "warning")
                    self.log(f"🚀 正在自動執行無痛切換：將 PM2 核心校正綁定至綠色環境 ({target_node})...", "info")
                    align_ok, align_msg = align_pm2_daemon_to_portable(self.log, root_dir)
                    elapsed = time.time() - start_time
                    self.set_progress(100, f"✅ 全部更新與 PM2 核心切換完成！(耗時 {elapsed:.1f} 秒)")
                    if not align_ok:
                        self.log(f"⚠️ PM2 核心切換回傳警示: {align_msg}", "warning")
                else:
                    self.set_progress(98, "正在執行 pm2 restart all 重啟 PM2 服務...")
                    pm2_ok, pm2_err = restart_pm2_services(self.log, root_dir)
                    elapsed = time.time() - start_time
                    self.set_progress(100, f"✅ 全部更新與 PM2 重啟作業完成！(耗時 {elapsed:.1f} 秒)")
                    if not pm2_ok:
                        warn_event = threading.Event()

                        def _warn_pm2():
                            try:
                                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                                messagebox.showwarning(
                                    "PM2 重啟提醒",
                                    f"執行 PM2 服務重啟時回傳非預期結果：\n{pm2_err}\n\n"
                                    "請稍後於終端機手動執行 pm2 restart all 或檢查 PM2 服務狀態。",
                                    parent=parent_w
                                )
                            finally:
                                warn_event.set()

                        self.root.after(0, _warn_pm2)
                        warn_event.wait()

                # 即時更新主視窗狀態標籤
                self.root.after(0, self.update_pm2_daemon_status)
            else:
                self.log("ℹ️ 未勾選「更新後重啟 PM2 服務」，略過 PM2 重啟作業。", "info")

            # 彈出完成與空間清理對話框
            def _post_update_dialog():
                parent_w = self.progress_dlg if (self.progress_dlg and self.progress_dlg.winfo_exists()) else self.root
                ask_cleanup = messagebox.askyesno(
                    "更新完成 - 清理空間確認",
                    f"🎉 所有選取的系統模組已順利更新完成！(耗時 {elapsed:.1f} 秒)\n\n"
                    "是否要一併刪除本次已更新成功的壓縮包 (.zip) 以釋放磁碟空間？\n"
                    "（點選「是」僅清除本次已完成更新的 zip 檔，說明文字檔仍會完整保留）",
                    parent=parent_w
                )
                if ask_cleanup:
                    del_count = 0
                    for pkg in selected_pkgs:
                        zpath = self.pkg_vars[pkg["id"]]["filepath"]
                        if os.path.isfile(zpath):
                            try:
                                os.remove(zpath)
                                self.log(f"🗑️ 已刪除壓縮包: {pkg['name']}", "info")
                                del_count += 1
                            except Exception as de:
                                self.log(f"⚠️ 刪除 {pkg['name']} 失敗: {de}", "warning")
                    if del_count > 0:
                        self.log(f"已清理 {del_count} 個更新壓縮包，釋放磁碟空間。", "success")
                        self.rescan()

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
            if should_restart_apache and svc_name and apache_was_stopped and not apache_restarted:
                self.log(f"⚠️ 偵測到更新中斷或異常，緊急嘗試恢復 Apache 服務 [{svc_name}]...", "warning")
                start_apache_service(svc_name, self.log)
                apache_restarted = True
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
