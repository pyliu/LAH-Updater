# -*- coding: utf-8 -*-
r"""
LAH-Updater 工具集通用版本號管理模組 (version_helper.py)
規範：
1. 以天為單位 +1：當天首次 BUILD 自動遞增版本號；同一天內再次 BUILD 保持相同版本號。
2. 數字只有 1-9：逢 9 進位且末碼歸零 (如 0.1.0 -> 0.1.1, 0.1.9 -> 0.2.0, 0.9.9 -> 1.0.0)。
3. 各工具獨立維護自身的 version.json 與建置日期。
4. 自動同步寫入 version.json、src-tauri/Cargo.toml、src-tauri/tauri.conf.json 與 ui/index.html。
"""
import datetime
import json
import os
import re
import sys


def parse_version(ver_str: str):
    """解析語義化版本字串，去除前綴 v"""
    cleaned = ver_str.strip().lstrip("vV")
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)$", cleaned)
    if not m:
        return (0, 1, 0)
    return (int(m.group(1)), int(m.group(2)), int(m.group(3)))


def format_version(major: int, minor: int, patch: int) -> str:
    """格式化為純版號字串"""
    return f"{major}.{minor}.{patch}"


def increment_version(ver_str: str) -> str:
    """
    依規範遞增版本號：
    以天為單位 +1，數字逢 9 進位且末碼歸零
    如：0.1.0 -> 0.1.1, 0.1.9 -> 0.2.0, 0.9.9 -> 1.0.0
    """
    major, minor, patch = parse_version(ver_str)
    if patch < 9:
        patch += 1
    else:
        patch = 0
        if minor < 9:
            minor += 1
        else:
            minor = 0
            major += 1
    return format_version(major, minor, patch)


def load_tool_version_info(tool_dir: str):
    """讀取工具的版號資訊，若無 version.json 則自 Cargo.toml 或 tauri.conf.json 探測"""
    v_file = os.path.join(tool_dir, "version.json")
    if os.path.isfile(v_file):
        try:
            with open(v_file, "r", encoding="utf-8") as f:
                data = json.load(f)
                return {
                    "version": data.get("version", "0.1.0"),
                    "last_build_date": data.get("last_build_date", ""),
                }
        except Exception:
            pass

    # Fallback：自 Cargo.toml 讀取
    cargo_file = os.path.join(tool_dir, "src-tauri", "Cargo.toml")
    if os.path.isfile(cargo_file):
        try:
            with open(cargo_file, "r", encoding="utf-8") as f:
                content = f.read()
                m = re.search(r'version\s*=\s*"([^"]+)"', content)
                if m:
                    return {"version": m.group(1), "last_build_date": ""}
        except Exception:
            pass

    # Fallback：自 tauri.conf.json 讀取
    tauri_file = os.path.join(tool_dir, "src-tauri", "tauri.conf.json")
    if os.path.isfile(tauri_file):
        try:
            with open(tauri_file, "r", encoding="utf-8") as f:
                tdata = json.load(f)
                return {"version": tdata.get("version", "0.1.0"), "last_build_date": ""}
        except Exception:
            pass

    return {"version": "0.1.0", "last_build_date": ""}


def sync_version_files(tool_dir: str, new_ver: str, today_str: str):
    """將新版本號同步寫入各相關設定檔與前端 UI"""
    # 1. 寫入 version.json
    v_file = os.path.join(tool_dir, "version.json")
    try:
        with open(v_file, "w", encoding="utf-8") as f:
            json.dump({"version": new_ver, "last_build_date": today_str}, f, indent=2, ensure_ascii=False)
    except Exception as e:
        print(f"[WARN] 寫入 {v_file} 失敗: {e}")

    # 2. 寫入 Cargo.toml
    cargo_file = os.path.join(tool_dir, "src-tauri", "Cargo.toml")
    if os.path.isfile(cargo_file):
        try:
            with open(cargo_file, "r", encoding="utf-8") as f:
                cargo_text = f.read()
            # 替換 [package] 下的 version = "..."
            new_cargo = re.sub(
                r'(\[package\][\s\S]*?version\s*=\s*")[^"]+(")',
                r"\g<1>" + new_ver + r"\g<2>",
                cargo_text,
                count=1,
            )
            with open(cargo_file, "w", encoding="utf-8") as f:
                f.write(new_cargo)
        except Exception as e:
            print(f"[WARN] 同步更新 {cargo_file} 失敗: {e}")

    # 3. 寫入 tauri.conf.json
    tauri_file = os.path.join(tool_dir, "src-tauri", "tauri.conf.json")
    if os.path.isfile(tauri_file):
        try:
            with open(tauri_file, "r", encoding="utf-8") as f:
                tdata = json.load(f)
            tdata["version"] = new_ver
            with open(tauri_file, "w", encoding="utf-8") as f:
                json.dump(tdata, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"[WARN] 同步更新 {tauri_file} 失敗: {e}")

    # 4. 同步更新 ui/index.html (若有 version-tag 或 vX.X.X 標籤)
    ui_html = os.path.join(tool_dir, "ui", "index.html")
    if os.path.isfile(ui_html):
        try:
            with open(ui_html, "r", encoding="utf-8") as f:
                html_text = f.read()

            # 替換 <span class="version-tag">vX.X.X</span>
            replaced_html = re.sub(
                r'(<span[^>]*class=["\'][^"\']*version-tag[^"\']*["\'][^>]*>)[^<]*(</span>)',
                rf"\g<1>v{new_ver}\g<2>",
                html_text,
            )
            # 若無 version-tag 類別但有一般版本字樣也可備援相容
            if replaced_html != html_text:
                with open(ui_html, "w", encoding="utf-8") as f:
                    f.write(replaced_html)
        except Exception as e:
            print(f"[WARN] 同步更新 {ui_html} 失敗: {e}")


def bump_version_if_needed(tool_dir: str, force: bool = False) -> str:
    """
    建置時調用：檢查並在跨日時自動遞增版本號
    回傳當前 (或遞增後) 的版本號 (純數字，如 '0.1.1')
    """
    today_str = datetime.date.today().strftime("%Y-%m-%d")
    info = load_tool_version_info(tool_dir)
    current_ver = info["version"]
    last_date = info["last_build_date"]

    if force or last_date != today_str:
        # 新的一天首次 BUILD 或強制更新：版號 +1
        target_ver = increment_version(current_ver)
        sync_version_files(tool_dir, target_ver, today_str)
        return target_ver
    else:
        # 當日已建置過：維持當前版號，確保各檔案版本一致
        sync_version_files(tool_dir, current_ver, today_str)
        return current_ver


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "status":
        tools_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        print("=" * 60)
        print("LAH-Updater 各工具版本號與建置日期狀態")
        print("=" * 60)
        for item in sorted(os.listdir(tools_dir)):
            full_path = os.path.join(tools_dir, item)
            if os.path.isdir(full_path) and os.path.isfile(os.path.join(full_path, "build.py")):
                info = load_tool_version_info(full_path)
                d_str = info["last_build_date"] or "尚未建置記錄"
                print(f"• {item:<18} : v{info['version']:<8} (最後建置: {d_str})")
        print("=" * 60)
    elif len(sys.argv) > 2 and sys.argv[1] == "bump":
        target = sys.argv[2]
        tools_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        tool_dir = os.path.join(tools_dir, target)
        if os.path.isdir(tool_dir):
            new_v = bump_version_if_needed(tool_dir, force=True)
            print(f"[OK] {target} 已強制遞增版號至: v{new_v}")
        else:
            print(f"[ERROR] 找不到工具目錄: {target}")
    else:
        print("用法：")
        print("  python version_helper.py status       # 查看所有工具版號狀態")
        print("  python version_helper.py bump <tool>  # 手動強制遞增指定工具版號")


if __name__ == "__main__":
    main()
