# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 管理儀表板 (dashboard)
Rust (Tauri 2) 單一執行檔打包發布腳本 (build.py)
"""
import os
import shutil
import subprocess
import sys

# 安全容錯：避免 Windows CMD 在 CP950 編碼下因特殊符號噴 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


def main():
    print("=" * 60)
    print("正在編譯打包「智慧控管與監控系統 - 管理儀表板」...")
    print("=" * 60)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    src_tauri_dir = os.path.join(script_dir, "src-tauri")

    if not os.path.isdir(src_tauri_dir):
        print(f"[ERROR] 找不到 Rust 原始碼目錄：{src_tauri_dir}")
        sys.exit(1)

    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        print("[ERROR] 系統未安裝或未於 PATH 找到 cargo 工具，請先安裝 Rust！")
        sys.exit(1)

    print(f"[*] 工作目錄：{src_tauri_dir}")
    print("[*] 執行指令：cargo build --release")
    print("-" * 60)

    build_rs = os.path.join(src_tauri_dir, "build.rs")
    if os.path.isfile(build_rs):
        os.utime(build_rs, None)

    ret = subprocess.run([cargo_bin, "build", "--release"], cwd=src_tauri_dir)

    print("-" * 60)
    if ret.returncode == 0:
        target_candidates = [
            r"C:\Temp\cargo-target\dashboard\release\lah-dashboard.exe",
            os.path.join(src_tauri_dir, "target", "release", "lah-dashboard.exe"),
        ]
        built_exe = next((p for p in target_candidates if os.path.isfile(p)), None)

        if built_exe:
            # 尋找 LAH-Updater 根目錄與 dist
            updater_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
            dist_dir = os.path.join(updater_root, "dist")
            os.makedirs(dist_dir, exist_ok=True)

            out_exe_name = "智慧控管與監控系統 - 管理儀表板.exe"
            out_exe_path = os.path.join(dist_dir, out_exe_name)

            try:
                shutil.copy2(built_exe, out_exe_path)
            except PermissionError:
                old_bak = out_exe_path + ".old"
                if os.path.isfile(old_bak):
                    try:
                        os.remove(old_bak)
                    except Exception:
                        pass
                try:
                    os.rename(out_exe_path, old_bak)
                    shutil.copy2(built_exe, out_exe_path)
                except Exception as e:
                    print(f"[WARN] 覆蓋執行檔時發生鎖定: {e}")

            size_mb = os.path.getsize(out_exe_path) / (1024 * 1024)
            print(f"[OK] 編譯成功！二進位檔案大小：{size_mb:.2f} MB")
            print(f"[OK] 產出發布路徑：\n  -> {out_exe_path}")

            # 若 C:\AppServ 存在，同步複製一份至安裝根目錄方便管理員隨點即用
            appserv_dir = r"C:\AppServ"
            if os.path.isdir(appserv_dir):
                target_appserv_exe = os.path.join(appserv_dir, out_exe_name)
                deployed = False
                try:
                    shutil.copy2(out_exe_path, target_appserv_exe)
                    deployed = True
                except Exception:
                    # Windows 進程鎖定防護：允許將運行中執行檔更名為 .old 後覆蓋新版
                    import time
                    bak_path = f"{target_appserv_exe}.old_{int(time.time())}"
                    try:
                        os.rename(target_appserv_exe, bak_path)
                        shutil.copy2(out_exe_path, target_appserv_exe)
                        deployed = True
                        print(f"[OK] 偵測到進程鎖定，已將舊版本更名備份為 {bak_path} 並成功替換新版！")
                    except Exception as e_rename:
                        print(f"[WARN] 同步部署至 {target_appserv_exe} 時發生鎖定且無法更名: {e_rename}")

                if deployed:
                    print(f"[OK] 已同步部署至安裝根目錄：\n  -> {target_appserv_exe}")
        else:
            print("[WARN] 編譯指令成功，但在預期目標路徑未找到 lah-dashboard.exe！")
    else:
        print(f"[ERROR] 編譯失敗，回傳碼：{ret.returncode}")
    print("=" * 60)


if __name__ == "__main__":
    main()
