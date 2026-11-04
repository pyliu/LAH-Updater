import os
import sys
import unittest
import zipfile
import tempfile
import shutil
import subprocess

# 將 pack 目錄加入模組搜尋路徑
CURRENT_DIR = os.path.dirname(os.path.abspath(__file__))
UPDATER_ROOT = os.path.dirname(CURRENT_DIR)
sys.path.insert(0, os.path.join(UPDATER_ROOT, "pack"))

from pack import (
    PackIntegrityError,
    extract_nuxt_runtime_dependencies,
    verify_fe_pack_integrity,
    verify_zip_integrity,
    collect_package_files,
    resolve_project_dirs,
    FE_INCLUDES,
    FE_EXCLUDES,
    MESSENGER_INCLUDES,
    MESSENGER_EXCLUDES,
)


class TestPackIntegrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        dirs = resolve_project_dirs(os.path.join(UPDATER_ROOT, "pack"))
        cls.api_dir = dirs[0]
        cls.api8_dir = dirs[1]
        cls.fe_dir = dirs[2]
        cls.messenger_dir = dirs[3]

    def test_dynamic_detection_of_server_middleware(self):
        """驗證能從 nuxt.config.js 自動動態偵測 serverMiddleware 與中介軟體目錄（單一真相來源）"""
        required_deps, dynamic_includes = extract_nuxt_runtime_dependencies(self.fe_dir)
        norm_required = [d.replace("\\", "/").lower() for d in required_deps]
        
        # 必須偵測到 security-headers.js
        self.assertTrue(
            any("server-middleware/security-headers.js" in d for d in norm_required),
            f"未偵測到 serverMiddleware 宣告：{norm_required}"
        )
        # 動態包含目錄必須納入 server-middleware
        self.assertIn("server-middleware", dynamic_includes)
        # 必須偵測到 scheduler/server.js
        self.assertTrue(
            any("scheduler/server.js" in d for d in norm_required),
            f"未偵測到 scheduler/server.js：{norm_required}"
        )

    def test_regression_server_middleware_omission_triggers_failure(self):
        """
        【真實歷史事故回歸測試】
        模擬過去未加入 server-middleware 之打包清單，證明此檢查必定能觸發攔截失敗！
        """
        # 建立一份故意漏掉 server-middleware 的待打包檔案清單（重現本次線上事故）
        mock_files = [
            (os.path.join(self.fe_dir, ".nuxt", "App.js"), ".nuxt/App.js"),
            (os.path.join(self.fe_dir, "nuxt.config.js"), "nuxt.config.js"),
            (os.path.join(self.fe_dir, "ecosystem.config.js"), "ecosystem.config.js"),
            (os.path.join(self.fe_dir, "scheduler", "server.js"), "scheduler/server.js"),
            # 故意不放 server-middleware/security-headers.js
        ]
        
        with self.assertRaises(PackIntegrityError) as ctx:
            verify_fe_pack_integrity(self.fe_dir, mock_files)
        
        err_msg = str(ctx.exception)
        self.assertIn("server-middleware", err_msg)
        self.assertIn("前端打包完整性守門員攔截到缺失的執行期程式碼", err_msg)

    def test_pre_pack_integrity_passes_with_effective_includes(self):
        """驗證搭配動態偵測後，現有 LAH-FE 專案打包清單能 100% 通過事前校驗"""
        _, dynamic_includes = extract_nuxt_runtime_dependencies(self.fe_dir)
        effective_includes = list(dict.fromkeys(FE_INCLUDES + dynamic_includes))
        files = collect_package_files(self.fe_dir, effective_includes, FE_EXCLUDES)
        
        # 必須通過校驗，無拋出例外
        result = verify_fe_pack_integrity(self.fe_dir, files)
        self.assertTrue(result)

    def test_messenger_wildcard_matching(self):
        """驗證即時通訊伺服器採用萬用字元 *.js 後，能自動覆蓋所有根目錄伺服端 JS 檔案"""
        files = collect_package_files(self.messenger_dir, MESSENGER_INCLUDES, MESSENGER_EXCLUDES)
        arcnames = {arcname.lower() for _, arcname in files}
        
        # 關鍵檔案必須全數在列
        expected_files = [
            "server.js", "ecosystem.config.js", "channel-db.js",
            "message-db.js", "message-watcher.js", "request-handler.js",
            "upload-router.js", "utils.js", "package.json"
        ]
        for exp in expected_files:
            self.assertIn(exp, arcnames, f"即時通伺服器打包清單缺少：{exp}")

    def test_zip_integrity_detects_corrupted_or_missing_files(self):
        """驗證 post-pack ZIP 校驗能攔截缺少中介軟體的破損更新包並自動刪除"""
        with tempfile.TemporaryDirectory() as tmpdir:
            test_zip = os.path.join(tmpdir, "corrupted-fe.zip")
            # 建立一個缺少 server-middleware 的破損 ZIP
            with zipfile.ZipFile(test_zip, "w") as zf:
                zf.writestr("nuxt.config.js", "// mock")
                zf.writestr("ecosystem.config.js", "// mock")
                zf.writestr("package.json", "{}")
                zf.writestr(".nuxt/dummy.txt", "dummy")
            
            with self.assertRaises(PackIntegrityError) as ctx:
                verify_zip_integrity(test_zip, "fe", self.fe_dir)
            
            err_msg = str(ctx.exception)
            self.assertIn("更新包成品完整性校驗失敗", err_msg)
            # 驗證破損 ZIP 已被自動銷毀
            self.assertFalse(os.path.exists(test_zip), "破損 ZIP 未被自動刪除！")

    def test_pack_summary_handles_missing_optional_node_without_unbound_error(self):
        """【防回歸測試】驗證當本機未安裝 Node.js 綠色環境時，打包結尾摘要不會拋出 UnboundLocalError 崩潰"""
        pack_script = os.path.join(UPDATER_ROOT, "pack", "pack.py")
        proc = subprocess.run(
            [sys.executable, pack_script, "--node-only", "--no-notes"],
            capture_output=True,
            text=True,
            encoding="utf-8"
        )
        self.assertEqual(proc.returncode, 0, f"執行失敗，stderr: {proc.stderr}")
        self.assertNotIn("UnboundLocalError", proc.stderr)
        self.assertIn("全部打包作業順利完成", proc.stdout)
        self.assertIn("略過，找不到 Node.js 綠色環境目錄", proc.stdout)


if __name__ == "__main__":
    unittest.main()
