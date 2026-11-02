# -*- coding: utf-8 -*-
import os
import sys
import unittest
import tempfile
import shutil
import json
import datetime

# 將 pack 目錄加入模組搜尋路徑
CURRENT_DIR = os.path.dirname(os.path.abspath(__file__))
UPDATER_ROOT = os.path.dirname(CURRENT_DIR)
sys.path.insert(0, os.path.join(UPDATER_ROOT, "pack"))

from generate_release_notes import (
    parse_and_classify_commit,
    extract_commit_purpose,
    build_alert_section,
    render_release_notes,
    write_release_notes_files,
    load_anchor_data,
    commit_release_anchor,
    generate_notes,
    format_subproject_details,
    TYPE_MAP,
    IGNORED_TYPES,
)


class TestGenerateReleaseNotes(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_parse_and_classify_commit_whitelist(self):
        """驗證 Conventional Commits 白名單正確解析與標籤映射"""
        cases = [
            ("feat: 新增使用者登入逾時自動重新整理功能", "feat", "功能", None, "新增使用者登入逾時自動重新整理功能"),
            ("fix(dashboard): 修復 CPU 折線圖貼齊下緣問題", "fix", "修復", "dashboard", "修復 CPU 折線圖貼齊下緣問題"),
            ("perf: 優化前端核心套件排除規則", "perf", "優化", None, "優化前端核心套件排除規則"),
            ("security: 升級 CVE-2024-4577 防護規則", "security", "資安", None, "升級 CVE-2024-4577 防護規則"),
            ("refactor: 移除 pack.py 前端核心套件別名包產出邏輯", "refactor", "重構", None, "移除 pack.py 前端核心套件別名包產出邏輯"),
            ("docs: 更新伺服器安裝操作手冊", "docs", "手冊", None, "更新伺服器安裝操作手冊"),
        ]

        for subj, expected_type, expected_label, expected_scope, expected_desc in cases:
            commit = {"subject": subj, "body": "", "hash": "abcdef123456", "author": "tester", "date": "2026-11-03"}
            res = parse_and_classify_commit(commit)
            self.assertIsNotNone(res, f"應收錄之提交卻被過濾: {subj}")
            self.assertEqual(res["type"], expected_type)
            self.assertEqual(res["type_label"], expected_label)
            self.assertEqual(res["scope"], expected_scope)
            self.assertEqual(res["desc"], expected_desc)

    def test_parse_and_classify_commit_ignore(self):
        """驗證 chore、test、style 瑣碎提交確實被自動過濾"""
        ignored_cases = [
            "chore: 更新依賴版本",
            "test: 新增打包完整性測試案例",
            "style: 修正排版與縮排空格",
            "ci: 更新 GitHub Actions 流程",
            "build: 調整打包輸出路徑",
        ]

        for subj in ignored_cases:
            commit = {"subject": subj, "body": "", "hash": "12345678", "author": "tester", "date": "2026-11-03"}
            res = parse_and_classify_commit(commit)
            self.assertIsNone(res, f"瑣碎提交應被排除卻被收錄: {subj}")

    def test_extract_commit_purpose(self):
        """驗證自 Commit Body 中精確擷取修改目的"""
        body_with_purpose = (
            "- 修改目的：解決原機升級情境下 PM2 程序清單重複疊加導致行程翻倍之問題。\n"
            "- 修改項目：\n"
            "  - tools/updater/src-tauri/src/pm2.rs: 新增冪等重建函式。\n"
            "- 相容性說明：向後相容既有設定。"
        )
        purpose = extract_commit_purpose(body_with_purpose)
        self.assertIn("解決原機升級情境下 PM2 程序清單重複疊加", purpose)
        self.assertNotIn("修改項目", purpose)
        self.assertNotIn("相容性說明", purpose)

        # 測試另一種冒號寫法
        body_alt = "- 目的: 強化後端 FastCGI 通訊逾時保護"
        purpose_alt = extract_commit_purpose(body_alt)
        self.assertEqual(purpose_alt, "強化後端 FastCGI 通訊逾時保護")

    def test_build_alert_section_differentiation(self):
        """驗證依據異動子專案動態生成專屬警示"""
        # 僅變更 LAH-API
        alert_api = build_alert_section({"LAH-API"})
        self.assertIn("後端核心更新", alert_api)
        self.assertIn("Port 8085", alert_api)
        self.assertNotIn("前端監控更新", alert_api)

        # 僅變更 LAH-FE
        alert_fe = build_alert_section({"LAH-FE"})
        self.assertIn("前端監控更新", alert_fe)
        self.assertIn("PM2", alert_fe)
        self.assertNotIn("後端核心更新", alert_fe)

        # 變更全量子專案
        all_repos = {"LAH-API", "LAH-FE", "lah-messenger-server", "LAH-Updater"}
        alert_all = build_alert_section(all_repos)
        self.assertIn("後端核心更新", alert_all)
        self.assertIn("前端監控更新", alert_all)
        self.assertIn("即時通訊更新", alert_all)
        self.assertIn("維運工具庫更新", alert_all)

        # 無變更
        alert_empty = build_alert_section(set())
        self.assertIn("常態維運發布", alert_empty)

    def test_render_release_notes_crlf(self):
        """驗證模板渲染後的文字符合 Windows CRLF 換行規範"""
        template = "標題：{{SYSTEM_VERSION}}\n日期：{{RELEASE_DATE}}\n內容：\n{{ALERT_SECTION}}"
        vars_dict = {
            "SYSTEM_VERSION": "v1.9.9",
            "RELEASE_DATE": "2026 年 11 月 3 日",
            "ALERT_SECTION": "測試警示",
        }
        rendered = render_release_notes(template, vars_dict)
        self.assertIn("標題：v1.9.9", rendered)
        self.assertIn("日期：2026 年 11 月 3 日", rendered)
        self.assertTrue(rendered.endswith("\r\n") or "\r\n" in rendered)
        self.assertNotIn("\r\n\r\n\n", rendered)

    def test_write_release_notes_files_dual_paths(self):
        """驗證雙軌落盤：歷史版本檔、標準覆蓋檔與 patch 目錄探測檔"""
        dist_dir = os.path.join(self.test_dir, "dist")
        patch_dir = os.path.join(dist_dir, "patch")
        os.makedirs(patch_dir, exist_ok=True)

        content = "測試更新說明文字"
        today = datetime.date(2026, 11, 3)
        written = write_release_notes_files(content, dist_dir, "1.9.9", today)

        # 檢查產出檔案個數 (應為 3 個)
        self.assertEqual(len(written), 3)

        history_path = os.path.join(dist_dir, "20261103_系統更新說明_v1.9.9.txt")
        standard_path = os.path.join(dist_dir, "系統更新說明.txt")
        patch_path = os.path.join(patch_dir, "系統更新說明.txt")

        self.assertTrue(os.path.isfile(history_path), "缺少歷史版本存檔")
        self.assertTrue(os.path.isfile(standard_path), "缺少標準覆蓋檔")
        self.assertTrue(os.path.isfile(patch_path), "缺少 patch/ 探測檔")

        with open(standard_path, "r", encoding="utf-8") as f:
            self.assertEqual(f.read(), content)

    def test_dry_run_does_not_modify_disk(self):
        """驗證 --dry-run 唯讀模式不寫入任何檔案且不修改錨點"""
        out_dir = os.path.join(self.test_dir, "dist")
        content, written = generate_notes(dry_run=True, output_dir=out_dir)

        self.assertEqual(written, [])
        self.assertFalse(os.path.exists(out_dir))
        self.assertTrue(len(content) > 0)


if __name__ == "__main__":
    unittest.main()
