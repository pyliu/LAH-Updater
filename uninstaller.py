# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 自動移除工具 (uninstaller.py)

功能職責：
1. 停止並解除安裝 Windows 原生系統服務 (LAH_PM2_Service 與 Apache 服務)。
2. 終止受控守護進程與殘留程序 (pm2 kill, 殘留 node.exe 與 php-cgi.exe)。
3. 清除開機自啟動設定 (啟動資料夾 VBS 腳本、Windows 工作排程器、使用者環境變數 PATH 登錄)。
4. 清除 PM2 快照狀態 (LocalSystem 與使用者目錄之 .pm2\dump.pm2)。
5. 刪除前自動將關鍵持久化資料 (即時通 db/、uploads/、各專案 .env、AD.php、logs/) 打包為備份 ZIP。
6. 安全刪除程式碼、Web 站點、執行環境 (node, php85, php7, instantclient_11_2) 與相關目錄。
7. 提供現代化 Tkinter 繁體中文 GUI 介面，支援細緻項目勾選、雙重確認防護與即時進度日誌。
"""
import ctypes
import os
import re
import shutil
import stat
import subprocess
import sys
import threading
import time
import winreg
import zipfile
from datetime import datetime

# 安全容錯：避免 Windows CMD 在 CP950 編碼下因 Emoji 符號噴 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(errors="replace")

import tkinter as tk
from tkinter import ttk, messagebox, filedialog, scrolledtext
import tkinter.font as tkfont


# ==============================================================================
# 系統輔助函式與安全性檢查
# ==============================================================================

def is_admin():
    """檢查目前行程是否具備 Windows 系統管理員權限"""
    try:
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except Exception:
        return False


def get_app_dir():
    """取得程式或 .exe 實際所在的目錄路徑（相容 PyInstaller 打包）"""
    if getattr(sys, 'frozen', False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(os.path.abspath(__file__))


def _decode_console_bytes(raw_bytes):
    """安全解碼 Windows 控制台輸出字節"""
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
    return re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", text)


def robust_rmtree(target_path, log_func=None):
    r"""
    強健目錄刪除常式：自動處理唯讀檔案屬性與短暫鎖定重試
    """
    if not os.path.exists(target_path):
        return True, "路徑不存在，無需刪除"

    def _remove_readonly(func, path, exc_info):
        try:
            os.chmod(path, stat.S_IWRITE)
            func(path)
        except Exception as e:
            if log_func:
                log_func(f"    ⚠️ 強制解除鎖定失敗 [{path}]: {e}", "warning")

    for attempt in range(3):
        try:
            if os.path.isdir(target_path):
                shutil.rmtree(target_path, onerror=_remove_readonly)
            else:
                os.chmod(target_path, stat.S_IWRITE)
                os.remove(target_path)
            return True, "已成功刪除"
        except Exception as e:
            time.sleep(1)
            if attempt == 2:
                msg = f"刪除失敗 [{target_path}]: {e}"
                if log_func:
                    log_func(f"  ❌ {msg}", "error")
                return False, msg


def remove_paths_from_user_environment(paths_to_remove, log_func=None):
    r"""
    自 Windows 目前使用者 (HKCU\Environment) 與系統 (HKLM\...\Environment) 的 PATH 環境變數中精確移除指定目錄。
    使用 winreg 讀寫註冊表，並透過 ctypes 廣播 WM_SETTINGCHANGE。
    """
    if not paths_to_remove:
        return True, "無指定路徑"

    norm_targets = [os.path.normpath(p).rstrip("\\/").lower() for p in paths_to_remove if p]
    removed_any = False

    # 1. 清理使用者 PATH (HKCU\Environment)
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment", 0, winreg.KEY_READ | winreg.KEY_WRITE) as key:
            try:
                cur_val, val_type = winreg.QueryValueEx(key, "Path")
            except FileNotFoundError:
                cur_val = ""
                val_type = winreg.REG_EXPAND_SZ

            parts = [p.strip() for p in cur_val.split(";") if p.strip()]
            new_parts = []
            for p in parts:
                p_norm = os.path.normpath(p).rstrip("\\/").lower()
                if p_norm in norm_targets:
                    removed_any = True
                    if log_func:
                        log_func(f"  ✓ 自使用者 PATH 環境變數剔除: [{p}]", "info")
                else:
                    new_parts.append(p)

            if removed_any:
                new_val = ";".join(new_parts)
                winreg.SetValueEx(key, "Path", 0, val_type, new_val)
    except Exception as e:
        if log_func:
            log_func(f"  ⚠️ 清理使用者 PATH 環境變數警示: {e}", "warning")

    # 2. 若具管理員權限，同時檢查並清理系統 PATH (HKLM\...\Environment)
    if is_admin():
        try:
            hklm_path = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"
            with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, hklm_path, 0, winreg.KEY_READ | winreg.KEY_WRITE) as key:
                try:
                    cur_val, val_type = winreg.QueryValueEx(key, "Path")
                except FileNotFoundError:
                    cur_val = ""
                    val_type = winreg.REG_EXPAND_SZ

                parts = [p.strip() for p in cur_val.split(";") if p.strip()]
                new_parts = []
                sys_removed = False
                for p in parts:
                    p_norm = os.path.normpath(p).rstrip("\\/").lower()
                    if p_norm in norm_targets:
                        sys_removed = True
                        removed_any = True
                        if log_func:
                            log_func(f"  ✓ 自系統 PATH 環境變數剔除: [{p}]", "info")
                    else:
                        new_parts.append(p)

                if sys_removed:
                    new_val = ";".join(new_parts)
                    winreg.SetValueEx(key, "Path", 0, val_type, new_val)
        except Exception as e:
            if log_func:
                log_func(f"  ⚠️ 清理系統 PATH 環境變數警示: {e}", "warning")

    if removed_any:
        HWND_BROADCAST = 0xFFFF
        WM_SETTINGCHANGE = 0x001A
        SMTO_ABORTIFHUNG = 0x0002
        result = ctypes.c_ulong()
        ctypes.windll.user32.SendMessageTimeoutW(
            HWND_BROADCAST, WM_SETTINGCHANGE, 0, "Environment",
            SMTO_ABORTIFHUNG, 5000, ctypes.byref(result)
        )
        if log_func:
            log_func("  ✅ 已廣播通知 Windows 系統環境變數更新完成！", "success")
    return True, "已清理 PATH 環境變數"


# ==============================================================================
# 服務與進程卸載常式
# ==============================================================================

def query_apache_service():
    """偵測系統中的 Apache 服務名稱"""
    candidates = ["Apache2.4", "Apache24", "Apache2.2", "Apache2", "Apache", "httpd"]
    for name in candidates:
        try:
            res = subprocess.run(
                ["sc.exe", "query", name],
                capture_output=True, text=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            if res.returncode == 0:
                return name
        except Exception:
            pass

    try:
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command",
             "Get-Service *apache* -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name -First 1"],
            capture_output=True, text=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        val = res.stdout.strip()
        if val:
            return val
    except Exception:
        pass
    return None


def uninstall_apache_service(log_func, root_dir):
    """停止並完全卸載 Apache 服務"""
    svc_name = query_apache_service() or "Apache2.4"
    log_func(f"⏳ 正在停止並解除安裝 Apache 服務 [{svc_name}]...", "info")

    # 1. 嘗試停止服務
    subprocess.run(["sc.exe", "stop", svc_name], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    time.sleep(1)

    # 2. 嘗試透過 httpd.exe -k uninstall 卸載
    httpd_exe = os.path.join(root_dir, "Apache24", "bin", "httpd.exe")
    uninstalled = False
    if os.path.isfile(httpd_exe):
        try:
            un_res = subprocess.run(
                [httpd_exe, "-k", "uninstall", "-n", svc_name],
                capture_output=True, text=True, timeout=15,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            if un_res.returncode == 0:
                uninstalled = True
                log_func(f"  ✓ 已透過 httpd.exe 解除安裝 Apache 服務 [{svc_name}]", "success")
        except Exception:
            pass

    # 3. 備援透過 sc.exe delete 徹底清除 SCM 註冊
    try:
        del_res = subprocess.run(
            ["sc.exe", "delete", svc_name],
            capture_output=True, text=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        if del_res.returncode == 0:
            uninstalled = True
            log_func(f"  ✓ 已自 Windows SCM 刪除 Apache 服務 [{svc_name}]", "success")
    except Exception:
        pass

    if uninstalled:
        return True, f"已成功解除安裝 Apache 服務 [{svc_name}]"
    else:
        return True, "未偵測到運作中之 Apache 服務或已卸載"


def clean_apache_php_configuration(log_func, root_dir, remove_php7=False, remove_php85=False):
    r"""
    自 Apache httpd.conf 中安全清理 PHP 7 與 PHP 8.5 相關模組載入、轉發與 Alias 規則，
    避免單獨移除 PHP 目錄後 Apache 因找不到模組 DLL 或 FastCGI 目錄而無法啟動。
    """
    conf_path = os.path.join(root_dir, "Apache24", "conf", "httpd.conf")
    if not os.path.isfile(conf_path):
        return True, "未偵測到 httpd.conf"

    try:
        with open(conf_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        orig_content = content

        # 1. 清理 PHP 8.5 FastCGI 與 Alias 轉發區塊
        if remove_php85:
            content = re.sub(
                r"# =====================================================================\s*# LAH-API \(PHP 8\.5 FastCGI.*?</Proxy>",
                "",
                content,
                flags=re.DOTALL
            )
            content = re.sub(
                r"# --- LAH-API PHP 8\.5 智慧無感分流規則 ---.*?</IfModule>",
                "",
                content,
                flags=re.DOTALL
            )
            log_func("  ✓ 已自 Apache httpd.conf 清理 LAH-API (PHP 8.5) FastCGI 轉發規則", "info")

        # 2. 清理 PHP 7 模組載入規則（註解 LoadModule 與 PHPIniDir）
        if remove_php7:
            content = re.sub(
                r"(?m)^(\s*LoadModule\s+php7_module\s+.*)$",
                r"# [自動移除工具已註銷] \1",
                content
            )
            content = re.sub(
                r"(?m)^(\s*PHPIniDir\s+.*)$",
                r"# [自動移除工具已註銷] \1",
                content
            )
            log_func("  ✓ 已自 Apache httpd.conf 註銷 PHP 7 模組 (LoadModule php7_module)", "info")

        if content != orig_content:
            bak_path = conf_path + f".bak_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            try:
                shutil.copyfile(conf_path, bak_path)
            except Exception:
                pass

            with open(conf_path, "w", encoding="utf-8") as f:
                f.write(content)
            log_func("  ✓ Apache httpd.conf 設定檔已安全更新與備份！", "success")

            # 檢驗 Apache 語法
            httpd_exe = os.path.join(root_dir, "Apache24", "bin", "httpd.exe")
            if os.path.isfile(httpd_exe):
                t_res = subprocess.run([httpd_exe, "-t"], capture_output=True, text=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
                if "Syntax OK" in (t_res.stderr + t_res.stdout):
                    log_func("  ✓ Apache 設定檔語法檢驗通過 (Syntax OK)！", "success")
                else:
                    err = (t_res.stderr or t_res.stdout).strip()
                    log_func(f"  ⚠️ Apache 語法檢驗回傳警示: {err}", "warning")

        return True, "已清理 Apache PHP 設定"
    except Exception as e:
        msg = f"清理 Apache PHP 設定發生異常: {e}"
        log_func(f"  ⚠️ {msg}", "warning")
        return False, msg


def uninstall_lah_pm2_service(log_func, root_dir):
    """停止並完全卸載 LAH_PM2_Service Windows 原生系統服務"""
    log_func("⏳ 正在停止並解除安裝 Windows 原生系統服務 (LAH_PM2_Service)...", "info")
    exe_path = os.path.join(root_dir, "node", "service", "lah-service.exe")

    # 1. 嘗試 WinSW 卸載
    if os.path.isfile(exe_path):
        try:
            subprocess.run([exe_path, "stop"], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        except Exception:
            pass
        try:
            un_res = subprocess.run([exe_path, "uninstall"], capture_output=True, text=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            if un_res.returncode == 0 or "successfully" in un_res.stdout.lower():
                log_func("  ✓ 已透過 WinSW 成功卸載系統服務: LAH_PM2_Service", "success")
        except Exception:
            pass

    # 2. 備援透過 sc.exe 刪除
    subprocess.run(["sc.exe", "stop", "LAH_PM2_Service"], capture_output=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    time.sleep(1)
    del_res = subprocess.run(["sc.exe", "delete", "LAH_PM2_Service"], capture_output=True, text=True, timeout=8, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    if del_res.returncode == 0:
        log_func("  ✓ 已自 Windows SCM 刪除系統服務: LAH_PM2_Service", "success")

    return True, "已解除安裝 LAH_PM2_Service 系統服務"


def kill_all_pm2_and_node(log_func, root_dir):
    """終止 PM2 Daemon 與由目標目錄衍生之 node.exe 與 php-cgi.exe 行程"""
    log_func("⏳ 正在終止 PM2 核心守護進程與殘留背景程序...", "info")

    # 1. 嘗試呼叫 pm2 kill
    pm2_cmd = os.path.join(root_dir, "node", "pm2.cmd")
    if os.path.isfile(pm2_cmd):
        try:
            subprocess.run([pm2_cmd, "kill"], capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            log_func("  ✓ 已執行 pm2 kill 停止所有 PM2 背景進程", "info")
        except Exception:
            pass

    # 2. 透過 PowerShell 精確終止 ExecutablePath 包含 root_dir 之殘留行程
    try:
        norm_root = os.path.normpath(root_dir).replace("'", "''")
        ps_cmd = (
            f"Get-CimInstance Win32_Process | "
            f"Where-Object {{ $_.ExecutablePath -and $_.ExecutablePath -like '{norm_root}*' }} | "
            f"ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; "
            f"Write-Output \"Terminated $($_.Name) ($($_.ProcessId))\" }}"
        )
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps_cmd],
            capture_output=True, text=True, timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
        )
        out = res.stdout.strip()
        if out:
            for line in out.splitlines():
                if line.strip():
                    log_func(f"  ✓ 強制終止殘留行程: {line.strip()}", "info")
    except Exception as e:
        log_func(f"  ⚠️ 行程清理警示: {e}", "warning")

    return True, "已清理相關背景行程"


def clean_autostart_and_scheduler(log_func, root_dir):
    """清除啟動資料夾捷徑、工作排程、註冊表 Run 與舊版 VBS 喚醒腳本"""
    log_func("⏳ 正在清除開機自啟動腳本、工作排程器與註冊表...", "info")

    # 1. 清除工作排程 (涵蓋 LAH_PM2_Autostart, PM2_Autostart, LAH_Autostart)
    for tn in ("LAH_PM2_Autostart", "PM2_Autostart", "LAH_Autostart"):
        try:
            res = subprocess.run(
                ["schtasks", "/Delete", "/TN", tn, "/F"],
                capture_output=True, text=True, timeout=8,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            if res.returncode == 0:
                log_func(f"  ✓ 已自 Windows 工作排程器刪除: {tn}", "success")
        except Exception:
            pass

    # 2. 清除啟動資料夾 (APPDATA 與 ProgramData / ALLUSERSPROFILE)
    startup_dirs = [
        os.path.join(os.environ.get("APPDATA", ""), r"Microsoft\Windows\Start Menu\Programs\Startup"),
        os.path.join(os.environ.get("ProgramData", r"C:\ProgramData"), r"Microsoft\Windows\Start Menu\Programs\Startup"),
        os.path.join(os.environ.get("ALLUSERSPROFILE", r"C:\ProgramData"), r"Microsoft\Windows\Start Menu\Programs\Startup"),
    ]
    seen_dirs = set()
    for sdir in startup_dirs:
        norm_sdir = os.path.normpath(sdir).lower()
        if norm_sdir in seen_dirs or not os.path.isdir(sdir):
            continue
        seen_dirs.add(norm_sdir)
        try:
            for fname in os.listdir(sdir):
                lower = fname.lower()
                if any(k in lower for k in ("pm2", "lah", "appserv")) and lower.endswith((".vbs", ".bat", ".cmd", ".lnk", ".ps1")):
                    target_file = os.path.join(sdir, fname)
                    try:
                        os.chmod(target_file, stat.S_IWRITE)
                        os.remove(target_file)
                        log_func(f"  ✓ 已清理啟動資料夾腳本/捷徑: {target_file}", "success")
                    except Exception as e:
                        log_func(f"  ⚠️ 清理 {target_file} 失敗: {e}", "warning")
        except Exception:
            pass

    # 3. 清理各處舊版 pm2-autostart.vbs 獨立檔案
    standalone_scripts = [
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\pm2-autostart.vbs"),
        r"C:\Windows\System32\config\systemprofile\.pm2\pm2-autostart.vbs",
        os.path.join(root_dir, "node", "pm2-autostart.vbs"),
    ]
    for sp in standalone_scripts:
        if os.path.isfile(sp):
            try:
                os.chmod(sp, stat.S_IWRITE)
                os.remove(sp)
                log_func(f"  ✓ 已清理獨立喚醒腳本: {sp}", "success")
            except Exception as e:
                log_func(f"  ⚠️ 清理 {sp} 失敗: {e}", "warning")

    # 4. 檢查並清理註冊表 Run 自啟動鍵值
    norm_root = os.path.normpath(root_dir).lower()
    reg_roots = [(winreg.HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run")]
    if is_admin():
        reg_roots.append((winreg.HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Run"))

    for hkey, subkey in reg_roots:
        try:
            with winreg.OpenKey(hkey, subkey, 0, winreg.KEY_READ | winreg.KEY_WRITE) as key:
                i = 0
                to_delete = []
                while True:
                    try:
                        vname, vval, _ = winreg.EnumValue(key, i)
                        vname_lower = vname.lower()
                        vval_str = str(vval).lower()
                        if any(k in vname_lower for k in ("pm2", "lah")) or (norm_root and norm_root in vval_str):
                            to_delete.append(vname)
                        i += 1
                    except OSError:
                        break
                for name in to_delete:
                    winreg.DeleteValue(key, name)
                    log_func(f"  ✓ 已清理註冊表 Run 自啟動項目: [{name}]", "success")
        except Exception:
            pass

    return True, "已清除開機自啟動設定"


def clean_pm2_dumps(log_func, root_dir=r"C:\AppServ"):
    """重置安裝根目錄、LocalSystem 與使用者家目錄中的 PM2 快照檔與執行緒狀態"""
    log_func("⏳ 正在清除 PM2 快照狀態 (dump.pm2)...", "info")
    candidates = [
        os.path.join(root_dir, r".pm2\dump.pm2"),
        os.path.join(root_dir, r".pm2\dump.pm2.bak"),
        os.path.join(root_dir, r".pm2\module_conf.json"),
        os.path.join(root_dir, r".pm2\pm2.pid"),
        os.path.join(root_dir, r".pm2\pm2.log"),
        r"C:\Windows\System32\config\systemprofile\.pm2\dump.pm2",
        r"C:\Windows\System32\config\systemprofile\.pm2\dump.pm2.bak",
        r"C:\Windows\System32\config\systemprofile\.pm2\module_conf.json",
        r"C:\Windows\System32\config\systemprofile\.pm2\pm2.pid",
        r"C:\Windows\System32\config\systemprofile\.pm2\pm2.log",
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\dump.pm2"),
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\dump.pm2.bak"),
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\module_conf.json"),
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\pm2.pid"),
        os.path.join(os.environ.get("USERPROFILE", ""), r".pm2\pm2.log"),
    ]
    for p in candidates:
        if os.path.isfile(p):
            try:
                os.chmod(p, stat.S_IWRITE)
                os.remove(p)
                log_func(f"  ✓ 已清除 PM2 狀態存檔: {p}", "info")
            except Exception as e:
                log_func(f"  ⚠️ 清除 {p} 失敗: {e}", "warning")
    return True, "已清理 PM2 快照"


# ==============================================================================
# 自動備份常式
# ==============================================================================

def backup_critical_data(log_func, root_dir, backup_dest, progress_cb=None):
    """
    將系統關鍵持久化資料 (db/, uploads/, www/assets/*, .pm2 設定與狀態檔, .env, AD.php, php.ini, httpd.conf, logs/) 打包備份至 ZIP。
    參數 backup_dest 可傳入「目標資料夾路徑」或「完整 .zip 檔案路徑」。
    """
    if str(backup_dest).lower().endswith(".zip"):
        zip_path = os.path.normpath(backup_dest)
        backup_dir = os.path.dirname(zip_path)
        if backup_dir:
            os.makedirs(backup_dir, exist_ok=True)
    else:
        backup_dir = os.path.normpath(backup_dest)
        os.makedirs(backup_dir, exist_ok=True)
        time_str = datetime.now().strftime("%Y%m%d_%H%M%S")
        zip_name = f"_LAH_Backup_{time_str}.zip"
        zip_path = os.path.join(backup_dir, zip_name)

    log_func("=" * 60, "info")
    log_func(f"📦 正在備份系統重要資料與設定至: {zip_path}...", "info")

    # 決定 .pm2 來源目錄（優先使用 <root_dir>\.pm2，若尚不存在則向下相容檢查 %USERPROFILE%\.pm2）
    pm2_src_dir = os.path.join(root_dir, ".pm2")
    if not os.path.isdir(pm2_src_dir):
        fallback_pm2 = os.path.join(os.environ.get("USERPROFILE", ""), ".pm2")
        if os.path.isdir(fallback_pm2):
            pm2_src_dir = fallback_pm2

    backup_items = [
        # (絕對路徑, 壓縮檔內部前綴)
        (os.path.join(root_dir, "lah-messenger-server", "db"), "lah-messenger-server/db"),
        (os.path.join(root_dir, "lah-messenger-server", "uploads"), "lah-messenger-server/uploads"),
        (os.path.join(root_dir, "lah-messenger-server", ".env"), "lah-messenger-server/.env"),
        (os.path.join(root_dir, "LAH-NUXTJS", ".env"), "LAH-NUXTJS/.env"),
        (os.path.join(root_dir, "LAH-API", ".env"), "LAH-API/.env"),
        (os.path.join(root_dir, "www", ".env"), "www/.env"),
        (os.path.join(root_dir, "www", "include", "config", "AD.php"), "www/include/config/AD.php"),
        (os.path.join(root_dir, "www", "assets"), "www/assets"),
        (os.path.join(pm2_src_dir, "dump.pm2"), ".pm2/dump.pm2"),
        (os.path.join(pm2_src_dir, "dump.pm2.bak"), ".pm2/dump.pm2.bak"),
        (os.path.join(pm2_src_dir, "module_conf.json"), ".pm2/module_conf.json"),
        (os.path.join(pm2_src_dir, "conf.js"), ".pm2/conf.js"),
        (os.path.join(pm2_src_dir, "modules"), ".pm2/modules"),
        (os.path.join(root_dir, "php85", "php.ini"), "php85/php.ini"),
        (os.path.join(root_dir, "php7", "php.ini"), "php7/php.ini"),
        (os.path.join(root_dir, "Apache24", "conf", "httpd.conf"), "Apache24/conf/httpd.conf"),
        (os.path.join(root_dir, "logs"), "logs"),
    ]

    total_backed = 0
    total_items = len(backup_items)
    try:
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for idx, (src_path, arc_prefix) in enumerate(backup_items, 1):
                if progress_cb:
                    pct = 10.0 + (idx / total_items) * 85.0
                    progress_cb(pct, f"正在打包備份: {arc_prefix}...")

                if not os.path.exists(src_path):
                    continue

                if os.path.isfile(src_path):
                    try:
                        zf.write(src_path, arc_prefix)
                        total_backed += 1
                        log_func(f"  ✓ 已備份設定檔: {arc_prefix}", "info")
                    except Exception as fe:
                        log_func(f"  ⚠️ 備份檔案 [{arc_prefix}] 略過: {fe}", "warning")
                elif os.path.isdir(src_path):
                    dir_file_count = 0
                    for root, _, files in os.walk(src_path):
                        for file in files:
                            full_path = os.path.join(root, file)
                            # 避免當備份檔剛好存放在被掃描目錄下時發生自我遞迴打包
                            if os.path.normpath(full_path).lower() == os.path.normpath(zip_path).lower():
                                continue
                            rel_path = os.path.relpath(full_path, src_path)
                            arcname = os.path.join(arc_prefix, rel_path).replace("\\", "/")
                            try:
                                zf.write(full_path, arcname)
                                total_backed += 1
                                dir_file_count += 1
                            except Exception:
                                pass
                    log_func(f"  ✓ 已備份目錄: {arc_prefix} (共 {dir_file_count} 個檔案)", "info")

        if total_backed > 0:
            size_mb = os.path.getsize(zip_path) / (1024 * 1024)
            log_func(f"🎉 備份完成！共備份 {total_backed} 個檔案 (壓縮檔大小: {size_mb:.2f} MB)", "success")
            return True, zip_path
        else:
            log_func("ℹ️ 未在目標根目錄偵測到需備份的現存資料，略過建立備份檔。", "info")
            if os.path.exists(zip_path) and os.path.getsize(zip_path) < 1024:
                try:
                    os.remove(zip_path)
                except Exception:
                    pass
            return True, None
    except Exception as e:
        msg = f"建立備份失敗: {e}"
        log_func(f"❌ {msg}", "error")
        return False, msg


# ==============================================================================
# 獨立彈出進度與日誌對話框 (ProgressDialog)
# ==============================================================================

class ProgressDialog(tk.Toplevel):
    def __init__(self, parent, root_dir, mode="uninstall"):
        super().__init__(parent)
        self.parent = parent
        self.root_dir = root_dir
        self.mode = mode
        self.is_running = True
        self.is_done = False

        if self.mode == "backup":
            self.title("智慧控管與監控系統 - 重要資料備份進度")
            init_status = "正在準備備份系統重要資料..."
        else:
            self.title("智慧控管與監控系統 - 移除作業執行進度")
            init_status = "正在準備移除作業..."

        self.geometry("980x620")
        self.minsize(820, 500)
        self.transient(parent)
        self.grab_set()

        self.progress_percent_var = tk.StringVar(value="0.0%")
        self.progress_status_var = tk.StringVar(value=init_status)

        self._center_window()
        self._build_ui()
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _center_window(self):
        self.update_idletasks()
        try:
            pw = self.parent.winfo_width()
            ph = self.parent.winfo_height()
            px = self.parent.winfo_rootx()
            py = self.parent.winfo_rooty()
            w = 980
            h = 620
            x = max(0, px + (pw - w) // 2)
            y = max(0, py + (ph - h) // 2)
            self.geometry(f"{w}x{h}+{x}+{y}")
        except Exception:
            pass

    def _build_ui(self):
        container = ttk.Frame(self, padding="14")
        container.pack(fill=tk.BOTH, expand=True)

        # 頂部標題與目標資訊
        hdr_frame = ttk.Frame(container)
        hdr_frame.pack(fill=tk.X, pady=(0, 10))

        hdr_text = "📦 系統重要資料與設定備份進行中..." if self.mode == "backup" else "🚨 系統移除清理作業進行中..."
        hdr_color = "#1565c0" if self.mode == "backup" else "#c62828"

        title_lbl = tk.Label(
            hdr_frame,
            text=hdr_text,
            font=("Microsoft JhengHei UI", 15, "bold"),
            fg=hdr_color
        )
        title_lbl.pack(side=tk.LEFT)

        target_lbl = tk.Label(
            hdr_frame,
            text=f"目標根目錄：{self.root_dir}",
            font=("Consolas", 11),
            fg="#555555"
        )
        target_lbl.pack(side=tk.RIGHT, padx=5)

        # 進度列資訊
        prog_info_frame = ttk.Frame(container)
        prog_info_frame.pack(fill=tk.X, pady=(4, 2))

        status_lbl = ttk.Label(prog_info_frame, textvariable=self.progress_status_var, font=("Microsoft JhengHei UI", 11, "bold"))
        status_lbl.pack(side=tk.LEFT)

        percent_lbl = ttk.Label(prog_info_frame, textvariable=self.progress_percent_var, font=("Consolas", 13, "bold"), foreground=hdr_color)
        percent_lbl.pack(side=tk.RIGHT)

        self.progress_bar = ttk.Progressbar(container, mode="determinate")
        self.progress_bar.pack(fill=tk.X, pady=(0, 8))

        # 即時日誌輸出視窗
        log_frame = ttk.LabelFrame(container, text=" 📜 執行日誌輸出 ", padding="6")
        log_frame.pack(fill=tk.BOTH, expand=True, pady=(0, 8))

        self.log_box = scrolledtext.ScrolledText(
            log_frame,
            wrap=tk.WORD,
            font=("Consolas", 10),
            bg="#1e1e1e",
            fg="#d4d4d4",
            insertbackground="#ffffff"
        )
        self.log_box.pack(fill=tk.BOTH, expand=True)

        self.log_box.tag_config("info", foreground="#9cdcfe")
        self.log_box.tag_config("success", foreground="#4ec9b0")
        self.log_box.tag_config("warning", foreground="#ce9178")
        self.log_box.tag_config("error", foreground="#f44747")
        self.log_box.tag_config("time", foreground="#6a9955")

        # 底部按鈕列
        bottom_frame = ttk.Frame(container)
        bottom_frame.pack(side=tk.BOTTOM, fill=tk.X)

        self.btn_close = ttk.Button(bottom_frame, text="關閉離開", command=self._on_close, state="disabled", width=14)
        self.btn_close.pack(side=tk.RIGHT)

    def log(self, message, level="info"):
        now_str = datetime.now().strftime("[%H:%M:%S] ")

        def _append():
            if hasattr(self, 'log_box') and self.log_box.winfo_exists():
                self.log_box.insert(tk.END, now_str, "time")
                self.log_box.insert(tk.END, message + "\n", level)
                self.log_box.see(tk.END)

        if threading.current_thread() is threading.main_thread():
            _append()
        else:
            self.after(0, _append)

    def set_progress(self, percent, text=None):
        def _update():
            p = max(0.0, min(100.0, float(percent)))
            self.progress_percent_var.set(f"{p:.1f}%")
            self.progress_bar["value"] = p
            if text:
                self.progress_status_var.set(text)

        if threading.current_thread() is threading.main_thread():
            _update()
        else:
            self.after(0, _update)

    def finish(self, success=True, elapsed=0, backup_path=None):
        def _done():
            self.is_running = False
            self.is_done = True
            if self.mode == "backup":
                if success and backup_path:
                    self.set_progress(100, f"✅ 重要資料備份已順利完成！(耗時 {elapsed:.1f} 秒)")
                    self.btn_close.configure(state="normal", text="完成並返回")
                    messagebox.showinfo(
                        "備份完成",
                        f"🎉 系統重要資料與設定已成功打包備份！\n\n"
                        f"📦 備份檔案位置：\n{backup_path}\n\n"
                        f"• 包含即時通 db/、uploads/、www/assets/*、.pm2 設定檔、各模組 .env、AD.php、httpd.conf、php.ini 與 logs/\n"
                        f"• 已自動為您取消勾選主畫面的「📦 執行前自動備份重要資料」選項，後續執行移除時不會重複備份。",
                        parent=self
                    )
                elif success and not backup_path:
                    self.set_progress(100, f"ℹ️ 未偵測到可備份資料 (耗時 {elapsed:.1f} 秒)")
                    self.btn_close.configure(state="normal", text="關閉返回")
                    messagebox.showinfo(
                        "備份提示",
                        "ℹ️ 未在指定之安裝根目錄偵測到需備份的現存資料或設定檔。",
                        parent=self
                    )
                else:
                    self.btn_close.configure(state="normal", text="關閉返回")
                    messagebox.showerror(
                        "備份失敗",
                        f"⚠️ 建立備份壓縮檔時發生錯誤：\n{backup_path or '請詳閱日誌視窗紀錄'}",
                        parent=self
                    )
                return

            if success:
                self.set_progress(100, f"✅ 全部選取項目已成功移除！(耗時 {elapsed:.1f} 秒)")
                self.btn_close.configure(state="normal", text="完成並關閉")
                messagebox.showinfo(
                    "移除完成",
                    f"🎉 系統移除作業已順利完成！\n\n"
                    f"• 所選之 Windows 系統服務與守護進程已停止並完全卸載\n"
                    f"• 開機自啟動腳本、工作排程器與環境變數 (PATH) 已全面清除\n"
                    f"• 安裝根目錄中之程式碼、執行環境與其餘殘留檔案已全數移除\n\n"
                    f"若有執行備份，刪除前之關鍵資料備份壓縮檔已安全保留。",
                    parent=self
                )
            else:
                self.btn_close.configure(state="normal", text="關閉離開")
                messagebox.showerror(
                    "移除過程發生警示",
                    f"⚠️ 系統移除清理作業執行期間發生錯誤，請詳閱日誌視窗紀錄之錯誤訊息！",
                    parent=self
                )

        if threading.current_thread() is threading.main_thread():
            _done()
        else:
            self.after(0, _done)

    def _on_close(self):
        if self.is_running:
            op_name = "備份作業" if self.mode == "backup" else "移除作業"
            messagebox.showwarning("警告", f"{op_name}正在背景進行中，請稍候完成再關閉視窗！", parent=self)
            return
        self.grab_release()
        self.destroy()
        if self.mode != "backup":
            self.parent.destroy()


# ==============================================================================
# GUI 應用程式主類別
# ==============================================================================

class UninstallerApp:
    def __init__(self, root):
        self.root = root
        self.root.title("智慧控管與監控系統 - 自動移除工具")
        self.root.geometry("1180x530")
        self.root.minsize(1050, 480)

        self.app_dir = get_app_dir()
        self.is_running = False
        self.last_manual_backup_path = None

        # 文字放大 25%：縮放 Tkinter 內建字型
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

    def _init_variables(self):
        self.root_dir_var = tk.StringVar(value=r"C:\AppServ")

        # 各項作業勾選變數（預設全選安全模式）
        self.do_backup_var = tk.BooleanVar(value=True)
        self.do_services_var = tk.BooleanVar(value=True)
        self.do_autostart_var = tk.BooleanVar(value=True)
        self.do_kill_procs_var = tk.BooleanVar(value=True)
        self.do_pm2_dumps_var = tk.BooleanVar(value=True)
        self.do_env_path_var = tk.BooleanVar(value=True)

        self.do_remove_fe_var = tk.BooleanVar(value=True)
        self.do_remove_wss_var = tk.BooleanVar(value=True)
        self.do_remove_api_var = tk.BooleanVar(value=True)
        self.do_remove_www_var = tk.BooleanVar(value=True)
        self.do_remove_apache_var = tk.BooleanVar(value=True)
        self.do_remove_node_var = tk.BooleanVar(value=True)
        self.do_remove_php85_var = tk.BooleanVar(value=True)
        self.do_remove_php7_var = tk.BooleanVar(value=True)
        self.do_remove_instantclient_var = tk.BooleanVar(value=True)
        self.do_remove_logs_var = tk.BooleanVar(value=True)
        self.do_clean_root_var = tk.BooleanVar(value=True)

    def _build_ui(self):
        main_frame = ttk.Frame(self.root, padding="14")
        main_frame.pack(fill=tk.BOTH, expand=True)

        # 1. 頂部標題列
        header_frame = ttk.Frame(main_frame)
        header_frame.pack(fill=tk.X, pady=(0, 10))

        title_lbl = tk.Label(
            header_frame,
            text="🗑️ 智慧控管與監控系統 - 自動移除工具",
            font=("Microsoft JhengHei UI", 16, "bold"),
            fg="#c62828"
        )
        title_lbl.pack(side=tk.LEFT)

        admin_badge = "👑 管理員權限" if is_admin() else "⚠️ 一般使用者權限"
        badge_fg = "#2e7d32" if is_admin() else "#d32f2f"
        admin_label = tk.Label(
            header_frame,
            text=admin_badge,
            fg=badge_fg,
            font=("Microsoft JhengHei UI", 11, "bold")
        )
        admin_label.pack(side=tk.RIGHT, padx=5)

        # 關鍵佈局：底部按鈕列優先向 side=tk.BOTTOM pack
        bottom_frame = ttk.Frame(main_frame)
        bottom_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(10, 0))

        self.btn_exit = ttk.Button(bottom_frame, text="關閉離開", command=self._on_close, width=12)
        self.btn_exit.pack(side=tk.RIGHT, padx=(8, 0))

        self.btn_start = tk.Button(
            bottom_frame,
            text="🚨 開始執行移除作業",
            command=self.start_uninstall_thread,
            font=("Microsoft JhengHei UI", 13, "bold"),
            bg="#d32f2f",
            fg="#ffffff",
            activebackground="#b71c1c",
            activeforeground="#ffffff",
            relief=tk.FLAT,
            padx=18,
            pady=5,
            cursor="hand2"
        )
        self.btn_start.pack(side=tk.RIGHT)

        self.btn_backup = tk.Button(
            bottom_frame,
            text="📦 立即備份重要資料",
            command=self.start_manual_backup,
            font=("Microsoft JhengHei UI", 13, "bold"),
            bg="#1976d2",
            fg="#ffffff",
            activebackground="#1565c0",
            activeforeground="#ffffff",
            relief=tk.FLAT,
            padx=18,
            pady=5,
            cursor="hand2"
        )
        self.btn_backup.pack(side=tk.RIGHT, padx=(0, 10))

        # 2. 根目錄選擇區
        dir_frame = ttk.LabelFrame(main_frame, text=" 📁 系統安裝根目錄 ", padding="8")
        dir_frame.pack(fill=tk.X, pady=(0, 8))

        lbl_dir = ttk.Label(dir_frame, text="安裝根目錄：")
        lbl_dir.pack(side=tk.LEFT, padx=(4, 6))

        entry_dir = ttk.Entry(dir_frame, textvariable=self.root_dir_var, font=("Consolas", 11))
        entry_dir.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 6))

        btn_browse = ttk.Button(dir_frame, text="瀏覽...", command=self._browse_root_dir, width=8)
        btn_browse.pack(side=tk.RIGHT)

        # 3. 移除項目核取區
        items_frame = ttk.LabelFrame(main_frame, text=" ⚙️ 移除作業與資料清理選項 ", padding="8")
        items_frame.pack(fill=tk.BOTH, expand=True, pady=(0, 6))

        # 快速選取按鈕
        btn_sel_row = ttk.Frame(items_frame)
        btn_sel_row.pack(fill=tk.X, pady=(0, 6))

        btn_all = ttk.Button(btn_sel_row, text="全選", command=lambda: self._set_all_checks(True), width=8)
        btn_all.pack(side=tk.LEFT, padx=(0, 6))

        btn_none = ttk.Button(btn_sel_row, text="全部取消", command=lambda: self._set_all_checks(False), width=10)
        btn_none.pack(side=tk.LEFT)

        # 項目網格（配置兩等寬欄位確保長文字完整顯示）
        grid_frame = ttk.Frame(items_frame)
        grid_frame.pack(fill=tk.BOTH, expand=True, padx=4, pady=2)
        grid_frame.columnconfigure(0, weight=1)
        grid_frame.columnconfigure(1, weight=1)

        # 第一行：資料備份與核心服務
        chk_backup = ttk.Checkbutton(grid_frame, text="📦 執行前自動備份重要資料 (db/, uploads/, www/assets/, .pm2, .env, AD.php)", variable=self.do_backup_var)
        chk_backup.grid(row=0, column=0, sticky="w", padx=8, pady=3)

        chk_svc = ttk.Checkbutton(grid_frame, text="🛑 停止並解除安裝 Windows 服務 (LAH_PM2_Service, Apache2.4)", variable=self.do_services_var)
        chk_svc.grid(row=0, column=1, sticky="w", padx=8, pady=3)

        # 第二行：守護行程與自啟動
        chk_proc = ttk.Checkbutton(grid_frame, text="⚡ 終止守護行程與背景程序 (pm2 kill, 殘留 node.exe / php-cgi)", variable=self.do_kill_procs_var)
        chk_proc.grid(row=1, column=0, sticky="w", padx=8, pady=3)

        chk_auto = ttk.Checkbutton(grid_frame, text="🧹 清除開機自啟動與環境變數 (啟動資料夾 VBS, 排程器, PATH)", variable=self.do_autostart_var)
        chk_auto.grid(row=1, column=1, sticky="w", padx=8, pady=3)

        # 第三行：程式碼目錄
        chk_fe = ttk.Checkbutton(grid_frame, text="🌐 移除智慧前端網站 (LAH-NUXTJS 及 node_modules)", variable=self.do_remove_fe_var)
        chk_fe.grid(row=2, column=0, sticky="w", padx=8, pady=3)

        chk_wss = ttk.Checkbutton(grid_frame, text="💬 移除即時通訊伺服器 (lah-messenger-server 及 node_modules)", variable=self.do_remove_wss_var)
        chk_wss.grid(row=2, column=1, sticky="w", padx=8, pady=3)

        # 第四行：後端與生產庫
        chk_api = ttk.Checkbutton(grid_frame, text="🔌 移除現代化後端 API 模組 (LAH-API)", variable=self.do_remove_api_var)
        chk_api.grid(row=3, column=0, sticky="w", padx=8, pady=3)

        chk_www = ttk.Checkbutton(grid_frame, text="🏢 移除舊版後端生產庫與入口網頁 (www)", variable=self.do_remove_www_var)
        chk_www.grid(row=3, column=1, sticky="w", padx=8, pady=3)

        # 第五行：伺服器與 Node.js
        chk_apache = ttk.Checkbutton(grid_frame, text="🌐 移除 Apache 網頁伺服器本體 (Apache24)", variable=self.do_remove_apache_var)
        chk_apache.grid(row=4, column=0, sticky="w", padx=8, pady=3)

        chk_node = ttk.Checkbutton(grid_frame, text="🟩 移除 Node.js 執行期環境 (node)", variable=self.do_remove_node_var)
        chk_node.grid(row=4, column=1, sticky="w", padx=8, pady=3)

        # 第六行：PHP 8.5 與 PHP 7 手動安裝目錄
        chk_php85 = ttk.Checkbutton(grid_frame, text="🐘 移除 PHP 8.5 手動安裝目錄 (php85)", variable=self.do_remove_php85_var)
        chk_php85.grid(row=5, column=0, sticky="w", padx=8, pady=3)

        chk_php7 = ttk.Checkbutton(grid_frame, text="🐘 移除 PHP 7 手動安裝目錄 (php7)", variable=self.do_remove_php7_var)
        chk_php7.grid(row=5, column=1, sticky="w", padx=8, pady=3)

        # 第七行：Oracle Instant Client 與系統日誌
        chk_instantclient = ttk.Checkbutton(grid_frame, text="🏛️ 移除 Oracle Instant Client (instantclient_11_2)", variable=self.do_remove_instantclient_var)
        chk_instantclient.grid(row=6, column=0, sticky="w", padx=8, pady=3)

        chk_logs = ttk.Checkbutton(grid_frame, text="📜 移除系統日誌目錄 (logs)", variable=self.do_remove_logs_var)
        chk_logs.grid(row=6, column=1, sticky="w", padx=8, pady=3)

        # 第八行：PM2 狀態快照與根目錄其餘殘留檔案
        chk_dumps = ttk.Checkbutton(grid_frame, text="🔄 重置 PM2 狀態快照 (.pm2\\dump.pm2)", variable=self.do_pm2_dumps_var)
        chk_dumps.grid(row=7, column=0, sticky="w", padx=8, pady=3)

        chk_clean_root = ttk.Checkbutton(grid_frame, text="🧹 清理安裝根目錄其餘殘留檔案 (僅保留備份檔)", variable=self.do_clean_root_var)
        chk_clean_root.grid(row=7, column=1, sticky="w", padx=8, pady=3)

    def _set_all_checks(self, state):
        self.do_backup_var.set(state)
        self.do_services_var.set(state)
        self.do_autostart_var.set(state)
        self.do_kill_procs_var.set(state)
        self.do_pm2_dumps_var.set(state)
        self.do_env_path_var.set(state)
        self.do_remove_fe_var.set(state)
        self.do_remove_wss_var.set(state)
        self.do_remove_api_var.set(state)
        self.do_remove_www_var.set(state)
        self.do_remove_apache_var.set(state)
        self.do_remove_node_var.set(state)
        self.do_remove_php85_var.set(state)
        self.do_remove_php7_var.set(state)
        self.do_remove_instantclient_var.set(state)
        self.do_remove_logs_var.set(state)
        self.do_clean_root_var.set(state)

    def _browse_root_dir(self):
        d = filedialog.askdirectory(title="選取系統安裝根目錄", initialdir=self.root_dir_var.get())
        if d:
            self.root_dir_var.set(os.path.normpath(d))

    def _on_close(self):
        if self.is_running:
            messagebox.showwarning("警告", "移除作業正在背景進行中，請稍候完成再關閉視窗！", parent=self.root)
            return
        self.root.destroy()

    def start_manual_backup(self):
        """獨立執行系統重要資料備份，讓使用者自由選擇儲存位置與檔名，完成後自動取消勾選執行前備份"""
        if self.is_running:
            return

        root_dir = os.path.normpath(self.root_dir_var.get().strip())
        if not os.path.isdir(root_dir):
            messagebox.showerror("錯誤", f"指定之安裝根目錄不存在：\n{root_dir}\n請確認目錄路徑正確！", parent=self.root)
            return

        time_str = datetime.now().strftime("%Y%m%d_%H%M%S")
        default_filename = f"_LAH_Backup_{time_str}.zip"
        init_dir = r"C:\Temp" if os.path.isdir(r"C:\Temp") else root_dir

        save_path = filedialog.asksaveasfilename(
            parent=self.root,
            title="請選擇重要資料備份壓縮檔儲存位置",
            initialdir=init_dir,
            initialfile=default_filename,
            defaultextension=".zip",
            filetypes=[("ZIP 備份壓縮檔", "*.zip"), ("所有檔案", "*.*")]
        )
        if not save_path:
            return

        save_path = os.path.normpath(save_path)
        self.is_running = True
        self.btn_backup.configure(state="disabled", text="備份進行中...")
        self.btn_start.configure(state="disabled")
        self.btn_exit.configure(state="disabled")

        prog_dlg = ProgressDialog(self.root, root_dir, mode="backup")

        def _backup_worker():
            start_time = time.time()
            try:
                prog_dlg.set_progress(5, "正在掃描安裝根目錄並準備打包重要資料...")
                b_ok, b_res = backup_critical_data(prog_dlg.log, root_dir, save_path, progress_cb=prog_dlg.set_progress)
                elapsed = time.time() - start_time
                if b_ok and b_res:
                    self.last_manual_backup_path = b_res
                    # 使用者已手動完成備份，自動取消勾選「執行前自動備份重要資料」
                    self.root.after(0, lambda: self.do_backup_var.set(False))
                    prog_dlg.finish(success=True, elapsed=elapsed, backup_path=b_res)
                elif b_ok and not b_res:
                    prog_dlg.finish(success=True, elapsed=elapsed, backup_path=None)
                else:
                    prog_dlg.finish(success=False, elapsed=elapsed, backup_path=b_res)
            except Exception as ex:
                elapsed = time.time() - start_time
                prog_dlg.log(f"❌ 執行獨立備份時發生未預期例外錯誤: {ex}", "error")
                prog_dlg.finish(success=False, elapsed=elapsed, backup_path=str(ex))
            finally:
                def _reset_ui():
                    self.is_running = False
                    self.btn_backup.configure(state="normal", text="📦 立即備份重要資料")
                    self.btn_start.configure(state="normal", text="🚨 開始執行移除作業")
                    self.btn_exit.configure(state="normal")
                self.root.after(0, _reset_ui)

        threading.Thread(target=_backup_worker, daemon=True).start()

    def start_uninstall_thread(self):
        if self.is_running:
            return

        root_dir = os.path.normpath(self.root_dir_var.get().strip())
        if not os.path.isdir(root_dir):
            messagebox.showerror("錯誤", f"指定之安裝根目錄不存在：\n{root_dir}\n請確認目錄路徑正確！", parent=self.root)
            return

        # 檢查是否有勾選任何項目
        has_any = any([
            self.do_services_var.get(),
            self.do_autostart_var.get(),
            self.do_kill_procs_var.get(),
            self.do_pm2_dumps_var.get(),
            self.do_remove_fe_var.get(),
            self.do_remove_wss_var.get(),
            self.do_remove_api_var.get(),
            self.do_remove_www_var.get(),
            self.do_remove_apache_var.get(),
            self.do_remove_node_var.get(),
            self.do_remove_php85_var.get(),
            self.do_remove_php7_var.get(),
            self.do_remove_instantclient_var.get(),
            self.do_remove_logs_var.get(),
            self.do_clean_root_var.get(),
        ])
        if not has_any:
            messagebox.showwarning("提示", "您尚未選取任何欲移除的項目！", parent=self.root)
            return

        # 整理即將執行的項目清單（單次確認對話框）
        items_summary = []
        if self.do_backup_var.get():
            items_summary.append("• 📦 執行前自動備份重要資料 (db/, uploads/, www/assets/, .pm2, .env, AD.php, php.ini, httpd.conf)")
        if self.do_services_var.get():
            items_summary.append("• 🛑 停止並解除安裝 Windows 系統服務 (LAH_PM2_Service, Apache2.4)")
        if self.do_kill_procs_var.get():
            items_summary.append("• ⚡ 終止守護行程與背景程序 (pm2 kill, 殘留 node.exe / php-cgi)")
        if self.do_autostart_var.get():
            items_summary.append("• 🧹 清除開機自啟動與環境變數 (工作排程器, 啟動資料夾, PATH)")
        if self.do_pm2_dumps_var.get():
            items_summary.append("• 🔄 重置 PM2 狀態快照 (.pm2\\dump.pm2)")
        if self.do_remove_fe_var.get():
            items_summary.append("• 🌐 移除智慧前端網站 (LAH-NUXTJS)")
        if self.do_remove_wss_var.get():
            items_summary.append("• 💬 移除即時通訊伺服器 (lah-messenger-server)")
        if self.do_remove_api_var.get():
            items_summary.append("• 🔌 移除現代化後端 API 模組 (LAH-API)")
        if self.do_remove_www_var.get():
            items_summary.append("• 🏢 移除舊版後端生產庫 (www)")
        if self.do_remove_apache_var.get():
            items_summary.append("• 🌐 移除 Apache 網頁伺服器本體 (Apache24)")
        if self.do_remove_node_var.get():
            items_summary.append("• 🟩 移除 Node.js 執行期環境 (node)")
        if self.do_remove_php85_var.get():
            items_summary.append("• 🐘 移除 PHP 8.5 手動安裝目錄 (php85)")
        if self.do_remove_php7_var.get():
            items_summary.append("• 🐘 移除 PHP 7 手動安裝目錄 (php7)")
        if self.do_remove_instantclient_var.get():
            items_summary.append("• 🏛️ 移除 Oracle Instant Client (instantclient_11_2)")
        if self.do_remove_logs_var.get():
            items_summary.append("• 📜 移除系統日誌目錄 (logs)")
        if self.do_clean_root_var.get():
            items_summary.append("• 🧹 深度清理安裝根目錄其餘殘留檔案 (僅保留備份檔)")

        items_text = "\n".join(items_summary)
        if self.do_backup_var.get():
            backup_hint = f"已勾選自動備份（備份檔將打包存放於 {root_dir}）"
        elif self.last_manual_backup_path and os.path.isfile(self.last_manual_backup_path):
            backup_hint = f"✅ 已手動完成備份（存放於 {self.last_manual_backup_path}），本次不重複備份"
        else:
            backup_hint = "⚠️ 【注意】未勾選備份，資料將被永久移除！"

        confirm = messagebox.askyesno(
            "⚠️ 系統移除作業確認",
            f"即將自伺服器移除選取之智慧控管與監控系統項目！\n\n"
            f"【目標安裝根目錄】：\n{root_dir}\n\n"
            f"【備份狀態】：\n{backup_hint}\n\n"
            f"【預計執行項目清單】：\n{items_text}\n\n"
            f"⚠️ 此操作將會停止系統服務、終止進程並刪除相關檔案。\n您確定要立即開始執行移除作業嗎？",
            icon="warning",
            parent=self.root
        )
        if not confirm:
            return

        self.is_running = True
        self.btn_backup.configure(state="disabled")
        self.btn_start.configure(state="disabled", text="移除作業進行中...")
        self.btn_exit.configure(state="disabled")

        # 彈出專屬執行進度與日誌視窗
        prog_dlg = ProgressDialog(self.root, root_dir, mode="uninstall")

        threading.Thread(target=self._uninstall_worker, args=(root_dir, prog_dlg), daemon=True).start()

    def _uninstall_worker(self, root_dir, dlg):
        start_time = time.time()
        try:
            dlg.log("=" * 60, "info")
            dlg.log(f"🚨 開始執行系統自動移除清理作業... (目標: {root_dir})", "info")
            dlg.set_progress(5, "正在準備移除作業...")

            # 階段 1：自動備份
            if self.do_backup_var.get():
                dlg.set_progress(10, "正在備份關鍵持久化資料...")
                backup_target_dir = root_dir
                b_ok, b_path = backup_critical_data(dlg.log, root_dir, backup_target_dir)
                if not b_ok:
                    dlg.log("⚠️ 自動備份過程發生警示，但依指示繼續執行後續移除程序...", "warning")

            # 階段 2：停止並解除安裝 Windows 服務
            if self.do_services_var.get():
                dlg.set_progress(25, "正在停止並解除安裝 Windows 服務...")
                uninstall_lah_pm2_service(dlg.log, root_dir)
                if self.do_remove_apache_var.get():
                    uninstall_apache_service(dlg.log, root_dir)

            # 若未移除 Apache，但移除了 PHP 7 或 PHP 8.5，自動清理 httpd.conf 中的轉發或載入設定
            if not self.do_remove_apache_var.get() and (self.do_remove_php7_var.get() or self.do_remove_php85_var.get()):
                clean_apache_php_configuration(
                    dlg.log,
                    root_dir,
                    remove_php7=self.do_remove_php7_var.get(),
                    remove_php85=self.do_remove_php85_var.get()
                )

            # 階段 3：終止 PM2 與背景殘留進程
            if self.do_kill_procs_var.get():
                dlg.set_progress(40, "正在終止受控守護行程與殘留進程...")
                kill_all_pm2_and_node(dlg.log, root_dir)

            # 階段 4：清除自啟動與環境變數
            if self.do_autostart_var.get():
                dlg.set_progress(55, "正在清除開機自啟動與工作排程...")
                clean_autostart_and_scheduler(dlg.log, root_dir)

                paths_to_remove = []
                if self.do_remove_node_var.get():
                    paths_to_remove.append(os.path.join(root_dir, "node"))
                if self.do_remove_instantclient_var.get():
                    paths_to_remove.append(os.path.join(root_dir, "instantclient_11_2"))
                if self.do_remove_php85_var.get():
                    paths_to_remove.append(os.path.join(root_dir, "php85"))
                if self.do_remove_php7_var.get():
                    paths_to_remove.append(os.path.join(root_dir, "php7"))
                if self.do_remove_apache_var.get():
                    paths_to_remove.append(os.path.join(root_dir, "Apache24", "bin"))

                if paths_to_remove:
                    remove_paths_from_user_environment(paths_to_remove, dlg.log)

            # 階段 5：重置 PM2 快照
            if self.do_pm2_dumps_var.get():
                dlg.set_progress(65, "正在清除 PM2 快照狀態檔...")
                clean_pm2_dumps(dlg.log, root_dir)

            # 階段 6：刪除檔案與目錄
            dlg.set_progress(75, "正在清理檔案與程式碼目錄...")

            dir_tasks = []
            if self.do_remove_fe_var.get():
                dir_tasks.append((os.path.join(root_dir, "LAH-NUXTJS"), "前端網站 (LAH-NUXTJS)"))
            if self.do_remove_wss_var.get():
                dir_tasks.append((os.path.join(root_dir, "lah-messenger-server"), "即時通訊伺服器 (lah-messenger-server)"))
            if self.do_remove_api_var.get():
                dir_tasks.append((os.path.join(root_dir, "LAH-API"), "現代化後端 API (LAH-API)"))
            if self.do_remove_www_var.get():
                dir_tasks.append((os.path.join(root_dir, "www"), "舊版後端生產庫 (www)"))
            if self.do_remove_apache_var.get():
                dir_tasks.append((os.path.join(root_dir, "Apache24"), "Apache 網頁伺服器 (Apache24)"))
            if self.do_remove_node_var.get():
                dir_tasks.append((os.path.join(root_dir, "node"), "Node.js 綠色環境 (node)"))
            if self.do_remove_php85_var.get():
                dir_tasks.append((os.path.join(root_dir, "php85"), "PHP 8.5 手動安裝目錄 (php85)"))
            if self.do_remove_php7_var.get():
                dir_tasks.append((os.path.join(root_dir, "php7"), "PHP 7 手動安裝目錄 (php7)"))
            if self.do_remove_instantclient_var.get():
                dir_tasks.append((os.path.join(root_dir, "instantclient_11_2"), "Oracle Instant Client (instantclient_11_2)"))
            if self.do_remove_logs_var.get():
                dir_tasks.append((os.path.join(root_dir, "logs"), "系統日誌目錄 (logs)"))

            total_dirs = len(dir_tasks)
            for idx, (dpath, dname) in enumerate(dir_tasks, 1):
                if os.path.exists(dpath):
                    dlg.log(f"  🗑️ 正在刪除 {dname} [{dpath}]...", "info")
                    ok, msg = robust_rmtree(dpath, dlg.log)
                    if ok:
                        dlg.log(f"  ✓ 已成功刪除 {dname}", "success")
                curr_pct = 75.0 + (idx / total_dirs) * 15.0 if total_dirs else 90.0
                dlg.set_progress(curr_pct, f"正在刪除: {dname}")

            # 階段 6.5：清理安裝根目錄中的其餘殘留檔案與目錄（僅保留備份壓縮檔）
            if self.do_clean_root_var.get() and os.path.isdir(root_dir):
                dlg.set_progress(92, "正在清理安裝根目錄其餘殘留檔案...")
                dlg.log("⏳ 正在清理安裝根目錄其餘殘留檔案 (僅保留刪除前備份壓縮檔)...", "info")

                # 建立使用者明確未勾選、需保留之子目錄集合
                keep_paths = set()
                if not self.do_remove_fe_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "LAH-NUXTJS")).lower())
                if not self.do_remove_wss_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "lah-messenger-server")).lower())
                if not self.do_remove_api_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "LAH-API")).lower())
                if not self.do_remove_www_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "www")).lower())
                if not self.do_remove_apache_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "Apache24")).lower())
                if not self.do_remove_node_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "node")).lower())
                if not self.do_remove_php85_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "php85")).lower())
                if not self.do_remove_php7_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "php7")).lower())
                if not self.do_remove_instantclient_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "instantclient_11_2")).lower())
                if not self.do_remove_logs_var.get():
                    keep_paths.add(os.path.normpath(os.path.join(root_dir, "logs")).lower())

                drive, tail = os.path.splitdrive(os.path.normpath(root_dir))
                if tail in ("\\", "/", ""):
                    dlg.log("⚠️ 偵測到安裝根目錄為磁碟機根路徑，為保護作業系統，略過整層目錄清理！", "warning")
                else:
                    try:
                        for item in os.listdir(root_dir):
                            item_path = os.path.join(root_dir, item)
                            norm_item = os.path.normpath(item_path).lower()

                            # 1. 嚴格保護備份壓縮檔（含預設命名與使用者手動另存於根目錄之備份檔）
                            is_manual_bkp = bool(
                                self.last_manual_backup_path and
                                norm_item == os.path.normpath(self.last_manual_backup_path).lower()
                            )
                            if (item.startswith("_LAH_Backup_") and item.endswith(".zip")) or is_manual_bkp:
                                dlg.log(f"  📦 保留刪除前關鍵備份壓縮檔: [{item}]", "info")
                                continue

                            # 2. 保留使用者未選取移除的目錄
                            if norm_item in keep_paths:
                                dlg.log(f"  ℹ️ 保留未勾選移除項目: [{item}]", "info")
                                continue

                            # 3. 清理殘留檔案（如 *.zip, *.bat, *.cmd, *.txt, *.tmp, .pm2 等）
                            if os.path.isfile(item_path) or os.path.islink(item_path):
                                try:
                                    os.chmod(item_path, stat.S_IWRITE)
                                    os.remove(item_path)
                                    dlg.log(f"  🗑️ 已清理根目錄殘留檔案: [{item}]", "info")
                                except Exception as e:
                                    dlg.log(f"  ⚠️ 清理檔案 [{item}] 失敗: {e}", "warning")
                            elif os.path.isdir(item_path):
                                ok_rm, msg_rm = robust_rmtree(item_path, dlg.log)
                                if ok_rm:
                                    dlg.log(f"  🗑️ 已清理根目錄殘留資料夾: [{item}]", "info")
                                else:
                                    dlg.log(f"  ⚠️ 清理資料夾 [{item}] 失敗: {msg_rm}", "warning")
                    except Exception as e:
                        dlg.log(f"  ⚠️ 掃描根目錄殘留項目時發生警示: {e}", "warning")

            # 若目標根目錄於清空後無其他檔案，安全移除該根目錄；若僅剩備份檔則保留備份檔
            if os.path.isdir(root_dir):
                try:
                    remaining = os.listdir(root_dir)
                    if not remaining:
                        os.rmdir(root_dir)
                        dlg.log(f"  ✓ 安裝根目錄已完全清空，已安全移除資料夾 [{root_dir}]", "success")
                    elif all(f.startswith("_LAH_Backup_") and f.endswith(".zip") for f in remaining):
                        dlg.log(f"  🎉 安裝根目錄其餘項目已全數清除，僅保留必要備份檔: [{', '.join(remaining)}] 於 [{root_dir}]", "success")
                    else:
                        dlg.log(f"  ℹ️ 安裝根目錄已完成清理，保留之項目 ({len(remaining)} 個): {', '.join(remaining)}", "info")
                except Exception:
                    pass

            elapsed = time.time() - start_time
            dlg.log("=" * 60, "success")
            dlg.log(f"🎉 智慧控管與監控系統 - 自動移除作業順利完成！(耗時 {elapsed:.1f} 秒)", "success")
            dlg.finish(success=True, elapsed=elapsed)
        except Exception as ex:
            elapsed = time.time() - start_time
            dlg.log(f"❌ 執行移除清理過程發生未預期例外錯誤: {ex}", "error")
            dlg.finish(success=False, elapsed=elapsed)
        finally:
            def _reset_main_ui():
                self.is_running = False
                self.btn_backup.configure(state="normal", text="📦 立即備份重要資料")
                self.btn_start.configure(state="normal", text="🚨 開始執行移除作業")
                self.btn_exit.configure(state="normal")
            self.root.after(0, _reset_main_ui)


# ==============================================================================
# 程式啟動點
# ==============================================================================

def main():
    # 支援 High DPI 清晰度
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(1)
    except Exception:
        try:
            ctypes.windll.user32.SetProcessDPIAware()
        except Exception:
            pass

    root = tk.Tk()
    app = UninstallerApp(root)
    root.protocol("WM_DELETE_WINDOW", app._on_close)
    root.mainloop()


if __name__ == "__main__":
    main()
