# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 地籍異動即時通簡訊控管 (moi-sms)
Rust (Tauri 2) 單一執行檔打包發布腳本 (build.py)
"""
import os
import shutil
import subprocess
import sys

if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


def main() -> None:
    print("=" * 60)
    print("正在編譯打包「智慧控管與監控系統 - 地籍異動即時通簡訊控管」...")
    print("=" * 60)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        candidate = os.path.join(os.path.expanduser("~"), ".cargo", "bin", "cargo.exe")
        if os.path.isfile(candidate):
            cargo_bin = candidate
    if not cargo_bin:
        print("[ERROR] 找不到 cargo，請先安裝 Rust 與 Windows C++ 建置工具。")
        sys.exit(1)

    print(f"[*] 專案目錄：{script_dir}")
    print("[1/3] 執行單元測試：cargo test --workspace --locked")
    test_ret = subprocess.run([cargo_bin, "test", "--workspace", "--locked"], cwd=script_dir)
    if test_ret.returncode != 0:
        print("[ERROR] 單元測試失敗，中止建置。")
        sys.exit(test_ret.returncode)

    print("[2/3] 編譯 Release 執行檔：cargo build -p moi-sms-desktop --release --locked")
    build_ret = subprocess.run(
        [cargo_bin, "build", "-p", "moi-sms-desktop", "--release", "--locked"],
        cwd=script_dir,
    )
    if build_ret.returncode != 0:
        print("[ERROR] Release 編譯失敗。")
        sys.exit(build_ret.returncode)

    target_candidates = [
        r"C:\Temp\cargo-target\moi-sms\release\moi-sms-desktop.exe",
        os.path.join(script_dir, "target", "release", "moi-sms-desktop.exe"),
    ]
    built_exe = next((p for p in target_candidates if os.path.isfile(p)), None)
    if not built_exe:
        print("[ERROR] 找不到編譯完成的 moi-sms-desktop.exe。")
        sys.exit(1)

    updater_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
    dist_dir = os.path.join(updater_root, "dist")
    os.makedirs(dist_dir, exist_ok=True)

    out_exe_name = "智慧控管與監控系統 - 地籍異動即時通簡訊控管.exe"
    out_exe_path = os.path.join(dist_dir, out_exe_name)

    print(f"[3/3] 發布至：{out_exe_path}")
    try:
        shutil.copy2(built_exe, out_exe_path)
    except PermissionError:
        old_bak = out_exe_path + ".old"
        if os.path.isfile(old_bak):
            try:
                os.remove(old_bak)
            except OSError:
                pass
        try:
            if os.path.isfile(out_exe_path):
                os.rename(out_exe_path, old_bak)
            shutil.copy2(built_exe, out_exe_path)
        except OSError as e:
            print(f"[ERROR] 無法寫入 {out_exe_path}（可能正被執行中）：{e}")
            sys.exit(1)

    try:
        os.utime(out_exe_path, None)
    except OSError:
        pass

    print("=" * 60)
    print(f"[OK] 完成發布：{out_exe_path}")
    print("=" * 60)


if __name__ == "__main__":
    main()
