const $ = (selector) => document.querySelector(selector);
const esc = (value) => String(value ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const pages = {
  main: ['已登案件', '已完成掃描入列，等待搬移的案件通知。'],
  wait: ['待傳案件', '等待派送與重試的通知；選取資料可閱讀完整內容。'],
  log: ['傳送紀錄', '查詢歷史派送結果、收件對象與錯誤原因。'],
  messages: ['傳送訊息', '查看本次執行的操作紀錄，最多保留 200 筆。'],
  logs: ['日誌追蹤', '即時檢視今日寫入磁碟的系統日誌檔案（支援自動追蹤與關鍵字篩選）。'],
  settings: ['系統設定', '管理 LAH-API 連線、排程與派送閘道設定。'],
};
let page = 'wait';
let snapshot;
let draft;
let dirty = false;
let unlistenClose;
let busy = false;
let sequence = 0;
let records = [];
let selected;
let pausedMessages = false;
let visibleActivities = [];
let logAutoRefresh = true;
let logFilter = '';
let logPollTimer;
let lastLogTail;
function todayRocDate() {
  const now = new Date();
  const rocYear = String(now.getFullYear() - 1911).padStart(3, '0');
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${rocYear}/${month}/${day}`;
}
let query = { keyword: '', channel: '', status: '', date: todayRocDate(), sort_by: 'time', ascending: false };
let toastTimer;
let promptedApiSetup = false;
const THEME_STORAGE_KEY = 'moi-sms-theme';
const THEME_ICONS = {
  dark: '<svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg><span>亮色模式</span>',
  light: '<svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg><span>深色模式</span>',
};

function getSavedTheme() {
  try {
    return localStorage.getItem(THEME_STORAGE_KEY) === 'dark' ? 'dark' : 'light';
  } catch {
    return 'light';
  }
}

function applyTheme(theme) {
  const next = theme === 'dark' ? 'dark' : 'light';
  document.documentElement.dataset.theme = next;
  const btn = $('#theme-toggle');
  if (btn) {
    btn.setAttribute('aria-pressed', String(next === 'dark'));
    btn.innerHTML = THEME_ICONS[next];
  }
  try {
    localStorage.setItem(THEME_STORAGE_KEY, next);
  } catch {
    // 忽略儲存限制
  }
}

function extractLahApiHost(urlStr) {
  const value = String(urlStr || '').trim();
  if (!value) return '';
  try {
    const parsed = new URL(/^https?:\/\//i.test(value) ? value : `http://${value}`);
    return parsed.protocol === 'https:' ? `https://${parsed.host}` : parsed.host;
  } catch {
    return value;
  }
}

function normalizeLahApiUrl(raw) {
  const value = String(raw || '').trim();
  if (!value) return '';
  const withScheme = /^https?:\/\//i.test(value) ? value : `http://${value}`;
  try {
    const parsed = new URL(withScheme);
    if (!parsed.hostname) return '';
    if (
      !parsed.pathname ||
      parsed.pathname === '/' ||
      /^\/(?:LAH-API\/)?api\/sms_v1_api\.php\/?$/i.test(parsed.pathname)
    ) {
      parsed.pathname = '/LAH-API/v1/sms_v1_api.php';
    }
    return parsed.toString();
  } catch {
    return '';
  }
}

function badge(value) {
  const style = value === '成功' ? 'good' : value === '失敗' ? 'bad' : 'warn';
  return `<span class="badge ${style}">${esc(value)}</span>`;
}

function toast(message, error = false) {
  const element = $('#toast');
  element.textContent = message;
  element.className = `toast${error ? ' error-toast' : ''}`;
  element.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { element.hidden = true; }, 6000);
}

function setBusy(value) {
  busy = value;
  $('#content').setAttribute('aria-busy', String(value));
  for (const button of document.querySelectorAll('#schedule, #sync, #refresh, .nav button, #content button[type="submit"], #test-lah-connection, #reload-env, #test-delivery, #cancel-settings, #enable-autostart, #clear-autostart')) button.disabled = value;
  for (const field of document.querySelectorAll('#settings-form input, #settings-form select, #settings-form textarea')) field.disabled = value;
}

async function invoke(command, args = {}) {
  if (!window.__TAURI__) throw new Error('請以 Tauri 執行檔開啟此介面。');
  return window.__TAURI__.core.invoke(command, args);
}

async function updateSnapshot() {
  snapshot = await invoke('get_snapshot');
  if (!dirty) draft = structuredClone(snapshot.settings);
  $('#schedule-state').textContent = `排程：${snapshot.schedule_phase || (snapshot.running ? '執行中' : '已暫停')}`;
  $('#schedule-state').className = `badge ${snapshot.running ? (snapshot.schedule_phase === '等待執行時段' ? 'warn' : 'good') : ''}`;
  const apiState = $('#api-state');
  if (apiState) {
    apiState.textContent = snapshot.site_label ? 'LAH-API：已連線' : 'LAH-API：未連線';
    apiState.className = `badge ${snapshot.site_label ? 'good' : 'warn'}`;
  }
  const apiAlert = $('#api-alert');
  if (apiAlert) apiAlert.hidden = Boolean(snapshot.site_label);
  if (snapshot.site_label) {
    promptedApiSetup = false;
  } else if (!promptedApiSetup) {
    promptedApiSetup = true;
    openApiSetupDialog();
  }
  $('#schedule-time').textContent = `執行時段 ${snapshot.settings.start_time}–${snapshot.settings.end_time}`;
  $('#schedule').textContent = snapshot.running ? '暫停排程' : '啟動排程';
  $('#main-count').textContent = snapshot.main_count;
  $('#wait-count').textContent = snapshot.wait_count;
  $('#log-count').textContent = snapshot.log_count ?? (snapshot.success_count + snapshot.failure_count);
  $('#messages-count').textContent = snapshot.activities.length;
  $('#last-update').textContent = `最後更新 ${new Date().toLocaleTimeString('zh-TW', { hour12: false })}`;
  $('#runtime-status').textContent = `排程狀態：${snapshot.schedule_phase || (snapshot.running ? '執行中' : '已暫停')}`;
}

async function action(work, { refresh = true } = {}) {
  if (busy) return;
  setBusy(true);
  try {
    const message = await work();
    if (refresh) {
      await updateSnapshot();
      if (page !== 'settings') await renderPage();
    }
    if (message) toast(message);
  } catch (error) {
    const info = describeLoadError(error);
    toast(info.isApiError ? `${info.title}：${info.summary}` : String(error), true);
  } finally {
    setBusy(false);
  }
}

function confirmDiscard(message) {
  return new Promise((resolve) => {
    const dialog = $('#confirm-dialog');
    $('#confirm-message').textContent = message;
    const finish = (result) => { dialog.close(); resolve(result); };
    $('#confirm-cancel').onclick = () => finish(false);
    $('#confirm-discard').onclick = () => finish(true);
    dialog.oncancel = (event) => { event.preventDefault(); finish(false); };
    dialog.showModal();
    $('#confirm-cancel').focus();
  });
}

async function setDirty(next) {
  dirty = next;
}

async function switchPage(next) {
  if (busy || page === next || $('#confirm-dialog').open) return;
  if (dirty && !(await confirmDiscard('切換頁面會捨棄尚未儲存的設定。'))) return;
  if (dirty) { await setDirty(false); draft = structuredClone(snapshot.settings); }
  if (logPollTimer) { clearInterval(logPollTimer); logPollTimer = undefined; }
  page = next;
  selected = undefined;
  query = { keyword: '', channel: '', status: '', date: todayRocDate(), sort_by: 'time', ascending: false };
  for (const tab of document.querySelectorAll('[data-page]')) {
    const active = tab.dataset.page === page;
    tab.classList.toggle('active', active);
    if (active) tab.setAttribute('aria-current', 'page'); else tab.removeAttribute('aria-current');
  }
  await action(async () => { await renderPage(); }, { refresh: false });
}

function filterView() {
  return `<form class="filters" id="filter-form">
    <label>通知日期（民國）<input name="date" value="${esc(query.date)}" placeholder="115/11/13" pattern="[0-9]{3}/[0-9]{2}/[0-9]{2}|[0-9]{7}" title="例如 115/11/13；可留空查詢全部資料"></label>
    <label>傳送管道<select name="channel"><option value="">全部管道</option><option value="sms" ${query.channel === 'sms' ? 'selected' : ''}>簡訊</option><option value="email" ${query.channel === 'email' ? 'selected' : ''}>郵件</option></select></label>
    ${page === 'log' ? `<label>傳送結果<select name="status"><option value="">全部結果</option>${['成功', '失敗'].map((s) => `<option ${query.status === s ? 'selected' : ''}>${s}</option>`).join('')}</select></label>` : ''}
    <label class="keyword">收件字號／收件者<input name="keyword" value="${esc(query.keyword)}" placeholder="例如 197020"></label>
    <button class="primary" type="submit">查詢</button><button type="button" id="clear-filter">清除</button>
  </form>`;
}

function sortIndicator(column) {
  if (query.sort_by !== column) return '';
  return query.ascending ? ' ↑' : ' ↓';
}

function ariaSort(column) {
  if (query.sort_by !== column) return 'none';
  return query.ascending ? 'ascending' : 'descending';
}

function tableView() {
  const prefix = page === 'main' ? '登錄' : page === 'wait' ? '待傳' : '傳送';
  return `<div class="tablewrap"><table aria-label="${pages[page][0]}"><thead><tr>
    <th>種類</th><th>收件年</th><th>收件字</th><th aria-sort="${ariaSort('number')}"><button id="sort-number" aria-label="依收件號切換排序">收件號${sortIndicator('number')}</button></th>
    <th>${prefix}日期</th><th aria-sort="${ariaSort('time')}"><button id="sort-time" aria-label="依${prefix}時間切換排序">${prefix}時間${sortIndicator('time')}</button></th><th>手機號碼</th><th>電子信箱</th><th>傳送種類</th><th>${page === 'wait' ? '已重試次數' : page === 'main' ? '案件狀態' : '傳送結果'}</th>
  </tr></thead><tbody>${records.length ? records.map((record) => `<tr data-record="${esc(record.id)}" tabindex="0" aria-selected="${record.id === selected}" class="${record.id === selected ? 'selected' : ''}">
    <td>${esc(record.kind)}</td><td>${esc(record.year)}</td><td>${esc(record.word)}</td><td>${esc(record.number)}</td><td>${formatDate(record.date)}</td><td>${esc(record.time)}</td>
    <td>${record.channel === 'sms' ? esc(record.recipient) : '—'}</td><td>${record.channel === 'email' ? esc(record.recipient) : '—'}</td><td>${record.channel === 'sms' ? '簡訊' : '郵件'}</td>
    <td>${page === 'wait' ? `${record.retry_count} 次` : badge(record.status)}</td></tr>`).join('') : `<tr><td colspan="10" class="empty">沒有符合條件的案件<br><small>可調整查詢條件，或按「清除」查看全部資料。</small></td></tr>`}</tbody></table></div>`;
}

function formatDate(value) {
  return esc(`${value.slice(0, 3)}/${value.slice(3, 5)}/${value.slice(5, 7)}`);
}

function detailView() {
  const record = records.find((r) => r.id === selected);
  if (!record) return '<h3>案件明細</h3><p class="muted">選取一筆案件以查看完整資訊。</p>';
  return `<div class="detailhead"><div><h3>${esc(record.subject)}</h3><p class="muted">${esc(record.year)} / ${esc(record.word)} / ${esc(record.number)}</p></div>${badge(record.status)}</div>
    <div class="detail-meta"><span>${record.channel === 'sms' ? '簡訊' : '郵件'}：${esc(record.recipient)}</span><span>${formatDate(record.date)} ${esc(record.time)}</span><span>已重試 ${record.retry_count} 次</span></div>
    ${record.error ? `<div class="inline-error">${esc(record.error)}</div>` : ''}
    <div class="body">${esc(record.message)}</div><button id="copy-message" type="button">複製通知內容</button>`;
}

function selectRecord(id, keyboard = false) {
  selected = id;
  for (const row of document.querySelectorAll('[data-record]')) {
    row.classList.toggle('selected', row.dataset.record === id);
    row.setAttribute('aria-selected', String(row.dataset.record === id));
  }
  $('#record-detail').innerHTML = detailView();
  bindCopy();
  if (keyboard) $('#record-detail').focus();
}

function bindCopy() {
  $('#copy-message')?.addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(records.find((r) => r.id === selected).message); toast('通知內容已複製。'); }
    catch { toast('請選取下方通知文字，再按 Ctrl+C 複製。'); }
  });
}

function describeLoadError(error, targetUrl = '') {
  const raw = String(error ?? '').trim() || '未知錯誤';
  const lower = raw.toLowerCase();
  const endpoint = targetUrl || snapshot?.settings?.lah_api_url || '';
  const host = extractLahApiHost(endpoint);
  if (lower.includes('10061') || raw.includes('拒絕連線') || lower.includes('connection failed')) {
    return {
      title: '無法連線至智慧控管系統（LAH-API）',
      summary: host
        ? `目標主機拒絕連線（${host}），目前無法取得案件資料。`
        : '目標主機拒絕連線，目前無法取得案件資料。',
      hints: [
        '確認智慧控管系統伺服器（Apache／PHP 8.5 FastCGI）已啟動。',
        '點擊下方「設定系統 IP」檢查主機 IP 與端點網址是否正確。',
        '若為本機執行，請確認本機服務已於 127.0.0.1 正常監聽。',
      ],
      isApiError: true,
      raw,
    };
  }
  if (lower.includes('timed out') || raw.includes('逾時')) {
    return {
      title: '智慧控管系統（LAH-API）連線逾時',
      summary: '伺服器回應時間過長，目前無法載入案件資料。',
      hints: [
        '請檢查網路連線品質與後端伺服器負載狀態。',
        '稍後點擊「重新連線」，或確認系統 IP 設定是否正確。',
      ],
      isApiError: true,
      raw,
    };
  }
  if (raw.includes('LAH-API') || lower.includes('http get') || lower.includes('http post')) {
    return {
      title: '智慧控管系統（LAH-API）存取失敗',
      summary: '無法從後端服務取得資料，請檢查服務狀態或端點設定。',
      hints: [
        '確認 LAH-API 服務運作正常且可回應請求。',
        '點擊「設定系統 IP」檢查端點位址是否正確。',
      ],
      isApiError: true,
      raw,
    };
  }
  return {
    title: '無法載入桌面服務',
    summary: '桌面程式初始化或讀取資料時發生異常。',
    hints: [
      '請點擊「重新載入」再試一次；若持續發生，請查看「日誌追蹤」或將技術細節提供給維運人員。',
    ],
    isApiError: false,
    raw,
  };
}

function renderLoadError(error, onRetry) {
  const info = describeLoadError(error);
  const content = $('#content');
  content.classList.remove('records-view');
  if (info.isApiError) {
    const apiAlert = $('#api-alert');
    if (apiAlert) apiAlert.hidden = true;
  }
  content.innerHTML = `<section class="load-error" role="alert">
    <div class="error-header">
      <svg aria-hidden="true" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
      <h2>${esc(info.title)}</h2>
    </div>
    <p class="error-summary">${esc(info.summary)}</p>
    <ul class="error-hints">${info.hints.map((hint) => `<li>${esc(hint)}</li>`).join('')}</ul>
    <div class="actions">
      <button class="primary" id="retry-load" type="button">${info.isApiError ? '重新連線' : '重新載入'}</button>
      ${info.isApiError ? '<button id="error-open-ip" type="button">設定系統 IP</button>' : ''}
      <button id="copy-error" type="button">複製錯誤資訊</button>
    </div>
    <details class="error-details">
      <summary>檢視技術細節（供維運人員排查）</summary>
      <pre class="error-raw">${esc(info.raw)}</pre>
    </details>
  </section>`;
  $('#retry-load').onclick = onRetry;
  $('#error-open-ip')?.addEventListener('click', openApiSetupDialog);
  $('#copy-error')?.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(info.raw);
      toast('錯誤資訊已複製。');
    } catch {
      toast('請展開技術細節選取文字，再按 Ctrl+C 複製。');
    }
  });
  return info;
}

async function renderRecords() {
  const request = ++sequence;
  const currentPage = page;
  let result;
  try {
    result = await invoke('query_records', { query: { ...query, table: currentPage } });
  } catch (error) {
    if (request !== sequence || page !== currentPage) return;
    renderLoadError(error, refresh);
    return;
  }
  if (request !== sequence || page !== currentPage) return;
  const oldScroll = $('.tablewrap')?.scrollTop || 0;
  records = result.items;
  if (!records.some((r) => r.id === selected)) selected = records[0]?.id;
  $('#content').innerHTML = `<div class="pagehead"><div><h2>${pages[page][0]}</h2><p>${pages[page][1]}</p></div><span class="muted count-label">符合 ${records.length} 筆／共 ${result.total} 筆</span></div>${filterView()}<div class="records-split">${tableView()}<aside id="record-detail" class="detail" tabindex="-1">${detailView()}</aside></div>`;
  $('.tablewrap').scrollTop = oldScroll;
  $('#filter-form').onsubmit = (event) => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    query = { ...query, date: values.get('date').trim(), keyword: values.get('keyword').trim(), channel: values.get('channel'), status: values.get('status') || '' };
    action(renderRecords, { refresh: false });
  };
  $('#clear-filter').onclick = () => { query = { keyword: '', date: '', channel: '', status: '', sort_by: 'time', ascending: false }; action(renderRecords, { refresh: false }); };
  const toggleSort = (column) => {
    if (query.sort_by === column) query.ascending = !query.ascending;
    else { query.sort_by = column; query.ascending = false; }
    action(renderRecords, { refresh: false });
  };
  $('#sort-number').onclick = () => toggleSort('number');
  $('#sort-time').onclick = () => toggleSort('time');
  for (const row of document.querySelectorAll('[data-record]')) {
    row.onclick = () => selectRecord(row.dataset.record);
    row.onkeydown = (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); selectRecord(row.dataset.record, true); } };
  }
  bindCopy();
}

function activityBadge(level) {
  if (level === 'success') return '<span class="badge good">完成</span>';
  if (level === 'error') return '<span class="badge bad">錯誤</span>';
  if (level === 'warn') return '<span class="badge warn">警告</span>';
  return '<span class="badge">資訊</span>';
}

function renderMessages() {
  if (!pausedMessages) visibleActivities = snapshot.activities;
  $('#content').innerHTML = `<div class="pagehead"><div><h2>傳送訊息</h2><p>${pages.messages[1]}</p></div><button id="pause-view">${pausedMessages ? '恢復畫面更新' : '暫停畫面更新'}</button></div>
    ${pausedMessages ? '<div class="notice">畫面更新已暫停；排程狀態不受影響。</div>' : ''}
    <div class="events">${visibleActivities.map((event) => `<article class="event"><time class="eventtime">${new Date(event.timestamp_ms).toLocaleTimeString('zh-TW', { hour12: false })}</time><div class="eventtext"><p><b>${esc(event.title)}</b></p><p class="activity-body">${esc(event.message)}</p></div>${activityBadge(event.level)}</article>`).join('') || '<p class="empty">本次尚無操作訊息。</p>'}</div>`;
  $('#pause-view').onclick = () => { pausedMessages = !pausedMessages; renderMessages(); };
}

function field(key, label, type = 'text', wide = false) {
  let limits = '';
  if (type === 'number') limits = `min="${key === 'search_minutes' ? 1 : 0}" max="${key === 'search_minutes' ? 1440 : 99}" step="1" required`;
  else if (['url', 'time'].includes(type)) limits = 'required';
  return `<label class="${wide ? 'wide' : ''}">${label}<input name="${key}" type="${type}" value="${esc(draft[key])}" ${limits} autocomplete="${type === 'password' ? 'new-password' : 'off'}"></label>`;
}

function renderSettings() {
  $('#content').innerHTML = `<div class="pagehead"><div><h2>系統設定</h2><p>${pages.settings[1]}</p></div><span class="badge" id="dirty-label">${dirty ? '尚未儲存' : '設定已載入'}</span></div>
    <form id="settings-form" class="settings">
    <section><h3>服務連線</h3><div class="fields"><label class="wide">智慧控管系統 IP<input name="lah_api_host" type="text" value="${esc(extractLahApiHost(draft.lah_api_url))}" placeholder="例如 127.0.0.1 或 192.168.1.100:8080" required autocomplete="off"></label><div class="wide actions"><button id="test-lah-connection" type="button">測試連線</button></div><p id="lah-site-label" class="wide muted">${esc(snapshot.site_label || '尚未連線測試（請輸入本機或遠端系統 IP 後按「測試連線」）')}</p></div><p class="formnote muted">輸入智慧控管系統 IP（可選填 :Port）後，按「儲存設定」即可永久記住；API 路徑會自動對應。</p></section>
    <section><h3>排程設定</h3><div class="fields">${field('search_minutes', '搜尋間隔（分鐘）', 'number')}${field('max_retries', '最大重試（次，不含首次）', 'number')}${field('start_time', '開始時間', 'time')}${field('end_time', '結束時間', 'time')}</div><label class="checkbox-label"><input name="auto_start_schedule" type="checkbox" ${draft.auto_start_schedule ? 'checked' : ''}>啟動程式時自動開始排程</label><p class="formnote muted">採單日執行時段；開關僅於下次啟動生效。</p><div class="autostart-row"><span class="badge ${snapshot.os_autostart_enabled ? 'good' : ''}" id="autostart-state">開機自動啟動：${snapshot.os_autostart_enabled ? '已啟用' : '未啟用'}</span><div class="actions"><button id="enable-autostart" type="button">啟用開機啟動</button><button id="clear-autostart" type="button">清除開機啟動</button></div></div></section>
    <section><h3>派送閘道</h3><div class="fields">${field('gateway_url', '閘道 URL', 'url', true)}${field('sms_user', '簡訊帳號')}${field('sms_password', '簡訊密碼', 'password')}${field('mail_account', '郵件帳號')}${field('mail_password', '郵件密碼', 'password')}${field('mail_from', '寄件地址', 'email')}${field('username', '寄件者名稱')}</div><label class="checkbox-label"><input id="show-password" type="checkbox">顯示密碼</label><p class="formnote muted">帳密將以 Base64 編碼無損寫回同目錄 env.properties。</p></section>
    <section><h3>傳送測試</h3><div class="fields"><label>傳送管道<select id="test-channel"><option value="sms">簡訊</option><option value="email">郵件</option></select></label><label>測試收件者<input id="test-recipient" placeholder="手機號碼或 Email"></label><label class="wide">測試內容<textarea id="test-message" rows="4">這是一則地籍異動系統測試通知。</textarea></label><div class="wide"><button id="test-delivery" type="button">發送測試訊息</button></div></div><p class="formnote muted">將透過設定的派送閘道實際發送測試訊息（不寫入正式佇列）。</p></section>
    <p id="settings-error" class="inline-error wide" role="alert" hidden></p>
    <div class="wide settings-footer"><div class="actions"><button class="primary" type="submit">儲存設定</button><button id="reload-env" type="button">重新讀取 env.properties</button><button id="cancel-settings" type="button">取消修改</button></div><details><summary>設定檔與紀錄檔位置</summary><p class="config-path">設定檔（env.properties）：${esc(snapshot.config_path)}</p><p class="config-path">紀錄檔：${esc(snapshot.log_path)}</p><p class="muted">寫回 env.properties 前會自動建立時間戳備份，並保留原有其他設定與註解。</p></details></div></form>`;
  $('#settings-form').oninput = (event) => {
    const target = event.target;
    if (!target.name) return;
    if (target.name === 'lah_api_host') {
      draft.lah_api_url = normalizeLahApiUrl(target.value) || target.value.trim();
    } else {
      draft[target.name] = target.type === 'checkbox'
        ? target.checked
        : target.type === 'number'
          ? Number(target.value)
          : target.value;
    }
    void setDirty(JSON.stringify(draft) !== JSON.stringify(snapshot.settings));
    $('#dirty-label').textContent = dirty ? '尚未儲存' : '設定已載入';
    $('#settings-error').hidden = true;
  };
  $('#settings-form').onsubmit = (event) => {
    event.preventDefault();
    action(async () => {
      try {
        snapshot = await invoke('save_settings', { settings: draft });
        await setDirty(false);
        draft = structuredClone(snapshot.settings);
        renderSettings();
        await updateSnapshot();
        return '設定已寫回 env.properties（已自動建立備份）。';
      } catch (error) {
        $('#settings-error').hidden = false;
        $('#settings-error').textContent = String(error);
        throw error;
      }
    }, { refresh: false });
  };
  $('#reload-env').onclick = () => action(async () => {
    snapshot = await invoke('reload_env_properties');
    await setDirty(true);
    draft = structuredClone(snapshot.settings);
    renderSettings();
    return '已重新讀取 env.properties（若尚未寫回，請按「儲存設定」）。';
  }, { refresh: false });
  $('#cancel-settings').onclick = async () => { await setDirty(false); draft = structuredClone(snapshot.settings); renderSettings(); toast('已還原至最後儲存的設定。'); };
  $('#show-password').onchange = (event) => {
    for (const key of ['sms_password', 'mail_password']) $(`[name="${key}"]`).type = event.target.checked ? 'text' : 'password';
  };
  $('#test-lah-connection').onclick = () => action(async () => {
    const meta = await invoke('test_lah_connection', { url: draft.lah_api_url });
    const label = `站台：${meta.site_id} · ${meta.office_name}（${meta.county_name}）`;
    snapshot.site_label = label;
    const labelEl = $('#lah-site-label');
    if (labelEl) labelEl.textContent = label;
    return `連線成功：${meta.site_id} ${meta.office_name}（請記得按「儲存設定」記住網址）。`;
  }, { refresh: false });
  $('#test-delivery').onclick = () => action(() => invoke('test_delivery', { channel: $('#test-channel').value, recipient: $('#test-recipient').value, message: $('#test-message').value }));
  const updateAutostart = (enabled) => action(async () => {
    snapshot = await invoke('set_os_autostart', { enabled });
    const badgeEl = $('#autostart-state');
    if (badgeEl) {
      badgeEl.textContent = `開機自動啟動：${snapshot.os_autostart_enabled ? '已啟用' : '未啟用'}`;
      badgeEl.className = `badge ${snapshot.os_autostart_enabled ? 'good' : ''}`;
    }
    return enabled ? '已啟用 Windows 開機自動啟動。' : '已清除 Windows 開機自動啟動。';
  }, { refresh: false });
  $('#enable-autostart').onclick = () => updateAutostart(true);
  $('#clear-autostart').onclick = () => updateAutostart(false);
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function logLinesText(tail) {
  const kw = logFilter.trim().toLowerCase();
  const filtered = kw ? tail.lines.filter((line) => line.toLowerCase().includes(kw)) : tail.lines;
  return { filtered, text: filtered.join('\n') };
}

function renderLogContentOnly(tail) {
  lastLogTail = tail;
  const meta = $('#log-meta');
  const viewer = $('#log-viewer');
  if (!meta || !viewer) return;
  const { filtered, text } = logLinesText(tail);
  const wasAtBottom = viewer.scrollHeight - viewer.scrollTop - viewer.clientHeight < 36;
  meta.textContent = `檔案：${tail.file_path}（${formatBytes(tail.file_size)} · 顯示 ${filtered.length} / 最近 ${tail.lines.length} 行 · 今日總計 ${tail.total_lines} 行）`;
  viewer.textContent = text || (tail.total_lines === 0 ? '今日尚無寫入紀錄。' : '沒有符合關鍵字的日誌行。');
  if (logAutoRefresh || wasAtBottom) viewer.scrollTop = viewer.scrollHeight;
}

function syncLogPollTimer() {
  if (logPollTimer) { clearInterval(logPollTimer); logPollTimer = undefined; }
  if (page === 'logs' && logAutoRefresh) {
    logPollTimer = setInterval(async () => {
      if (page !== 'logs' || busy) return;
      try {
        const tail = await invoke('read_current_log', { maxLines: 300 });
        if (page === 'logs') renderLogContentOnly(tail);
      } catch { /* 輪詢失敗時靜默略過，待手動更新時提示 */ }
    }, 3000);
  }
}

async function renderLogs() {
  const request = ++sequence;
  const tail = await invoke('read_current_log', { maxLines: 300 });
  if (request !== sequence || page !== 'logs') return;
  $('#content').innerHTML = `<div class="pagehead"><div><h2>日誌追蹤</h2><p>${pages.logs[1]}</p></div><div class="actions"><button id="refresh-log" type="button">立即更新</button><button id="copy-log" type="button">複製日誌</button></div></div>
    <div class="log-toolbar"><input id="log-filter" type="search" value="${esc(logFilter)}" placeholder="篩選關鍵字（例如 ERROR、WARN、排程步驟）"><label class="checkbox-label log-auto-label"><input id="log-auto" type="checkbox" ${logAutoRefresh ? 'checked' : ''}>自動追蹤（每 3 秒）</label></div>
    <p id="log-meta" class="muted log-meta"></p>
    <pre id="log-viewer" class="log-viewer" tabindex="0"></pre>`;
  renderLogContentOnly(tail);
  $('#log-filter').oninput = (event) => {
    logFilter = event.target.value;
    if (lastLogTail) renderLogContentOnly(lastLogTail);
  };
  $('#log-auto').onchange = (event) => {
    logAutoRefresh = event.target.checked;
    syncLogPollTimer();
  };
  $('#refresh-log').onclick = () => action(async () => {
    const latest = await invoke('read_current_log', { maxLines: 300 });
    renderLogContentOnly(latest);
  }, { refresh: false });
  $('#copy-log').onclick = async () => {
    const content = $('#log-viewer')?.textContent || '';
    try { await navigator.clipboard.writeText(content); toast('目前顯示的日誌內容已複製。'); }
    catch { toast('請選取日誌視窗文字，再按 Ctrl+C 複製。'); }
  };
  syncLogPollTimer();
}

async function renderPage() {
  $('#content').classList.toggle('records-view', !['settings', 'messages'].includes(page));
  if (page === 'settings') renderSettings();
  else if (page === 'messages') renderMessages();
  else if (page === 'logs') await renderLogs();
  else await renderRecords();
}

async function refresh() {
  await action(async () => {
    await updateSnapshot();
    if (page !== 'settings') await renderPage();
    return dirty ? '資料已更新，未儲存的設定仍保留。' : '資料已重新載入。';
  }, { refresh: false });
}

function openApiSetupDialog() {
  const dialog = $('#api-setup-dialog');
  const input = $('#api-setup-input');
  const errBox = $('#api-setup-error');
  const saveBtn = $('#api-setup-save');
  const cancelBtn = $('#api-setup-cancel');
  if (!dialog || !input || dialog.open) return;

  input.value = extractLahApiHost(snapshot?.settings?.lah_api_url || '');
  errBox.hidden = true;
  input.oninput = () => { errBox.hidden = true; };
  cancelBtn.onclick = () => dialog.close();
  dialog.oncancel = (event) => { event.preventDefault(); dialog.close(); };

  $('#api-setup-form').onsubmit = async (event) => {
    event.preventDefault();
    const url = normalizeLahApiUrl(input.value);
    if (!url) {
      errBox.hidden = false;
      errBox.textContent = '請輸入有效的智慧控管系統 IP（例如 192.168.1.100）。';
      input.focus();
      return;
    }
    saveBtn.disabled = true;
    cancelBtn.disabled = true;
    const prevSaveLabel = saveBtn.textContent;
    saveBtn.textContent = '測試中…';
    errBox.hidden = true;
    try {
      const meta = await invoke('test_lah_connection', { url });
      const nextSettings = { ...(snapshot?.settings || draft), lah_api_url: url };
      snapshot = await invoke('save_settings', { settings: nextSettings });
      if (!dirty) {
        draft = structuredClone(snapshot.settings);
      } else if (draft) {
        draft.lah_api_url = url;
      }
      dialog.close();
      await updateSnapshot();
      await renderPage();
      toast(`連線成功：${meta.site_id} ${meta.office_name}，已儲存智慧控管系統位址。`);
    } catch (error) {
      const info = describeLoadError(error, url);
      errBox.hidden = false;
      errBox.innerHTML = `<strong>${esc(info.title)}</strong><p>${esc(info.summary)}</p><details class="error-details"><summary>檢視技術細節</summary><pre class="error-raw">${esc(info.raw)}</pre></details>`;
    } finally {
      saveBtn.textContent = prevSaveLabel;
      saveBtn.disabled = false;
      cancelBtn.disabled = false;
    }
  };

  dialog.showModal();
  input.focus();
  input.select();
}

async function start() {
  applyTheme(getSavedTheme());
  $('#theme-toggle')?.addEventListener('click', () => {
    const current = document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
    applyTheme(current === 'dark' ? 'light' : 'dark');
  });
  for (const tab of document.querySelectorAll('[data-page]')) tab.onclick = () => switchPage(tab.dataset.page);
  $('#open-api-setup')?.addEventListener('click', openApiSetupDialog);
  $('#schedule').onclick = () => action(async () => {
    await invoke('set_schedule', { running: !snapshot.running });
    return '排程狀態已更新。';
  });
  $('#sync').onclick = () => action(async () => {
    const syncBtn = $('#sync');
    const prevLabel = syncBtn.textContent;
    syncBtn.textContent = '同步中，請稍候…';
    try {
      return await invoke('sync_cases');
    } finally {
      syncBtn.textContent = prevLabel;
    }
  });
  $('#refresh').onclick = refresh;
  setBusy(true);
  try {
    await updateSnapshot();
    await renderPage();
    if (snapshot.startup_warning) toast(snapshot.startup_warning, true);
  } catch (error) {
    const info = renderLoadError(error, () => location.reload());
    toast(info.title, true);
  } finally {
    setBusy(false);
    if (!snapshot) for (const button of document.querySelectorAll('#schedule, #sync, .nav button')) button.disabled = true;
  }
}

document.addEventListener('keydown', (event) => {
  if (event.key === 'F5') { event.preventDefault(); refresh(); }
  if (event.ctrlKey && event.key.toLowerCase() === 's' && page === 'settings') { event.preventDefault(); if (!busy) $('#settings-form').requestSubmit(); }
});
function clock() { $('#clock').textContent = new Date().toLocaleString('zh-TW', { hour12: false }); }
clock();
setInterval(clock, 1000);
start();
