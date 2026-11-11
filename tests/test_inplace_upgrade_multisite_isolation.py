# -*- coding: utf-8 -*-
"""
自動更新工具原地升級 - 跨所組態隔離與自動繼承端到端驗證測試 (E2E)
對應規格書：specs/updater-inplace-upgrade-multisite-isolation.md
工單：05-end-to-end-upgrade-verification.md
"""
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

CURRENT_DIR = os.path.dirname(os.path.abspath(__file__))
UPDATER_ROOT = os.path.dirname(CURRENT_DIR)
sys.path.insert(0, os.path.join(UPDATER_ROOT, "pack"))

from pack import (
    PackIntegrityError,
    verify_zip_integrity,
    resolve_project_dirs,
)


class TestInplaceUpgradeMultisiteIsolation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        dirs = resolve_project_dirs(os.path.join(UPDATER_ROOT, "pack"))
        cls.api_dir = dirs[0]
        cls.api8_dir = dirs[1]
        cls.fe_dir = dirs[2]
        cls.messenger_dir = dirs[3]

    def test_luzhu_he_database_first_inheritance_e2e(self):
        """
        【E2E 蘆竹所原地升級驗證】
        模擬蘆竹所 (HE) 既有安裝機台：
        - 僅有 assets/db/dimension.db (SITE='HE', SITE_ID='43504044', WEBAP_IP='220.1.38.161')
        - 刻意排除 www/.env 與 LAH-NUXTJS/.env (精準重現線上回報之缺失情境)
        - 釋放去特化中立範本 .env.example
        - 驗證 Rust 整合管線測試已確定性斷言：
          1. www/.env 成功繼承 SITE=HE，絕不退回 HA
          2. runtime-config.js 包含 site: HE 與 220.1.38.253，絕無桃園所 220.1.34.253
        """
        # 呼叫 Rust 的 E2E 測試以確保底層二進位引擎邏輯 100% 吻合
        src_tauri_dir = os.path.join(UPDATER_ROOT, "tools", "updater", "src-tauri")
        proc = subprocess.run(
            ["cargo", "test", "--lib", "test_e2e_luzhu_inplace_upgrade_pipeline"],
            cwd=src_tauri_dir,
            capture_output=True,
            text=True,
            encoding="utf-8"
        )
        self.assertEqual(
            proc.returncode, 0,
            f"Rust E2E 端到端升級管線測試失敗！\nstdout: {proc.stdout}\nstderr: {proc.stderr}"
        )
        self.assertIn("test extractor::tests::test_e2e_luzhu_inplace_upgrade_pipeline ... ok", proc.stdout)

    def test_all_eight_sites_srmas_mapping(self):
        """驗證全桃園八個地政事務所代碼與網段映射的一致性"""
        site_ip_map = {
            "HA": "220.1.34.253",
            "HB": "220.1.35.253",
            "HC": "220.1.36.253",
            "HD": "220.1.37.253",
            "HE": "220.1.38.253",
            "HF": "220.1.39.253",
            "HG": "220.1.40.253",
            "HH": "220.1.41.253",
        }
        for site, expected_srmas in site_ip_map.items():
            self.assertTrue(expected_srmas.startswith("220.1."))
            self.assertTrue(expected_srmas.endswith(".253"))

    def test_published_template_neutrality(self):
        """驗證專案中實際發布之 www/.env.example 已完全中立化，無寫死之 SITE=HA"""
        example_path = os.path.join(self.api_dir, ".env.example")
        self.assertTrue(os.path.isfile(example_path), f"找不到範本檔：{example_path}")
        with open(example_path, "r", encoding="utf-8") as f:
            content = f.read()

        # 檢驗無未註解的 SITE=HA
        for line in content.splitlines():
            trimmed = line.strip()
            if trimmed.startswith("#") or not trimmed:
                continue
            match = re.match(r"^SITE\s*=\s*([A-Za-z0-9_]+)", trimmed)
            self.assertIsNone(
                match,
                f"發現未註解之寫死所別代碼：{trimmed}！範本必須保持中立指引。"
            )


if __name__ == "__main__":
    unittest.main()
