# -*- coding: utf-8 -*-
"""
LAH-Updater & LAH-Workspace 全系統架構技術手冊產生器
產出純 HTML/CSS/JS (零外網 CDN，離線可用) 之視覺化技術手冊。
"""

import os
import sys

OUTPUT_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "README.html")

# 定義 4 大內嵌向量 SVG 圖表

SVG_TOPOLOGY = """
<svg viewBox="0 0 960 540" xmlns="http://www.w3.org/2000/svg" class="architecture-svg">
  <defs>
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
    <linearGradient id="clientGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e3a8a" />
      <stop offset="100%" stop-color="#2563eb" />
    </linearGradient>
    <linearGradient id="serviceGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#065f46" />
      <stop offset="100%" stop-color="#059669" />
    </linearGradient>
    <linearGradient id="dbGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#7c2d12" />
      <stop offset="100%" stop-color="#d97706" />
    </linearGradient>
    <linearGradient id="opsGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#4c1d95" />
      <stop offset="100%" stop-color="#7c3aed" />
    </linearGradient>
    <filter id="shadow" x="-5%" y="-5%" width="110%" height="110%">
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.4" />
    </filter>
    <marker id="arrow" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#60a5fa" />
    </marker>
    <marker id="arrowGreen" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#34d399" />
    </marker>
    <marker id="arrowAmber" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#fbbf24" />
    </marker>
  </defs>

  <!-- 外框背景 -->
  <rect width="960" height="540" rx="16" fill="url(#bgGrad)" stroke="#334155" stroke-width="1.5" />

  <!-- 標題 -->
  <text x="480" y="38" text-anchor="middle" fill="#f8fafc" font-size="20" font-weight="700" letter-spacing="1">
    LAH 智慧控管與監控系統 全域架構拓撲圖
  </text>
  <text x="480" y="60" text-anchor="middle" fill="#94a3b8" font-size="12">
    工作區 5 大平級 Git 子專案與服務層級互動契約
  </text>

  <!-- 第 1 層：客戶端層 (Client Layer) -->
  <rect x="40" y="85" width="880" height="110" rx="12" fill="#1e293b" fill-opacity="0.6" stroke="#3b82f6" stroke-dasharray="4 4" stroke-width="1.2" />
  <text x="55" y="108" fill="#93c5fd" font-size="13" font-weight="700">【客戶端層 Client Layer】</text>

  <!-- LAH-FE 節點 -->
  <g filter="url(#shadow)">
    <rect x="70" y="120" width="370" height="62" rx="8" fill="url(#clientGrad)" stroke="#60a5fa" stroke-width="1" />
    <text x="85" y="144" fill="#ffffff" font-size="15" font-weight="700">LAH-FE 智慧管控前端</text>
    <text x="85" y="166" fill="#bfdbfe" font-size="11">Nuxt 2 (Vue 2) / BootstrapVue / 內建 lah-messenger-*.vue 元件群 (Port 8080)</text>
  </g>

  <!-- lah-messenger 桌面端節點 -->
  <g filter="url(#shadow)">
    <rect x="520" y="120" width="370" height="62" rx="8" fill="url(#clientGrad)" stroke="#60a5fa" stroke-width="1" />
    <text x="535" y="144" fill="#ffffff" font-size="15" font-weight="700">lah-messenger 即時通桌面客戶端</text>
    <text x="535" y="166" fill="#bfdbfe" font-size="11">Electron 39 / Nuxtron / Vue 2 / 系統匣常駐與本機通知</text>
  </g>

  <!-- 第 2 層：服務中介層 (Service Layer) -->
  <rect x="40" y="225" width="880" height="110" rx="12" fill="#1e293b" fill-opacity="0.6" stroke="#10b981" stroke-dasharray="4 4" stroke-width="1.2" />
  <text x="55" y="248" fill="#6ee7b7" font-size="13" font-weight="700">【服務中介層 Service Layer】</text>

  <!-- LAH-API 節點 -->
  <g filter="url(#shadow)">
    <rect x="70" y="260" width="370" height="62" rx="8" fill="url(#serviceGrad)" stroke="#34d399" stroke-width="1" />
    <text x="85" y="284" fill="#ffffff" font-size="15" font-weight="700">LAH-API 業務核心 REST API</text>
    <text x="85" y="306" fill="#a7f3d0" font-size="11">PHP 7.3 / Apache 2.4 (Port 80/443) / 唯一 Oracle 資料樞紐 / 排程 Watchdog</text>
  </g>

  <!-- lah-messenger-server 節點 -->
  <g filter="url(#shadow)">
    <rect x="520" y="260" width="370" height="62" rx="8" fill="url(#serviceGrad)" stroke="#34d399" stroke-width="1" />
    <text x="535" y="284" fill="#ffffff" font-size="15" font-weight="700">lah-messenger-server 即時通訊伺服器</text>
    <text x="535" y="306" fill="#a7f3d0" font-size="11">Node.js / ws (WebSocket :8081) / Express (附件 HTTP :8082)</text>
  </g>

  <!-- 第 3 層：資料持久層 (Persistence Layer) -->
  <rect x="40" y="365" width="410" height="145" rx="12" fill="#1e293b" fill-opacity="0.6" stroke="#f59e0b" stroke-dasharray="4 4" stroke-width="1.2" />
  <text x="55" y="388" fill="#fde68a" font-size="13" font-weight="700">【資料持久層 Persistence Layer】</text>

  <!-- Oracle 9i 節點 -->
  <g filter="url(#shadow)">
    <rect x="60" y="400" width="175" height="95" rx="8" fill="url(#dbGrad)" stroke="#fbbf24" stroke-width="1" />
    <text x="147" y="430" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">Oracle 9i</text>
    <text x="147" y="450" text-anchor="middle" fill="#fde68a" font-size="11">地政核心業務資料庫</text>
    <text x="147" y="475" text-anchor="middle" fill="#fef3c7" font-size="10">(唯一由 LAH-API 存取)</text>
  </g>

  <!-- SQLite 3 節點 -->
  <g filter="url(#shadow)">
    <rect x="255" y="400" width="175" height="95" rx="8" fill="url(#dbGrad)" stroke="#fbbf24" stroke-width="1" />
    <text x="342" y="430" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">SQLite 3</text>
    <text x="342" y="450" text-anchor="middle" fill="#fde68a" font-size="11">頻道獨立分庫 / 快取</text>
    <text x="342" y="475" text-anchor="middle" fill="#fef3c7" font-size="10">db/&lt;channel&gt;.db 分庫讀寫</text>
  </g>

  <!-- 第 4 層：維運治理層 (LAH-Updater) -->
  <rect x="480" y="365" width="440" height="145" rx="12" fill="#1e293b" fill-opacity="0.6" stroke="#a855f7" stroke-dasharray="4 4" stroke-width="1.2" />
  <text x="495" y="388" fill="#d8b4fe" font-size="13" font-weight="700">【自動更新與維運治理層 LAH-Updater】</text>

  <g filter="url(#shadow)">
    <rect x="500" y="400" width="400" height="95" rx="8" fill="url(#opsGrad)" stroke="#c084fc" stroke-width="1" />
    <text x="700" y="425" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">LAH-Updater 系統維運工具套件</text>
    <text x="700" y="445" text-anchor="middle" fill="#e9d5ff" font-size="11">Tauri 2 (Rust) / 零 CDN 離線介面 / pack.py 打包器</text>
    <text x="700" y="475" text-anchor="middle" fill="#f3e8ff" font-size="10">自動更新(updater) ‧ 可攜Node(node-portable) ‧ 備份(backuper) ‧ 移除(uninstaller)</text>
  </g>

  <!-- 連線箭頭與標註 -->
  <!-- FE -> API -->
  <path d="M 210 182 L 210 260" stroke="#60a5fa" stroke-width="2" marker-end="url(#arrow)" />
  <text x="215" y="215" fill="#93c5fd" font-size="10" font-weight="600">HTTP REST (業務)</text>

  <!-- FE -> WSS -->
  <path d="M 330 182 C 330 215, 620 215, 620 260" fill="none" stroke="#34d399" stroke-width="2" marker-end="url(#arrowGreen)" />
  <text x="410" y="210" fill="#6ee7b7" font-size="10" font-weight="600">WebSocket / HTTP 附件</text>

  <!-- Client -> API -->
  <path d="M 600 182 C 600 215, 310 215, 310 260" fill="none" stroke="#60a5fa" stroke-width="2" marker-end="url(#arrow)" />
  <text x="440" y="235" fill="#93c5fd" font-size="10" font-weight="600">HTTP REST (查詢)</text>

  <!-- Client -> WSS -->
  <path d="M 780 182 L 780 260" stroke="#34d399" stroke-width="2" marker-end="url(#arrowGreen)" />
  <text x="785" y="215" fill="#6ee7b7" font-size="10" font-weight="600">WS (:8081) / 附件 (:8082)</text>

  <!-- API -> Oracle -->
  <path d="M 147 322 L 147 400" stroke="#fbbf24" stroke-width="2" marker-end="url(#arrowAmber)" />
  <text x="152" y="358" fill="#fde68a" font-size="10" font-weight="600">OCI / SQL 唯一存取</text>

  <!-- API -> SQLite -->
  <path d="M 300 322 L 300 400" stroke="#fbbf24" stroke-width="1.8" stroke-dasharray="3 3" marker-end="url(#arrowAmber)" />
  <text x="270" y="358" fill="#fde68a" font-size="9">快取 / 排程</text>

  <!-- WSS -> SQLite -->
  <path d="M 650 322 C 650 355, 380 355, 380 400" fill="none" stroke="#fbbf24" stroke-width="2" marker-end="url(#arrowAmber)" />
  <text x="460" y="348" fill="#fde68a" font-size="10" font-weight="600">better-sqlite3 頻道讀寫</text>

  <!-- Updater -> Service Management -->
  <path d="M 680 400 C 680 350, 560 350, 560 322" fill="none" stroke="#c084fc" stroke-width="1.8" stroke-dasharray="4 2" />
  <path d="M 520 400 C 520 340, 240 340, 240 322" fill="none" stroke="#c084fc" stroke-width="1.8" stroke-dasharray="4 2" />
  <text x="475" y="332" fill="#d8b4fe" font-size="9" font-weight="600">服務生命週期管理與環境接管</text>
</svg>
"""

SVG_PACK_FLOW = """
<svg viewBox="0 0 960 420" xmlns="http://www.w3.org/2000/svg" class="architecture-svg">
  <defs>
    <linearGradient id="flowBg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
    <marker id="flowArrow" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#38bdf8" />
    </marker>
  </defs>

  <rect width="960" height="420" rx="16" fill="url(#flowBg)" stroke="#334155" stroke-width="1.5" />
  
  <text x="480" y="34" text-anchor="middle" fill="#f8fafc" font-size="18" font-weight="700">
    系統資產打包與更新部署流向圖 (Packaging & Distribution Flow)
  </text>
  <text x="480" y="54" text-anchor="middle" fill="#94a3b8" font-size="12">
    pack.py 三向自動分流 ➔ 自動更新工具 (updater) 智慧探測 ➔ 正式伺服器解壓覆蓋
  </text>

  <!-- 左區：原始碼與專案來源 -->
  <rect x="30" y="80" width="180" height="305" rx="10" fill="#1e293b" stroke="#475569" stroke-width="1" />
  <text x="120" y="105" text-anchor="middle" fill="#38bdf8" font-size="14" font-weight="700">來源儲存庫 / 核心</text>

  <rect x="45" y="120" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="141" text-anchor="middle" fill="#e2e8f0" font-size="11">LAH-API (PHP7/8)</text>

  <rect x="45" y="160" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="181" text-anchor="middle" fill="#e2e8f0" font-size="11">LAH-FE (Nuxt 2)</text>

  <rect x="45" y="200" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="221" text-anchor="middle" fill="#e2e8f0" font-size="11">lah-messenger-server</text>

  <rect x="45" y="240" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="261" text-anchor="middle" fill="#e2e8f0" font-size="11">各專案 node_modules</text>

  <rect x="45" y="280" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="301" text-anchor="middle" fill="#e2e8f0" font-size="11">Apache24 / PHP7 / 85</text>

  <rect x="45" y="320" width="150" height="32" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="120" y="341" text-anchor="middle" fill="#e2e8f0" font-size="11">Node 16.20.2 綠色環境</text>

  <!-- 中間左：pack.py 打包器 -->
  <rect x="250" y="180" width="110" height="90" rx="10" fill="#0369a1" stroke="#38bdf8" stroke-width="1.5" />
  <text x="305" y="215" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">pack.py</text>
  <text x="305" y="235" text-anchor="middle" fill="#bae6fd" font-size="10">自動打包腳本</text>
  <text x="305" y="252" text-anchor="middle" fill="#e0f2fe" font-size="9">零相依 Python3</text>

  <!-- 來源到 pack.py 連線 -->
  <path d="M 210 225 L 250 225" stroke="#38bdf8" stroke-width="2" marker-end="url(#flowArrow)" />

  <!-- 中間右：三向分流目標目錄 -->
  <rect x="400" y="80" width="240" height="305" rx="10" fill="#1e293b" stroke="#475569" stroke-width="1" />
  <text x="520" y="105" text-anchor="middle" fill="#facc15" font-size="14" font-weight="700">dist/ 規範分流目錄</text>

  <!-- patch 目錄 -->
  <rect x="415" y="120" width="210" height="70" rx="6" fill="#0f172a" stroke="#38bdf8" />
  <text x="425" y="138" fill="#38bdf8" font-size="11" font-weight="700">📁 dist/patch/ (業務修補更新包)</text>
  <text x="425" y="156" fill="#94a3b8" font-size="10">www.zip (PHP7), LAH-API.zip (PHP8)</text>
  <text x="425" y="172" fill="#94a3b8" font-size="10">LAH-NUXTJS.zip, lah-messenger-server.zip</text>

  <!-- node_modules 目錄 -->
  <rect x="415" y="200" width="210" height="70" rx="6" fill="#0f172a" stroke="#a855f7" />
  <text x="425" y="218" fill="#c084fc" font-size="11" font-weight="700">📁 dist/node_modules/ (核心依賴套件)</text>
  <text x="425" y="236" fill="#94a3b8" font-size="10">LAH-NUXTJS-node_modules.zip</text>
  <text x="425" y="252" fill="#94a3b8" font-size="10">lah-messenger-server-node_modules.zip</text>

  <!-- base 目錄 -->
  <rect x="415" y="280" width="210" height="85" rx="6" fill="#0f172a" stroke="#10b981" />
  <text x="425" y="298" fill="#34d399" font-size="11" font-weight="700">📁 dist/base/ (基礎執行環境包)</text>
  <text x="425" y="316" fill="#94a3b8" font-size="10">Apache24.zip, php7.zip, php85.zip</text>
  <text x="425" y="332" fill="#94a3b8" font-size="10">node.zip (可攜 Node 16.20.2 + PM2)</text>
  <text x="425" y="348" fill="#6ee7b7" font-size="9">⭐ node-portable 亦優先讀取此處</text>

  <!-- pack.py 到 dist 分流連線 -->
  <path d="M 360 215 C 380 215, 380 155, 415 155" fill="none" stroke="#38bdf8" stroke-width="1.8" marker-end="url(#flowArrow)" />
  <path d="M 360 225 L 415 225" stroke="#a855f7" stroke-width="1.8" marker-end="url(#flowArrow)" />
  <path d="M 360 235 C 380 235, 380 320, 415 320" fill="none" stroke="#10b981" stroke-width="1.8" marker-end="url(#flowArrow)" />

  <!-- 右側：自動更新工具 (updater) -->
  <rect x="680" y="125" width="250" height="110" rx="10" fill="#1e3a8a" stroke="#60a5fa" stroke-width="1.5" />
  <text x="805" y="152" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">智慧控管與監控系統</text>
  <text x="805" y="172" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">自動更新工具.exe</text>
  <text x="805" y="195" text-anchor="middle" fill="#bfdbfe" font-size="11">跨目錄探測 (dist/patch, dist/node_modules, dist/base)</text>
  <text x="805" y="215" text-anchor="middle" fill="#93c5fd" font-size="10">單一頂層目錄自動剝除 ‧ 雙層目錄自癒攤平</text>

  <!-- dist 到 updater 連線 -->
  <path d="M 625 155 C 650 155, 650 170, 680 170" fill="none" stroke="#60a5fa" stroke-width="2" marker-end="url(#flowArrow)" />
  <path d="M 625 235 C 650 235, 650 185, 680 185" fill="none" stroke="#60a5fa" stroke-width="2" marker-end="url(#flowArrow)" />
  <path d="M 625 320 C 655 320, 655 200, 680 200" fill="none" stroke="#60a5fa" stroke-width="2" marker-end="url(#flowArrow)" />

  <!-- 下方：目標伺服器部署路徑 -->
  <rect x="680" y="270" width="250" height="115" rx="10" fill="#065f46" stroke="#34d399" stroke-width="1.5" />
  <text x="805" y="298" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">目標正式伺服器</text>
  <text x="805" y="320" text-anchor="middle" fill="#a7f3d0" font-size="12">C:\\AppServ 或 D:\\DEV</text>
  <text x="805" y="342" text-anchor="middle" fill="#d1fae5" font-size="10">Apache 2.4 (mod_php + FCGI API Proxy)</text>
  <text x="805" y="360" text-anchor="middle" fill="#d1fae5" font-size="10">PM2 WinSW 系統服務 (LAH-FE, LAH-WSS, Sched)</text>

  <!-- updater 到 目標伺服器 連線 -->
  <path d="M 805 235 L 805 270" stroke="#34d399" stroke-width="2.5" marker-end="url(#flowArrow)" />
</svg>
"""

SVG_PIPELINE = """
<svg viewBox="0 0 960 360" xmlns="http://www.w3.org/2000/svg" class="architecture-svg">
  <defs>
    <linearGradient id="pipeBg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
    <linearGradient id="step1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b" />
      <stop offset="100%" stop-color="#334155" />
    </linearGradient>
    <marker id="pipeArrow" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#38bdf8" />
    </marker>
  </defs>

  <rect width="960" height="360" rx="16" fill="url(#pipeBg)" stroke="#334155" stroke-width="1.5" />

  <text x="480" y="34" text-anchor="middle" fill="#f8fafc" font-size="18" font-weight="700">
    自動更新管線 5 階段自動化與容錯流程 (5-Stage Pipeline)
  </text>
  <text x="480" y="54" text-anchor="middle" fill="#94a3b8" font-size="12">
    多程序安全停止 ➔ 智慧對齊解壓 ➔ 雙 PHP 環境配置 ➔ PM2 綠色環境治理 ➔ 健康巡檢
  </text>

  <!-- 步驟 1 -->
  <g>
    <rect x="30" y="80" width="165" height="230" rx="10" fill="#1e293b" stroke="#38bdf8" stroke-width="1.2" />
    <circle cx="112" cy="110" r="18" fill="#0284c7" />
    <text x="112" y="116" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">1</text>
    <text x="112" y="145" text-anchor="middle" fill="#f8fafc" font-size="13" font-weight="700">預備與安全停止</text>
    <line x1="45" y1="158" x2="180" y2="158" stroke="#334155" />
    <text x="45" y="180" fill="#94a3b8" font-size="11">‧ 停止 Apache 服務</text>
    <text x="45" y="200" fill="#94a3b8" font-size="11">‧ 停止 PM2 系統服務</text>
    <text x="45" y="220" fill="#94a3b8" font-size="11">‧ kill_pm2_daemon 終止</text>
    <text x="45" y="240" fill="#94a3b8" font-size="11">‧ 釋放 node.exe 行程鎖</text>
    <text x="45" y="260" fill="#94a3b8" font-size="11">‧ 避免解壓時鎖檔失敗</text>
    <text x="112" y="295" text-anchor="middle" fill="#38bdf8" font-size="10" font-weight="600">[進度: 0% - 5%]</text>
  </g>

  <!-- 箭頭 1 -> 2 -->
  <path d="M 195 195 L 215 195" stroke="#38bdf8" stroke-width="2" marker-end="url(#pipeArrow)" />

  <!-- 步驟 2 -->
  <g>
    <rect x="215" y="80" width="165" height="230" rx="10" fill="#1e293b" stroke="#38bdf8" stroke-width="1.2" />
    <circle cx="297" cy="110" r="18" fill="#0284c7" />
    <text x="297" y="116" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">2</text>
    <text x="297" y="145" text-anchor="middle" fill="#f8fafc" font-size="13" font-weight="700">解壓與目錄自癒</text>
    <line x1="230" y1="158" x2="365" y2="158" stroke="#334155" />
    <text x="230" y="180" fill="#94a3b8" font-size="11">‧ 依序解壓縮選取安裝包</text>
    <text x="230" y="200" fill="#94a3b8" font-size="11">‧ 剝除頂層單一目錄前綴</text>
    <text x="230" y="220" fill="#94a3b8" font-size="11">‧ 防止 node\node 等雙層</text>
    <text x="230" y="240" fill="#94a3b8" font-size="11">‧ 自癒攤平巢狀目錄結構</text>
    <text x="230" y="260" fill="#94a3b8" font-size="11">‧ 確保 .env 基礎範本齊全</text>
    <text x="297" y="295" text-anchor="middle" fill="#38bdf8" font-size="10" font-weight="600">[進度: 5% - 85%]</text>
  </g>

  <!-- 箭頭 2 -> 3 -->
  <path d="M 380 195 L 400 195" stroke="#38bdf8" stroke-width="2" marker-end="url(#pipeArrow)" />

  <!-- 步驟 3 -->
  <g>
    <rect x="400" y="80" width="165" height="230" rx="10" fill="#1e293b" stroke="#38bdf8" stroke-width="1.2" />
    <circle cx="482" cy="110" r="18" fill="#0284c7" />
    <text x="482" y="116" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">3</text>
    <text x="482" y="145" text-anchor="middle" fill="#f8fafc" font-size="13" font-weight="700">雙 PHP 與資安強化</text>
    <line x1="415" y1="158" x2="550" y2="158" stroke="#334155" />
    <text x="415" y="180" fill="#94a3b8" font-size="11">‧ 配置 PHP 7.4 php.ini</text>
    <text x="415" y="200" fill="#94a3b8" font-size="11">‧ 掛載 Apache mod_php7</text>
    <text x="415" y="220" fill="#94a3b8" font-size="11">‧ 配置 PHP 8.5 FCGI Proxy</text>
    <text x="415" y="240" fill="#94a3b8" font-size="11">‧ CVE-2024-4577 防禦</text>
    <text x="415" y="260" fill="#94a3b8" font-size="11">‧ 隱藏 Apache/PHP 標頭</text>
    <text x="482" y="295" text-anchor="middle" fill="#38bdf8" font-size="10" font-weight="600">[進度: 85% - 94%]</text>
  </g>

  <!-- 箭頭 3 -> 4 -->
  <path d="M 565 195 L 585 195" stroke="#38bdf8" stroke-width="2" marker-end="url(#pipeArrow)" />

  <!-- 步驟 4 -->
  <g>
    <rect x="585" y="80" width="175" height="230" rx="10" fill="#1e293b" stroke="#a855f7" stroke-width="1.2" />
    <circle cx="672" cy="110" r="18" fill="#7c3aed" />
    <text x="672" y="116" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">4</text>
    <text x="672" y="145" text-anchor="middle" fill="#f8fafc" font-size="13" font-weight="700">Node 與 PM2 治理</text>
    <line x1="600" y1="158" x2="745" y2="158" stroke="#334155" />
    <text x="600" y="180" fill="#c084fc" font-size="11">‧ 零全域污染沙盒隔離</text>
    <text x="600" y="200" fill="#c084fc" font-size="11">‧ 接管 PM2_HOME 至 .pm2</text>
    <text x="600" y="220" fill="#c084fc" font-size="11">‧ 淨化 @jessety 舊設定</text>
    <text x="600" y="240" fill="#c084fc" font-size="11">‧ 防禦 MODULE_NOT_FOUND</text>
    <text x="600" y="260" fill="#c084fc" font-size="11">‧ 預載 pm2-logrotate 模組</text>
    <text x="600" y="278" fill="#c084fc" font-size="11">‧ 註冊 WinSW 系統服務</text>
    <text x="672" y="295" text-anchor="middle" fill="#c084fc" font-size="10" font-weight="600">[進度: 94% - 97%]</text>
  </g>

  <!-- 箭頭 4 -> 5 -->
  <path d="M 760 195 L 780 195" stroke="#38bdf8" stroke-width="2" marker-end="url(#pipeArrow)" />

  <!-- 步驟 5 -->
  <g>
    <rect x="780" y="80" width="150" height="230" rx="10" fill="#1e293b" stroke="#10b981" stroke-width="1.2" />
    <circle cx="855" cy="110" r="18" fill="#059669" />
    <text x="855" y="116" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="700">5</text>
    <text x="855" y="145" text-anchor="middle" fill="#f8fafc" font-size="13" font-weight="700">重啟與健康巡檢</text>
    <line x1="795" y1="158" x2="915" y2="158" stroke="#334155" />
    <text x="795" y="180" fill="#6ee7b7" font-size="11">‧ 熱重啟 Apache 服務</text>
    <text x="795" y="200" fill="#6ee7b7" font-size="11">‧ 喚醒 PM2 應用進程</text>
    <text x="795" y="220" fill="#6ee7b7" font-size="11">‧ 巡檢 Port 80 (API)</text>
    <text x="795" y="240" fill="#6ee7b7" font-size="11">‧ 巡檢 Port 8080 (FE)</text>
    <text x="795" y="260" fill="#6ee7b7" font-size="11">‧ 巡檢 Port 8081 (WSS)</text>
    <text x="795" y="278" fill="#6ee7b7" font-size="11">‧ 產出完整健康報告</text>
    <text x="855" y="295" text-anchor="middle" fill="#34d399" font-size="10" font-weight="600">[進度: 97% - 100%]</text>
  </g>
</svg>
"""

SVG_ISOLATION = """
<svg viewBox="0 0 960 380" xmlns="http://www.w3.org/2000/svg" class="architecture-svg">
  <defs>
    <linearGradient id="isoBg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
  </defs>

  <rect width="960" height="380" rx="16" fill="url(#isoBg)" stroke="#334155" stroke-width="1.5" />

  <text x="480" y="34" text-anchor="middle" fill="#f8fafc" font-size="18" font-weight="700">
    可攜式 Node.js 16 與系統全域升級 四重安全隔離機制
  </text>
  <text x="480" y="54" text-anchor="middle" fill="#94a3b8" font-size="12">
    即使外部因弱點掃描強制升級至 Node.js 22+，智慧控管系統仍能 100% 穩定自立運行
  </text>

  <!-- 左側：全域環境 (外部影響) -->
  <rect x="40" y="80" width="260" height="270" rx="10" fill="#1e293b" stroke="#ef4444" stroke-width="1.2" />
  <text x="170" y="110" text-anchor="middle" fill="#f87171" font-size="15" font-weight="700">外部全域環境</text>
  <text x="170" y="130" text-anchor="middle" fill="#94a3b8" font-size="11">(資安弱掃強制升級目標)</text>
  
  <rect x="55" y="150" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="70" y="172" fill="#e2e8f0" font-size="12" font-weight="600">全域 Node.js v22.x</text>
  <text x="70" y="190" fill="#94a3b8" font-size="10">路徑: C:\\Program Files\\nodejs</text>

  <rect x="55" y="215" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="70" y="237" fill="#e2e8f0" font-size="12" font-weight="600">全域 PM2 / npm 套件</text>
  <text x="70" y="255" fill="#94a3b8" font-size="10">路徑: %APPDATA%\\npm</text>

  <rect x="55" y="280" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="70" y="302" fill="#e2e8f0" font-size="12" font-weight="600">舊版 PM2 服務 (pm2-installer)</text>
  <text x="70" y="320" fill="#f87171" font-size="10">常見殘留 @jessety 模組配置</text>

  <!-- 中間：四重隔離護欄 -->
  <rect x="330" y="80" width="300" height="270" rx="10" fill="#1e293b" stroke="#38bdf8" stroke-width="1.5" />
  <text x="480" y="108" text-anchor="middle" fill="#38bdf8" font-size="15" font-weight="700">🛡️ 四重隔離機制保證</text>

  <rect x="345" y="125" width="270" height="46" rx="6" fill="#0f172a" stroke="#0284c7" />
  <text x="355" y="145" fill="#e0f2fe" font-size="11" font-weight="700">1. WinSW 實體絕對路徑綁定</text>
  <text x="355" y="162" fill="#94a3b8" font-size="10">服務直接指向 &lt;root&gt;\\node\\node.exe，絕不飄移</text>

  <rect x="345" y="180" width="270" height="46" rx="6" fill="#0f172a" stroke="#0284c7" />
  <text x="355" y="200" fill="#e0f2fe" font-size="11" font-weight="700">2. PM2_HOME 專案隔離</text>
  <text x="355" y="217" fill="#94a3b8" font-size="10">強制設為 &lt;root&gt;\\.pm2，隔離 C:\\ProgramData\\pm2</text>

  <rect x="345" y="235" width="270" height="46" rx="6" fill="#0f172a" stroke="#0284c7" />
  <text x="355" y="255" fill="#e0f2fe" font-size="11" font-weight="700">3. 零全域污染與專屬終端 (lah-cmd.bat)</text>
  <text x="355" y="272" fill="#94a3b8" font-size="10">清理舊登錄檔殘留，絕不污染系統全域環境變數</text>

  <rect x="345" y="290" width="270" height="46" rx="6" fill="#0f172a" stroke="#0284c7" />
  <text x="355" y="310" fill="#e0f2fe" font-size="11" font-weight="700">4. 模組淨化與 try-catch 異常防禦</text>
  <text x="355" y="327" fill="#94a3b8" font-size="10">自動清除 @jessety 殘留，防禦缺失模組崩潰</text>

  <!-- 右側：LAH 智慧控管專屬環境 -->
  <rect x="660" y="80" width="260" height="270" rx="10" fill="#1e293b" stroke="#10b981" stroke-width="1.2" />
  <text x="790" y="110" text-anchor="middle" fill="#34d399" font-size="15" font-weight="700">LAH 可攜版專屬環境</text>
  <text x="790" y="130" text-anchor="middle" fill="#6ee7b7" font-size="11">(&lt;root_dir&gt; = C:\\AppServ 或 D:\\DEV)</text>

  <rect x="675" y="150" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="690" y="172" fill="#e2e8f0" font-size="12" font-weight="600">可攜 Node.js v16.20.2</text>
  <text x="690" y="190" fill="#34d399" font-size="10">路徑: &lt;root&gt;\\node (完整相容 Nuxt2)</text>

  <rect x="675" y="215" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="690" y="237" fill="#e2e8f0" font-size="12" font-weight="600">PM2 v5.3.1 + pm2-logrotate</text>
  <text x="690" y="255" fill="#34d399" font-size="10">隨附預裝、自動 gzip 壓縮輪替</text>

  <rect x="675" y="280" width="230" height="50" rx="6" fill="#0f172a" stroke="#334155" />
  <text x="690" y="302" fill="#e2e8f0" font-size="12" font-weight="600">專屬系統服務 (LAH_PM2_Service)</text>
  <text x="690" y="320" fill="#34d399" font-size="10">由 WinSW 守護，開機全自動拉起</text>
</svg>
"""

def generate_html():
    html_content = f"""<!DOCTYPE html>
<html lang="zh-Hant-TW">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>LAH 智慧控管與監控系統 - 工作區全域架構與維運工具實作手冊</title>
  <style>
    /* 現代深色科技美學樣式表 (零外部 CDN，純純本機相容) */
    :root {{
      --bg-main: #0b0f19;
      --bg-card: #111827;
      --bg-card-sub: #1e293b;
      --bg-accent: #1e3a8a;
      --border-color: #1f2937;
      --border-focus: #3b82f6;
      --text-main: #f3f4f6;
      --text-muted: #9ca3af;
      --text-light: #d1d5db;
      --accent-blue: #3b82f6;
      --accent-cyan: #06b6d4;
      --accent-emerald: #10b981;
      --accent-amber: #f59e0b;
      --accent-purple: #8b5cf6;
      --accent-rose: #f43f5e;
      --font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", "Microsoft JhengHei", "微軟正黑體", sans-serif;
      --code-font: "Consolas", "Cascadia Code", "Courier New", monospace;
    }}

    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}

    body {{
      background-color: var(--bg-main);
      color: var(--text-main);
      font-family: var(--font-family);
      line-height: 1.6;
      overflow-x: hidden;
      display: flex;
    }}

    /* 側邊導航目錄 */
    #sidebar {{
      width: 300px;
      height: 100vh;
      position: sticky;
      top: 0;
      background-color: #0d1322;
      border-right: 1px solid var(--border-color);
      display: flex;
      flex-direction: column;
      flex-shrink: 0;
      z-index: 100;
      overflow-y: auto;
    }}

    .sidebar-header {{
      padding: 24px 20px 16px 20px;
      border-bottom: 1px solid var(--border-color);
    }}

    .sidebar-title {{
      font-size: 1.15rem;
      font-weight: 700;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 8px;
    }}

    .sidebar-subtitle {{
      font-size: 0.75rem;
      color: var(--text-muted);
      margin-top: 4px;
    }}

    .search-box {{
      padding: 16px 20px 8px 20px;
    }}

    .search-input {{
      width: 100%;
      padding: 8px 12px;
      background-color: #1e293b;
      border: 1px solid #334155;
      border-radius: 6px;
      color: #ffffff;
      font-size: 0.85rem;
      outline: none;
      transition: border-color 0.2s;
    }}

    .search-input:focus {{
      border-color: var(--accent-blue);
    }}

    .nav-list {{
      list-style: none;
      padding: 8px 12px 32px 12px;
    }}

    .nav-item {{
      margin-bottom: 4px;
    }}

    .nav-link {{
      display: block;
      padding: 8px 12px;
      color: var(--text-light);
      text-decoration: none;
      border-radius: 6px;
      font-size: 0.88rem;
      font-weight: 500;
      transition: all 0.15s ease-in-out;
    }}

    .nav-link:hover {{
      background-color: #1e293b;
      color: var(--accent-cyan);
      transform: translateX(3px);
    }}

    .nav-link.active {{
      background-color: #1e3a8a;
      color: #ffffff;
      font-weight: 600;
    }}

    .nav-category {{
      font-size: 0.72rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: #64748b;
      margin: 16px 12px 6px 12px;
      font-weight: 700;
    }}

    /* 主內容區 */
    #main-content {{
      flex: 1;
      min-width: 0;
      padding: 32px 48px;
      max-width: 1280px;
      margin: 0 auto;
    }}

    /* 頂部 Hero Banner */
    .hero-banner {{
      background: linear-gradient(135deg, #0f172a 0%, #1e1b4b 50%, #0f172a 100%);
      border: 1px solid #312e81;
      border-radius: 16px;
      padding: 36px 40px;
      margin-bottom: 36px;
      position: relative;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.4);
    }}

    .hero-title {{
      font-size: 2.1rem;
      font-weight: 800;
      color: #ffffff;
      margin-bottom: 10px;
      letter-spacing: -0.02em;
    }}

    .hero-desc {{
      font-size: 1.05rem;
      color: #c7d2fe;
      margin-bottom: 20px;
      max-width: 900px;
    }}

    .badge-container {{
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
    }}

    .badge {{
      display: inline-flex;
      align-items: center;
      padding: 4px 10px;
      border-radius: 20px;
      font-size: 0.78rem;
      font-weight: 600;
      letter-spacing: 0.02em;
    }}

    .badge-blue {{ background-color: rgba(59, 130, 246, 0.2); color: #93c5fd; border: 1px solid rgba(59, 130, 246, 0.4); }}
    .badge-emerald {{ background-color: rgba(16, 185, 129, 0.2); color: #a7f3d0; border: 1px solid rgba(16, 185, 129, 0.4); }}
    .badge-purple {{ background-color: rgba(168, 85, 247, 0.2); color: #e9d5ff; border: 1px solid rgba(168, 85, 247, 0.4); }}
    .badge-amber {{ background-color: rgba(245, 158, 11, 0.2); color: #fde68a; border: 1px solid rgba(245, 158, 11, 0.4); }}

    /* 區塊卡片 */
    .section-card {{
      background-color: var(--bg-card);
      border: 1px solid var(--border-color);
      border-radius: 14px;
      padding: 28px 32px;
      margin-bottom: 32px;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.2);
    }}

    .section-title {{
      font-size: 1.5rem;
      font-weight: 700;
      color: #ffffff;
      margin-bottom: 8px;
      display: flex;
      align-items: center;
      gap: 10px;
      padding-bottom: 12px;
      border-bottom: 1px solid #1f2937;
    }}

    .section-intro {{
      font-size: 0.95rem;
      color: var(--text-light);
      margin: 14px 0 20px 0;
    }}

    /* 圖表容器 */
    .diagram-container {{
      background-color: #080c14;
      border: 1px solid #1e293b;
      border-radius: 12px;
      padding: 20px;
      margin: 20px 0;
      text-align: center;
    }}

    .architecture-svg {{
      width: 100%;
      max-width: 960px;
      height: auto;
      display: inline-block;
    }}

    .diagram-caption {{
      font-size: 0.82rem;
      color: var(--text-muted);
      margin-top: 10px;
    }}

    /* 專案卡片網格 */
    .grid-2 {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
      gap: 20px;
      margin-top: 20px;
    }}

    .grid-3 {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      gap: 16px;
      margin-top: 20px;
    }}

    .project-card {{
      background-color: var(--bg-card-sub);
      border: 1px solid #334155;
      border-radius: 10px;
      padding: 20px;
      transition: transform 0.2s, border-color 0.2s;
    }}

    .project-card:hover {{
      transform: translateY(-2px);
      border-color: var(--accent-blue);
    }}

    .project-card-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }}

    .project-card-title {{
      font-size: 1.15rem;
      font-weight: 700;
      color: #ffffff;
    }}

    .project-tech {{
      font-size: 0.8rem;
      color: var(--accent-cyan);
      font-family: var(--code-font);
      background-color: rgba(6, 182, 212, 0.1);
      padding: 2px 8px;
      border-radius: 4px;
      border: 1px solid rgba(6, 182, 212, 0.3);
    }}

    .project-detail-list {{
      list-style: none;
      font-size: 0.88rem;
      color: var(--text-light);
    }}

    .project-detail-list li {{
      margin-bottom: 8px;
      position: relative;
      padding-left: 18px;
    }}

    .project-detail-list li::before {{
      content: "▹";
      position: absolute;
      left: 0;
      color: var(--accent-blue);
      font-weight: bold;
    }}

    /* 頁籤控制 */
    .tabs-header {{
      display: flex;
      border-bottom: 1px solid #334155;
      margin-bottom: 20px;
      gap: 8px;
    }}

    .tab-btn {{
      padding: 10px 18px;
      background: none;
      border: none;
      color: var(--text-muted);
      font-size: 0.92rem;
      font-weight: 600;
      cursor: pointer;
      border-radius: 6px 6px 0 0;
      transition: all 0.2s;
      border-bottom: 2px solid transparent;
    }}

    .tab-btn:hover {{
      color: #ffffff;
      background-color: rgba(255, 255, 255, 0.05);
    }}

    .tab-btn.active {{
      color: var(--accent-cyan);
      border-bottom: 2px solid var(--accent-cyan);
      background-color: rgba(6, 182, 212, 0.1);
    }}

    .tab-content {{
      display: none;
    }}

    .tab-content.active {{
      display: block;
    }}

    /* 程式碼與命令方塊 */
    .code-block {{
      background-color: #0a0e17;
      border: 1px solid #1e293b;
      border-radius: 8px;
      padding: 14px 18px;
      margin: 12px 0;
      font-family: var(--code-font);
      font-size: 0.88rem;
      color: #38bdf8;
      overflow-x: auto;
      position: relative;
    }}

    .code-block code {{
      white-space: pre-wrap;
    }}

    .copy-btn {{
      position: absolute;
      top: 8px;
      right: 8px;
      background-color: #1e293b;
      color: var(--text-light);
      border: 1px solid #334155;
      border-radius: 4px;
      padding: 3px 8px;
      font-size: 0.72rem;
      cursor: pointer;
      transition: all 0.2s;
    }}

    .copy-btn:hover {{
      background-color: var(--accent-blue);
      color: #ffffff;
    }}

    /* 表格樣式 */
    .table-container {{
      overflow-x: auto;
      margin: 16px 0;
      border: 1px solid #334155;
      border-radius: 8px;
    }}

    table {{
      width: 100%;
      border-collapse: collapse;
      font-size: 0.88rem;
      text-align: left;
    }}

    th {{
      background-color: #1e293b;
      color: #f8fafc;
      padding: 12px 16px;
      font-weight: 600;
      border-bottom: 1px solid #334155;
    }}

    td {{
      padding: 12px 16px;
      border-bottom: 1px solid #1e293b;
      color: var(--text-light);
    }}

    tr:nth-child(even) {{
      background-color: rgba(30, 41, 59, 0.3);
    }}

    tr:hover {{
      background-color: rgba(59, 130, 246, 0.08);
    }}

    /* 警示提示框 */
    .alert-box {{
      padding: 14px 18px;
      border-radius: 8px;
      margin: 16px 0;
      font-size: 0.9rem;
      display: flex;
      gap: 12px;
      align-items: flex-start;
    }}

    .alert-info {{
      background-color: rgba(59, 130, 246, 0.12);
      border: 1px solid rgba(59, 130, 246, 0.35);
      color: #bfdbfe;
    }}

    .alert-success {{
      background-color: rgba(16, 185, 129, 0.12);
      border: 1px solid rgba(16, 185, 129, 0.35);
      color: #a7f3d0;
    }}

    .alert-warning {{
      background-color: rgba(245, 158, 11, 0.12);
      border: 1px solid rgba(245, 158, 11, 0.35);
      color: #fde68a;
    }}

    /* 頁尾 */
    footer {{
      margin-top: 48px;
      padding-top: 24px;
      border-top: 1px solid var(--border-color);
      text-align: center;
      color: var(--text-muted);
      font-size: 0.85rem;
    }}

    @media (max-width: 900px) {{
      body {{ flex-direction: column; }}
      #sidebar {{ width: 100%; height: auto; position: static; }}
      #main-content {{ padding: 20px; }}
      .grid-2 {{ grid-template-columns: 1fr; }}
    }}
  </style>
</head>
<body>

  <!-- 左側快速導航側邊欄 -->
  <aside id="sidebar">
    <div class="sidebar-header">
      <div class="sidebar-title">
        <span>🛡️</span> LAH 系統架構手冊
      </div>
      <div class="sidebar-subtitle">工作區 5 大專案 ‧ 4 大工具 ‧ 離線技術全解</div>
    </div>

    <div class="search-box">
      <input type="text" id="filterInput" class="search-input" placeholder="即時篩選關鍵字 (如 PM2, Oracle)..." />
    </div>

    <ul class="nav-list" id="navList">
      <li class="nav-category">工作區全域架構</li>
      <li class="nav-item"><a href="#workspace-topology" class="nav-link active">全域系統拓撲與邊界契約</a></li>
      <li class="nav-item"><a href="#subprojects" class="nav-link">五大平級 Git 子專案解析</a></li>
      
      <li class="nav-category">維運工具與模組</li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link">LAH-Updater 工具套件總覽</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-updater')">1. 自動更新工具 (updater)</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-portable')">2. 可攜 Node 安裝工具</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-uninstaller')">3. 解除安裝工具 (uninstaller)</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-backuper')">4. 自動備份工具 (backuper)</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-pack')">5. 原始碼打包腳本 (pack.py)</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-sms')">6. 簡訊設定工具 (sms-env-editor)</a></li>
      <li class="nav-item"><a href="#updater-overview" class="nav-link" onclick="switchTab('tab-dashboard')">7. 管理儀表板 (dashboard)</a></li>

      <li class="nav-category">資產與目錄規格</li>
      <li class="nav-item"><a href="#directory-specs" class="nav-link">安裝更新包目錄分流規範</a></li>
      <li class="nav-item"><a href="#quadruple-isolation" class="nav-link">四重隔離與資安升級防護</a></li>

      <li class="nav-category">實戰維運與除錯</li>
      <li class="nav-item"><a href="#crash-troubleshooting" class="nav-link">實機崩潰根因與模組防禦</a></li>
      <li class="nav-item"><a href="#quick-scripts" class="nav-link">本機快捷維運指令清單</a></li>
    </ul>
  </aside>

  <!-- 右側主要內容展現區 -->
  <main id="main-content">

    <!-- 頂部 Hero 區域 -->
    <header class="hero-banner">
      <h1 class="hero-title">智慧控管與監控系統 (LAH-Workspace)</h1>
      <p class="hero-desc">
        本技術手冊提供工作區 5 個獨立 Git 子專案與 LAH-Updater 維運工具組之完整實作說明、通訊契約、資料流向、安裝部署分流規範與實機故障排除手冊。
        全文件採無外部 CDN 離線打造，內嵌向量架構圖與純原生 JavaScript 互動。
      </p>
      <div class="badge-container">
        <span class="badge badge-blue">Tauri 2 (Rust)</span>
        <span class="badge badge-emerald">Nuxt 2 (Vue 2)</span>
        <span class="badge badge-purple">PHP 7.3 & PHP 8.5</span>
        <span class="badge badge-amber">Oracle 9i & SQLite 3</span>
        <span class="badge badge-blue">可攜 Node 16.20.2 & PM2</span>
        <span class="badge badge-emerald">100% 離線純靜態手冊</span>
      </div>
    </header>

    <!-- 第 1 區：工作區全域架構拓撲 -->
    <section id="workspace-topology" class="section-card">
      <h2 class="section-title">
        <span>🌐</span> 工作區全域架構拓撲與邊界契約
      </h2>
      <p class="section-intro">
        工作區根目錄透過 Junction 整合 5 個平級且版本各自獨立控管的 Git 子專案。各子專案嚴格遵守架構邊界契約，任何業務查詢、即時長連線與資料庫操作皆具備清晰的單一責任。
      </p>

      <div class="diagram-container">
        {SVG_TOPOLOGY}
        <div class="diagram-caption">圖 1：LAH 智慧控管與監控系統 全域架構拓撲與通訊契約（點擊各節點可對應閱讀）</div>
      </div>

      <div class="alert-box alert-info">
        <div>💡</div>
        <div>
          <strong>全域核心安全護欄：</strong>
          1. <code>LAH-API</code> 是後端唯一的 Oracle 9i 地政業務資料樞紐，前端 <code>LAH-FE</code> 與 <code>lah-messenger</code> 桌面端嚴禁直接穿透存取資料庫。<br>
          2. <code>lah-messenger-server</code> 專職負責高併發 WebSocket 雙向長連線推播與附件處理，採用頻道獨立分庫 <code>db/&lt;channel&gt;.db</code>。<br>
          3. <code>LAH-Updater</code> 為全系統維運與更新中心，透過跨目錄探索自動辨識並部署 11 大系統安裝包。
        </div>
      </div>
    </section>

    <!-- 第 2 區：五大平級 Git 子專案解析 -->
    <section id="subprojects" class="section-card">
      <h2 class="section-title">
        <span>📦</span> 五大平級 Git 子專案深度解析
      </h2>
      <p class="section-intro">
        工作區包含 5 個平級 Git 儲存庫，各司其職並由根目錄統一調度：
      </p>

      <div class="grid-2">
        <!-- LAH-API -->
        <div class="project-card">
          <div class="project-card-header">
            <span class="project-card-title">1. LAH-API (後端 REST API 核心)</span>
            <span class="project-tech">PHP 7.3 / OCI / SQLite</span>
          </div>
          <ul class="project-detail-list">
            <li><strong>核心技術：</strong>PHP 7.3.x（嚴禁使用 PHP 7.4+ 新語法，相容舊型地政主機）、Apache 2.4 mod_php7。</li>
            <li><strong>資料樞紐：</strong>全系統唯一負責對接 Oracle 9i 核心地政資料庫，並提供本機 SQLite 3 快取。</li>
            <li><strong>開發防護：</strong>支援 <code>MOCK=true</code> 模式，避免開發時對實體 Oracle 產生不必要的連線與誤寫入。</li>
            <li><strong>排程服務：</strong>內建 <code>watchdog.php</code> 系統監控與日誌巡檢定時任務。</li>
          </ul>
        </div>

        <!-- LAH-FE -->
        <div class="project-card">
          <div class="project-card-header">
            <span class="project-card-title">2. LAH-FE (智慧管控系統前端)</span>
            <span class="project-tech">Nuxt 2.15.8 / Vue 2 / BS 4</span>
          </div>
          <ul class="project-detail-list">
            <li><strong>核心技術：</strong>Nuxt 2 (Vue 2 Options API)、BootstrapVue 2.23.0、chart.js。</li>
            <li><strong>雙核心通訊：</strong>透過 HTTP REST API 呼叫 <code>LAH-API</code> 取得地政業務資料，透過 WebSocket 連線 <code>lah-messenger-server</code> (:8081) 取得即時通報。</li>
            <li><strong>即時通元件群：</strong>完整移植即時通客戶端之 <code>lah-messenger-*.vue</code> 元件（聊天室、側邊欄、公告看板）。</li>
            <li><strong>背景排程：</strong>內建 <code>yarn scheduler</code> 支援背景定時輪詢與資料快取維護。</li>
          </ul>
        </div>

        <!-- lah-messenger-server -->
        <div class="project-card">
          <div class="project-card-header">
            <span class="project-card-title">3. lah-messenger-server (即時通伺服端)</span>
            <span class="project-tech">Node.js / ws ^7.4.6 / SQLite</span>
          </div>
          <ul class="project-detail-list">
            <li><strong>通訊核心：</strong>Node.js CommonJS 架構，採用 <code>ws</code> 打造低延遲長連線通訊（Port 8081）。</li>
            <li><strong>附件服務：</strong>Express 4 + Multer 提供高效率附件上傳、縮圖生成與下載服務（Port 8082）。</li>
            <li><strong>分庫架構：</strong>採用獨立 SQLite 頻道分庫（<code>db/&lt;channel&gt;.db</code>），達到群組訊息實體物理隔離。</li>
            <li><strong>守護行程：</strong>生產環境由可攜版 PM2 守護，崩潰後具備 0 秒自動重啟機制。</li>
          </ul>
        </div>

        <!-- lah-messenger -->
        <div class="project-card">
          <div class="project-card-header">
            <span class="project-card-title">4. lah-messenger (桌面即時通客戶端)</span>
            <span class="project-tech">Electron 39 / Nuxtron / Vue 2</span>
          </div>
          <ul class="project-detail-list">
            <li><strong>核心技術：</strong>Electron 39.8.10 + Nuxtron 整合框架，提供視窗渲染與桌面原生體驗。</li>
            <li><strong>系統整合：</strong>支援 Windows 系統匣常駐 (Tray)、開機自啟、多螢幕通知視窗提示。</li>
            <li><strong>離線快取：</strong>離線時提供本機對話紀錄檢視與自動斷線重試機制。</li>
            <li><strong>打包支援：</strong>支援 <code>npm run build:win</code> 自動產出 32 位元與 64 位元離線安裝包。</li>
          </ul>
        </div>
      </div>

      <!-- LAH-Updater -->
      <div class="project-card" style="margin-top: 20px; border-color: var(--accent-purple);">
        <div class="project-card-header">
          <span class="project-card-title">5. LAH-Updater (系統自動更新與環境治理套件)</span>
          <span class="project-tech" style="color: #c084fc; border-color: rgba(192, 132, 252, 0.3);">Tauri 2 (Rust) / 零 CDN 離線純前端 / Python pack.py</span>
        </div>
        <p style="font-size: 0.9rem; color: var(--text-light); margin-bottom: 10px;">
          作為全系統的維運發布中心，整合自動更新（updater）、可攜 Node 安裝（node-portable）、自動備份（backuper）、解除安裝（uninstaller）、管理儀表板（dashboard）、簡訊設定工具（sms-env-editor）全系列桌面原生工具與全系統打包腳本（pack.py）。
        </p>
      </div>
    </section>

    <!-- 第 3 區：LAH-Updater 七大維運與自動化模組實作詳解 -->
    <section id="updater-overview" class="section-card">
      <h2 class="section-title">
        <span>🚀</span> LAH-Updater 七大維運與自動化模組實作詳解
      </h2>
      <p class="section-intro">
        點擊下方頁籤切換檢視各獨立工具的架構設計、核心管線、防呆自癒與操作指引：
      </p>

      <!-- 頁籤按鈕 -->
      <div class="tabs-header">
        <button class="tab-btn active" onclick="switchTab('tab-updater')">1. 自動更新工具 (updater)</button>
        <button class="tab-btn" onclick="switchTab('tab-portable')">2. 可攜 Node 工具 (node-portable)</button>
        <button class="tab-btn" onclick="switchTab('tab-uninstaller')">3. 解除安裝工具 (uninstaller)</button>
        <button class="tab-btn" onclick="switchTab('tab-backuper')">4. 自動備份工具 (backuper)</button>
        <button class="tab-btn" onclick="switchTab('tab-pack')">5. 原始碼打包 (pack.py)</button>
        <button class="tab-btn" onclick="switchTab('tab-sms')">6. 簡訊設定工具 (sms-env-editor)</button>
        <button class="tab-btn" onclick="switchTab('tab-dashboard')">7. 管理儀表板 (dashboard)</button>
      </div>

      <!-- Tab 1: updater -->
      <div id="tab-updater" class="tab-content active">
        <h3 style="color: var(--accent-cyan); margin-bottom: 10px;">智慧控管與監控系統 - 自動更新工具 (updater)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/智慧控管與監控系統 - 自動更新工具.exe</code> (約 11.86 MB)<br>
          核心職責：一鍵覆蓋更新應用、自動配置 Web 沙盒服務 (LAH_Web_Service)、VC++ 依賴補齊、PHP 7.4 / PHP 8.5 FCGI Proxy、套用資安加固、接管可攜 Node 16 與 PM2 零全域污染沙盒服務 (LAH_PM2_Service)。
        </p>

        <div class="diagram-container">
          {SVG_PIPELINE}
          <div class="diagram-caption">圖 2：自動更新工具 5 階段自動化執行管線 (5-Stage Auto-Update Pipeline)</div>
        </div>

        <div class="grid-2">
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🌟 核心防呆與自癒特點</h4>
            <ul class="project-detail-list">
              <li><strong>頂層目錄自動剝除：</strong>壓縮包內不論是否包含頂層資料夾（如包含 <code>Apache24/</code> 或純檔案），自動偵測並對齊解壓至正確層級。</li>
              <li><strong>巢狀雙層目錄自癒：</strong>解壓後自動掃描 <code>node/node</code>、<code>php7/php7</code> 等雙層巢狀目錄，自動平移提升並清除孤立空資料夾。</li>
              <li><strong>行程安全釋放：</strong>在解壓縮覆蓋前，自動停止服務並終止 PM2 Daemon，避免 Windows 下 <code>node.exe</code> 被鎖定造成寫入失敗。</li>
            </ul>
          </div>
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🛡️ 資安強化與防禦機制</h4>
            <ul class="project-detail-list">
              <li><strong>CVE-2024-4577 緩解：</strong>自動於 <code>httpd.conf</code> 注入 RewriteRule 阻擋 Windows CGI 參數注入攻擊。</li>
              <li><strong>資訊洩漏隱藏：</strong>配置 <code>ServerTokens Prod</code> 與 <code>expose_php = Off</code>，遮蔽伺服器軟體版本資訊。</li>
              <li><strong>PHP2Shell 緩解：</strong>強制禁用 <code>allow_url_include</code> 與危險命令執行函式。</li>
            </ul>
          </div>
        </div>
      </div>

      <!-- Tab 2: node-portable -->
      <div id="tab-portable" class="tab-content">
        <h3 style="color: var(--accent-purple); margin-bottom: 10px;">智慧控管與監控系統 - 可攜 Node 安裝工具 (node-portable)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/智慧控管與監控系統 - 可攜 Node 安裝工具.exe</code> (約 11.50 MB)<br>
          核心職責：專注於極速離線安裝綠色版 Node.js 16.20.2 與 PM2 5.3.1，採零全域污染沙盒隔離模式，建立開機自啟 WinSW 系統服務 (<code>PM2_Portable_Service</code>)，並掛載 <code>pm2-logrotate</code>。
        </p>

        <div class="grid-2">
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🔧 模組功能清單</h4>
            <ul class="project-detail-list">
              <li><strong>零全域污染沙盒模式：</strong>絕不更動系統全域環境變數，自動清理登錄檔舊殘留並生成專屬維運終端 <code>lah-cmd.bat</code>。</li>
              <li><strong>PM2_HOME 統一：</strong>將全域 PM2 根目錄導向至 <code>&lt;root&gt;/.pm2</code>，徹底杜絕多版本衝突。</li>
              <li><strong>pm2-logrotate 預先掛載：</strong>在 Node 解壓時即預先就緒日誌輪替模組，支援自動 gzip 壓縮與自訂保留天數。</li>
            </ul>
          </div>
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🔨 便捷編譯與產出</h4>
            <div class="code-block">
              <code># 在 LAH-Updater 目錄下執行
建立可攜NODE工具.bat
# 或執行 Python 建置腳本
python tools/node-portable/build.py</code>
              <button class="copy-btn" onclick="copyCode(this)">複製</button>
            </div>
            <p style="font-size: 0.8rem; color: var(--text-muted);">編譯後產出自動複製至 <code>dist/智慧控管與監控系統 - 可攜 Node 安裝工具.exe</code>。</p>
          </div>
        </div>
      </div>

      <!-- Tab 3: uninstaller -->
      <div id="tab-uninstaller" class="tab-content">
        <h3 style="color: var(--accent-rose); margin-bottom: 10px;">智慧控管與監控系統 - 解除安裝工具 (uninstaller)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/智慧控管與監控系統 - 自動移除工具.exe</code> (約 10.71 MB)<br>
          核心職責：完全安全地註銷 LAH_Web_Service 與 LAH_PM2_Service 系統服務、終止後台進程、內置二次確認彈窗防呆並安全清理系統安裝目錄。
        </p>
        <ul class="project-detail-list">
          <li><strong>服務註銷：</strong>自動執行 <code>winsw uninstall</code> 移除 <code>LAH_Web_Service</code> 與 <code>LAH_PM2_Service</code>，並向下相容清理舊版 <code>Apache2.4</code>。</li>
          <li><strong>擬真確認：</strong>內置自訂二次確認對話框，清楚列出即將執行的操作項目清單，支援 Esc 與遮罩取消。</li>
          <li><strong>行程終止：</strong>精準終止該安裝目錄下運行的 <code>node.exe</code>、<code>httpd.exe</code> 與 <code>php-cgi.exe</code>，不干擾其他獨立程序。</li>
        </ul>
      </div>

      <!-- Tab 4: backuper -->
      <div id="tab-backuper" class="tab-content">
        <h3 style="color: var(--accent-amber); margin-bottom: 10px;">智慧控管與監控系統 - 自動備份工具 (backuper)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/智慧控管與監控系統 - 備份與還原工具.exe</code> (約 11.09 MB)<br>
          核心職責：在執行重大升級或移轉前，將自訂組態、SQLite 資料庫、環境變數與重要設定檔打包為時間戳記壓縮包。
        </p>
        <ul class="project-detail-list">
          <li><strong>備份範圍：</strong><code>Apache24/conf/httpd.conf</code>、<code>php7/php.ini</code>、<code>php85/php.ini</code>、<code>*.db</code> (SQLite 資料庫)、各子專案 <code>.env</code>、Oracle 驅動、自訂資料庫 DLL 與 <code>lah-cmd.bat</code>。</li>
          <li><strong>相容還原：</strong>產出之備份包（<code>backup_YYYYMMDD_HHMMSS.zip</code>）可直接由自動更新工具勾選「從備份還原自訂配置」一鍵回補。</li>
        </ul>
      </div>

      <!-- Tab 5: pack -->
      <div id="tab-pack" class="tab-content">
        <h3 style="color: #38bdf8; margin-bottom: 10px;">系統原始碼打包腳本 (pack.py)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          腳本位置：<code>LAH-Updater/pack/pack.py</code> (由 <code>打包智慧控管及監控系統.bat</code> 呼叫)<br>
          核心職責：採用 Python 3.10+ 標準庫（零第三方相依套件），自動對工作區子專案執行規範打包（含自動封裝 VC++ 執行期 <code>vcredist-x64.zip</code>），並依照安裝包類型<strong>自動三向分流至 dist 目錄</strong>。
        </p>
        <div class="diagram-container">
          {SVG_PACK_FLOW}
          <div class="diagram-caption">圖 3：pack.py 三向自動分流與更新器端對齊探測流向圖</div>
        </div>
      </div>

      <!-- Tab 6: sms -->
      <div id="tab-sms" class="tab-content">
        <h3 style="color: #ec4899; margin-bottom: 10px;">地籍異動簡訊參數設定工具 (sms-env-editor)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/地籍異動簡訊參數設定工具.exe</code> (約 10.5 MB)<br>
          核心職責：專為內政部地籍異動簡訊與郵件發送服務（SMSWEB_MOI）提供可視化設定介面，支援 BASE64 自動加解密、Big-5/CP950 編碼無損相容、變更差異預覽與自動建立時間戳備份。
        </p>
        <div class="grid-2">
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🌟 核心功能與防呆設計</h4>
            <ul class="project-detail-list">
              <li><strong>預設連線路徑：</strong>預設自動載入 <code>\\\\220.1.34.221\\C$\\SMSWEB_MOI\\env.properties</code>，亦支援自由切換本機與網路磁碟。</li>
              <li><strong>BASE64 自動編解碼：</strong>簡訊帳密、郵件帳密、Oracle 帳密與連線 URL 於讀取時自動解密呈現，儲存時自動加密寫回。</li>
              <li><strong>Big-5 繁體中文支援：</strong>完美相容 Java 舊版程式對 Big-5 / CP950 編碼之嚴格要求，不破壞既有參數結構。</li>
              <li><strong>差異預覽與備份：</strong>儲存前彈出變更差異（Diff）確認彈窗，寫入前自動於同目錄建立時間戳記備份。</li>
            </ul>
          </div>
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🔨 便捷編譯與產出</h4>
            <div class="code-block">
              <code># 在 LAH-Updater 目錄下執行
建立簡訊設定工具.bat
# 或執行 Python 建置腳本
python tools/sms-env-editor/build.py</code>
              <button class="copy-btn" onclick="copyCode(this)">複製</button>
            </div>
            <p style="font-size: 0.8rem; color: var(--text-muted);">編譯後產出自動複製至 <code>dist/地籍異動簡訊參數設定工具.exe</code>。</p>
          </div>
        </div>
      </div>

      <!-- Tab 7: dashboard -->
      <div id="tab-dashboard" class="tab-content">
        <h3 style="color: #38bdf8; margin-bottom: 10px;">智慧控管與監控系統 - 管理儀表板 (dashboard)</h3>
        <p style="font-size: 0.92rem; color: var(--text-light); margin-bottom: 14px;">
          執行檔產物：<code>dist/智慧控管與監控系統 - 管理儀表板.exe</code> (約 11.20 MB)<br>
          核心職責：整合 Windows 系統服務監控與控制、PM2 應用進程層控管、動態即時日誌串流檢視（Auto-Tail、關鍵字即時篩選、記事本開啟）、主機資源指標（CPU、RAM、磁碟容量）與五大連接埠（80, 8080, 8081, 8082, 8085）健康探針之全功能原生管理儀表板。
        </p>
        <div class="grid-2">
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🌟 核心功能與視圖架構</h4>
            <ul class="project-detail-list">
              <li><strong>雙軌服務控管：</strong>統一管控 Windows 服務層（<code>LAH_Web_Service</code>、<code>LAH_PM2_Service</code>）與 PM2 進程層（<code>LAH-FE</code>、<code>LAH-WSS</code>、<code>LAH-SCHEDULER</code>、<code>lah-api-v85</code>），支援個別啟動/停止/重啟與頂部一鍵全域重啟。</li>
              <li><strong>即時串流日誌：</strong>自動探測全系統各模組日誌，支援 Auto-Tail 動態追蹤、關鍵字即時篩選、清空畫面與以記事本直接開啟實體日誌檔案。</li>
              <li><strong>指標與健康探針：</strong>主機 CPU 總負載、實體記憶體使用率、安裝磁碟剩餘空間，外加五大連接埠（80, 8080, 8081, 8082, 8085）即時 TCP 心跳檢查。</li>
              <li><strong>快捷維運箱：</strong>一鍵喚醒 <code>lah-cmd.bat</code> 沙盒終端、前端入口、Apache 組態語法驗證（<code>httpd -t</code>）與日誌目錄。</li>
            </ul>
          </div>
          <div>
            <h4 style="color: #ffffff; margin-bottom: 8px;">🔨 便捷編譯與產出</h4>
            <div class="code-block">
              <code># 在 LAH-Updater 目錄下執行
建立管理儀表板工具.bat
# 或執行 Python 建置腳本
python tools/dashboard/build.py</code>
              <button class="copy-btn" onclick="copyCode(this)">複製</button>
            </div>
            <p style="font-size: 0.8rem; color: var(--text-muted);">編譯後產出自動複製至 <code>dist/智慧控管與監控系統 - 管理儀表板.exe</code>，若本機存在 <code>C:\\AppServ</code> 亦同步部署。</p>
          </div>
        </div>
      </div>
    </section>

    <!-- 第 4 區：安裝更新包規格與目錄分流規範 -->
    <section id="directory-specs" class="section-card">
      <h2 class="section-title">
        <span>📁</span> 安裝更新包規格與目錄分流規範
      </h2>
      <p class="section-intro">
        為提升部署維運效率與分流管理，所有更新壓縮包均依照檔案特性與更新頻率歸納至三個標準目錄：
      </p>

      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>存放子目錄</th>
              <th>壓縮包名稱</th>
              <th>對應安裝目錄</th>
              <th>預設期望資料夾</th>
              <th>說明與更新週期</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><code>dist/patch/</code></td>
              <td><code>www.zip</code></td>
              <td><code>&lt;root&gt;\\www</code></td>
              <td><code>www</code></td>
              <td>PHP 7 後端業務核心與舊版 HTML 頁面（高頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/patch/</code></td>
              <td><code>LAH-API.zip</code></td>
              <td><code>&lt;root&gt;\\LAH-API</code></td>
              <td><code>LAH-API</code></td>
              <td>PHP 8 後端 RESTful API 核心（中頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/patch/</code></td>
              <td><code>LAH-NUXTJS.zip</code></td>
              <td><code>&lt;root&gt;\\LAH-NUXTJS</code></td>
              <td><code>LAH-NUXTJS</code></td>
              <td>智慧管控前端應用原始碼與 Nuxt 編譯產物（高頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/patch/</code></td>
              <td><code>lah-messenger-server.zip</code></td>
              <td><code>&lt;root&gt;\\lah-messenger-server</code></td>
              <td><code>lah-messenger-server</code></td>
              <td>即時通訊 WebSocket / HTTP 附件伺服器原始碼（中頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/node_modules/</code></td>
              <td><code>LAH-NUXTJS-node_modules.zip</code></td>
              <td><code>&lt;root&gt;\\LAH-NUXTJS\node_modules</code></td>
              <td><code>node_modules</code></td>
              <td>前端網站核心依賴套件（體積較大，低頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/node_modules/</code></td>
              <td><code>lah-messenger-server-node_modules.zip</code></td>
              <td><code>&lt;root&gt;\\lah-messenger-server\node_modules</code></td>
              <td><code>node_modules</code></td>
              <td>即時通伺服端核心依賴套件（低頻更新）</td>
            </tr>
            <tr>
              <td><code>dist/base/</code></td>
              <td><code>Apache24.zip</code></td>
              <td><code>&lt;root&gt;\\Apache24</code></td>
              <td><code>Apache24</code></td>
              <td>Apache 2.4 網頁伺服器執行環境（基礎環境）</td>
            </tr>
            <tr>
              <td><code>dist/base/</code></td>
              <td><code>php7.zip</code></td>
              <td><code>&lt;root&gt;\\php7</code></td>
              <td><code>php7</code></td>
              <td>PHP 7.4 執行期與 OCI8 擴充套件（基礎環境）</td>
            </tr>
            <tr>
              <td><code>dist/base/</code></td>
              <td><code>php85.zip</code></td>
              <td><code>&lt;root&gt;\\php85</code></td>
              <td><code>php85</code></td>
              <td>PHP 8.5 執行期與 FastCGI 套件（基礎環境）</td>
            </tr>
            <tr>
              <td><code>dist/base/</code></td>
              <td><code>node.zip</code></td>
              <td><code>&lt;root&gt;\node</code></td>
              <td><code>node</code></td>
              <td>Node.js 16.20.2 綠色版 + PM2 + pm2-logrotate（可攜環境）</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="alert-box alert-success">
        <div>🛡️</div>
        <div>
          <strong>多目錄自動探測相容性：</strong>
          自動更新工具具備智慧型路徑解析功能，探測順序優先為指定子目錄（如 <code>dist\\patch\\www.zip</code>），若不存在則自動回退探測同層目錄（<code>dist\\www.zip</code>）與 <code>C:\\Temp</code>，確保維運人員操作無論放置於何處皆能自動辨識！
        </div>
      </div>
    </section>

    <!-- 第 5 區：四重隔離與資安升級防護 -->
    <section id="quadruple-isolation" class="section-card">
      <h2 class="section-title">
        <span>🔒</span> 四重隔離與全域 Node.js 升級防護機制
      </h2>
      <p class="section-intro">
        地政伺服器可能因機關資安弱點掃描（CVE）要求將系統全域 Node.js 升級至 Node 22 或更高版本。為保證本系統核心 Nuxt 2 (Vue 2) 不受新版 Node 語法與引擎破壞，全工具導入四重隔離架構：
      </p>

      <div class="diagram-container">
        {SVG_ISOLATION}
        <div class="diagram-caption">圖 4：可攜式 Node.js 16.20.2 與全域升級四重隔離機制防護架構</div>
      </div>

      <div class="grid-2">
        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">隔離 1：WinSW 服務實體絕對路徑綁定</h4>
          <p style="font-size: 0.88rem; color: var(--text-light);">
            在 <code>lah_pm2_service.xml</code> 中，服務執行器直接鎖定為 <code>&lt;root_dir&gt;\\node\\node.exe</code> 與 <code>&lt;root_dir&gt;\\node\\node_modules\\pm2\\bin\\pm2</code>。作業系統開機啟動服務時，直接載入該專屬執行檔，絕不透過全域 <code>PATH</code> 尋找，即使全域 Node 升級至 v22，服務依舊穩定運行於 v16。
          </p>
        </div>

        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">隔離 2：專案獨立 PM2_HOME 邊界</h4>
          <p style="font-size: 0.88rem; color: var(--text-light);">
            強制作業系統環境變數 <code>PM2_HOME = &lt;root_dir&gt;\\.pm2</code>。所有 PM2 Daemon 行程、pid 鎖定檔、模組配置與 dump.pm2 存檔皆收斂於專案目錄內，完全脫鉤全域 <code>C:\\ProgramData\\pm2</code> 或使用者家目錄 <code>%USERPROFILE%\\.pm2</code>。
          </p>
        </div>

        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">隔離 3：雙層登錄檔環境變數治理 (HKLM + HKCU)</h4>
          <p style="font-size: 0.88rem; color: var(--text-light);">
            工具執行時會自動巡檢系統層級 (<code>HKLM\\...\\Environment</code>) 與使用者層級 (<code>HKCU\\Environment</code>) 之 <code>PATH</code>。自動移除指向 <code>C:\\Program Files\\nodejs</code> 之殘留路徑，並將 <code>&lt;root_dir&gt;\\node</code> 置於首位，確保命令提示字元直接呼叫可攜版。
          </p>
        </div>

        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">隔離 4：隨附預載 pm2-logrotate 與日誌壓縮</h4>
          <p style="font-size: 0.88rem; color: var(--text-light);">
            在 Node 解壓時直接將隨附的 <code>node_modules\\pm2-logrotate</code> 連結至 PM2 模組目錄，並預先將 <code>compress: true</code> 與 <code>max_size: 10M</code> 寫入 <code>module_conf.json</code>，無需連外聯網 <code>pm2 install</code> 即可離線立即啟用日誌自動輪替與壓縮。
          </p>
        </div>
      </div>
    </section>

    <!-- 第 6 區：實機崩潰根因與模組防禦機制 -->
    <section id="crash-troubleshooting" class="section-card">
      <h2 class="section-title">
        <span>⚠️</span> 實機安裝崩潰根因排查與防禦機制
      </h2>
      <p class="section-intro">
        在實際伺服器環境中，曾發生安裝可攜 Node 工具執行至 86% 時發生崩潰（<code>pm2 -v</code> 驗證失敗）。經過深入排查與程式碼根治，已建立全方位自動防禦：
      </p>

      <div class="alert-box alert-warning">
        <div>🔍</div>
        <div>
          <strong>錯誤根因分析：</strong><br>
          1. <strong>舊全域殘留模組：</strong>伺服器先前曾以舊版 <code>pm2-installer</code> 安裝，在舊全域 <code>module_conf.json</code> 中登記了第三方分支模組 <code>@jessety/pm2-logrotate</code>。<br>
          2. <strong>存檔遷移觸發異常：</strong>當工具將舊設定遷移至 <code>.pm2</code> 後，呼叫 <code>pm2 -v</code> 會拉起 PM2 守護程序，Modularizer 讀取 <code>module-db-v2</code> 並嘗試啟動所有登記模組。<br>
          3. <strong>缺乏防禦崩潰：</strong>PM2 原生 <code>NPM.js</code> 在啟動模組時直接 <code>require(package.json)</code>，因本機並無 <code>@jessety/pm2-logrotate</code>，丟出未捕獲之 <code>MODULE_NOT_FOUND</code> 致命異常導致 Daemon 崩潰。
        </div>
      </div>

      <h3 style="color: #ffffff; margin: 16px 0 10px 0;">🛡️ 具體修復方案（已實作於 updater 與 node-portable）</h3>
      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>防禦環節</th>
              <th>實作位置</th>
              <th>具體機制說明</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>自動淨化舊設定</strong></td>
              <td><code>src-tauri/src/pm2.rs</code><br><code>wire_bundled_pm2_logrotate</code></td>
              <td>主動檢測並移除 <code>module_conf.json</code> 與 <code>module-db-v2</code> 中之 <code>@jessety/pm2-logrotate</code>，並自動將舊自訂參數無損繼承至官方標準 <code>pm2-logrotate</code>。</td>
            </tr>
            <tr>
              <td><strong>孤立模組清理</strong></td>
              <td><code>src-tauri/src/pm2.rs</code><br><code>service_runner.js</code></td>
              <td>比對 <code>module-db-v2</code> 登記之模組清單，若在磁碟上完全不存在對應 <code>package.json</code>，自動從登記表中剔除，防止幽靈模組阻礙啟動。</td>
            </tr>
            <tr>
              <td><strong>NPM.js 容錯防禦</strong></td>
              <td><code>src-tauri/src/pm2.rs</code><br><code>patch_pm2_runtime_in_place</code></td>
              <td>於 PM2 核心 <code>NPM.js</code> 注入檢查：若模組目錄不存在則印出警告並略過；將 <code>require()</code> 封裝於 <code>try { ... } catch (e)</code>，防止缺失模組拖垮守護程序！</td>
            </tr>
            <tr>
              <td><strong>單元測試覆蓋</strong></td>
              <td><code>test_wire_bundled_pm2_logrotate_idempotent</code></td>
              <td>模擬包含 <code>@jessety/pm2-logrotate</code> 與幽靈模組之損毀設定，斷言清理、參數繼承與冪等執行 100% 正確通過。</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- 第 7 區：本機快捷維運指令清單 -->
    <section id="quick-scripts" class="section-card">
      <h2 class="section-title">
        <span>⚡</span> 本機開發與維運快速指令清單
      </h2>
      <p class="section-intro">
        工作區根目錄與 <code>LAH-Updater</code> 內建完整的一鍵批次檔，免去繁瑣的終端指令：
      </p>

      <div class="grid-2">
        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">工作區根目錄常用腳本</h4>
          <ul class="project-detail-list">
            <li><code>start_dev.bat</code>：一鍵啟動後端 API、前端 Nuxt 伺服器與即時通服務，並自動開啟瀏覽器至 <code>http://127.0.0.1:8080</code>。</li>
            <li><code>stop_dev.bat</code>：一鍵安全釋放 Port 80, 8080, 8081, 8082 占用進程。</li>
            <li><code>git-pull.bat</code>：批次同步根目錄與底下五個子專案的最新程式碼。</li>
          </ul>
        </div>

        <div class="project-card">
          <h4 style="color: var(--accent-cyan); margin-bottom: 8px;">LAH-Updater 建置與打包指令</h4>
          <ul class="project-detail-list">
            <li><code>打包智慧控管及監控系統.bat</code>：全系統原始碼與依賴打包，自動輸出至 <code>dist/patch</code>, <code>dist/node_modules</code>, <code>dist/base</code>。</li>
            <li><code>建立自動更新工具.bat</code>：編譯 Tauri 2 自動更新工具二進位檔。</li>
            <li><code>建立可攜NODE工具.bat</code>：編譯可攜式 Node 安裝工具。</li>
            <li><code>建立解除安裝工具.bat</code>：編譯系統安全解除安裝工具。</li>
            <li><code>建立自動備份工具.bat</code>：編譯系統重要組態自動備份工具。</li>
            <li><code>建立管理儀表板工具.bat</code>：編譯智慧控管服務管理儀表板。</li>
            <li><code>建立簡訊設定工具.bat</code>：編譯地籍異動簡訊參數設定工具。</li>
            <li><code>校準現有服務資安脫敏名稱.bat</code>：免重裝服務一鍵線上熱校準 services.msc 顯示名稱與描述。</li>
          </ul>
        </div>
      </div>
    </section>

    <!-- 頁尾 -->
    <footer>
      <p>智慧控管與監控系統 (LAH-Workspace) ‧ 系統維運手冊 ‧ 純 HTML5/CSS3/SVG/Vanilla JS 離線打造</p>
      <p style="margin-top: 6px; font-size: 0.78rem; color: #64748b;">文件版本：2026.10 ‧ 適用環境：Windows Server 2012R2 / 2016 / 2019 / 2022 / Windows 10 & 11</p>
    </footer>

  </main>

  <!-- 原生 JavaScript (零依賴純原生互動) -->
  <script>
    // 頁籤切換器
    function switchTab(tabId) {{
      const contents = document.querySelectorAll('.tab-content');
      const buttons = document.querySelectorAll('.tab-btn');
      
      contents.forEach(c => c.classList.remove('active'));
      buttons.forEach(b => {{
        b.classList.remove('active');
        if (b.getAttribute('onclick') && b.getAttribute('onclick').includes(tabId)) {{
          b.classList.add('active');
        }}
      }});

      const target = document.getElementById(tabId);
      if (target) {{
        target.classList.add('active');
      }}
    }}

    // 一鍵代碼複製
    function copyCode(btn) {{
      const codeBlock = btn.parentElement.querySelector('code');
      if (!codeBlock) return;
      const text = codeBlock.innerText;
      navigator.clipboard.writeText(text).then(() => {{
        const originalText = btn.innerText;
        btn.innerText = '已複製！';
        btn.style.backgroundColor = '#10b981';
        btn.style.color = '#ffffff';
        setTimeout(() => {{
          btn.innerText = originalText;
          btn.style.backgroundColor = '';
          btn.style.color = '';
        }}, 2000);
      }});
    }}

    // 側邊欄即時關鍵字篩選
    const filterInput = document.getElementById('filterInput');
    filterInput.addEventListener('input', function(e) {{
      const query = e.target.value.toLowerCase().trim();
      const navLinks = document.querySelectorAll('#navList .nav-link');
      
      navLinks.forEach(link => {{
        const text = link.innerText.toLowerCase();
        const parent = link.parentElement;
        if (text.includes(query) || query === '') {{
          parent.style.display = 'block';
        }} else {{
          parent.style.display = 'none';
        }}
      }});
    }});

    // 滾動高亮目錄 (ScrollSpy)
    window.addEventListener('scroll', () => {{
      const sections = document.querySelectorAll('section');
      const navLinks = document.querySelectorAll('.nav-link');
      
      let currentSectionId = '';
      sections.forEach(section => {{
        const sectionTop = section.offsetTop - 100;
        if (window.pageYOffset >= sectionTop) {{
          currentSectionId = section.getAttribute('id');
        }}
      }});

      navLinks.forEach(link => {{
        link.classList.remove('active');
        if (link.getAttribute('href') === '#' + currentSectionId) {{
          link.classList.add('active');
        }}
      }});
    }});
  </script>
</body>
</html>
"""
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        f.write(html_content)
    print(f"成功產出: {OUTPUT_FILE} (大小: {len(html_content.encode('utf-8'))} 位元組)")

    # 驗證無外部 CDN 與 URL (除 SVG 命名空間 http://www.w3.org 外)
    import re
    external_urls = [u for u in re.findall(r'https?://[^\s"\'<>]+', html_content) if 'w3.org' not in u]
    print(f"外部資源連線數量: {len(external_urls)}")
    if external_urls:
        print("警告，偵測到外部連線:", external_urls)
    else:
        print("✅ 完美驗證：零外部連線，100% 離線純靜態手冊！")

if __name__ == '__main__':
    generate_html()
