# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 自動移除工具 (uninstaller)
Rust (Tauri 2) 單一執行檔打包輔助腳本 (build.py)
"""
import os
import shutil
import subprocess
import sys

# 安全容錯：避免 Windows CMD 在 CP950 編碼下因特殊符號噴 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(errors="replace")


def main():
    print("=" * 60)
    print("正在編譯打包「智慧控管與監控系統 - 自動移除工具 (uninstaller)」...")
    print("=" * 60)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    src_tauri_dir = os.path.join(script_dir, "src-tauri")

    if not os.path.isdir(src_tauri_dir):
        print(f"[ERROR] 找不到 Rust 原始碼目錄：{src_tauri_dir}")
        sys.exit(1)

    # 檢查 cargo 是否可用
    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        print("[ERROR] 系統未安裝或未於 PATH 找到 cargo 工具，請先安裝 Rust！")
        sys.exit(1)

    print(f"[*] 工作目錄：{src_tauri_dir}")
    print("[*] 執行指令：cargo build --release")
    print("-" * 60)

    ret = subprocess.run([cargo_bin, "build", "--release"], cwd=src_tauri_dir)

    print("-" * 60)
    if ret.returncode == 0:
        # 依據 .cargo/config.toml 之 target-dir 定位產出二進位檔案
        target_candidates = [
            r"C:\Temp\cargo-target\lah-uninstaller\release\lah-uninstaller.exe",
            os.path.join(src_tauri_dir, "target", "release", "lah-uninstaller.exe"),
        ]
        built_exe = next((p for p in target_candidates if os.path.isfile(p)), None)

        if built_exe:
            root_dir = os.path.abspath(os.path.join(script_dir, "..", ".."))
            dist_dir = os.path.join(root_dir, "dist")
            os.makedirs(dist_dir, exist_ok=True)

            out_exe_name = "智慧控管與監控系統 - 自動移除工具.exe"
            out_exe_path = os.path.join(dist_dir, out_exe_name)

            shutil.copy2(built_exe, out_exe_path)

            # 確保 dist 目錄僅保留中文檔名之發布執行檔，若存在舊版英文檔名則主動清除
            legacy_std_path = os.path.join(dist_dir, "lah-uninstaller.exe")
            if os.path.isfile(legacy_std_path):
                try:
                    os.remove(legacy_std_path)
                except Exception:
                    pass

            size_mb = os.path.getsize(out_exe_path) / (1024 * 1024)
            print(f"[OK] 編譯成功！二進位檔案大小：{size_mb:.2f} MB")
            print(f"[OK] 產出發布路徑：\n  -> {out_exe_path}")
        else:
            print("[WARN] 編譯指令成功，但在預期目標路徑未找到 lah-uninstaller.exe！")
    else:
        print(f"[ERROR] 編譯失敗，回傳碼：{ret.returncode}")
    print("=" * 60)


if __name__ == "__main__":
    main()
