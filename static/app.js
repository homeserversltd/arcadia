const PopupManager = (() => {
  const overlay = () => document.getElementById('modal-overlay');
  const title = () => document.getElementById('modal-title');
  const content = () => document.getElementById('modal-content');
  const actions = () => document.querySelector('#modal-overlay .modal-actions');
  const toasts = () => document.getElementById('toast-container');
  let previousFocus = null;

  function showModal({ title: modalTitle, body, hideDefaultAction = false }) {
    const el = overlay();
    if (!el) return;
    previousFocus = document.activeElement;
    title().textContent = modalTitle || 'Arcadia Console';
    const target = content();
    target.textContent = '';
    if (body instanceof Node) target.appendChild(body);
    else target.textContent = body || '';
    actions()?.toggleAttribute('hidden', hideDefaultAction);
    el.hidden = false;
    const focusable = el.querySelector('button, [href], input, select, textarea, details, [tabindex]:not([tabindex="-1"])');
    focusable?.focus();
  }

  function closeModal() {
    const el = overlay();
    if (!el) return;
    el.hidden = true;
    content().textContent = '';
    actions()?.removeAttribute('hidden');
    if (previousFocus && typeof previousFocus.focus === 'function') previousFocus.focus();
  }

  function trapFocus(event) {
    const el = overlay();
    if (!el || el.hidden || event.key !== 'Tab') return false;
    const focusable = Array.from(el.querySelectorAll('button, [href], input, select, textarea, details, [tabindex]:not([tabindex="-1"])')).filter((node) => !node.disabled && node.offsetParent !== null);
    if (!focusable.length) return false;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); return true; }
    if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); return true; }
    return false;
  }

  function showToast(message, variant = 'info') {
    const root = toasts();
    if (!root) return;
    const node = document.createElement('button');
    const label = document.createElement('span');
    const dismiss = document.createElement('span');
    node.className = `toast ${variant}`;
    node.type = 'button';
    node.setAttribute('aria-label', `${message}; dismiss notification`);
    node.title = 'Dismiss notification';
    label.className = 'toast-message';
    label.textContent = message;
    dismiss.className = 'toast-dismiss';
    dismiss.setAttribute('aria-hidden', 'true');
    dismiss.textContent = '×';
    node.append(label, dismiss);
    node.addEventListener('click', () => node.remove());
    root.appendChild(node);
    setTimeout(() => node.remove(), 3600);
  }

  return { showModal, closeModal, showToast, trapFocus };
})();


function arcadiaThemeNames() {
  const declared = Array.isArray(window.ARCADIA_THEMES) ? window.ARCADIA_THEMES : [];
  return declared.map((theme) => theme.name).filter(Boolean);
}

function normalizeThemeName(name) {
  const themes = arcadiaThemeNames();
  return themes.includes(name) ? name : (themes[0] || 'ember-aubergine');
}

function setArcadiaTheme(name, announce = false) {
  const theme = normalizeThemeName(name);
  document.documentElement.dataset.theme = theme;
  try { localStorage.setItem('arcadia-theme', theme); } catch (_) {}
  document.querySelectorAll('[data-theme-cycle]').forEach((button) => {
    button.dataset.themeCurrent = theme;
    const label = button.querySelector('.theme-name');
    if (label) label.textContent = theme.replace(/-/g, ' ');
    const pretty = theme.replace(/-/g, ' ');
    button.setAttribute('aria-label', `Toggle theme, current theme ${pretty}`);
    button.dataset.tooltip = `Toggle theme; current theme ${pretty}`;
  });
  if (announce) PopupManager.showToast(`Theme ${theme.replace(/-/g, ' ')}`, 'success');
}

function initializeArcadiaTheme() {
  let stored = '';
  try { stored = localStorage.getItem('arcadia-theme') || ''; } catch (_) {}
  setArcadiaTheme(stored || document.documentElement.dataset.theme || '', false);
  document.querySelectorAll('[data-theme-cycle]').forEach((button) => {
    button.addEventListener('click', () => {
      const themes = arcadiaThemeNames();
      const current = normalizeThemeName(document.documentElement.dataset.theme);
      const next = themes[(themes.indexOf(current) + 1) % themes.length] || current;
      setArcadiaTheme(next, true);
    });
  });
}

async function checkGuiPinStatus() {
  const res = await fetch('/api/gui-pin/status', { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error('GUI PIN status request failed');
  return await res.json();
}

function openArcadia() {
  document.body.classList.add('pin-open');
  document.body.dataset.guiPinRequired = 'false';
  document.getElementById('app')?.removeAttribute('aria-hidden');
}

function keepGuiPinGate() {
  document.body.classList.remove('pin-open');
  document.body.dataset.guiPinRequired = 'true';
  document.getElementById('app')?.setAttribute('aria-hidden', 'true');
  document.querySelector('.field--pin')?.focus();
}

async function initializeGuiPinGate() {
  try {
    const status = await checkGuiPinStatus();
    if (status.pin_required) keepGuiPinGate(); else openArcadia();
    setPinIndicator(Boolean(status.pin_required));
  } catch (_) {
    openArcadia();
    PopupManager.showToast('GUI PIN status unavailable; opening Arcadia', 'error');
  }
}

function setPinIndicator(required) {
  document.querySelectorAll('.status-badge').forEach((badge) => {
    if (badge.dataset.chipKind !== 'pin') return;
    badge.setAttribute('aria-label', required ? 'PIN required for GUI access' : 'GUI open without PIN');
    badge.classList.toggle('status-badge--warn', required);
    badge.classList.toggle('status-badge--idle', !required);
  });
}

function bindNavigation() {
  const buttons = Array.from(document.querySelectorAll('.launcher-button[data-view]'));
  const panels = Array.from(document.querySelectorAll('[data-view-panel]'));
  const activate = (view) => {
    if (view === 'advanced') view = 'system';
    if (view === 'ai-model' || view === 'lan-inference') view = 'local-ai';
    const next = panels.some((panel) => panel.dataset.viewPanel === view) ? view : 'home';
    buttons.forEach((button) => {
      const active = button.dataset.view === next;
      button.classList.toggle('is-active', active);
      button.setAttribute('aria-current', active ? 'page' : 'false');
    });
    panels.forEach((panel) => {
      const active = panel.dataset.viewPanel === next;
      panel.classList.toggle('is-active', active);
      panel.hidden = !active;
      if (active) panel.focus({ preventScroll: true });
    });
    try { localStorage.setItem('arcadia-active-view', next); } catch (_) {}
    document.dispatchEvent(new CustomEvent('arcadia:view-change', { detail: { view: next } }));
  };
  window.activateArcadiaView = activate;
  buttons.forEach((button) => button.addEventListener('click', () => activate(button.dataset.view)));
  document.querySelectorAll('[data-nav-target]').forEach((button) => button.addEventListener('click', () => {
    activate(button.dataset.navTarget);
    if (button.dataset.focusTarget) {
      setTimeout(() => {
        const node = document.getElementById(button.dataset.focusTarget);
        if (node) { node.focus({ preventScroll: false }); node.scrollIntoView({ block: 'center', behavior: 'smooth' }); }
        if (button.dataset.focusTarget === 'wifi-management') requestWifiScan(true);
      }, 40);
    }
  }));
  let stored = 'home';
  try { stored = localStorage.getItem('arcadia-active-view') || 'home'; } catch (_) {}
  activate(stored);
}

function bindHomeLoadSubscription() {
  const card = document.querySelector('[data-load-card]');
  if (!card) return;
  const retryMs = Math.max(2000, Number(card.dataset.loadRetryMs || 5000));
  const state = {
    source: null,
    lease: null,
    heartbeat: null,
    renewalTimer: null,
    retryTimer: null,
    inFlight: false,
    events: 0,
    fallbackSnapshots: 0,
    fallback: false,
    cachedRoot: null,
    lastContactUnix: null,
    expiresAtUnix: null,
  };
  window.arcadiaHomeLoadSubscriptionState = state;
  const homeIsActive = () => Boolean(document.querySelector('[data-view-panel="home"].is-active')) && document.visibilityState === 'visible';
  const setText = (selector, text) => { const node = card.querySelector(selector); if (node) node.textContent = text; };
  const setChip = (key, text, stateName = 'idle') => {
    setText(`[data-load-chip-value="${key}"]`, text);
    const chip = card.querySelector(`[data-load-chip="${key}"]`);
    if (chip) chip.className = `load-chip load-chip--${stateName}`;
  };
  const number = (value) => Number.isFinite(Number(value)) ? Number(value) : null;
  const fmtLoad = (value) => value == null ? '—' : value.toFixed(2);
  const fmtTemp = (value) => value == null ? '—' : `${value.toFixed(1)}°C`;
  const fmtPressure = (value) => value == null ? '—' : `${value.toFixed(1)}%`;
  const updateSpark = (key, value, cores) => {
    const pct = value == null ? 0 : Math.max(0, Math.min(100, Math.round((value / Math.max(1, cores)) * 100)));
    setText(`[data-load-spark-value="${key}"]`, fmtLoad(value));
    const bar = card.querySelector(`[data-load-spark-bar="${key}"]`);
    if (bar) bar.style.width = `${pct}%`;
  };
  const apply = (root) => {
    state.cachedRoot = root;
    const telemetry = (root.children || []).find((node) => node.id === 'telemetry') || {};
    const data = telemetry.data || {};
    const load = data.load || {};
    const io = data.io || {};
    const disk = io.disk || {};
    const one = number(load.oneMinute);
    const five = number(load.fiveMinute);
    const fifteen = number(load.fifteenMinute);
    const cores = Number(navigator.hardwareConcurrency || 1);
    const pct = one == null ? 0 : Math.max(0, Math.min(100, Math.round((one / Math.max(1, cores)) * 100)));
    setText('[data-load-headline]', fmtLoad(one));
    setText('[data-load-percent]', `${pct}%`);
    const orb = card.querySelector('[data-load-orb]');
    if (orb) {
      orb.style.setProperty('--load-pct', pct);
      orb.setAttribute('aria-label', `${pct} percent load`);
      orb.classList.toggle('load-orb--warn', pct >= 90);
      orb.classList.toggle('load-orb--ok', pct < 90 && one != null);
      orb.classList.toggle('load-orb--idle', one == null);
    }
    updateSpark('oneMinute', one, cores);
    updateSpark('fiveMinute', five, cores);
    updateSpark('fifteenMinute', fifteen, cores);
    const temp = number(data.cpu?.temperatureCelsius);
    const pressure = number(io.pressureAvg10);
    setChip('cpu', fmtTemp(temp), temp == null ? 'idle' : (temp >= 82 ? 'warn' : 'ok'));
    setChip('io', fmtPressure(pressure), pressure == null ? 'idle' : (pressure >= 10 ? 'warn' : 'ok'));
    setChip('read', disk.readBytesApprox == null ? '—' : formatBytes(Number(disk.readBytesApprox)), 'idle');
    setChip('write', disk.writtenBytesApprox == null ? '—' : formatBytes(Number(disk.writtenBytesApprox)), 'idle');
  };
  const clearRenewal = () => { if (state.renewalTimer) clearTimeout(state.renewalTimer); state.renewalTimer = null; };
  const clearRetry = () => { if (state.retryTimer) clearTimeout(state.retryTimer); state.retryTimer = null; };
  const stopEvents = () => { if (state.source) state.source.close(); state.source = null; clearRenewal(); clearRetry(); };
  const fetchSnapshotOnce = async () => {
    if (!homeIsActive() || state.inFlight) return;
    state.inFlight = true;
    try {
      const res = await fetch('/api/root', { headers: { Accept: 'application/json' }, cache: 'no-store' });
      if (res.ok) { apply(await res.json()); state.fallbackSnapshots += 1; }
    } catch (_) {
      // Snapshot fallback stays silent; the card keeps its cached values.
    } finally {
      state.inFlight = false;
    }
  };
  const scheduleRetry = () => {
    clearRetry();
    if (!homeIsActive()) return;
    state.retryTimer = setTimeout(() => {
      state.retryTimer = null;
      startEvents();
    }, retryMs);
  };
  const scheduleRenewal = () => {
    clearRenewal();
    if (!homeIsActive() || !state.lease?.leaseId) return;
    const renewMs = Math.max(1000, Number(state.lease.renewAfterSeconds || 10) * 1000);
    state.renewalTimer = setTimeout(renewLease, renewMs);
  };
  const renewLease = async () => {
    if (!homeIsActive() || !state.lease?.leaseId) return clearRenewal();
    try {
      const res = await fetch('/api/root/events/renew', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({ leaseId: state.lease.leaseId }),
        cache: 'no-store',
        keepalive: true,
      });
      if (!res.ok) throw new Error('lease expired');
      state.lease = await res.json();
      state.lastContactUnix = state.lease.lastContactUnix;
      state.expiresAtUnix = state.lease.expiresAtUnix;
      scheduleRenewal();
    } catch (_) {
      stopEvents();
      if (homeIsActive()) {
        state.fallback = true;
        fetchSnapshotOnce();
        scheduleRetry();
      }
    }
  };
  const startEvents = () => {
    clearRetry();
    state.fallback = false;
    if (!homeIsActive()) return stopEvents();
    if (!('EventSource' in window)) {
      state.fallback = true;
      fetchSnapshotOnce();
      return scheduleRetry();
    }
    if (state.source) return;
    try {
      const source = new EventSource('/api/root/events');
      state.source = source;
      const onRoot = (event) => {
        if (!homeIsActive()) return stopEvents();
        try {
          apply(JSON.parse(event.data));
          state.events += 1;
        } catch (_) {
          // Ignore malformed event payloads and keep the last known values.
        }
      };
      const onLease = (event) => {
        try {
          state.lease = JSON.parse(event.data);
          state.lastContactUnix = state.lease.lastContactUnix;
          state.expiresAtUnix = state.lease.expiresAtUnix;
          scheduleRenewal();
        } catch (_) {}
      };
      const onHeartbeat = (event) => {
        try {
          state.heartbeat = JSON.parse(event.data);
          state.expiresAtUnix = state.heartbeat.expiresAtUnix;
        } catch (_) {}
      };
      source.addEventListener('snapshot', onRoot);
      source.addEventListener('root', onRoot);
      source.addEventListener('lease', onLease);
      source.addEventListener('heartbeat', onHeartbeat);
      source.addEventListener('expired', () => {
        stopEvents();
        if (homeIsActive()) {
          state.fallback = true;
          fetchSnapshotOnce();
          scheduleRetry();
        }
      });
      source.onmessage = onRoot;
      source.onerror = () => {
        stopEvents();
        if (homeIsActive()) {
          state.fallback = true;
          fetchSnapshotOnce();
          scheduleRetry();
        }
      };
    } catch (_) {
      state.fallback = true;
      fetchSnapshotOnce();
      scheduleRetry();
    }
  };
  const stop = () => {
    stopEvents();
  };
  const start = () => {
    if (!homeIsActive()) return stop();
    if (state.cachedRoot) apply(state.cachedRoot);
    startEvents();
  };
  window.arcadiaHomeLoadSubscription = { start, stop, homeIsActive, startEvents, fetchSnapshotOnce, renewLease };
  document.addEventListener('arcadia:view-change', () => { if (homeIsActive()) start(); else stop(); });
  document.addEventListener('visibilitychange', () => { if (homeIsActive()) start(); else stop(); });
  start();
}

function bindGuiPinUnlock() {
  const form = document.getElementById('gui-pin-unlock-form');
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = form.querySelector('input[name="pin"]');
    const error = document.getElementById('gui-pin-auth-error');
    const button = form.querySelector('button[type="submit"]');
    if (!input?.value) { error.textContent = 'GUI PIN is required'; error.hidden = false; return; }
    error.hidden = true;
    button.disabled = true;
    button.textContent = 'Opening...';
    try {
      const data = await postJson('/pre-unlock', { pin: input.value });
      input.value = '';
      if (data.ok) { openArcadia(); PopupManager.showToast('Arcadia opened', 'success'); }
      else { error.textContent = data.message || 'GUI PIN rejected.'; error.hidden = false; }
    } catch (_) {
      input.value = '';
      error.textContent = 'GUI PIN request failed.';
      error.hidden = false;
    } finally {
      button.disabled = false;
      button.textContent = 'Open Arcadia';
    }
  });
}

function renderGuiPinMode(required) {
  document.body.dataset.guiPinRequired = String(required);
  setPinIndicator(Boolean(required));
  document.querySelectorAll('[data-pin-required-toggle]').forEach((toggle) => { toggle.checked = Boolean(required); });
  document.querySelectorAll('[data-pin-mode-label]').forEach((node) => { node.textContent = required ? 'PIN required' : 'Open without PIN'; });
  document.querySelectorAll('[data-pin-mode-copy]').forEach((node) => { node.textContent = required ? 'PIN required before accessing HomeConsole.' : 'HomeConsole opens without a PIN.'; });
}

function bindGuiPinAccess() {
  document.querySelectorAll('[data-pin-required-toggle]').forEach((toggle) => {
    toggle.addEventListener('change', async () => {
      const nextRequired = Boolean(toggle.checked);
      const previousRequired = !nextRequired;
      toggle.disabled = true;
      try {
        const data = await postJson('/api/gui-pin/access', { pin_required: nextRequired });
        PopupManager.showToast(data.message || (data.ok ? 'GUI PIN setting saved' : 'GUI PIN setting not saved'), data.ok ? 'success' : 'error');
        if (data.ok) renderGuiPinMode(Boolean(data.pin_required));
        else renderGuiPinMode(previousRequired);
      } catch (_) {
        renderGuiPinMode(previousRequired);
        PopupManager.showToast('GUI PIN access request failed.', 'error');
      } finally {
        toggle.disabled = false;
      }
    });
  });
}

function setMessage(id, text, variant = 'info') {
  const node = document.getElementById(id);
  if (!node) return;
  node.textContent = text;
  node.className = `message message--${variant}`;
  node.hidden = false;
}

function clearMessage(id) {
  const node = document.getElementById(id);
  if (!node) return;
  node.textContent = '';
  node.hidden = true;
}

function formatActionResult(data) {
  const bits = [data.message || (data.ok ? 'Done.' : 'Failed.')];
  if (data.exit_code !== undefined && data.exit_code !== null) bits.push(`exit ${data.exit_code}`);
  if (data.stdout) bits.push(data.stdout);
  if (data.stderr) bits.push(data.stderr);
  return bits.join('\n');
}

function confirmationFor(action) {
  if (action === 'reboot-console') return window.confirm('Restart this console now? Games and services will close.') ? { confirm: 'REBOOT' } : null;
  if (action === 'shutdown-console') return window.confirm('Shut down this console now? The appliance will power off.') ? { confirm: 'SHUTDOWN' } : null;
  if (action === 'restart-gamescope') return window.confirm('Restarting GameScope may close the active game session.') ? { confirm: 'RESTART_GAMESCOPE' } : null;
  if (action === 'restart-arcadia') return window.confirm('Restart Arcadia web GUI only? The page may reconnect.') ? { confirm: 'RESTART_ARCADIA' } : null;
  if (action === 'controllers-rescan' || action === 'controllers-test') return {};
  if (action === 'enable-ssh') return { mode: 'enable' };
  if (action === 'disable-ssh') return window.confirm('Disable SSH service? Current remote SSH access may disconnect.') ? { mode: 'disable', confirm: 'DISABLE_SSH' } : null;
  if (action === 'enable-ssh-password') return window.confirm('Enable SSH password login? Key login remains available.') ? { mode: 'enable', confirm: 'ENABLE_SSH_PASSWORD' } : null;
  if (action === 'disable-ssh-password') return window.confirm('Disable SSH password login? Key login remains available.') ? { mode: 'disable', confirm: 'DISABLE_SSH_PASSWORD' } : null;
  if (action === 'trust-mode-http') return window.confirm('Switch Arcadia to HTTP mode? HTTPS can be re-enabled after Root CA validation.') ? { mode: 'http', confirm: 'ENABLE_HTTP' } : null;
  if (action === 'trust-mode-https') return window.confirm('Switch Arcadia to HTTPS with Home Root CA? The current HTTP mode stays active if validation fails.') ? { mode: 'https', confirm: 'ENABLE_HTTPS' } : null;
  if (action === 'clear-artwork-cache') return window.confirm('Clear artwork cache? This does not delete games. Artwork can be downloaded again during Sync.') ? { confirm: 'CLEAR_ARTWORK' } : null;
  if (action === 'remove-ai-model') return window.confirm('Remove this local AI model file from console storage? This does not affect games.') ? { confirm: 'REMOVE_MODEL' } : null;
  if (action === 'clean-temporary-files') return window.confirm('Clean safe temporary files? This will not remove games, artwork intentionally kept, or installed AI models.') ? { confirm: 'CLEAN_TEMPORARY' } : null;
  if (action === 'clear-partial-ai-downloads') return window.confirm('Clear partial AI downloads? Installed models, games, and artwork are not affected.') ? { confirm: 'CLEAR_PARTIAL_DOWNLOADS' } : null;
  if (action === 'clear-old-updates') return window.confirm('Clear old update packages? Current installed software is not removed.') ? { confirm: 'CLEAR_OLD_UPDATES' } : null;
  if (action === 'prune-logs') return window.confirm('Prune managed logs? Games, artwork, and models are not affected.') ? { confirm: 'PRUNE_LOGS' } : null;
  return {};
}

function bindConsoleActions() {
  document.querySelectorAll('.btn[data-action][data-endpoint]').forEach((button) => {
    if (button.dataset.pinRequired !== undefined) return;
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.action;
      const endpoint = button.dataset.endpoint;
      if (action === 'sync-games' && !prepareSyncStart()) return;
      const original = button.textContent;
      if (action === 'storage-rescan') {
        button.disabled = true;
        button.textContent = 'Scanning...';
        try { await postJson(endpoint, {}); window.location.reload(); }
        catch (_) { PopupManager.showToast('Rescan failed', 'error'); button.disabled = false; button.textContent = original; }
        return;
      }
      const body = confirmationFor(action);
      if (body === null) return;
      clearMessage('console-action-message');
      button.disabled = true;
      button.textContent = action === 'sync-games' ? 'Sync Running' : 'Running...';
      let syncProgress = null;
      if (action === 'sync-games') syncProgress = startSyncProgress();
      try {
        const data = await postJson(endpoint, body);
        const variant = data.ok ? 'success' : 'error';
        setMessage('console-action-message', formatActionResult(data), variant);
        PopupManager.showToast(data.message || (data.ok ? 'Done' : 'Failed'), variant);
        if (action === 'sync-games') finishSyncProgress(Boolean(data.ok), data, syncProgress);
      } catch (_) {
        setMessage('console-action-message', 'Action request failed.', 'error');
        PopupManager.showToast('Action request failed', 'error');
        if (action === 'sync-games') finishSyncProgress(false, { message: 'Sync failed. Open the ledger for the reason and fix action.' }, syncProgress);
      } finally {
        button.disabled = false;
        button.textContent = original;
      }
    });
  });
  document.querySelectorAll('.btn[data-copy-value]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const value = button.dataset.copyValue || '';
      if (!validCopyValue(value)) {
        PopupManager.showToast('Address unavailable', 'error');
        return;
      }
      const ok = await copyToClipboard(value);
      PopupManager.showToast(ok ? `Copied ${value}` : `Copy unavailable: ${value}`, ok ? 'success' : 'error');
    });
  });
  document.querySelectorAll('[data-create-managed-folder]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const path = button.dataset.createManagedFolder || '';
      if (!path || !window.confirm(`Create managed folder?\n${path}`)) return;
      const data = await postJson('/api/storage/create-managed-folder', { path });
      PopupManager.showToast(data.message || (data.ok ? 'Managed folder created' : 'Folder not created'), data.ok ? 'success' : 'error');
      if (data.ok) window.location.reload();
    });
  });
  document.querySelectorAll('.btn[data-url]').forEach((button) => {
    button.addEventListener('click', (event) => {
      event.preventDefault();
      window.location.href = button.dataset.url;
    });
  });
}


async function getJson(url) {
  const res = await fetch(url, { headers: { accept: 'application/json' } });
  let data = {};
  try { data = await res.json(); } catch (_) { data = {}; }
  if (!res.ok) data.ok = false;
  return data;
}

function storageModalShell(title, crumbs = ['Storage']) {
  const body = document.createElement('div');
  body.className = 'storage-detail-modal';
  const bar = document.createElement('div');
  bar.className = 'storage-breadcrumb';
  crumbs.forEach((crumb, index) => {
    const item = document.createElement('span');
    item.textContent = crumb;
    bar.appendChild(item);
    if (index < crumbs.length - 1) {
      const sep = document.createElement('em');
      sep.textContent = '/';
      bar.appendChild(sep);
    }
  });
  const content = document.createElement('div');
  content.className = 'storage-detail-content';
  body.append(bar, content);
  PopupManager.showModal({ title, body, hideDefaultAction: true });
  return content;
}

function storageLoading(content, label) {
  content.innerHTML = `<div class="empty-state"><strong>${escapeHtml(label)}</strong></div>`;
}

function storageSummaryLine(label, value) {
  const row = document.createElement('div');
  row.className = 'network-detail-row';
  row.innerHTML = `<span><em>${escapeHtml(label)}</em><strong>${escapeHtml(value ?? '—')}</strong></span>`;
  return row;
}

function storageTable(headers, rows) {
  const table = document.createElement('div');
  table.className = 'storage-detail-table';
  const head = document.createElement('div');
  head.className = 'storage-detail-table-row storage-detail-table-head';
  headers.forEach((h) => { const c = document.createElement('strong'); c.textContent = h; head.appendChild(c); });
  table.appendChild(head);
  rows.forEach((cells) => {
    const row = document.createElement('div');
    row.className = 'storage-detail-table-row';
    cells.forEach((cell) => {
      const c = document.createElement('span');
      if (cell instanceof Node) c.appendChild(cell); else c.textContent = cell ?? '—';
      row.appendChild(c);
    });
    table.appendChild(row);
  });
  return table;
}

function buttonNode(label, className = 'btn btn--secondary') {
  const b = document.createElement('button');
  b.type = 'button';
  b.className = className;
  b.textContent = label;
  return b;
}

function copyButtonNode(label, value) {
  const b = buttonNode(label);
  b.addEventListener('click', async () => {
    if (!validCopyValue(value)) return PopupManager.showToast('Address unavailable', 'error');
    const ok = await copyToClipboard(value);
    PopupManager.showToast(ok ? `Copied ${value}` : `Copy unavailable: ${value}`, ok ? 'success' : 'error');
  });
  return b;
}

async function openStorageModal(view) {
  const titles = {
    games: 'Games', 'artwork-detail': 'Artwork', 'ai-models-detail': 'Local AI', 'updates-detail': 'Updates', 'logs-detail': 'Logs', 'temporary-detail': 'Temporary Files', 'system-detail': 'System', 'category-other': 'Other', 'category-free': 'Free', locations: 'Managed Locations', 'cleanup-review': 'Cleanup', diagnostics: 'Diagnostics'
  };
  const title = titles[view] || 'Storage';
  const content = storageModalShell(title, ['Storage', title]);
  storageLoading(content, view === 'games' ? 'Scanning game folders…' : `Loading ${title}…`);
  try {
    if (view === 'games') return renderGamesModal(content, await getJson('/api/storage/games'));
    if (view === 'locations') return renderLocationsModal(content, await getJson('/api/storage/locations'));
    if (view === 'cleanup-review') return renderCleanupModal(content, await getJson('/api/storage/cleanup'));
    if (view === 'diagnostics') return renderDiagnosticsModal(content, await getJson('/api/storage/diagnostics'));
    if (view === 'artwork-detail') return renderFolderList(content, 'Artwork', await getJson('/api/storage/artwork'));
    if (view === 'ai-models-detail') return renderAiModelsModal(content, await getJson('/api/storage/category/ai-models'));
    const category = view.replace('-detail', '').replace('category-', '');
    return renderCategoryModal(content, title, await getJson(`/api/storage/category/${category}`));
  } catch (_) {
    content.innerHTML = '<div class="empty-state"><strong>Could not load storage details.</strong></div>';
  }
}

function renderGamesModal(content, data) {
  content.textContent = '';
  const folders = data.folders || [];
  content.appendChild(storageSummaryLine('Summary', `${folders.length} managed folders · ${(data.summary && data.summary.size) || '0 B'} used`));
  if (data.syncStatusAvailable === false) {
    const note = document.createElement('div');
    note.className = 'warning';
    note.textContent = data.syncUnavailableMessage || 'Sync status unavailable for game folders.';
    content.appendChild(note);
  }
  const rows = folders.map((f) => {
    const actions = document.createElement('div');
    actions.className = 'storage-row-actions';
    const open = buttonNode('Open');
    open.addEventListener('click', () => openGameFolderDetail(f.platform || f.id));
    actions.append(open, copyButtonNode('Copy path', f.path));
    return [f.platform || f.displayName, f.size, String(f.fileCount ?? 0), data.syncStatusAvailable === false ? '—' : String(f.syncedEntries ?? 'Unknown'), actions];
  });
  content.appendChild(storageTable(['Platform', 'Size', 'Files', 'Sync State', 'Actions'], rows));
}

async function openGameFolderDetail(platform) {
  const key = String(platform || '').toLowerCase();
  const content = storageModalShell(platform, ['Storage', 'Games', platform]);
  storageLoading(content, `Scanning ${platform} folder…`);
  const data = await getJson(`/api/storage/games/${encodeURIComponent(key)}`);
  const f = data.folder || {};
  content.textContent = '';
  const back = buttonNode('Back');
  back.addEventListener('click', () => openStorageModal('games'));
  content.appendChild(back);
  [['Actual path', f.path], ['Samba share', f.sambaShareName], ['Files', String(f.fileCount ?? 0)], ['Total size', f.size], ['Last modified', 'Read from folder scan']].forEach(([k, v]) => content.appendChild(storageSummaryLine(k, v)));
  const actions = document.createElement('div');
  actions.className = 'inline-actions inline-actions--compact';
  [['Copy Windows path', f.windowsUNC], ['Copy Windows IP fallback', f.windowsUNCByIp], ['Copy Linux/macOS path', f.smbUrl], ['Copy Linux/macOS IP fallback', f.smbUrlByIp], ['Copy local path', f.path]].filter(([,v]) => validCopyValue(v)).forEach(([label, value]) => actions.appendChild(copyButtonNode(label, value)));
  content.appendChild(actions);
  const largest = (f.largestFiles || []).map((x) => [x.name, x.size, x.path]);
  if (largest.length) content.appendChild(storageTable(['Largest files', 'Size', 'Path'], largest));
}

function renderFolderList(content, label, rows) {
  content.textContent = '';
  const list = Array.isArray(rows) ? rows : [];
  content.appendChild(storageSummaryLine('Summary', `${list.length} roots`));
  content.appendChild(storageTable(['Name', 'Size', 'Files', 'State', 'Path'], list.map((r) => [r.displayName, r.size, String(r.fileCount ?? 0), r.state || '—', r.path])));
}

function renderAiModelsModal(content, data) {
  content.textContent = '';
  const models = data.items || [];
  if (!models.length) content.appendChild(storageSummaryLine('Installed models', 'No models installed.'));
  else content.appendChild(storageTable(['Model', 'Size', 'State', 'Path'], models.map((m) => [m.name, m.size, m.loaded ? 'Hot-loaded' : (m.selected ? 'Selected' : 'Installed'), m.path])));
  const roots = data.roots || [];
  if (roots.length) content.appendChild(storageTable(['Search Roots', 'Purpose', 'Path'], roots.map((r) => [r.displayName, r.purpose || '—', r.path])));
}

function renderCategoryModal(content, title, data) {
  content.textContent = '';
  const s = data.summary || {};
  content.append(storageSummaryLine('Size', s.size || data.size || '—'), storageSummaryLine('Detail', s.detail || title));
  if (data.roots) content.appendChild(storageTable(['Location', 'Purpose', 'Path'], data.roots.map((r) => [r.displayName, r.purpose || '—', r.path])));
}

function renderCleanupModal(content, cleanup) {
  content.textContent = '';
  const rows = [
    ['Artwork cache', cleanup.artworkBytesClearable, 'clear-artwork-cache', '/api/storage/cleanup/artwork', 'Clear'],
    ['Temporary files', cleanup.temporaryBytesClearable, 'clean-temporary-files', '/api/storage/cleanup/temporary', 'Clear'],
    ['Old updates', cleanup.oldUpdateBytesClearable, 'clear-old-updates', '/api/storage/cleanup/old-updates', 'Clear'],
    ['Logs', cleanup.logsBytesClearable, 'prune-logs', '/api/storage/cleanup/logs', 'Prune'],
    ['Partial downloads', cleanup.partialDownloadsBytesClearable, 'clear-partial-ai-downloads', '/api/storage/cleanup/partial-ai-downloads', 'Clear'],
  ].map(([label, bytes, action, endpoint, text]) => {
    const b = Number(bytes || 0) > 0 ? buttonNode(text) : document.createTextNode('—');
    if (b instanceof HTMLButtonElement) b.addEventListener('click', () => runStorageCleanup(action, endpoint));
    return [label, formatBytes(Number(bytes || 0)), b];
  });
  content.appendChild(storageTable(['Item', 'Clearable', 'Action'], rows));
}

function renderLocationsModal(content, registry) {
  content.textContent = '';
  const cats = registry.categories || {};
  const rows = [];
  Object.entries(cats).forEach(([name, cat]) => (cat.roots || []).forEach((r) => rows.push([name, r.displayName || r.platform, r.path, copyButtonNode('Copy', r.path)])));
  content.appendChild(storageTable(['Category', 'Name', 'Path', 'Action'], rows));
}

function renderDiagnosticsModal(content, d) {
  content.textContent = '';
  [['Mount point', d.mountPoint], ['Filesystem', d.filesystem || 'Unknown'], ['Scan duration', `${d.scanDurationMs || 0} ms`], ['Scanner', d.scannerVersion], ['Last scan', d.lastScanTimestamp || 'Unknown']].forEach(([k, v]) => content.appendChild(storageSummaryLine(k, v)));
  const messages = [...(d.warnings || []), ...(d.overlapWarnings || []), ...(d.categoryScanErrors || []), ...(d.missingDirs || []).map((p) => `Missing folder: ${p}`), ...(d.permissionErrors || []).map((p) => `Permission denied: ${p}`)];
  if (!messages.length) content.appendChild(storageSummaryLine('Warnings', 'None'));
  else content.appendChild(storageTable(['Diagnostics'], messages.map((m) => [m])));
}

async function runStorageCleanup(action, endpoint) {
  const body = confirmationFor(action);
  if (body === null) return;
  const data = await postJson(endpoint, body);
  PopupManager.showToast(data.message || (data.ok ? 'Cleanup complete' : 'Cleanup failed'), data.ok ? 'success' : 'error');
  if (data.ok) openStorageModal('cleanup-review');
}

function updateControllerLiveInput(data) {
  const root = document.querySelector('[data-controller-live-input]');
  if (!root || !data) return;
  const state = root.querySelector('[data-controller-input-state]');
  if (state) state.textContent = titleCase((data.state || 'listening').replace(/-/g, ' '));
  const device = root.querySelector('[data-controller-input-device]');
  if (device) device.textContent = data.device || 'No controller detected';
  const pressed = new Set((data.pressed || []).map((item) => item.control));
  document.querySelectorAll('[data-controller-control]').forEach((pill) => {
    const active = pressed.has(pill.dataset.controllerControl || '');
    pill.classList.toggle('controller-button-dot--active', active);
    pill.classList.toggle('is-active', active);
  });
  const axes = root.querySelector('[data-controller-axes]');
  if (axes) {
    const values = data.axes || [];
    axes.innerHTML = '';
    if (!values.length) {
      const idle = document.createElement('span');
      idle.textContent = 'Move a stick or hold a button to light this pane.';
      axes.appendChild(idle);
    } else {
      values.forEach((axis) => {
        const pill = document.createElement('span');
        pill.className = 'controller-axis-pill';
        pill.textContent = `${axis.control || 'Axis'} ${axis.binding || ''}`.trim();
        axes.appendChild(pill);
      });
    }
  }
}


function formatControllerBinding(binding) {
  return String(binding || '').replace('button ', 'B').replace('axis ', 'AX').replace('hat 0', 'Hat');
}

function controllerProgrammerRoot() {
  return document.querySelector('[data-controller-programmer-modal]') || document.querySelector('[data-view-panel="controllers"]');
}

function bindControllerProgramming() {
  const panel = document.querySelector('[data-view-panel="controllers"]');
  if (!panel) return;
  let selected = null;
  let programmerTimer = null;
  let programmerPaused = false;
  const intervalMs = 60;

  const bindProfileCards = (root) => {
    root.querySelectorAll('button[data-controller-profile-action="apply"]:not([data-controller-profile-bound])').forEach((button) => {
      button.dataset.controllerProfileBound = 'true';
      button.addEventListener('click', async (event) => {
        event.preventDefault();
        const profile = button.dataset.controllerProfile || '';
        button.disabled = true;
        try {
          const data = await postJson('/api/actions/controllers-apply-profile', { profile });
          panel.querySelectorAll('[data-controller-profile-action="apply"]').forEach((node) => node.classList.toggle('controller-profile-card--active', node === button));
          const state = button.querySelector('em');
          if (state) state.textContent = data.ok ? 'Active' : 'Apply';
          PopupManager.showToast(data.message || `${profile} profile applied`, data.ok ? 'success' : 'error');
        } catch (_) {
          PopupManager.showToast(`${profile} profile failed`, 'error');
        } finally {
          button.disabled = false;
        }
      });
    });
  };

  const bindControlButtons = (root) => {
    root.querySelectorAll('button[data-controller-control]:not([data-controller-control-bound])').forEach((button) => {
      button.dataset.controllerControlBound = 'true';
      button.addEventListener('click', (event) => {
        event.preventDefault();
        const control = button.dataset.controllerControl || '';
        if (!control || control === 'D-pad') return;
        selected = control;
        root.querySelectorAll('[data-controller-control]').forEach((node) => node.classList.toggle('is-selected', node === button));
        const state = root.querySelector('[data-controller-programmer-state]');
        if (state) state.textContent = `Press controller for ${control}`;
        PopupManager.showToast(`Press controller input for ${control}`, 'info');
      });
    });
  };

  const ingestProgrammerInput = async (root, data) => {
    updateControllerLiveInput(data);
    const pressed = new Set((data.pressed || []).map((item) => item.control));
    root.querySelectorAll('[data-controller-control]').forEach((pill) => {
      const active = pressed.has(pill.dataset.controllerControl || '');
      pill.classList.toggle('controller-button-dot--active', active);
      pill.classList.toggle('is-active', active);
    });
    const device = root.querySelector('[data-controller-programmer-device]');
    if (device) device.textContent = data.device || 'No controller detected';
    const axes = root.querySelector('[data-controller-programmer-axes]');
    if (axes) {
      axes.textContent = '';
      (data.axes || []).forEach((axis) => {
        const pill = document.createElement('span');
        pill.className = 'controller-axis-pill';
        pill.textContent = `${axis.control || 'Axis'} ${axis.binding || ''}`.trim();
        axes.appendChild(pill);
      });
    }
    if (selected && data.pressed && data.pressed.length) {
      const input = data.pressed[0].binding || data.pressed[0].input || data.pressed[0].control;
      if (input) {
        const result = await postJson('/api/actions/controllers-bind', { control: selected, binding: input });
        const selectedButton = root.querySelector(`[data-controller-control="${selected}"]`);
        const label = selectedButton?.querySelector('span, em');
        if (label && result.stdout) label.textContent = formatControllerBinding(result.stdout);
        PopupManager.showToast(result.message || `${selected} mapped`, result.ok ? 'success' : 'error');
        selected = null;
      }
    }
  };

  const startProgrammerLoop = (root) => {
    if (programmerTimer) window.clearInterval(programmerTimer);
    bindControlButtons(root);
    const readout = root.querySelector('[data-controller-broadcast-readout]');
    if (readout) readout.textContent = `${intervalMs}ms`;
    programmerTimer = window.setInterval(async () => {
      if (programmerPaused || !document.querySelector('[data-controller-programmer-modal]')) return;
      try { await ingestProgrammerInput(root, await getJson('/api/controllers/input')); } catch (_) {}
    }, intervalMs);
    root.querySelector('[data-controller-broadcast-toggle]')?.addEventListener('click', (event) => {
      programmerPaused = !programmerPaused;
      event.currentTarget.textContent = programmerPaused ? 'Resume broadcast' : 'Pause broadcast';
    });
  };

  bindProfileCards(panel);
  bindControlButtons(panel);
  panel.querySelectorAll('[data-controller-programmer-open]:not([data-controller-programmer-bound])').forEach((button) => {
    button.dataset.controllerProgrammerBound = 'true';
    button.addEventListener('click', () => {
      const template = document.getElementById('controller-programmer-template');
      const body = template?.content?.firstElementChild?.cloneNode(true);
      if (!body) return;
      PopupManager.showModal({ title: 'Controller programmer', body, hideDefaultAction: true });
      const root = document.querySelector('[data-controller-programmer-modal]');
      if (root) startProgrammerLoop(root);
    });
  });

  window.arcadiaControllerProgramming = {
    selectedControl: () => selected,
    broadcastMs: () => intervalMs,
    modalOpen: () => Boolean(document.querySelector('[data-controller-programmer-modal]')),
  };
}

function bindControllerLiveInput() {
  const root = document.querySelector('[data-controller-live-input]');
  if (!root) return;
  bindControllerProgramming();
  const poll = async () => {
    const active = document.querySelector('[data-view-panel="controllers"].is-active, [data-view-panel="controllers"].view--active, [data-view-panel="controllers"].active');
    if (!active || document.querySelector('[data-controller-programmer-modal]')) return;
    try { updateControllerLiveInput(await getJson('/api/controllers/input')); } catch (_) {}
  };
  poll();
  window.setInterval(poll, 650);
}

function bindStorageModals() {
  document.querySelectorAll('[data-storage-modal]').forEach((button) => button.addEventListener('click', () => openStorageModal(button.dataset.storageModal)));
  document.querySelectorAll('[data-storage-cleanup]').forEach((button) => button.addEventListener('click', () => {
    const endpoints = { 'clear-artwork-cache': '/api/storage/cleanup/artwork', 'clean-temporary-files': '/api/storage/cleanup/temporary', 'clear-old-updates': '/api/storage/cleanup/old-updates', 'prune-logs': '/api/storage/cleanup/logs', 'clear-partial-ai-downloads': '/api/storage/cleanup/partial-ai-downloads' };
    runStorageCleanup(button.dataset.storageCleanup, endpoints[button.dataset.storageCleanup]);
  }));
  document.querySelectorAll('[data-sync-add-games]').forEach((button) => button.addEventListener('click', () => openSyncAddGamesModal()));
}

function validCopyValue(value) {
  return Boolean(value) && !/smb::|smb:\/[^/]|\\undefined|smb:\/\/undefined|undefined/i.test(value);
}

async function copyToClipboard(value) {
  try {
    if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(value); return true; }
  } catch (_) {}
  try {
    const ta = document.createElement('textarea');
    ta.value = value;
    ta.setAttribute('readonly', '');
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    return ok;
  } catch (_) { return false; }
}

function prepareSyncStart() {
  const root = document.querySelector('[data-sync-root]');
  const stateNode = document.getElementById('sync-state');
  if (['scanning', 'syncing'].includes(stateNode?.dataset.syncState) || root?.dataset.syncState === 'running') {
    PopupManager.showToast('Sync is already running.', 'error');
    return false;
  }
  if (root?.dataset.storageBlocked === 'true' || root?.dataset.storageHealth === 'Full') {
    const message = 'Storage is full. Free space before syncing.';
    setMessage('console-action-message', message, 'error');
    PopupManager.showToast(message, 'error');
    document.querySelector('[data-nav-target="storage"]')?.focus();
    return false;
  }
  if (root?.dataset.storageLow === 'true') {
    PopupManager.showToast('Storage is low. Artwork or library updates may need more room.', 'error');
  }
  return true;
}

function setSyncState(label, state = label) {
  const value = String(state || label).toLowerCase().replace(/\s+/g, '-');
  const node = document.getElementById('sync-state');
  if (node) {
    node.textContent = label;
    node.dataset.syncState = value;
  }
  const root = document.querySelector('[data-sync-root]');
  if (root) root.dataset.syncState = value;
  document.querySelectorAll('.status-badge').forEach((badge) => {
    if (badge.dataset.chipKind !== 'games') return;
    badge.setAttribute('aria-label', `Games: ${label}`);
    badge.title = `Games: ${label}`;
  });
}

function setSyncReadback(kind, message) {
  const result = document.querySelector('[data-sync-result]');
  const resultCopy = document.getElementById('sync-result-copy');
  if (result) result.dataset.syncResult = kind;
  if (resultCopy) resultCopy.textContent = message;
}

function startSyncProgress() {
  setSyncState('Scanning', 'scanning');
  const panel = document.getElementById('sync-running-panel');
  const progress = document.getElementById('sync-progress-text');
  if (panel) panel.hidden = false;
  if (progress) progress.textContent = 'Preparing game import…';
  setSyncReadback('running', 'The machine is importing games now.');
  return { stop() { if (panel) panel.hidden = true; } };
}

function finishSyncProgress(ok, data = {}, progressHandle = null) {
  if (progressHandle && typeof progressHandle.stop === 'function') progressHandle.stop();
  const button = document.querySelector('[data-action="sync-games"]');
  const progress = document.getElementById('sync-progress-text');
  if (ok) {
    setSyncState('Synced', 'synced');
    if (button) button.textContent = 'Synced';
    const message = data.message || 'Games synced. Receipt ready.';
    if (progress) progress.textContent = message;
    setSyncReadback('success', 'Games synced. Receipt ready.');
    markOnboardingFirstSyncComplete(data);
  } else {
    setSyncState('Sync failed', 'sync-failed');
    if (button) button.textContent = 'Sync failed';
    const message = data.message || 'Sync failed. Open the ledger for the reason and fix action.';
    if (progress) progress.textContent = message;
    setSyncReadback('error', 'Sync failed. Open the ledger for the reason and fix action.');
  }
}

const SYNC_GAME_KINDS = [
  ['gba', 'GBA'],
  ['genesis', 'Genesis'],
  ['snes', 'SNES'],
  ['nes', 'NES'],
  ['ps1', 'PS1'],
  ['n64', 'N64'],
  ['ps2', 'PS2'],
  ['sega-cd', 'Sega CD'],
  ['psp', 'PSP'],
  ['gamecube', 'GameCube'],
  ['wii', 'Wii'],
  ['dos', 'DOS'],
];
let selectedSyncGameKind = null;

function openSyncAddGamesModal() {
  selectedSyncGameKind = null;
  const body = document.createElement('div');
  body.className = 'sync-add-games-modal';
  const intro = document.createElement('p');
  intro.className = 'modal-note';
  intro.textContent = 'Select the game kind first. The next files you choose go to that kind.';
  body.appendChild(intro);

  const grid = document.createElement('div');
  grid.className = 'sync-kind-grid';
  SYNC_GAME_KINDS.forEach(([value, label]) => {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'sync-kind-option';
    button.dataset.syncGameKind = value;
    button.textContent = label;
    button.addEventListener('click', () => selectSyncGameKind(value, label, body));
    grid.appendChild(button);
  });
  body.appendChild(grid);

  const drop = document.createElement('div');
  drop.className = 'sync-kind-dropzone';
  drop.dataset.syncKindDropzone = 'true';
  drop.hidden = true;
  drop.innerHTML = '<strong data-sync-kind-title>Select a game kind</strong><span>Drop files here after selecting the kind.</span>';
  body.appendChild(drop);

  const actions = document.createElement('div');
  actions.className = 'inline-actions';
  const choose = document.createElement('button');
  choose.type = 'button';
  choose.className = 'btn btn--primary';
  choose.dataset.syncChooseFiles = 'true';
  choose.disabled = true;
  choose.textContent = 'Select a kind first';
  const cancel = document.createElement('button');
  cancel.type = 'button';
  cancel.className = 'btn btn--secondary';
  cancel.textContent = 'Cancel';
  cancel.addEventListener('click', () => PopupManager.closeModal());
  actions.append(choose, cancel);
  body.appendChild(actions);

  const input = document.createElement('input');
  input.type = 'file';
  input.multiple = true;
  input.className = 'sync-upload-input';
  input.dataset.syncUpload = 'true';
  input.dataset.endpoint = '/api/actions/add-games';
  input.addEventListener('change', () => uploadSyncGames(input));
  body.appendChild(input);

  choose.addEventListener('click', () => {
    if (!selectedSyncGameKind) return PopupManager.showToast('Select a game kind first.', 'error');
    input.click();
  });
  bindSyncDropzone(drop);
  PopupManager.showModal({ title: 'Add games', body, hideDefaultAction: true });
}

function selectSyncGameKind(value, label, root) {
  selectedSyncGameKind = value;
  root.querySelectorAll('[data-sync-game-kind]').forEach((button) => button.dataset.selected = String(button.dataset.syncGameKind === value));
  const choose = root.querySelector('[data-sync-choose-files]');
  if (choose) { choose.disabled = false; choose.textContent = `Add ${label} files`; }
  const drop = root.querySelector('[data-sync-kind-dropzone]');
  if (drop) {
    drop.hidden = false;
    const title = drop.querySelector('[data-sync-kind-title]');
    if (title) title.textContent = `${label} selected`;
  }
}

function bindSyncDropzone(dropzone) {
  dropzone.addEventListener('dragover', (event) => { event.preventDefault(); if (selectedSyncGameKind) dropzone.dataset.dragActive = 'true'; });
  dropzone.addEventListener('dragleave', () => { dropzone.dataset.dragActive = 'false'; });
  dropzone.addEventListener('drop', (event) => {
    event.preventDefault();
    dropzone.dataset.dragActive = 'false';
    if (!selectedSyncGameKind) return PopupManager.showToast('Select a game kind first.', 'error');
    uploadSyncFiles(Array.from(event.dataTransfer?.files || []), selectedSyncGameKind, '/api/actions/add-games');
  });
}

async function uploadSyncGames(input) {
  const files = Array.from(input.files || []);
  await uploadSyncFiles(files, selectedSyncGameKind, input.dataset.endpoint || '/api/actions/add-games');
  input.value = '';
}

async function uploadSyncFiles(files, gameKind, endpoint = '/api/actions/add-games') {
  if (!gameKind) return PopupManager.showToast('Select a game kind first.', 'error');
  if (!files.length) return;
  const form = new FormData();
  form.append('system', gameKind);
  files.forEach((file) => form.append('games', file, file.name));
  setSyncReadback('running', 'Checking added games…');
  PopupManager.showToast(`${files.length} game file${files.length === 1 ? '' : 's'} entering the machine.`, 'info');
  try {
    const response = await fetch(endpoint, { method: 'POST', body: form });
    const data = await response.json().catch(() => ({}));
    if (!response.ok || data.ok === false) {
      const message = data.message || 'Some games were rejected. Open the ledger for the reason and fix action.';
      setSyncReadback('error', message);
      PopupManager.showToast(message, 'error');
      return;
    }
    const message = data.message || 'Games staged. Press Sync games.';
    setSyncReadback('success', message);
    PopupManager.showToast(message, 'success');
    PopupManager.closeModal();
  } catch (_) {
    const message = 'The machine could not accept those games.';
    setSyncReadback('error', message);
    PopupManager.showToast(message, 'error');
  }
}


function initializeOnboarding() {
  const card = document.querySelector('[data-onboarding-card]');
  if (!card) return;
  let done = false;
  let hidden = false;
  try {
    done = localStorage.getItem('onboarding.firstSyncComplete') === 'true';
    hidden = sessionStorage.getItem('onboarding.hideForNow') === 'true';
  } catch (_) {}
  card.hidden = done || hidden;
  card.querySelector('[data-onboarding-hide-session]')?.addEventListener('click', () => {
    try { sessionStorage.setItem('onboarding.hideForNow', 'true'); } catch (_) {}
    card.hidden = true;
  });
  updateOnboardingSteps(done);
}

function updateOnboardingSteps(firstSyncComplete = false) {
  const steps = document.querySelectorAll('[data-onboarding-step]');
  steps.forEach((step) => {
    const n = step.dataset.onboardingStep;
    let state = 'Not Started';
    if (n === '1' || n === '2') state = 'Ready';
    if (firstSyncComplete) state = 'Complete';
    if (n === '3' && !firstSyncComplete && document.getElementById('sync-state')?.textContent === 'Sync Running') state = 'In Progress';
    step.dataset.onboardingState = state;
    const badge = step.querySelector('.onboarding-state');
    if (badge) badge.textContent = state;
  });
}

function markOnboardingFirstSyncComplete(data = {}) {
  if (data.ok === false) return;
  try { localStorage.setItem('onboarding.firstSyncComplete', 'true'); } catch (_) {}
  updateOnboardingSteps(true);
  const card = document.querySelector('[data-onboarding-card]');
  if (card) card.hidden = true;
}

async function requestNetworkState() {
  const res = await fetch('/api/network/state', { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error('network state failed');
  return await res.json();
}

async function requestWifiScan(quiet = false) {
  clearMessage('wifi-message');
  try {
    const data = await postJson('/api/network/wifi/scan', {});
    openWifiNetworkPicker(data.state, data.message || 'Wi-Fi scan complete.');
    if (data.ok) clearMessage('wifi-message');
    else setMessage('wifi-message', data.message || 'Wi-Fi scan failed.', 'error');
    if (!quiet) PopupManager.showToast(data.message || 'Wi-Fi scan complete.', data.ok ? 'success' : 'error');
  } catch (_) {
    setMessage('wifi-message', 'Wi-Fi scan failed.', 'error');
    if (!quiet) PopupManager.showToast('Wi-Fi scan failed', 'error');
  }
}

function normalizeWifiNetworks(stateOrNetworks) {
  const state = Array.isArray(stateOrNetworks) ? { wifi: { scanResults: stateOrNetworks } } : (stateOrNetworks || {});
  const wifi = state.wifi || {};
  const saved = new Map((wifi.savedNetworks || []).filter((n) => n.ssid).map((n) => [n.ssid, n]));
  const connectedSsid = wifi.connectedSsid || null;
  const groups = new Map();
  (wifi.scanResults || []).forEach((ap) => {
    const ssid = (ap.ssid || '').trim();
    if (!ssid) return;
    const known = Boolean(ap.saved || saved.has(ssid));
    const connected = Boolean(ap.connected || ssid === connectedSsid);
    const item = {
      ssid,
      bssid: ap.bssid || undefined,
      signalPercent: Number.isFinite(ap.signalPercent) ? ap.signalPercent : undefined,
      security: ap.security || saved.get(ssid)?.security || undefined,
      channel: ap.channel || undefined,
      known,
      connected,
    };
    if (!groups.has(ssid)) groups.set(ssid, { ssid, connected: false, known: false, accessPoints: [] });
    const group = groups.get(ssid);
    group.connected = group.connected || connected;
    group.known = group.known || known;
    group.accessPoints.push(item);
  });
  saved.forEach((network, ssid) => {
    if (!ssid || groups.has(ssid)) return;
    groups.set(ssid, {
      ssid,
      connected: ssid === connectedSsid,
      known: true,
      accessPoints: [{ ssid, security: network.security, known: true, connected: ssid === connectedSsid }],
    });
  });
  return Array.from(groups.values()).map((group) => {
    group.accessPoints.sort((a, b) => (b.signalPercent ?? -1) - (a.signalPercent ?? -1));
    const best = group.accessPoints[0] || {};
    group.bestSignalPercent = best.signalPercent;
    group.security = best.security;
    return group;
  }).sort((a, b) => {
    if (a.connected !== b.connected) return a.connected ? -1 : 1;
    if (a.known !== b.known) return a.known ? -1 : 1;
    if ((a.bestSignalPercent ?? -1) !== (b.bestSignalPercent ?? -1)) return (b.bestSignalPercent ?? -1) - (a.bestSignalPercent ?? -1);
    return a.ssid.localeCompare(b.ssid);
  });
}

function wifiSignalText(percent) {
  return Number.isFinite(percent) && percent > 0 ? `${percent}% signal` : 'Signal unavailable';
}

function openWifiNetworkPicker(state, message = '') {
  const body = document.createElement('div');
  body.className = 'wifi-picker-modal';
  const controls = document.createElement('div');
  controls.className = 'modal-control-row';
  const scan = document.createElement('button');
  scan.type = 'button';
  scan.className = 'btn btn--primary';
  scan.textContent = 'Scan';
  scan.addEventListener('click', () => requestWifiScan(true));
  const hidden = document.createElement('button');
  hidden.type = 'button';
  hidden.className = 'btn btn--secondary';
  hidden.textContent = 'Join Hidden Network';
  hidden.addEventListener('click', () => openHiddenNetworkModal());
  controls.append(scan, hidden);
  body.appendChild(controls);
  if (message) {
    const note = document.createElement('p');
    note.className = 'modal-note';
    note.textContent = message;
    body.appendChild(note);
  }
  const list = document.createElement('div');
  list.className = 'wifi-network-list wifi-network-list--modal';
  const groups = normalizeWifiNetworks(state || {});
  if (groups.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'empty-state';
    empty.innerHTML = '<strong>No visible networks found</strong><p>Scan again or join a hidden network.</p>';
    list.appendChild(empty);
  }
  groups.forEach((group) => list.appendChild(wifiGroupRow(group)));
  body.appendChild(list);
  PopupManager.showModal({ title: 'Choose Wi-Fi Network', body, hideDefaultAction: true });
}

function wifiGroupRow(group) {
  const row = document.createElement('div');
  row.className = `network-row wifi-network-row ${group.connected ? 'network-row--active' : ''}`;
  const summary = document.createElement('span');
  const title = document.createElement('strong');
  title.textContent = group.ssid;
  const meta = document.createElement('em');
  meta.textContent = [wifiSignalText(group.bestSignalPercent), securityLabel(group.security), group.connected ? 'Connected' : '', group.known ? 'Saved' : ''].filter(Boolean).join(' · ');
  summary.append(title, meta);
  const connect = document.createElement('button');
  connect.type = 'button';
  connect.className = 'btn btn--secondary';
  connect.textContent = group.connected ? 'Disconnect' : 'Connect';
  connect.addEventListener('click', () => {
    if (group.connected) return postNetworkAction('/api/network/wifi/disconnect', {}, 'wifi-disconnect');
    openWifiConnectModal(group.ssid, group.security !== 'open', group.known);
  });
  row.append(summary, connect);
  if (group.known || group.connected) {
    const forget = document.createElement('button');
    forget.type = 'button';
    forget.className = 'btn btn--secondary';
    forget.textContent = 'Forget';
    forget.addEventListener('click', () => postNetworkAction('/api/network/wifi/forget', { ssid: group.ssid }, 'wifi-forget'));
    row.appendChild(forget);
  }
  const details = document.createElement('details');
  details.className = 'wifi-ap-details';
  const detailSummary = document.createElement('summary');
  detailSummary.textContent = group.accessPoints.length > 1 ? `${group.accessPoints.length} access points` : 'Details';
  details.appendChild(detailSummary);
  group.accessPoints.forEach((ap) => {
    const line = document.createElement('div');
    line.className = 'wifi-ap-line';
    line.textContent = [ap.bssid || 'BSSID unavailable', wifiSignalText(ap.signalPercent), securityLabel(ap.security), ap.channel ? `Channel ${ap.channel}` : ''].filter(Boolean).join(' · ');
    details.appendChild(line);
  });
  row.appendChild(details);
  return row;
}

function securityLabel(value) {
  if (value === 'open') return 'Open';
  if (value === 'wpa2') return 'WPA2';
  if (value === 'wpa3') return 'WPA3';
  if (value === 'wpa-wpa2') return 'WPA/WPA2';
  if (!value) return '';
  return String(value).toUpperCase();
}

function openWifiConnectModal(ssid = '', secured = true, saved = false) {
  const form = document.createElement('form');
  form.className = 'settings-form wifi-connect-panel';
  form.autocomplete = 'off';
  form.innerHTML = `
    <div class="selected-network"><span>Selected network</span><strong></strong></div>
    <label ${ssid ? 'hidden' : ''}><span>Network name</span><input class="field" name="ssid" autocomplete="off"></label>
    <label ${secured ? '' : 'hidden'}><span>Password</span><input class="field" type="password" name="password" autocomplete="new-password"></label>
    <label class="wifi-show-password" ${secured ? '' : 'hidden'}><input type="checkbox" data-toggle-modal-password="password">Show password</label>
    <div class="inline-actions"><button class="btn btn--primary" type="submit">Connect</button><button class="btn btn--secondary" type="button" data-modal-cancel>Cancel</button>${saved ? '<button class="btn btn--secondary" type="button" data-modal-forget>Forget network</button>' : ''}</div>
    <div class="message" hidden></div>`;
  form.querySelector('.selected-network strong').textContent = ssid || 'Hidden network';
  const ssidInput = form.querySelector('input[name="ssid"]');
  if (ssidInput) ssidInput.value = ssid;
  form.querySelector('[data-toggle-modal-password]')?.addEventListener('change', (event) => {
    const input = form.querySelector('input[name="password"]');
    if (input) input.type = event.target.checked ? 'text' : 'password';
  });
  form.querySelector('[data-modal-cancel]')?.addEventListener('click', () => PopupManager.closeModal());
  form.querySelector('[data-modal-forget]')?.addEventListener('click', () => postNetworkAction('/api/network/wifi/forget', { ssid }, 'wifi-forget'));
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const chosenSsid = form.querySelector('input[name="ssid"]')?.value || ssid;
    const passwordInput = form.querySelector('input[name="password"]');
    const password = passwordInput?.value || '';
    const msg = form.querySelector('.message');
    if (!chosenSsid.trim()) { msg.textContent = 'Wi-Fi network name is required.'; msg.className = 'message message--error'; msg.hidden = false; return; }
    const button = form.querySelector('button[type="submit"]');
    const old = button.textContent;
    button.disabled = true; button.textContent = 'Joining…';
    msg.textContent = 'Joining network…\nAuthenticating…\nRequesting IP address…\nTesting LAN…\nTesting Internet…';
    msg.className = 'message message--info'; msg.hidden = false;
    try {
      const data = await postJson('/api/network/wifi/connect', { ssid: chosenSsid, password });
      if (passwordInput) passwordInput.value = '';
      msg.textContent = data.message || 'Wi-Fi connect complete.';
      msg.className = `message message--${data.ok ? 'success' : 'error'}`;
      PopupManager.showToast(data.message || 'Wi-Fi connect complete.', data.ok ? 'success' : 'error');
      if (data.ok) PopupManager.closeModal();
    } catch (_) {
      if (passwordInput) passwordInput.value = '';
      msg.textContent = 'Wi-Fi connect request failed.';
      msg.className = 'message message--error';
    } finally {
      button.disabled = false; button.textContent = old;
    }
  });
  PopupManager.showModal({ title: ssid ? 'Connect Wi-Fi' : 'Join Hidden Network', body: form, hideDefaultAction: true });
  (secured ? form.querySelector('input[name="password"]') : ssidInput)?.focus();
}

function openHiddenNetworkModal() {
  openWifiConnectModal('', true, false);
}

function openWiredDetailsModal() {
  requestNetworkState().then((state) => {
    const body = document.createElement('div');
    body.className = 'network-detail-modal';
    const e = state.ethernet || {};
    const a = state.activeConnection || {};
    const rows = [
      ['Link state', e.connected ? 'Connected' : 'Cable disconnected'],
      ['Speed', e.speedMbps ? `${e.speedMbps} Mbps` : 'Unknown'],
      ['IP address', e.ip || a.ip || 'Unavailable'],
      ['MAC address', e.macAddress || 'Unknown'],
      ['Mode', e.dhcp ? 'DHCP' : 'Manual'],
      ['Gateway', e.gateway || a.gateway || 'Unknown'],
      ['DNS servers', (e.dnsServers || a.dnsServers || []).join(', ') || 'Unknown'],
      ['Hostname', state.appliance?.hostname || 'homeconsole'],
      ['Web console URL', state.appliance?.webOrigin || 'http://console.home.arpa'],
    ];
    rows.forEach(([label, value]) => body.appendChild(detailRow(label, value, label.includes('URL') || label.includes('address') || label === 'Gateway')));
    PopupManager.showModal({ title: 'Wired LAN Details', body });
  }).catch(() => PopupManager.showToast('Network details unavailable', 'error'));
}

function detailRow(label, value, copy = false) {
  const row = document.createElement('div');
  row.className = 'network-detail-row';
  const text = document.createElement('span');
  text.innerHTML = `<em></em><strong></strong>`;
  text.querySelector('em').textContent = label;
  text.querySelector('strong').textContent = value;
  row.appendChild(text);
  if (copy && validCopyValue(value)) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'btn btn--secondary';
    button.textContent = 'Copy';
    button.addEventListener('click', async () => PopupManager.showToast((await copyToClipboard(value)) ? `Copied ${value}` : `Copy unavailable: ${value}`));
    row.appendChild(button);
  }
  return row;
}

function openIpSettingsModal() {
  const form = document.createElement('form');
  form.className = 'settings-form';
  form.autocomplete = 'off';
  form.innerHTML = `
    <label><span>Mode</span><select class="field" name="mode"><option value="dhcp" selected>Automatic DHCP</option><option value="manual">Manual IPv4</option></select></label>
    <label><span>IP address</span><input class="field" name="ip" inputmode="numeric" placeholder="192.168.123.54"></label>
    <label><span>Subnet prefix</span><input class="field" name="prefixLength" inputmode="numeric" placeholder="24"></label>
    <label><span>Gateway</span><input class="field" name="gateway" inputmode="numeric" placeholder="192.168.123.1"></label>
    <label><span>DNS servers</span><input class="field" name="dnsServers" placeholder="192.168.123.1 1.1.1.1"></label>
    <p class="warning">Changing IP settings may disconnect the web console. Confirm reachability after applying or roll back.</p>
    <div class="inline-actions"><button class="btn btn--primary" type="submit">Apply Settings</button><button class="btn btn--secondary" type="button" data-network-action="rollback-ip">Rollback</button><button class="btn btn--secondary" type="button" data-modal-cancel>Cancel</button></div>`;
  form.querySelector('[data-modal-cancel]')?.addEventListener('click', () => PopupManager.closeModal());
  form.querySelector('[data-network-action="rollback-ip"]')?.addEventListener('click', () => postNetworkAction('/api/network/ip/rollback', {}, 'rollback-ip'));
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const dns = (form.querySelector('input[name="dnsServers"]')?.value || '').split(/[ ,]+/).filter(Boolean);
    const body = {
      mode: form.querySelector('[name="mode"]')?.value || 'dhcp',
      ip: form.querySelector('input[name="ip"]')?.value || null,
      prefixLength: Number(form.querySelector('input[name="prefixLength"]')?.value || 0) || null,
      gateway: form.querySelector('input[name="gateway"]')?.value || null,
      dnsServers: dns,
    };
    const data = await postJson('/api/network/ip/apply', body);
    PopupManager.showModal({ title: data.ok ? 'Network settings changed' : 'Network settings not applied', body: data.message || '' });
  });
  PopupManager.showModal({ title: 'IP Settings', body: form, hideDefaultAction: true });
}

async function postNetworkAction(url, body, actionName) {
  const data = await postJson(url, body);
  if (data.ok) clearMessage('wifi-message');
  else setMessage('wifi-message', data.message || 'Network action failed.', 'error');
  PopupManager.showToast(data.message || 'Network action complete.', data.ok ? 'success' : 'error');
  return data;
}

function bindNetworkControls() {
  document.querySelectorAll('[data-network-action]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.networkAction;
      if (action === 'choose-wifi' || action === 'scan-wifi') return requestWifiScan(false);
      if (action === 'wifi-toggle') return postNetworkAction('/api/network/wifi/set-enabled', { enabled: button.dataset.enabled === 'true' }, action);
      if (action === 'disconnect-wifi') return postNetworkAction('/api/network/wifi/disconnect', {}, action);
      if (action === 'forget-wifi') return postNetworkAction('/api/network/wifi/forget', { ssid: button.dataset.ssid || '' }, action);
      if (action === 'renew-dhcp') return postNetworkAction('/api/network/ethernet/renew-dhcp', {}, action);
      if (action === 'rollback-ip') return postNetworkAction('/api/network/ip/rollback', {}, action);
    });
  });
  document.querySelectorAll('[data-open-hidden-wifi]').forEach((button) => button.addEventListener('click', () => openHiddenNetworkModal()));
  document.querySelectorAll('[data-open-wired-details]').forEach((button) => button.addEventListener('click', () => openWiredDetailsModal()));
  document.querySelectorAll('[data-open-ip-settings]').forEach((button) => button.addEventListener('click', () => openIpSettingsModal()));
  document.querySelectorAll('[data-diagnostic]').forEach((button) => button.addEventListener('click', async () => {
    const data = await postJson('/api/network/diagnostics/run', { tests: [button.dataset.diagnostic] });
    const root = document.getElementById('diagnostics-results');
    if (root) root.innerHTML = (data.results || []).map((r) => `<div class="network-row"><span><strong>${escapeHtml(r.name)}</strong><em>${escapeHtml(r.message)}</em></span><b class="system-status system-status--${r.ok ? 'available' : 'error'}">${r.ok ? 'OK' : 'Check'}</b></div>`).join('');
    PopupManager.showToast((data.results && data.results[0]?.message) || 'Diagnostic complete', data.ok ? 'success' : 'error');
  }));
}

function escapeHtml(value) {
  return String(value || '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
}


async function requestAIState() {
  const res = await fetch('/api/ai/state', { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error('AI state failed');
  return await res.json();
}

async function postAI(endpoint, body = {}, label = 'Local AI action') {
  const data = await postJson(endpoint, body);
  if (data.ok) clearMessage('ai-message');
  else setMessage('ai-message', data.message || label, 'error');
  PopupManager.showToast(data.message || label, data.ok ? 'success' : 'error');
  return data;
}

function bindLocalAIControls() {
  document.querySelectorAll('[data-ai-action]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.aiAction;
      const modelId = button.dataset.modelId || null;
      const filename = button.dataset.filename || null;
      const original = button.textContent;
      button.disabled = true;
      button.textContent = action.includes('download') ? 'Downloading…' : 'Working…';
      try {
        if (action === 'runtime-check-update') await postAI('/api/ai/runtime/check-update', {}, 'Runtime check complete');
        else if (action === 'runtime-update') await postAI('/api/ai/runtime/update', {}, 'Runtime update started');
        else if (action === 'runtime-restart') await postAI('/api/ai/runtime/restart', {}, 'Runtime restart complete');
        else if (action === 'install-recommended') await postAI('/api/ai/models/install-recommended', { id: modelId || 'recommended' }, 'Recommended model install');
        else if (action === 'model-select') await postAI('/api/ai/model/select', { modelId }, 'Model selected');
        else if (action === 'model-load') await postAI('/api/ai/model/load', { modelId }, 'Model load requested');
        else if (action === 'model-unload') await postAI('/api/ai/model/unload', {}, 'Model unloaded');
        else if (action === 'model-remove') {
          if (!window.confirm('Remove this model from console storage?\nGames and artwork are not affected.')) return;
          await postAI('/api/ai/models/remove', { modelId, filename, confirm: 'REMOVE_MODEL' }, 'Model removed');
        }
        else if (action === 'inference-enable') await postAI('/api/ai/inference/set-enabled', { enabled: true }, 'API enabled');
        else if (action === 'inference-disable') await postAI('/api/ai/inference/set-enabled', { enabled: false }, 'API disabled');
        else if (action === 'inference-test') await postAI('/api/ai/inference/test', {}, 'API tested');
        else if (action === 'models-rescan') await postAI('/api/ai/models/rescan', {}, 'Models rescanned');
        else if (action === 'lan-enable') await applyLocalAIPort(true);
        else if (action === 'lan-disable') await postAI('/api/ai/inference/set-lan-access', { enabled: false }, 'LAN disabled');
        else if (action === 'token-generate') { if (window.confirm('Generate a new local client token? Existing client configs may need updating.')) await postAI('/api/ai/token/generate', { confirm: 'GENERATE_TOKEN' }, 'Token generated'); }
        else if (action === 'token-revoke') { if (window.confirm('Revoke the local client token?')) await postAI('/api/ai/token/revoke', { confirm: 'REVOKE_TOKEN' }, 'Token revoked'); }
        else if (action === 'hf-list-files') await fetchHFFiles();
        else if (action === 'hf-download') await downloadHFModel();
      } catch (_) {
        setMessage('ai-message', 'Local AI request failed.', 'error');
        PopupManager.showToast('Local AI request failed', 'error');
      } finally {
        button.disabled = false;
        button.textContent = original;
      }
    });
  });
  const importForm = document.getElementById('ai-import-form');
  if (importForm) importForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = importForm.querySelector('input[type="file"]');
    const file = input?.files?.[0];
    if (!file) return PopupManager.showToast('Choose a .gguf model file first', 'error');
    if (!file.name.toLowerCase().endsWith('.gguf')) return PopupManager.showToast('Only .gguf model files are supported', 'error');
    const progress = document.getElementById('ai-import-progress');
    const data = new FormData();
    data.append('model', file, file.name);
    if (progress) { progress.hidden = false; progress.value = 10; }
    try {
      const res = await fetch('/api/ai/models/import', { method: 'POST', body: data, headers: { accept: 'application/json' } });
      const payload = await res.json().catch(() => ({}));
      if (progress) progress.value = 100;
      PopupManager.showToast(payload.message || (res.ok ? 'Model imported' : 'Model import failed'), res.ok && payload.ok !== false ? 'success' : 'error');
      if (!res.ok || payload.ok === false) setMessage('ai-message', payload.message || 'Model import failed.', 'error'); else clearMessage('ai-message');
    } catch (_) { PopupManager.showToast('Model import request failed', 'error'); }
  });
  const lanForm = document.getElementById('ai-lan-form');
  if (lanForm) lanForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    await saveLocalAIPort();
  });
  lanForm?.querySelector('[data-ai-port-revert]')?.addEventListener('click', () => {
    const active = lanForm.dataset.activePort || '7777';
    const input = lanForm.querySelector('input[name="port"]');
    if (input) input.value = active;
    setLocalAIPortState(`Active ${active}`, 'unknown');
  });
  const settingsForm = document.getElementById('ai-settings-form');
  if (settingsForm) settingsForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const val = (name) => settingsForm.querySelector(`[name="${name}"]`);
    await postAI('/api/ai/settings', {
      contextSize: Number(val('contextSize')?.value || 4096),
      gpuLayers: Number(val('gpuLayers')?.value || -1),
      threads: Number(val('threads')?.value || 0),
      batch: Number(val('batch')?.value || 512),
      startApiOnBoot: Boolean(val('startApiOnBoot')?.checked),
      autoLoadLastModel: Boolean(val('autoLoadLastModel')?.checked),
    }, 'Local AI settings saved');
  });
  document.querySelectorAll('[data-ai-logs]').forEach((button) => button.addEventListener('click', async () => {
    try {
      const state = await requestAIState();
      const a = state.activity || {};
      PopupManager.showModal({ title: 'Local AI Logs', body: [a.runtimeUpdateLog, a.modelDownloadLog, a.modelLoadLog, a.inferenceServerLog].filter(Boolean).join('\n\n') || 'No Local AI logs reported.' });
    } catch (_) { PopupManager.showToast('Local AI logs unavailable', 'error'); }
  }));
}

function localAIPortPayload() {
  const form = document.getElementById('ai-lan-form');
  const rawPort = form?.querySelector('input[name="port"]')?.value.trim() || '';
  const lanCidr = form?.querySelector('input[name="lanCidr"]')?.value.trim() || '192.168.123.0/24';
  if (!/^\d+$/.test(rawPort)) throw new Error('Port must be a whole number.');
  const port = Number(rawPort);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Port must be between 1024 and 65535.');
  if ([22, 80, 443, 445, 8080].includes(port)) throw new Error('That port is reserved for HomeConsole services.');
  if (!/^\d{1,3}(\.\d{1,3}){3}\/\d{1,2}$/.test(lanCidr)) throw new Error('LAN CIDR must look like 192.168.123.0/24.');
  return { port, lanCidr };
}

function setLocalAIPortState(text, state = 'unknown') {
  const node = document.getElementById('ai-port-state');
  if (!node) return;
  node.textContent = text;
  node.className = `system-status system-status--${state}`;
}

function updateLocalAIPortReadback(data) {
  const state = data?.state || data;
  const port = state?.inference?.port;
  const endpoints = state?.inference?.endpointUrls || [];
  const endpoint = endpoints[0] || state?.clientHandoff?.endpoint || '';
  const form = document.getElementById('ai-lan-form');
  if (port && form) form.dataset.activePort = String(port);
  if (port) setLocalAIPortState(`Active ${port}`, data?.ok === false ? 'error' : 'available');
  const readback = document.getElementById('ai-endpoint-readback');
  if (readback && endpoint) {
    readback.textContent = endpoint;
    readback.dataset.aiEndpoint = endpoint;
  }
}

async function saveLocalAIPort() {
  let payload;
  try { payload = localAIPortPayload(); }
  catch (error) {
    setMessage('ai-message', error.message, 'error');
    PopupManager.showToast(error.message, 'error');
    setLocalAIPortState('Invalid port', 'error');
    return null;
  }
  setLocalAIPortState(`Saving ${payload.port}`, 'unknown');
  const data = await postAI('/api/ai/settings', { lanPort: payload.port, lanCidr: payload.lanCidr }, 'Local AI port saved');
  updateLocalAIPortReadback(data);
  return data;
}

async function applyLocalAIPort(enableLan) {
  let payload;
  try { payload = localAIPortPayload(); }
  catch (error) {
    setMessage('ai-message', error.message, 'error');
    PopupManager.showToast(error.message, 'error');
    setLocalAIPortState('Invalid port', 'error');
    return null;
  }
  const data = await postAI('/api/ai/inference/set-lan-access', { enabled: Boolean(enableLan), port: payload.port, lanCidr: payload.lanCidr }, 'LAN access applied');
  updateLocalAIPortReadback(data);
  return data;
}

function hfForm() { return document.querySelector('[data-hf-installer]'); }
function hfRequest() {
  const form = hfForm();
  return {
    repoId: form?.querySelector('input[name="repoId"]')?.value.trim() || '',
    filename: form?.querySelector('input[name="filename"]')?.value.trim() || '',
    revision: form?.querySelector('input[name="revision"]')?.value.trim() || 'main',
  };
}
async function fetchHFFiles() {
  const req = hfRequest();
  const data = await postJson('/api/ai/models/huggingface/list-files', { repoId: req.repoId, revision: req.revision });
  const root = document.getElementById('hf-file-results');
  if (root) {
    root.textContent = '';
    (data.files || []).forEach((file) => {
      const row = document.createElement('div');
      row.className = 'network-row';
      row.innerHTML = `<span><strong>${escapeHtml(file.filename)}</strong><em>${file.sizeBytes ? formatBytes(file.sizeBytes) : 'Size unknown'}</em></span>`;
      const select = document.createElement('button');
      select.className = 'btn btn--secondary';
      select.type = 'button';
      select.textContent = 'Select';
      select.addEventListener('click', () => { const f = hfForm()?.querySelector('input[name="filename"]'); if (f) f.value = file.filename; });
      row.appendChild(select);
      root.appendChild(row);
    });
    if (!data.files?.length) root.textContent = data.message || 'No compatible .gguf files found.';
  }
  PopupManager.showToast(data.message || 'Hugging Face files fetched', data.ok ? 'success' : 'error');
}
async function downloadHFModel() {
  const req = hfRequest();
  if (!req.filename.endsWith('.gguf')) return PopupManager.showToast('Only .gguf files are supported.', 'error');
  await postAI('/api/ai/models/huggingface/download', req, 'Model download finished');
}
function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  const gb = bytes / 1024 / 1024 / 1024;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 / 1024)} MB`;
}

async function postJson(url, body) {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json' },
    body: JSON.stringify(body),
  });
  let data = {};
  try { data = await res.json(); } catch (_) { data = {}; }
  if (!res.ok) data.ok = false;
  return data;
}

function bindGuiPinChange() {
  const form = document.getElementById('gui-pin-change-form');
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    clearMessage('gui-pin-change-message');
    const current = form.querySelector('input[name="current_pin"]')?.value || '';
    const next = form.querySelector('input[name="new_pin"]')?.value || '';
    const confirm = form.querySelector('input[name="confirm_pin"]')?.value || '';
    const button = form.querySelector('button[type="submit"]');

    if (!current || !next) return setMessage('gui-pin-change-message', 'Current PIN and new PIN are required.', 'error');
    if (next.length < 4) return setMessage('gui-pin-change-message', 'New PIN must be at least 4 characters.', 'error');
    if (next !== confirm) return setMessage('gui-pin-change-message', 'New PIN confirmation does not match.', 'error');

    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/gui-pin/change', { current_pin: current, new_pin: next });
      form.reset();
      if (data.ok) clearMessage('gui-pin-change-message');
      else setMessage('gui-pin-change-message', data.message || 'GUI PIN change failed.', 'error');
      PopupManager.showToast(data.ok ? 'Access PIN changed' : 'Access PIN change failed', data.ok ? 'success' : 'error');
    } catch (_) {
      form.reset();
      setMessage('gui-pin-change-message', 'GUI PIN change request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Change access PIN';
    }
  });
}

function bindGuiPinResetDefault() {
  const button = document.querySelector('[data-gui-pin-reset-default]');
  if (!button) return;
  button.addEventListener('click', async () => {
    clearMessage('gui-pin-reset-message');
    if (!window.confirm('Reset the access PIN to the console factory/default value? Games, settings, storage, and the operating system are not reset.')) return;
    const original = button.textContent;
    button.disabled = true;
    button.textContent = 'Resetting...';
    try {
      const data = await postJson('/api/gui-pin/reset-default', { confirm: 'RESET' });
      if (data.ok) clearMessage('gui-pin-reset-message');
      else setMessage('gui-pin-reset-message', data.message || 'PIN reset failed.', 'error');
      PopupManager.showToast(data.message || (data.ok ? 'PIN reset to default' : 'PIN reset failed'), data.ok ? 'success' : 'error');
      if (data.ok) renderGuiPinMode(Boolean(data.pin_required));
    } catch (_) {
      setMessage('gui-pin-reset-message', 'PIN reset request failed.', 'error');
      PopupManager.showToast('PIN reset request failed', 'error');
    } finally {
      button.disabled = false;
      button.textContent = original;
    }
  });
}

function providerLabel(id) {
  return { steamgriddb: 'SteamGridDB', thegamesdb: 'TheGamesDB', screenscraper: 'ScreenScraper' }[id] || id;
}

function setProviderStatusBadges(providers = []) {
  const map = new Map(providers.map((p) => [p.id, p]));
  document.querySelectorAll('[data-provider-status]').forEach((node) => {
    const item = map.get(node.dataset.providerStatus);
    node.textContent = item?.configured ? 'Configured' : 'Missing';
    node.dataset.configured = item?.configured ? 'true' : 'false';
  });
}

async function loadProviderKeyStatus() {
  try {
    const data = await getJson('/api/provider-keys/status');
    setProviderStatusBadges(data.providers || []);
    return data;
  } catch (_) {
    return { ok: false, providers: [], message: 'Provider key status unavailable.' };
  }
}

async function openProviderKeysModal() {
  const body = document.createElement('form');
  body.className = 'settings-form provider-key-modal';
  body.autocomplete = 'off';
  body.innerHTML = `
    <p class="modal-note">Scraper API keys are optional. They improve title and artwork lookups during Sync. Existing values are never shown here.</p>
    <div class="provider-status-grid provider-status-grid--modal" aria-label="Configured scraper keys">
      <div class="provider-status"><span>SteamGridDB</span><strong data-provider-status="steamgriddb">Checking</strong></div>
      <div class="provider-status"><span>TheGamesDB</span><strong data-provider-status="thegamesdb">Checking</strong></div>
      <div class="provider-status"><span>ScreenScraper</span><strong data-provider-status="screenscraper">Checking</strong></div>
    </div>
    <label><span>SteamGridDB API key</span><input class="field" type="password" name="steamgriddb_api_key" autocomplete="off" placeholder="Leave blank to keep unset"></label>
    <label><span>TheGamesDB API key</span><input class="field" type="password" name="thegamesdb_api_key" autocomplete="off" placeholder="Leave blank to keep unset"></label>
    <label><span>ScreenScraper API key</span><input class="field" type="password" name="screenscraper_api_key" autocomplete="off" placeholder="Leave blank to keep unset"></label>
    <label class="wifi-show-password"><input type="checkbox" data-toggle-provider-secrets>Show keys while editing</label>
    <div id="provider-keys-message" class="message" hidden></div>
    <div class="inline-actions"><button class="btn btn--primary" type="submit">Save API Keys</button><button class="btn btn--secondary" type="button" data-modal-cancel>Cancel</button></div>`;
  body.querySelector('[data-toggle-provider-secrets]')?.addEventListener('change', (event) => {
    body.querySelectorAll('input[type="password"], input[data-provider-secret-visible]').forEach((input) => {
      input.type = event.target.checked ? 'text' : 'password';
      input.toggleAttribute('data-provider-secret-visible', event.target.checked);
    });
  });
  body.querySelector('[data-modal-cancel]')?.addEventListener('click', () => PopupManager.closeModal());
  body.addEventListener('submit', async (event) => {
    event.preventDefault();
    clearMessage('provider-keys-message');
    const button = body.querySelector('button[type="submit"]');
    const payload = {
      steamgriddb_api_key: body.querySelector('input[name="steamgriddb_api_key"]')?.value || '',
      thegamesdb_api_key: body.querySelector('input[name="thegamesdb_api_key"]')?.value || '',
      screenscraper_api_key: body.querySelector('input[name="screenscraper_api_key"]')?.value || '',
    };
    if (!Object.values(payload).some((value) => value.trim())) return setMessage('provider-keys-message', 'Enter at least one API key to save.', 'error');
    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/provider-keys/save', payload);
      body.querySelectorAll('input[name$="api_key"]').forEach((input) => { input.value = ''; });
      if (data.ok) {
        clearMessage('provider-keys-message');
        PopupManager.showToast('Scraper API keys saved', 'success');
        await loadProviderKeyStatus();
        PopupManager.closeModal();
      } else {
        setMessage('provider-keys-message', data.message || 'API keys not saved.', 'error');
        PopupManager.showToast('API keys not saved', 'error');
      }
    } catch (_) {
      setMessage('provider-keys-message', 'API key save request failed.', 'error');
      PopupManager.showToast('API key save request failed', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Save API Keys';
    }
  });
  PopupManager.showModal({ title: 'Scraper API Keys', body, hideDefaultAction: true });
  await loadProviderKeyStatus();
  body.querySelector('input[name="steamgriddb_api_key"]')?.focus();
}

function bindProviderKeys() {
  document.querySelectorAll('[data-provider-keys-open]').forEach((button) => button.addEventListener('click', openProviderKeysModal));
  loadProviderKeyStatus();
}



function moduleLabelFromId(id) {
  return String(id || '').split('-').filter(Boolean).map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
}

async function toggleHarmoniaModule(moduleId, enabled, sourceButton = null) {
  const label = moduleLabelFromId(moduleId);
  const old = sourceButton?.textContent;
  if (sourceButton) { sourceButton.disabled = true; sourceButton.textContent = enabled ? 'Turning on…' : 'Turning off…'; }
  try {
    const data = await postJson('/api/harmonia/module', { module_id: moduleId, enabled });
    PopupManager.showToast(data.message || (data.ok ? `${label} updated` : `${label} not updated`), data.ok ? 'success' : 'error');
    if (data.ok) window.location.reload();
  } catch (_) {
    PopupManager.showToast(`${label} request failed`, 'error');
  } finally {
    if (sourceButton) { sourceButton.disabled = false; sourceButton.textContent = old; }
  }
}

function openHarmoniaModuleMenu() {
  const body = document.createElement('div');
  body.className = 'harmonia-module-menu';
  const modules = Array.from(document.querySelectorAll('[data-harmonia-module]'));
  if (!modules.length) {
    body.innerHTML = '<div class="empty-state"><strong>No Harmonia modules reported.</strong></div>';
    return PopupManager.showModal({ title: 'Harmonia Modules', body, hideDefaultAction: true });
  }
  modules.forEach((module) => {
    const moduleId = module.dataset.harmoniaModule;
    const enabled = module.dataset.moduleEnabled === 'true';
    const row = document.createElement('label');
    row.className = 'harmonia-module-choice';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = enabled;
    const track = document.createElement('span');
    track.className = 'pin-toggle-track';
    const thumb = document.createElement('span');
    thumb.className = 'pin-toggle-thumb';
    track.appendChild(thumb);
    const copy = document.createElement('span');
    copy.className = 'harmonia-module-choice-copy';
    copy.innerHTML = `<strong>${escapeHtml(moduleLabelFromId(moduleId))}</strong><em>${escapeHtml(moduleId)}</em>`;
    input.addEventListener('change', () => toggleHarmoniaModule(moduleId, input.checked, null));
    row.append(input, track, copy);
    body.appendChild(row);
  });
  PopupManager.showModal({ title: 'Harmonia Modules', body, hideDefaultAction: true });
}


function ledgerValue(value) {
  if (value === undefined || value === null || value === '') return '—';
  if (typeof value === 'boolean') return value ? 'yes' : 'no';
  return String(value);
}

function renderHarmoniaLedgerPage(content, data) {
  content.textContent = '';
  const page = data.page || 1;
  const totalPages = data.totalPages || 1;
  const entries = data.entries || [];
  const meta = document.createElement('div');
  meta.className = 'harmonia-ledger-meta';
  meta.innerHTML = `<strong>${escapeHtml(data.message || 'Harmonia ledger')}</strong><span>${escapeHtml(data.ledgerPath || '')}</span>`;
  content.appendChild(meta);
  const list = document.createElement('div');
  list.className = 'harmonia-ledger-list';
  if (!entries.length) {
    list.innerHTML = '<div class="empty-state"><strong>No ledger entries found.</strong></div>';
  }
  entries.forEach((entry) => {
    const row = document.createElement('article');
    row.className = 'harmonia-ledger-row';
    row.innerHTML = `
      <div><strong>${escapeHtml(entry.stamp || `entry-${entry.ordinal}`)}</strong><span>${escapeHtml(entry.schema || 'ledger')}</span></div>
      <b class="system-status system-status--${entry.ok === false ? 'error' : 'available'}">${entry.ok === false ? 'Failed' : 'OK'}</b>
      <div class="harmonia-ledger-fields">
        <span><em>Profile</em><strong>${escapeHtml(ledgerValue(entry.profileId))}</strong></span>
        <span><em>Module</em><strong>${escapeHtml(ledgerValue(entry.moduleId))}</strong></span>
        <span><em>Changed</em><strong>${escapeHtml(ledgerValue(entry.changed))}</strong></span>
        <span><em>Signal</em><strong>${escapeHtml(ledgerValue(entry.firstMissingSignal))}</strong></span>
      </div>`;
    const details = document.createElement('details');
    details.className = 'harmonia-ledger-json';
    const summary = document.createElement('summary');
    summary.textContent = 'Entry JSON';
    const pre = document.createElement('pre');
    pre.textContent = JSON.stringify(entry.entry || {}, null, 2);
    details.append(summary, pre);
    row.appendChild(details);
    list.appendChild(row);
  });
  content.appendChild(list);
  const pager = document.createElement('div');
  pager.className = 'harmonia-ledger-pager';
  const prev = buttonNode('Previous');
  const count = document.createElement('span');
  count.textContent = `${page} / ${totalPages}`;
  const next = buttonNode('Next');
  prev.disabled = page <= 1;
  next.disabled = page >= totalPages;
  prev.addEventListener('click', () => loadHarmoniaLedgerPage(page - 1, content));
  next.addEventListener('click', () => loadHarmoniaLedgerPage(page + 1, content));
  pager.append(prev, count, next);
  content.appendChild(pager);
}

async function loadHarmoniaLedgerPage(page, content) {
  storageLoading(content, 'Loading Harmonia ledger…');
  try {
    const data = await getJson(`/api/harmonia/ledger?page=${encodeURIComponent(page)}&per_page=8`);
    renderHarmoniaLedgerPage(content, data);
  } catch (_) {
    content.innerHTML = '<div class="empty-state"><strong>Could not load Harmonia ledger.</strong></div>';
  }
}

function openHarmoniaLedger() {
  const content = storageModalShell('Harmonia Ledger', ['Updates', 'Ledger']);
  loadHarmoniaLedgerPage(1, content);
}

function bindHarmoniaModules() {
  document.querySelectorAll('[data-harmonia-module-menu]').forEach((button) => button.addEventListener('click', openHarmoniaModuleMenu));
  document.querySelectorAll('[data-harmonia-ledger-open]').forEach((button) => button.addEventListener('click', openHarmoniaLedger));
  document.querySelectorAll('[data-harmonia-module-toggle]').forEach((button) => button.addEventListener('click', () => {
    const enabled = button.dataset.enabled === 'true';
    toggleHarmoniaModule(button.dataset.harmoniaModuleToggle, !enabled, button);
  }));
}

function bindSystemTrustAndAccessForms() {
  const keyForm = document.getElementById('ssh-key-form');
  if (keyForm) {
    keyForm.addEventListener('submit', async (event) => {
      event.preventDefault();
      clearMessage('ssh-key-message');
      const key = keyForm.querySelector('[name="public_key"]')?.value || '';
      if (!/^\s*(ssh-ed25519|ssh-rsa|ecdsa-sha2-nistp(256|384|521))\s+[A-Za-z0-9+/=]+(\s+\S+)?\s*$/.test(key)) {
        return setMessage('ssh-key-message', 'Enter one valid SSH public key.', 'error');
      }
      if (!window.confirm('Install this public key for console SSH login?')) return;
      const button = keyForm.querySelector('button[type="submit"]');
      const old = button.textContent;
      button.disabled = true; button.textContent = 'Installing…';
      try {
        const data = await postJson('/api/system/ssh/authorized-key', { public_key: key });
        if (data.ok) { keyForm.reset(); clearMessage('ssh-key-message'); }
        else setMessage('ssh-key-message', data.message || 'Public key not installed.', 'error');
        PopupManager.showToast(data.message || (data.ok ? 'Public key installed' : 'Public key not installed'), data.ok ? 'success' : 'error');
      } catch (_) {
        setMessage('ssh-key-message', 'Public key request failed.', 'error');
        PopupManager.showToast('Public key request failed', 'error');
      } finally { button.disabled = false; button.textContent = old; }
    });
  }

  const caForm = document.getElementById('root-ca-form');
  if (caForm) {
    caForm.addEventListener('submit', async (event) => {
      event.preventDefault();
      clearMessage('root-ca-message');
      const ca = caForm.querySelector('[name="ca_bundle"]')?.value || '';
      if (!ca.includes('-----BEGIN CERTIFICATE-----') || !ca.includes('-----END CERTIFICATE-----')) {
        return setMessage('root-ca-message', 'Paste a PEM/CRT certificate bundle.', 'error');
      }
      if (!window.confirm('Install this Home Root CA into appliance trust?')) return;
      const button = caForm.querySelector('button[type="submit"]');
      const old = button.textContent;
      button.disabled = true; button.textContent = 'Installing…';
      try {
        const data = await postJson('/api/system/trust/root-ca', { ca_bundle: ca, confirm: 'INSTALL_ROOT_CA' });
        if (data.ok) { caForm.reset(); clearMessage('root-ca-message'); }
        else setMessage('root-ca-message', data.message || 'Root CA not installed.', 'error');
        PopupManager.showToast(data.message || (data.ok ? 'Root CA installed' : 'Root CA not installed'), data.ok ? 'success' : 'error');
      } catch (_) {
        setMessage('root-ca-message', 'Root CA request failed.', 'error');
        PopupManager.showToast('Root CA request failed', 'error');
      } finally { button.disabled = false; button.textContent = old; }
    });
  }
}

initializeArcadiaTheme();
bindNavigation();
bindHomeLoadSubscription();
bindConsoleActions();
bindControllerLiveInput();
bindStorageModals();
bindProviderKeys();
bindHarmoniaModules();
bindGuiPinUnlock();
bindGuiPinAccess();
bindGuiPinChange();
bindGuiPinResetDefault();
bindNetworkControls();
bindSystemTrustAndAccessForms();
bindLocalAIControls();
initializeOnboarding();
initializeGuiPinGate();

document.addEventListener('click', (event) => {
  const close = event.target.closest('#modal-close, [data-action="modal-ok"]');
  if (close) return PopupManager.closeModal();
  if (event.target.id === 'modal-overlay') return PopupManager.closeModal();

  const button = event.target.closest('.btn[data-modal-title]');
  if (!button) return;
  event.stopPropagation();
  PopupManager.showModal({ title: button.dataset.modalTitle, body: button.dataset.modalBody });
});

document.addEventListener('keydown', (event) => {
  if (PopupManager.trapFocus(event)) return;
  if (event.key === 'Escape') PopupManager.closeModal();
});
