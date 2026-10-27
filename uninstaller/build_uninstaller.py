# -*- coding: utf-8 -*-
r"""
智慧控管與監控系統 - 自動移除工具 (uninstaller)
轉發腳本至 build.py (Tauri 2 / Rust 發布建置)
"""
import os
import runpy

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    build_py = os.path.join(script_dir, "build.py")
    runpy.run_path(build_py, run_name="__main__")
