# -*- coding: utf-8 -*-
r"""
LAH 全系統更新說明自動化生成管線 (generate_release_notes.py)
職責：
1. 自動定位工作區 5 大子專案儲存庫 (LAH-API, LAH-FE, lah-messenger-server, lah-messenger, LAH-Updater)。
2. 基於發布錨點 (.release_anchor.json) 或指定天數，萃取各子專案最新 Git Commit 紀錄。
3. 結構化過濾 Conventional Commits (白名單: feat, fix, perf, security, refactor; 忽略: chore, test, style)。
4. 依實際異動子專案動態注入差異化維運警示標籤 (Alerts)。
5. 雙軌落盤至 dist/ 歷史版本檔與標準覆蓋檔 (系統更新說明.txt)。
6. 提供 --dry-run 唯讀終端彩色預覽模式，並實作發布成功後事務性錨點更新。
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
from typing import Dict, List, Optional, Tuple, Set


# 支援的 Conventional Commit 類型與中文標籤
TYPE_MAP = {
    "feat": "功能",
    "fix": "修復",
    "perf": "優化",
    "security": "資安",
    "refactor": "重構",
    "docs": "手冊",
}

# 忽略的提交類型
IGNORED_TYPES = {"chore", "test", "style", "ci", "build"}

# Conventional Commits 正則匹配
CONVENTIONAL_RE = re.compile(
    r"^(?P<type>[a-zA-Z]+)(?:\((?P<scope>[^)]+)\))?:\s*(?P<desc>.+)$"
)

# 子專案顯示資訊與順序
REPO_META = [
    {
        "id": "LAH-Updater",
        "name": "智慧控管與監控系統 - 維運部署工具庫 (LAH-Updater)",
        "service_alert": "⚠️ 【維運工具庫更新】維運管理工具已升級，請以最新版管理儀表板或自動更新工具執行巡檢與部署。"
    },
    {
        "id": "LAH-API",
        "name": "業務核心樞紐 RESTful API 伺服器 (LAH-API)",
        "service_alert": "⚠️ 【後端核心更新】涉及 API 業務邏輯或底層設定異動，請務必確認 Apache 2.4 (Port 80) 反向代理正常，並確認 PHP 8.5 FastCGI (Port 8085) 已重載就緒。"
    },
    {
        "id": "LAH-FE",
        "name": "智慧管控系統前端監控平台 (LAH-FE)",
        "service_alert": "⚠️ 【前端監控更新】涉及 Web 介面或排程引擎更新，請確認 Apache 靜態站台 (Port 8080) 與 PM2 進程 LAH-SCHEDULER 正常運行。"
    },
    {
        "id": "lah-messenger-server",
        "name": "即時通訊 WebSocket 伺服器 (lah-messenger-server)",
        "service_alert": "⚠️ 【即時通訊更新】涉及通訊協定或長連線服務，請確認 PM2 進程 LAH-WSS (Port 8081 WebSocket) 與 Port 8082 (HTTP 附件) 正常監聽。"
    },
    {
        "id": "lah-messenger",
        "name": "桃園即時通桌面客戶端 (lah-messenger)",
        "service_alert": "⚠️ 【即時通客戶端更新】即時通桌面應用程式更新，請通知同仁重啟客戶端套用最新修正。"
    },
]


def resolve_project_dirs(script_dir: str) -> Tuple[str, str, Dict[str, str]]:
    """
    定位工作區根目錄、LAH-Updater 根目錄與 5 大子專案目錄。
    """
    updater_dir = os.path.abspath(os.path.join(script_dir, ".."))
    workspace_root = os.path.abspath(os.path.join(updater_dir, ".."))

    # 若 updater_dir 本身就是根目錄則向上相容
    candidate_root = workspace_root
    if not (os.path.isdir(os.path.join(candidate_root, "LAH-API")) and os.path.isdir(os.path.join(candidate_root, "LAH-FE"))):
        # 嘗試以 updater_dir 自身探尋平級目錄
        candidate_root = os.path.dirname(updater_dir)

    repos = {
        "workspace": candidate_root,
        "LAH-Updater": updater_dir,
        "LAH-API": os.path.join(candidate_root, "LAH-API"),
        "LAH-FE": os.path.join(candidate_root, "LAH-FE"),
        "lah-messenger-server": os.path.join(candidate_root, "lah-messenger-server"),
        "lah-messenger": os.path.join(candidate_root, "lah-messenger"),
    }

    return candidate_root, updater_dir, repos


def get_fe_version(fe_dir: str) -> str:
    """從 LAH-FE/package.json 讀取系統版號"""
    pkg_path = os.path.join(fe_dir, "package.json")
    if os.path.isfile(pkg_path):
        try:
            with open(pkg_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                return data.get("version", "1.0.0")
        except Exception:
            pass
    return "1.0.0"


def load_anchor_data(anchor_file: str) -> Optional[dict]:
    """載入發布錨點紀錄檔案"""
    if os.path.isfile(anchor_file):
        try:
            with open(anchor_file, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as e:
            print(f"[WARN] 讀取發布錨點失敗: {e}", file=sys.stderr)
    return None


def get_repo_head(repo_path: str) -> Optional[str]:
    """取得指定 Repo 之 HEAD Commit Hash"""
    if not os.path.isdir(repo_path):
        return None
    try:
        res = subprocess.run(
            ["git", "-C", repo_path, "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True
        )
        return res.stdout.strip()
    except Exception:
        return None


def is_commit_in_repo(repo_path: str, commit_hash: str) -> bool:
    """檢查指定 Commit Hash 是否存在於該 Repo"""
    try:
        res = subprocess.run(
            ["git", "-C", repo_path, "cat-file", "-e", f"{commit_hash}^{{commit}}"],
            capture_output=True,
            check=False
        )
        return res.returncode == 0
    except Exception:
        return False


def get_repo_commits(
    repo_path: str,
    anchor_hash: Optional[str] = None,
    days: Optional[int] = None,
    since_date: Optional[str] = None
) -> List[dict]:
    """
    取得指定 Repo 自錨點或指定時間以來的 Commit 列表。
    每筆 Commit 包含：hash, author, date, subject, body
    """
    if not os.path.isdir(repo_path):
        return []

    cmd = [
        "git", "-C", repo_path, "log",
        "--format=%H%x1f%an%x1f%ad%x1f%s%x1f%b%x1e",
        "--date=short"
    ]

    range_spec = None
    if since_date:
        cmd.extend([f"--since={since_date}", "HEAD"])
    elif days is not None and days > 0:
        cmd.extend([f"--since={days} days ago", "HEAD"])
    elif anchor_hash and is_commit_in_repo(repo_path, anchor_hash):
        head_hash = get_repo_head(repo_path)
        if head_hash and head_hash.lower() == anchor_hash.lower():
            # HEAD 與錨點相同，無新增提交
            return []
        range_spec = f"{anchor_hash}..HEAD"
        cmd.append(range_spec)
    else:
        # 回退至近 7 天
        cmd.extend(["--since=7 days ago", "HEAD"])

    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True
        )
        raw_output = res.stdout
    except Exception as e:
        print(f"[WARN] 查詢 Git Log 失敗 ({repo_path}): {e}", file=sys.stderr)
        return []

    commits = []
    raw_records = raw_output.strip("\x1e\r\n").split("\x1e")
    for rec in raw_records:
        rec = rec.strip()
        if not rec:
            continue
        parts = rec.split("\x1f")
        if len(parts) >= 5:
            commits.append({
                "hash": parts[0].strip(),
                "author": parts[1].strip(),
                "date": parts[2].strip(),
                "subject": parts[3].strip(),
                "body": parts[4].strip()
            })
        elif len(parts) == 4:
            commits.append({
                "hash": parts[0].strip(),
                "author": parts[1].strip(),
                "date": parts[2].strip(),
                "subject": parts[3].strip(),
                "body": ""
            })

    return commits


def extract_commit_purpose(body: str) -> str:
    """自 Commit Body 中擷取修改目的或重點說明"""
    if not body:
        return ""

    lines = [line.strip() for line in body.splitlines() if line.strip()]
    purpose_lines = []
    recording = False

    for line in lines:
        m = re.search(r"^-\s*(?:修改目的|目的|背景與目的|修改背景)[：:]\s*(.*)", line)
        if m:
            content = m.group(1).strip()
            if content:
                purpose_lines.append(content)
            recording = True
            continue
        elif line.startswith("- 修改項目") or line.startswith("- 具體修改") or line.startswith("- 相容性") or line.startswith("- 介面相容"):
            recording = False
            break
        elif recording:
            if line.startswith("- ") or line.startswith("* "):
                purpose_lines.append(line.lstrip("-* ").strip())
            else:
                purpose_lines.append(line)

    if purpose_lines:
        return "；".join(purpose_lines[:2])

    # 回退啟發式抽取：尋找首個合適的非結構標籤行
    for line in lines:
        clean = re.sub(r"^[-*]\s*", "", line).strip()
        if (
            clean
            and not clean.startswith("修改項目")
            and not clean.startswith("相容性")
            and not clean.startswith("修改目的")
            and len(clean) >= 6
        ):
            return clean[:90]

    return ""


def parse_and_classify_commit(commit: dict) -> Optional[dict]:
    """
    解析 Conventional Commit，過濾非業務瑣碎類型，並結構化提取資訊。
    """
    subject = commit.get("subject", "").strip()
    m = CONVENTIONAL_RE.match(subject)
    if not m:
        # 非標準 Conventional Commit：檢查是否包含已知關鍵詞
        lower_subj = subject.lower()
        if any(lower_subj.startswith(p) for p in ("chore", "test", "style", "ci")):
            return None
        # 當作常規修改納入
        ctype = "feat" if ("新增" in subject or "支援" in subject) else "fix"
        scope = None
        desc = subject
    else:
        ctype = m.group("type").lower()
        scope = m.group("scope")
        desc = m.group("desc").strip()

    if ctype in IGNORED_TYPES:
        return None

    # 對應中文分類標籤
    tag_name = TYPE_MAP.get(ctype, "其他")
    purpose = extract_commit_purpose(commit.get("body", ""))

    return {
        "hash": commit.get("hash", "")[:8],
        "author": commit.get("author", ""),
        "date": commit.get("date", ""),
        "type": ctype,
        "type_label": tag_name,
        "scope": scope,
        "desc": desc,
        "purpose": purpose,
    }


def build_alert_section(changed_repo_ids: Set[str]) -> str:
    """依據本次異動之子專案清單，動態生成差異化維運警示與重點摘要"""
    alerts = []
    for meta in REPO_META:
        if meta["id"] in changed_repo_ids:
            alerts.append(meta["service_alert"])

    if not alerts:
        return "● 本期為全系統版本校準與常態維運發布，無重大破壞性變更，各服務依標準流程重啟即可。"

    lines = []
    for i, alert in enumerate(alerts, 1):
        lines.append(f"{i}. {alert}")
    return "\n".join(lines)


def format_subproject_details(commits_by_repo: Dict[str, List[dict]]) -> str:
    """排版各子專案之詳細更新歷程"""
    sections = []
    idx = 1

    for meta in REPO_META:
        repo_id = meta["id"]
        commits = commits_by_repo.get(repo_id, [])
        if not commits:
            continue

        header = f"■ {idx}. {meta['name']}"
        divider = "-" * 80
        lines = [header, divider]

        for c in commits:
            scope_prefix = f"[{c['scope']}] " if c.get("scope") else ""
            type_tag = c.get("type_label", "更新")
            desc = c.get("desc", "")
            lines.append(f"  ● [{type_tag}] {scope_prefix}{desc}")
            if c.get("purpose"):
                lines.append(f"    - 目的：{c['purpose']}")

        sections.append("\n".join(lines))
        idx += 1

    if not sections:
        return "  (本次發布區間內各子專案查無符合條件之業務與架構更新紀錄)"

    return "\n\n".join(sections)


def render_release_notes(template_content: str, vars_dict: dict) -> str:
    """替換模板中之預留變數標籤"""
    content = template_content
    for key, value in vars_dict.items():
        placeholder = f"{{{{{key}}}}}"
        content = content.replace(placeholder, str(value))
    # 確保標準 Windows CRLF 換行
    content = content.replace("\r\n", "\n").replace("\n", "\r\n")
    return content


def write_release_notes_files(
    content: str,
    dist_dir: str,
    version_str: str,
    release_date_raw: datetime.date
) -> List[str]:
    """將單一整潔之標準更新說明檔寫入 dist/（避免多檔分散堆疊）"""
    os.makedirs(dist_dir, exist_ok=True)
    written_files = []

    # 清理舊有分散之歷史日期前綴 txt 檔
    for item in os.listdir(dist_dir):
        if "系統更新說明" in item and item != "系統更新說明.txt" and item.endswith(".txt"):
            try:
                os.remove(os.path.join(dist_dir, item))
            except OSError:
                pass

    # 標準單一覆蓋檔案 (dist/系統更新說明.txt)
    standard_path = os.path.join(dist_dir, "系統更新說明.txt")
    with open(standard_path, "w", encoding="utf-8", newline="") as f:
        f.write(content)
    written_files.append(standard_path)

    return written_files


def commit_release_anchor(
    anchor_file: str,
    version_str: str,
    repos_dict: Dict[str, str]
) -> dict:
    """將當前各專案之 HEAD Commit Hash 與最後打包日期寫入 .release_anchor.json 與 LAST_PACK_DATE.txt"""
    now = datetime.datetime.now().astimezone()
    today_str = now.strftime("%Y-%m-%d")
    anchors = {}
    for repo_id, repo_path in repos_dict.items():
        head = get_repo_head(repo_path)
        if head:
            anchors[repo_id] = head

    data = {
        "version": version_str.lstrip("vV"),
        "released_at": now.isoformat(),
        "last_packed_date": today_str,
        "last_packed_at": now.isoformat(),
        "anchors": anchors
    }

    anchor_dir = os.path.dirname(os.path.abspath(anchor_file))
    os.makedirs(anchor_dir, exist_ok=True)
    with open(anchor_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")

    # 同步落盤純文字最後打包日期檔案，便利維運人員與批次腳本隨手檢視
    txt_path = os.path.join(anchor_dir, "LAST_PACK_DATE.txt")
    try:
        with open(txt_path, "w", encoding="utf-8") as f:
            f.write(f"{today_str}\n")
    except Exception:
        pass

    return data


def generate_notes(
    dry_run: bool = False,
    days: Optional[int] = None,
    since: Optional[str] = None,
    output_dir: Optional[str] = None,
    commit_anchor: bool = False
) -> Tuple[str, List[str]]:
    """
    更新說明生成主進入點。
    返回：(產出之更新說明文字, 寫入之檔案路徑清單)
    """
    script_dir = os.path.dirname(os.path.abspath(__file__))
    workspace_root, updater_dir, repos = resolve_project_dirs(script_dir)
    template_path = os.path.join(script_dir, "templates", "release_notes.tpl")
    anchor_path = os.path.join(script_dir, ".release_anchor.json")
    dist_dir = output_dir if output_dir else os.path.join(updater_dir, "dist")

    if not os.path.isfile(template_path):
        raise FileNotFoundError(f"找不到更新說明模板：{template_path}")

    with open(template_path, "r", encoding="utf-8") as f:
        tpl_content = f.read()

    fe_dir = repos.get("LAH-FE", "")
    version_str = get_fe_version(fe_dir)
    clean_ver = f"v{version_str}" if not version_str.startswith("v") else version_str

    anchor_data = load_anchor_data(anchor_path)
    old_anchors = anchor_data.get("anchors", {}) if anchor_data else {}
    last_release_str = anchor_data.get("released_at", "") if anchor_data else ""
    last_packed_date = anchor_data.get("last_packed_date", "") if anchor_data else ""

    # 決定涵蓋週期起點字串 (優先採用上次記錄之打包斷點日期)
    if since:
        start_date_str = since
    elif days:
        d = datetime.date.today() - datetime.timedelta(days=days)
        start_date_str = d.strftime("%Y-%m-%d")
    elif last_packed_date:
        start_date_str = last_packed_date
    elif last_release_str:
        try:
            dt = datetime.datetime.fromisoformat(last_release_str)
            start_date_str = dt.strftime("%Y-%m-%d")
        except Exception:
            start_date_str = (datetime.date.today() - datetime.timedelta(days=7)).strftime("%Y-%m-%d")
    else:
        start_date_str = (datetime.date.today() - datetime.timedelta(days=7)).strftime("%Y-%m-%d")

    today = datetime.date.today()
    end_date_str = today.strftime("%Y-%m-%d")
    coverage_period = f"{start_date_str} ～ {end_date_str}"
    release_date_display = f"{today.year} 年 {today.month} 月 {today.day} 日"

    commits_by_repo = {}
    changed_repos = set()

    for meta in REPO_META:
        repo_id = meta["id"]
        repo_path = repos.get(repo_id)
        if not repo_path or not os.path.isdir(repo_path):
            continue

        anchor_hash = old_anchors.get(repo_id) if not since and not days else None
        raw_commits = get_repo_commits(
            repo_path=repo_path,
            anchor_hash=anchor_hash,
            days=days,
            since_date=since
        )

        classified = []
        for c in raw_commits:
            parsed = parse_and_classify_commit(c)
            if parsed:
                classified.append(parsed)

        if classified:
            commits_by_repo[repo_id] = classified
            changed_repos.add(repo_id)

    alert_section = build_alert_section(changed_repos)
    details_section = format_subproject_details(commits_by_repo)

    vars_dict = {
        "RELEASE_DATE": release_date_display,
        "SYSTEM_VERSION": clean_ver,
        "COVERAGE_PERIOD": coverage_period,
        "ALERT_SECTION": alert_section,
        "SUBPROJECT_DETAILS": details_section,
    }

    rendered = render_release_notes(tpl_content, vars_dict)

    if dry_run:
        return rendered, []

    written_files = write_release_notes_files(rendered, dist_dir, clean_ver, today)

    if commit_anchor:
        commit_release_anchor(anchor_path, version_str, repos)

    return rendered, written_files


def main():
    if sys.platform == "win32":
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
            sys.stderr.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    parser = argparse.ArgumentParser(description="LAH 全系統更新說明自動化生成管線")
    parser.add_argument("--dry-run", action="store_true", help="唯讀終端預覽模式，不產出實體檔案，不修改錨點")
    parser.add_argument("--days", type=int, default=None, help="覆寫以近 N 天之 Commit 作為起點")
    parser.add_argument("--since", type=str, default=None, help="覆寫以指定日期 (YYYY-MM-DD) 作為起點")
    parser.add_argument("--output-dir", type=str, default=None, help="指定輸出目錄 (預設為 LAH-Updater/dist)")
    parser.add_argument("--commit-anchor", action="store_true", help="強制更新發布錨點 (.release_anchor.json)")
    args = parser.parse_args()

    try:
        content, written_files = generate_notes(
            dry_run=args.dry_run,
            days=args.days,
            since=args.since,
            output_dir=args.output_dir,
            commit_anchor=args.commit_anchor
        )
    except Exception as e:
        print(f"[ERROR] 產生更新說明失敗: {e}", file=sys.stderr)
        sys.exit(1)

    if args.dry_run:
        print("\n" + "=" * 80)
        print(" [DRY-RUN 預覽模式] 產出更新說明文字如下 (未寫入磁碟):")
        print("=" * 80 + "\n")
        print(content)
        print("=" * 80)
        print(" [OK] 預覽完畢，發布錨點與實體檔案保持未變更。")
    else:
        print("\n" + "=" * 60)
        print(f"[OK] 系統更新說明產生完成！共輸出 {len(written_files)} 處：")
        for p in written_files:
            print(f"  - {p}")
        if args.commit_anchor:
            print("  - [已更新發布錨點] .release_anchor.json")
        print("=" * 60)


if __name__ == "__main__":
    main()
