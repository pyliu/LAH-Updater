# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 自動移除工具
PyInstaller 單一執行檔打包輔助腳本 (build_uninstaller.py)
"""
import os
import sys
import subprocess

# 安全容錯：避免 Windows CMD 在 CP950 編碼下因特殊符號噴 UnicodeEncodeError
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(errors="replace")


def main():
    print("=" * 60)
    print("正在編譯打包「智慧控管與監控系統 - 自動移除工具」...")
    print("=" * 60)

    # 確保工作目錄為腳本所在目錄
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    root_dir = os.path.dirname(script_dir)
    dist_dir = os.path.join(root_dir, "dist")
    os.makedirs(dist_dir, exist_ok=True)

    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--noconfirm",
        "--onefile",
        "--windowed",
        "--uac-admin",
        "--distpath", dist_dir,
        "--name", "智慧控管與監控系統 - 自動移除工具",
        "uninstaller.py"
    ]

    ret = subprocess.run(cmd)

    print("-" * 60)
    if ret.returncode == 0:
        exe_path = os.path.join(dist_dir, "智慧控管與監控系統 - 自動移除工具.exe")
        print(f"[OK] 編譯成功！產出路徑：\n{exe_path}")
    else:
        print(f"[ERROR] 編譯失敗，回傳碼：{ret.returncode}")
    print("=" * 60)


if __name__ == "__main__":
    main()
