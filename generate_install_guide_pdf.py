# -*- coding: utf-8 -*-
"""
桃園市各地政事務所 智慧控管與監控系統 (LAH) - 伺服器安裝與版本升級操作手冊 PDF 產生器
產出標準 8 頁公務維運手冊：包含封面、環境先決條件、備份指引、原機升級、全新安裝(卸載+安裝還原)、五大埠驗證、回滾SOP與附錄檢核表。
"""

import os
import sys
import base64
import subprocess

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DIST_DIR = os.path.join(SCRIPT_DIR, "dist")
ASSET_IMG_DIR = os.path.join(SCRIPT_DIR, "assets", "img")
HTML_OUTPUT = os.path.join(DIST_DIR, "智慧控管與監控系統_系統安裝與版本升級操作手冊.html")
PDF_OUTPUT = os.path.join(DIST_DIR, "智慧控管與監控系統_系統安裝與版本升級操作手冊.pdf")

def get_base64_img(filename):
    for base_dir in [ASSET_IMG_DIR, DIST_DIR]:
        path = os.path.join(base_dir, filename)
        if os.path.exists(path):
            with open(path, "rb") as f:
                data = f.read()
                b64 = base64.b64encode(data).decode('utf-8')
                ext = os.path.splitext(filename)[1].lower().replace('.', '')
                if ext == 'jpg':
                    ext = 'jpeg'
                return f"data:image/{ext};base64,{b64}"
    return ""

img_upgrade = get_base64_img("01.升級更新-自動更新工作選項勾選圖解.jpg")
img_backup = get_base64_img("02.備份-備份工具圖解.jpg")
img_uninstall = get_base64_img("03.移除-移除工具圖解.jpg")
img_clean_install = get_base64_img("04.全新安裝-自動更新工作選項勾選圖解.jpg")

HTML_CONTENT = f"""<!DOCTYPE html>
<html lang="zh-Hant">
<head>
<meta charset="utf-8">
<title>智慧控管與監控系統 - 伺服器安裝與版本升級操作手冊</title>
<style>
  @page {{
    size: A4 portrait;
    margin: 12mm 14mm 14mm 14mm;
    @bottom-center {{
      content: counter(page);
      font-size: 8.5pt;
      font-family: "Microsoft JhengHei UI", sans-serif;
      color: #64748b;
    }}
  }}

  * {{
    box-sizing: border-box;
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }}

  body {{
    font-family: "Microsoft JhengHei UI", "Microsoft JhengHei", "Segoe UI", -apple-system, sans-serif;
    color: #1e293b;
    background: #ffffff;
    line-height: 1.5;
    font-size: 9.5pt;
    margin: 0;
    padding: 0;
  }}

  .page {{
    page-break-before: always;
    height: 100%;
    min-height: 980px;
    max-height: 1040px;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    position: relative;
    overflow: hidden;
  }}

  .page:first-child {{
    page-break-before: avoid;
  }}

  .avoid-break {{
    break-inside: avoid;
    page-break-inside: avoid;
  }}

  /* 封面頁專屬設計 (Page 1) */
  .cover-page {{
    padding: 24px 26px;
    border: 2.5px solid #0284c7;
    border-radius: 10px;
    background: linear-gradient(175deg, #f0f9ff 0%, #ffffff 45%, #f8fafc 100%);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 100%;
    min-height: 980px;
    max-height: 1030px;
    box-sizing: border-box;
  }}

  .cover-badge {{
    display: inline-block;
    background: #0284c7;
    color: #ffffff;
    font-size: 8.5pt;
    font-weight: 700;
    padding: 4px 14px;
    border-radius: 20px;
    letter-spacing: 1px;
    margin-bottom: 12px;
  }}

  .cover-title {{
    font-size: 24pt;
    font-weight: 900;
    color: #0f172a;
    line-height: 1.25;
    margin: 0 0 8px 0;
  }}

  .cover-subtitle {{
    font-size: 13.5pt;
    font-weight: 700;
    color: #0369a1;
    margin: 0 0 12px 0;
  }}

  .cover-desc {{
    font-size: 9.5pt;
    color: #475569;
    line-height: 1.6;
    margin-bottom: 16px;
  }}

  .cover-highlights {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin: 10px 0;
  }}

  .highlight-card {{
    background: #ffffff;
    border: 1px solid #cbd5e1;
    border-left: 4.5px solid #0284c7;
    border-radius: 6px;
    padding: 10px 14px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.03);
  }}

  .highlight-card.green {{
    border-left-color: #059669;
  }}

  .highlight-card.amber {{
    border-left-color: #d97706;
  }}

  .highlight-card.purple {{
    border-left-color: #7c3aed;
  }}

  .highlight-card h4 {{
    margin: 0 0 4px 0;
    font-size: 10pt;
    color: #0f172a;
    display: flex;
    align-items: center;
    gap: 6px;
  }}

  .highlight-card p {{
    margin: 0;
    font-size: 8.5pt;
    color: #64748b;
    line-height: 1.45;
  }}

  .cover-meta {{
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 12px 18px;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
    font-size: 8.5pt;
    margin-top: 10px;
  }}

  .meta-item strong {{
    color: #475569;
    display: inline-block;
    width: 75px;
  }}

  .meta-item span {{
    color: #0f172a;
    font-weight: 600;
  }}

  .cover-footer {{
    text-align: center;
    border-top: 1px solid #cbd5e1;
    padding-top: 10px;
    font-size: 8pt;
    color: #64748b;
    font-weight: 500;
  }}

  /* 內頁通用標頭標尾 */
  .header-bar {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid #e2e8f0;
    padding-bottom: 5px;
    margin-bottom: 12px;
    font-size: 8pt;
    color: #64748b;
  }}

  .header-bar .logo-text {{
    font-weight: 700;
    color: #0284c7;
    display: flex;
    align-items: center;
    gap: 6px;
  }}

  .page-footer-note {{
    margin-top: auto;
    border-top: 1px solid #e2e8f0;
    padding-top: 4px;
    font-size: 7.5pt;
    color: #94a3b8;
    display: flex;
    justify-content: space-between;
  }}

  h2 {{
    font-size: 13pt;
    color: #0f172a;
    border-left: 4.5px solid #0284c7;
    padding-left: 8px;
    margin: 8px 0 8px 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }}

  h3 {{
    font-size: 10.5pt;
    color: #0369a1;
    margin: 10px 0 6px 0;
    padding-bottom: 2px;
    border-bottom: 1px dashed #cbd5e1;
  }}

  p {{
    margin: 4px 0 6px 0;
    color: #334155;
  }}

  /* 提示框 */
  .alert {{
    border-radius: 6px;
    padding: 8px 12px;
    margin: 8px 0;
    font-size: 8.5pt;
    line-height: 1.5;
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }}

  .alert-icon {{
    font-size: 11pt;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 1px;
  }}

  .alert-info {{
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    color: #0369a1;
  }}

  .alert-warning {{
    background: #fffbeb;
    border: 1px solid #fde68a;
    color: #92400e;
  }}

  .alert-danger {{
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #991b1b;
  }}

  .alert-success {{
    background: #ecfdf5;
    border: 1px solid #a7f3d0;
    color: #065f46;
  }}

  /* 步驟卡片 */
  .step-box {{
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 6px 10px;
    margin-bottom: 6px;
    box-shadow: 0 1px 2px rgba(0,0,0,0.02);
  }}

  .step-header {{
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 3px;
  }}

  .step-number {{
    background: #0284c7;
    color: #ffffff;
    font-weight: 700;
    font-size: 8pt;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }}

  .step-number.green {{
    background: #059669;
  }}

  .step-number.red {{
    background: #dc2626;
  }}

  .step-number.purple {{
    background: #7c3aed;
  }}

  .step-title {{
    font-size: 9.5pt;
    font-weight: 700;
    color: #0f172a;
  }}

  /* 表格樣式 */
  table {{
    width: 100%;
    border-collapse: collapse;
    margin: 4px 0 6px 0;
    font-size: 8pt;
  }}

  th, td {{
    padding: 3.5px 7px;
    text-align: left;
    border: 1px solid #cbd5e1;
    line-height: 1.35;
  }}

  th {{
    background: #f1f5f9;
    color: #1e293b;
    font-weight: 700;
  }}

  tr:nth-child(even) td {{
    background: #f8fafc;
  }}

  .badge {{
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 7.5pt;
    font-weight: 700;
  }}

  .badge-ok {{
    background: #dcfce7;
    color: #15803d;
    border: 1px solid #bbf7d0;
  }}

  .badge-req {{
    background: #fee2e2;
    color: #b91c1c;
    border: 1px solid #fecaca;
  }}

  .badge-opt {{
    background: #f1f5f9;
    color: #475569;
    border: 1px solid #cbd5e1;
  }}

  /* 截圖展示樣式 */
  .screenshot-card {{
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 6px;
    margin: 8px 0;
    text-align: center;
  }}

  .screenshot-card img {{
    max-width: 98%;
    max-height: 250px;
    height: auto;
    border-radius: 4px;
    border: 1px solid #94a3b8;
    box-shadow: 0 2px 4px rgba(0,0,0,0.08);
  }}

  .screenshot-caption {{
    margin-top: 4px;
    font-size: 8pt;
    font-weight: 600;
    color: #475569;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }}

  /* 檢核清單 Checklist 表格 */
  .chk-table td {{
    padding: 6px 8px;
    vertical-align: middle;
  }}
  .chk-box {{
    width: 14px;
    height: 14px;
    border: 1.5px solid #64748b;
    border-radius: 3px;
    display: inline-block;
    margin-right: 4px;
  }}
</style>
</head>
<body>

<!-- ==========================================
     PAGE 1: 封面 (Cover Page)
     ========================================== -->
<div class="page">
  <div class="cover-page">
    <div>
      <span class="cover-badge">桃園市政府地政局 智慧監控維運體系</span>
      <h1 class="cover-title">桃園市各地政事務所<br>智慧控管與監控系統 (LAH)</h1>
      <div class="cover-subtitle">伺服器端安裝與版本升級標準操作手冊 (SOP)</div>
      <div class="cover-desc">
        本手冊專為各地政事務所資訊系統管理師編寫，詳述智慧控管伺服器端於「原機版本升級 (In-place Upgrade)」與「全新伺服器安裝 (Clean Install)」兩種實務情境之標準作業程序。內容完整涵蓋事前 VM 快照防禦、關鍵 SQLite 資料庫封裝備份、Node/PM2 零全域污染沙盒隔離架構（自動附帶專屬維運終端 lah-cmd.bat）、Apache 與 PHP 組態自癒清理、五大服務連接埠自動巡檢及異常雙重還原防線。
      </div>
    </div>

    <div class="cover-highlights">
      <div class="highlight-card">
        <h4>⚡ 情境一：原機版本升級</h4>
        <p>針對現正穩定運行之主機快速完成補丁更新、漏洞修補與 PHP 8.5 雙軌佈署，保留現有 Apache/PHP 7 核心環境。</p>
      </div>
      <div class="highlight-card green">
        <h4>🛡️ 情境二：全新伺服器安裝</h4>
        <p>乾淨虛擬機建置或原機環境重灌，搭配「自動移除工具」徹底清除殘留，並全選更新包與一鍵掛載還原歷史資料庫。</p>
      </div>
      <div class="highlight-card amber">
        <h4>🔒 關鍵資料防護備份</h4>
        <p>完整備份即時通全科室頻道 SQLite 資料庫、Oracle 9i/11g OCI8 驅動與 API 連線組態，確保升級零資料遺失風險。</p>
      </div>
      <div class="highlight-card purple">
        <h4>🩺 五大連接埠自我巡檢</h4>
        <p>內建 Port 80、8085、8080、8081、8082 守聽巡檢表與 VM 快照回滾指引，提供全方位驗證與上線復原保障。</p>
      </div>
    </div>

    <div>
      <div class="cover-meta">
        <div class="meta-item"><strong>文件編號：</strong><span>LAH-OPS-SOP-202610</span></div>
        <div class="meta-item"><strong>發布日期：</strong><span>2026 年 10 月 31 日 (管理儀表板增補修訂版)</span></div>
        <div class="meta-item"><strong>適用系統：</strong><span>智慧控管與監控系統 (LAH)</span></div>
        <div class="meta-item"><strong>維運工具：</strong><span>LAH-Updater v1.7.0+ (Rust 核心，含全功能管理儀表板)</span></div>
        <div class="meta-item"><strong>適用單位：</strong><span>桃園市各地政事務所 資訊小組</span></div>
        <div class="meta-item"><strong>保密等級：</strong><span>內部維運限閱 (內部專用)</span></div>
      </div>
    </div>

    <div class="cover-footer">
      桃園市各地政事務所 智慧控管與監控系統維運團隊 版權所有 © 2026
    </div>
  </div>
</div>

<!-- ==========================================
     PAGE 2: 發布包清單與三大先決環境準備
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 1：發布包說明與環境先決條件</span>
  </div>

  <h2>📦 1. 更新包完整內容說明</h2>
  <p>本更新發布包存放於 <code>LAH-Updater\\dist\\</code> 目錄，全工具採用 <strong>Tauri 2 (Rust)</strong> 核心開發，體積僅 10~11MB，啟動隨點秒開並大幅降低防毒軟體誤判。目錄各組件職責如下：</p>

  <table>
    <thead>
      <tr>
        <th style="width: 34%;">檔案 / 資料夾名稱</th>
        <th style="width: 16%;">角色屬性</th>
        <th>功能職責與使用時機說明</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><strong>智慧控管與監控系統 - 自動更新工具.exe</strong></td>
        <td><span class="badge badge-req">主安裝引擎</span></td>
        <td>負責執行更新解壓、環境設定、開機服務註冊、資安注入防護及服務啟動。</td>
      </tr>
      <tr>
        <td><strong>智慧控管與監控系統 - 備份與還原工具.exe</strong></td>
        <td><span class="badge badge-req">安全備份</span></td>
        <td>升級前必備！自動掃描並封裝 SQLite DB、API 組態、Oracle OCI8 驅動為 ZIP 檔。</td>
      </tr>
      <tr>
        <td><strong>智慧控管與監控系統 - 自動移除工具.exe</strong></td>
        <td><span class="badge badge-opt">清理工具</span></td>
        <td>原機徹底重裝時使用。安全停止 Windows 服務、終止佔用行程並乾淨移除目錄。</td>
      </tr>
      <tr>
        <td><strong>智慧控管與監控系統 - 可攜 Node 安裝工具.exe</strong></td>
        <td><span class="badge badge-opt">專屬環境</span></td>
        <td>獨立部署或修復綠色版 Node 16 與 PM2 守護服務，保證零全域污染。</td>
      </tr>
      <tr>
        <td><strong>智慧控管與監控系統 - 管理儀表板.exe</strong></td>
        <td><span class="badge badge-req">專屬維運</span></td>
        <td>專屬單一執行檔原生儀表板！更新完成自動同步至根目錄並建立桌面捷徑；整合服務監控控制、Auto-Tail 動態日誌、資源效能即時折線圖與服務記憶體排名、核心組態編輯器（含語法檢驗、自動備份與範本比對），以及 xterm.js 專業沙盒終端（含 UTF-8/CP950 編碼切換）。</td>
      </tr>
      <tr>
        <td><strong>地籍異動簡訊參數設定工具.exe</strong></td>
        <td><span class="badge badge-opt">維運工具</span></td>
        <td>獨立免安裝簡訊設定工具 (SMSWEB_MOI)，支援 BASE64 編解碼與自動備份。</td>
      </tr>
      <tr>
        <td><strong>MicrosoftEdgeWebView2RuntimeInstallerX64.exe</strong></td>
        <td><span class="badge badge-opt">系統依賴</span></td>
        <td>微軟 WebView2 離線安裝包。若在純淨伺服器點擊工具無反應時請優先安裝。</td>
      </tr>
      <tr>
        <td><code>base\\</code> (基礎環境包)</td>
        <td>基礎組件</td>
        <td>內含 Apache 2.4、PHP 7.4、PHP 8.5 FastCGI 及綠色可攜版 Node.js 執行期環境。</td>
      </tr>
      <tr>
        <td><code>node_modules\\</code> (核心套件包)</td>
        <td>套件相依</td>
        <td>內含前端 (LAH-NUXTJS) 與即時通伺服器 (lah-messenger-server) 預編譯套件。</td>
      </tr>
      <tr>
        <td><code>patch\\</code> (程式補丁包)</td>
        <td>業務程式</td>
        <td>收納 LAH-API (後端)、LAH-NUXTJS (前端)、即時通服務端及 www 站台更新檔。</td>
      </tr>
    </tbody>
  </table>

  <h2>🛡️ 2. 操作前三大先決環境準備 (Pre-requisites)</h2>
  <p>在執行任何更新或安裝作業前，資訊管理師請務必依序完成下列三大安全準備：</p>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number">1</span>
      <span class="step-title">建立虛擬機快照 (VM Snapshot) —— 第一道安全防線</span>
    </div>
    <p>智慧控管伺服器若建置於 VMware ESXi、Hyper-V 或 Proxmox 平台，在作業前請務必於虛擬化後台拍攝快照（建議命名：<code>Pre_LAH_Update_YYYYMMDD</code>）。若升級發生非預期衝突，可在 1 分鐘內極速回滾，保障為民服務不中斷。</p>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number">2</span>
      <span class="step-title">執行階段檢查 (WebView2 Runtime)</span>
    </div>
    <p>工具界面依賴微軟 Edge WebView2 執行階段。若您的主機為封閉虛擬機或點擊 <code>.exe</code> 出現閃退無反應，請先執行目錄內的 <strong><code>MicrosoftEdgeWebView2RuntimeInstallerX64.exe</code></strong> 進行離線安裝，安裝完成後即可順暢啟動。</p>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number">3</span>
      <span class="step-title">Node 與 PM2 零全域污染沙盒隔離架構 (免移除全域 Node)</span>
    </div>
    <p>最新架構已全面實作沙盒隔離機制。工具不會更動系統登錄檔 <code>Path</code> 與 <code>PM2_HOME</code>，<strong>無須移除本機全域已安裝的 Node.js</strong>（如 Node 20/22）。所有維運工具皆支援多磁碟路徑動態偵測，更新完成後自動於安裝根目錄產出 <strong><code>lah-cmd.bat</code></strong> 專屬維運終端，關閉視窗即自動釋放環境變數。</p>
  </div>
  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 3: 重要關鍵設定與 SQLite DB 備份指引
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 2：重要關鍵設定與 SQLite DB 備份</span>
  </div>

  <h2>💾 3. 關鍵設定與 SQLite 資料庫備份指引</h2>
  <p>智慧控管系統的歷史即時通話、公告看板、同仁設定與在地連線組態均儲存於本機 SQLite 資料庫及組態檔中。在進行<strong>版本升級</strong>或<strong>全新安裝</strong>前，<strong>必須優先執行備份作業</strong>！</p>

  <div class="step-box">
    <div class="step-header">
      <span class="step-number green">SOP</span>
      <span class="step-title">使用「備份與還原工具」產出標準備份封裝檔</span>
    </div>
    <ol style="margin: 3px 0 0 16px; padding: 0; line-height: 1.6;">
      <li>以系統管理員身分執行 <strong><code>智慧控管與監控系統 - 備份與還原工具.exe</code></strong>。</li>
      <li>確認「系統安裝根目錄」路徑（工具已內建動態偵測，亦可手動點擊瀏覽選取）。</li>
      <li>設定「備份輸出目錄」（請選擇具足夠容量磁區，例如 <code>D:\\DEV</code> 或專用備份槽）。</li>
      <li>點擊 <strong>「📦 立即備份重要資料」</strong> 按鈕，工具將自動高速掃描並打包重要資產。</li>
      <li>完成後將產出如 <strong><code>D:\\DEV\\_LAH_Backup_20261028_091435.zip</code></strong> 之完整備份封裝檔。</li>
    </ol>
  </div>

  <div class="screenshot-card avoid-break">
    <img src="{img_backup}" alt="備份工具圖解">
    <div class="screenshot-caption">
      📷 圖 1：智慧控管與監控系統 - 備份與還原工具操作介面 (備份產出範例)
    </div>
  </div>

  <h3>🔍 實務解析：備份檔 (如 _LAH_Backup_20261028_091435.zip) 內含重要資產清單</h3>
  <p>本系統備份工具已全面擴充保護機制，備份檔內完整收納以下 7 大關鍵系統資產：</p>
  <table>
    <thead>
      <tr>
        <th style="width: 36%;">資產目錄 / 檔案路徑</th>
        <th>資產類型與重要維運價值說明</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><code>lah-messenger-server/db/*.db</code></td>
        <td><strong>即時通 SQLite 資料庫：</strong>收納全所各科室（行政、登記、測量、地價、資訊等）頻道與同仁私訊歷史紀錄、公告看板庫，不可遺失。</td>
      </tr>
      <tr>
        <td><code>www/.env</code> & <code>www/config.php</code></td>
        <td><strong>後端 API 組態檔：</strong>包含系統通訊金鑰、本所代碼、Oracle/AD 連線組態與各項排程快取。</td>
      </tr>
      <tr>
        <td><code>instantclient_11_2/</code></td>
        <td><strong>Oracle 9i/11g 用戶端套件：</strong>保障地政核心業務資料庫 OCI8 驅動連線環境。</td>
      </tr>
      <tr>
        <td><code>php7/ext/php_oci8*.dll</code></td>
        <td><strong>PHP 核心擴充庫：</strong>OCI8 與 MSSQL 專用連線延伸模組 DLL。</td>
      </tr>
      <tr>
        <td><code>Apache24/conf/</code></td>
        <td><strong>網頁伺服器設定：</strong>包含 Apache 主設定檔 <code>httpd.conf</code> 與各所專用憑證。</td>
      </tr>
      <tr>
        <td><code>LAH-NUXTJS/</code></td>
        <td><strong>前端系統環境：</strong>智慧管控前端網頁站台之在地化環境參數與客製化設定。</td>
      </tr>
      <tr>
        <td><code>lah-cmd.bat</code></td>
        <td><strong>專屬維運終端腳本：</strong>保留沙盒環境變數設定，還原後可立即啟動維運終端。</td>
      </tr>
    </tbody>
  </table>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 4: 情境一：原機版本升級 (In-Place Upgrade)
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 3：情境一：原機版本升級</span>
  </div>

  <h2>🚀 4. 情境一：原機版本升級操作步驟 (In-Place Upgrade)</h2>
  <p><strong>適用時機：</strong>現有主機已正常運行舊版智慧控管系統，本次目的為升級前端、補丁後端漏洞，並<strong>無縫引入 PHP 8.5 雙軌架構</strong>，保留現有機台原有 Apache 與 PHP 7 基礎環境。</p>

  <div class="screenshot-card avoid-break">
    <img src="{img_upgrade}" alt="升級更新工作選項勾選圖解">
    <div class="screenshot-caption">
      📷 圖 2：原機升級更新 - 自動更新工具工作選項勾選圖解
    </div>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number green">1</span>
      <span class="step-title">步驟一：啟動自動更新工具</span>
    </div>
    <p>以系統管理員身分執行 <strong><code>智慧控管與監控系統 - 自動更新工具.exe</code></strong>。工具將自動偵測安裝路徑（支援 C 槽與非 C 槽動態探測）與各項服務即時狀態。</p>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number green">2</span>
      <span class="step-title">步驟二：精準調整工作選項勾選（關鍵操作對齊圖 2）</span>
    </div>
    <p>為避免覆蓋現有正常運作之 Apache 與 PHP 7 設定，可善用中央快選工具列點選「程式修補 (patch)」或依序調整：</p>
    <ul>
      <li><strong style="color: #dc2626;">【取消勾選】</strong> <strong>APACHE 網頁伺服器核心安裝包</strong>（保持主機原 Apache 穩定運行）。</li>
      <li><strong style="color: #dc2626;">【取消勾選】</strong> <strong>PHP 7.4 執行環境安裝包</strong>（保留既有機台 PHP 7 設定與 OCI8 驅動）。</li>
      <li><strong style="color: #dc2626;">【取消勾選】</strong> <strong>核心相依套件模組 (node_modules)</strong>（既有套件完整時無需重複解壓）。</li>
      <li><strong style="color: #059669;">【保持勾選】</strong> <strong>可攜式 Node.js 執行環境</strong>（歸納於基礎包）與 <strong>PHP 8.5 FastCGI 核心環境</strong>。</li>
      <li><strong style="color: #059669;">【保持勾選】</strong> <strong>所有程式更新補丁包</strong>（<code>LAH-API.zip</code>、<code>lah-messenger-server.zip</code>、<code>LAH-NUXTJS.zip</code>、<code>www.zip</code>）。</li>
      <li><strong style="color: #059669;">【保持勾選】</strong> <strong>自動配置系統服務、資安強化配置、更新後自動重啟 Apache 與 PM2</strong>。</li>
    </ul>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number green">3</span>
      <span class="step-title">步驟三：點擊「開始執行更新作業」等待結束</span>
    </div>
    <p>點擊右下角 <strong>「🚀 開始執行更新作業」</strong>。安裝管線將依序解壓程式碼、注入 PHP2Shell 防護規則、啟動 PHP 8.5 FastCGI 並重啟服務，最後彈現健康巡檢報告即完成升級！</p>
    <p style="margin-top:2px; font-size:8pt; color:#0369a1;">💡 <strong>管理儀表板熱部署與捷徑：</strong>更新完成自動將管理儀表板 EXE 同步至安裝根目錄並建立桌面捷徑，彈窗提供「📊 開啟管理儀表板」按鈕供即刻喚醒確認。</p>
  </div>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 5: 情境二 (上篇)：原機清理與自動移除工具
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 4：情境二：全新安裝 / 原機重灌 (上篇)</span>
  </div>

  <h2>🧹 5. 情境二：全新安裝 / 原機重灌 —— (A) 原機清理作業</h2>
  <p><strong>適用時機：</strong>若欲在原有主機徹底重整混亂環境，或建置新主機。（若是全新建立的乾淨 VM 虛擬機，可直接略過清理步驟，跳至下一頁安裝）。</p>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number red">1</span>
      <span class="step-title">步驟一：原機清理前請再次確認已完成備份 (極重要！)</span>
    </div>
    <p>在執行移除前，請務必確認已依第 3 頁步驟利用「備份與還原工具」產出備份 ZIP 檔（如 <code>D:\\DEV\\_LAH_Backup_20261028_091435.zip</code>），並將備份檔移置安全磁區。</p>
  </div>

  <div class="screenshot-card avoid-break">
    <img src="{img_uninstall}" alt="移除工具圖解">
    <div class="screenshot-caption">
      📷 圖 3：智慧控管與監控系統 - 自動移除工具 (安全乾淨卸載舊有環境)
    </div>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number red">2</span>
      <span class="step-title">步驟二：使用「自動移除工具」徹底清理原機環境</span>
    </div>
    <p>以系統管理員身分開啟 <strong><code>智慧控管與監控系統 - 自動移除工具.exe</code></strong>：</p>
    <ol style="margin: 3px 0 0 16px; padding: 0; line-height: 1.6;">
      <li>確認系統安裝根目錄路徑（工具支援多磁碟動態偵測，亦可手動瀏覽變更）。</li>
      <li>保持預設<strong>「全選」</strong>所有清理項（安全卸載 Windows 服務、終止所有進程含 php-cgi、徹底清除登錄檔殘留與清空目錄）。</li>
      <li>點擊 <strong>「🚨 開始執行移除作業」</strong> 按鈕。</li>
      <li>於彈出之確認對話框中點選 <strong>「確認並立即執行移除」</strong>，等待進度完成並關閉工具。</li>
    </ol>
  </div>

  <div class="alert alert-danger avoid-break">
    <div class="alert-icon">🚨</div>
    <div>
      <strong>資安防呆叮嚀：</strong>自動移除工具將安全停止 Web 入口服務與背景服務總線 (LAH_Web_Service / LAH_PM2_Service) 並釋放佔用之連接埠。執行前請再三確認備份檔已妥善存放於非 <code>C:\\AppServ</code> 之目錄！
    </div>
  </div>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 6: 情境二 (下篇)：全新安裝與掛載還原
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 4：情境二：全新安裝 / 原機重灌 (下篇)</span>
  </div>

  <h2>✨ 6. 情境二：全新安裝 / 原機重灌 —— (B) 全量安裝與掛載還原</h2>
  <p><strong>適用時機：</strong>完成原機清理後，或是全新建立的乾淨 Windows Server 虛擬機，進行全系統軟體組件佈署並將歷史資料庫與設定檔自動還原歸位。</p>

  <div class="screenshot-card avoid-break">
    <img src="{img_clean_install}" alt="全新安裝更新工作選項勾選圖解">
    <div class="screenshot-caption">
      📷 圖 4：全新安裝 - 自動更新工具全選並選取備份還原檔操作圖解
    </div>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number red">3</span>
      <span class="step-title">步驟三：開啟更新工具，執行「全選 + 掛載備份還原」</span>
    </div>
    <p>以系統管理員身分執行 <strong><code>智慧控管與監控系統 - 自動更新工具.exe</code></strong>：</p>
    <ul>
      <li>點擊 <strong>「全選」</strong> 按鈕，確保基礎環境包（Apache、PHP7、PHP85、Node）、核心套件包（node_modules）與程式更新補丁包<strong>全數打勾</strong>。</li>
      <li><strong style="color: #0369a1;">【全新安裝核心關鍵】</strong> 勾選 <strong>「從備份檔案還原資料庫與系統設定」</strong>。</li>
      <li>點擊其右側 <strong>「瀏覽...」</strong> 按鈕，選取步驟一所備份的 ZIP 檔（例如 <code>D:\\DEV\\_LAH_Backup_20261028_091435.zip</code>）。</li>
      <li>點擊右下角 <strong>「🚀 開始執行更新作業」</strong>。</li>
    </ul>
  </div>

  <div class="step-box avoid-break">
    <div class="step-header">
      <span class="step-number green">4</span>
      <span class="step-title">步驟四：等待管線全自動佈署完成</span>
    </div>
    <p>更新引擎將自動完成：① 解壓 Apache 與 PHP 7/8.5 ➔ ② 解壓前端與即時通依賴庫 ➔ ③ 釋放最新業務程式碼 ➔ ④ 掛載還原備份之 SQLite 資料庫與 Oracle 驅動 ➔ ⑤ 註冊 Windows 原生服務、配置開機自啟並產出 lah-cmd.bat 終端 ➔ ⑥ 自動同步部署管理儀表板至安裝根目錄並建立桌面捷徑 ➔ ⑦ 全系統服務啟動與健康巡檢。</p>
  </div>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 7: 系統完成驗證與健康巡檢標準
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>章節 5：系統驗證與雙重回滾機制</span>
  </div>

  <h2>🩺 7. 系統完成驗證與健康巡檢標準</h2>
  <p>更新管線結束後會自動彈現「連接埠健康巡檢報告」。管理師亦可開啟 CMD / PowerShell 執行 <code>netstat -ano</code>，確認以下 <strong>五大關鍵服務連接埠皆為 [OK] 運行中</strong>：</p>

  <table>
    <thead>
      <tr>
        <th style="width: 13%;">服務埠號</th>
        <th style="width: 22%;">服務名稱</th>
        <th style="width: 20%;">守護進程 / 架構</th>
        <th>驗證判定合格標準</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><strong>Port 80</strong></td>
        <td>Apache 網頁伺服器</td>
        <td>WinSW 沙盒守護 (LAH_Web_Service)</td>
        <td><span class="badge badge-ok">[OK] LISTENING</span> 承載舊版網頁與 PHP 7.4 業務 API 轉發。</td>
      </tr>
      <tr>
        <td><strong>Port 8085</strong></td>
        <td>PHP 8.5 FastCGI 核心</td>
        <td>PM2 守護 (php-cgi)</td>
        <td><span class="badge badge-ok">[OK] LISTENING</span> 雙軌高速後端核心，專責重度報表、批次統計運算。</td>
      </tr>
      <tr>
        <td><strong>Port 8080</strong></td>
        <td>LAH-FE 智慧管控前端</td>
        <td>PM2 守護 (Nuxt 2)</td>
        <td><span class="badge badge-ok">[OK] LISTENING</span> 智慧監控管理儀表板，提供地政即時通訊功能。</td>
      </tr>
      <tr>
        <td><strong>Port 8081</strong></td>
        <td>即時通 WebSocket 埠</td>
        <td>PM2 守護 (server.js)</td>
        <td><span class="badge badge-ok">[OK] LISTENING</span> 提供全所同仁高併發雙向即時通訊與推播服務。</td>
      </tr>
      <tr>
        <td><strong>Port 8082</strong></td>
        <td>即時通 HTTP 附件埠</td>
        <td>PM2 守護 (Express)</td>
        <td><span class="badge badge-ok">[OK] LISTENING</span> 處理即時通多媒體檔案、截圖與圖片之上傳與下載。</td>
      </tr>
    </tbody>
  </table>

  <h3>🖥️ 一站式維運管理儀表板與瀏覽器實機驗證</h3>
  <ol style="margin: 2px 0 4px 16px; padding: 0; line-height: 1.5; font-size: 8.5pt;">
    <li><strong>開啟管理儀表板：</strong>點擊「📊 開啟管理儀表板」，確認服務與 PM2 進程綠燈、五大埠健康連線；儀表板已整合資源效能折線圖與記憶體排名、核心組態編輯器（含語法檢驗與範本比對），以及 xterm.js 專業沙盒終端（含 UTF-8/CP950 編碼切換）。</li>
    <li><strong>前端首頁載入：</strong>開啟瀏覽器輸入 <code>http://127.0.0.1:8080</code>，確認管控首頁正常載入並能查詢地政統計數據。</li>
    <li><strong>即時通訊狀態：</strong>檢視右側即時通看板，確認連線狀態圖示為<strong>綠燈（連線中）</strong>且歷史公告完整。</li>
  </ol>

  <div class="alert alert-success avoid-break" style="margin: 6px 0 8px 0;">
    <div class="alert-icon">💻</div>
    <div>
      <strong>專屬沙盒維運終端 (lah-cmd.bat) 與端點相容：</strong><br>
      若需檢視或重啟 PM2 守護行程，雙擊執行安裝根目錄 <code>lah-cmd.bat</code>（如 <code>C:\\AppServ\\lah-cmd.bat</code>），即可在隔離環境執行 <code>pm2 list</code>、<code>pm2 restart all</code> 或 <code>pm2 logs</code>，無需手動配置系統環境變數。此外，Apache 已注入串流端點向後相容 FastCGI 別名映射，前端圖片與報表輸出皆自動對齊雙軌核心。
    </div>
  </div>

  <h2>🔄 8. 異常排除與雙重防護回滾指引 (Rollback SOP)</h2>
  <div class="alert alert-info avoid-break">
    <div class="alert-icon">🛡️</div>
    <div>
      <strong>第一道防線：使用「備份與還原工具」一鍵還原資料庫與組態</strong><br>
      若更新後發現 SQLite 資料遺漏或組態有誤，啟動 <code>智慧控管與監控系統 - 備份與還原工具.exe</code>，於下方選取備份檔（如 <code>_LAH_Backup_20261028_091435.zip</code>），點擊「🚨 還原到安裝根目錄」即可安全歸位。
    </div>
  </div>

  <div class="alert alert-danger avoid-break">
    <div class="alert-icon">⚡</div>
    <div>
      <strong>第二道防線：虛擬機快照 (VM Snapshot) 終極秒級回滾</strong><br>
      若遇作業系統嚴重異常或服務損毀無法啟動，直接登入 VMware / Hyper-V 後台，選取更新前建立的 Snapshot（<code>Pre_LAH_Update_202610</code>）並執行回滾，系統於 1 分鐘內精準復原至作業前狀態。
    </div>
  </div>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

<!-- ==========================================
     PAGE 8: 附錄：資訊管理師升級檢核清單 (Checklist)
     ========================================== -->
<div class="page">
  <div class="header-bar">
    <span class="logo-text">🏛️ 桃園市各地政事務所 智慧控管與監控系統 (LAH)</span>
    <span>附錄：管理師升級檢核清單 (Checklist)</span>
  </div>

  <h2>📋 附錄：智慧控管系統升級更新檢核表 (Checklist)</h2>
  <p>本檢核表供各地政事務所資訊管理師於現場升級作業時列印並逐項核對簽署存查：</p>

  <table class="chk-table">
    <thead>
      <tr>
        <th style="width: 14%;">作業階段</th>
        <th style="width: 60%;">具體檢核作業項目</th>
        <th style="width: 14%;">執行狀態</th>
        <th style="width: 12%;">確認簽核</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td rowspan="3" style="font-weight:700; background:#f0f9ff; text-align:center;">前置準備</td>
        <td><span class="chk-box"></span> <strong>VM 快照建立：</strong>已於虛擬化平台拍攝快照 (Snapshot) 並確認建立成功。</td>
        <td>□ 符合 □ 不適用</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>WebView2 檢查：</strong>已確認工具可正常開啟（若閃退已執行離線安裝包）。</td>
        <td>□ 符合 □ 已安裝</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>Node/PM2 沙盒確認：</strong>確認採用零全域污染架構，無須卸載全域 Node，且根目錄已自動產出 <code>lah-cmd.bat</code>。</td>
        <td>□ 符合 □ 已確認</td>
        <td></td>
      </tr>
      <tr>
        <td rowspan="2" style="font-weight:700; background:#fefce8; text-align:center;">資料備份</td>
        <td><span class="chk-box"></span> <strong>重要資料打包：</strong>已執行「備份工具」產出標準備份檔 (如 _LAH_Backup_*.zip)。</td>
        <td>□ 符合 □ 新建VM</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>備份安全保存：</strong>備份 ZIP 檔已妥善存放於非 <code>C:\\AppServ</code> 之獨立磁區或外接碟。</td>
        <td>□ 符合 □ 新建VM</td>
        <td></td>
      </tr>
      <tr>
        <td rowspan="2" style="font-weight:700; background:#ecfdf5; text-align:center;">執行更新</td>
        <td><span class="chk-box"></span> <strong>情境一 (原機升級)：</strong>已<strong>取消勾選</strong> Apache、PHP7 及 node_modules（可善用集中快選工具列）。</td>
        <td>□ 符合 □ 走情境二</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>情境二 (全新安裝)：</strong>原機已徹底清理進程與服務，更新工具<strong>全選並選取備份 ZIP</strong>。</td>
        <td>□ 符合 □ 走情境一</td>
        <td></td>
      </tr>
      <tr>
        <td rowspan="4" style="font-weight:700; background:#fdf2f8; text-align:center;">驗證上線</td>
        <td><span class="chk-box"></span> <strong>連接埠巡檢：</strong>Port 80, 8085, 8080, 8081, 8082 經檢測全數為 [OK]。</td>
        <td>□ 全數合格</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>管理儀表板：</strong>確認桌面已建立捷徑，開啟確認 Apache 與 PM2 各服務進程全數綠燈運行。</td>
        <td>□ 測試正常</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>前端網站測試：</strong>瀏覽器開啟 <code>http://127.0.0.1:8080</code> 首頁儀表板渲染正常。</td>
        <td>□ 測試正常</td>
        <td></td>
      </tr>
      <tr>
        <td><span class="chk-box"></span> <strong>即時通訊驗證：</strong>右側即時通狀態燈為綠燈連線中，歷史公告資料完整。</td>
        <td>□ 測試正常</td>
        <td></td>
      </tr>
    </tbody>
  </table>

  <div style="margin-top: 14px; background: #ffffff; border: 1px solid #cbd5e1; border-radius: 6px; padding: 12px 16px;">
    <div style="font-weight: 700; color: #0f172a; margin-bottom: 8px;">維運簽核確認欄</div>
    <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; font-size: 8.5pt;">
      <div>執行事務所：____________________</div>
      <div>執行工程師：____________________</div>
      <div>作業日期時間：____年____月____日</div>
      <div>主機伺服器 IP：________________</div>
      <div>備份檔案名稱：________________</div>
      <div>複核主管簽名：________________</div>
    </div>
  </div>

  <div class="page-footer-note">
    <span>桃園市各地政事務所 智慧控管與監控系統維運手冊</span>
    <span>機密等級：內部維運限閱</span>
  </div>
</div>

</body>
</html>
"""

def generate_pdf():
    print(f"[1/3] 寫入 HTML 手冊檔案: {HTML_OUTPUT}")
    with open(HTML_OUTPUT, "w", encoding="utf-8") as f:
        f.write(HTML_CONTENT)
    print(f"      HTML 檔案大小: {os.path.getsize(HTML_OUTPUT):,} 位元組")

    chrome_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ]
    browser_exe = None
    for p in chrome_paths:
        if os.path.exists(p):
            browser_exe = p
            break

    if not browser_exe:
        print("[錯誤] 找不到 Chrome 或 Edge 執行檔，無法產出 PDF！")
        return False

    print(f"[2/3] 使用無頭瀏覽器產出 PDF: {browser_exe}")
    html_url = "file:///" + HTML_OUTPUT.replace("\\", "/")
    
    cmd = [
        browser_exe,
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        f"--print-to-pdf={PDF_OUTPUT}",
        html_url
    ]
    
    # 執行無頭瀏覽器列印，設定 utf-8 解碼以防 Windows 主機 CP950 解碼報錯
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    
    if os.path.exists(PDF_OUTPUT) and os.path.getsize(PDF_OUTPUT) > 1000:
        print(f"[3/3] 成功產出 PDF 安裝手冊！")
        print(f"      檔案路徑: {PDF_OUTPUT}")
        print(f"      檔案大小: {os.path.getsize(PDF_OUTPUT):,} 位元組")
        return True
    else:
        err_msg = res.stderr.decode('utf-8', errors='ignore')
        print(f"[失敗] PDF 產出異常，stderr: {err_msg}")
        return False

if __name__ == "__main__":
    success = generate_pdf()
    sys.exit(0 if success else 1)
