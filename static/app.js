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

  return { showModal, closeModal, showToast };
})();

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

function bindGuiPinAccess() {
  document.querySelectorAll('[data-module="gui-pin-access"] .btn[data-pin-required]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      clearMessage('gui-pin-access-message');
      const pinRequired = button.dataset.pinRequired === 'true';
      const original = button.textContent;
      button.disabled = true;
      button.textContent = 'Saving...';
      try {
        const data = await postJson('/api/gui-pin/access', { pin_required: pinRequired });
        setMessage('gui-pin-access-message', data.message || 'GUI PIN access setting returned no message.', data.ok ? 'success' : 'error');
        PopupManager.showToast(data.ok ? 'GUI PIN setting saved' : 'GUI PIN setting not saved', data.ok ? 'success' : 'error');
        if (data.ok) {
          document.body.dataset.guiPinRequired = String(data.pin_required);
          setPinIndicator(Boolean(data.pin_required));
        }
      } catch (_) {
        setMessage('gui-pin-access-message', 'GUI PIN access request failed.', 'error');
      } finally {
        button.disabled = false;
        button.textContent = original;
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
        if (action === 'sync-games') finishSyncProgress(Boolean(data.ok), data);
      } catch (_) {
        setMessage('console-action-message', 'Action request failed.', 'error');
        PopupManager.showToast('Action request failed', 'error');
        if (action === 'sync-games') finishSyncProgress(false, { message: 'Sync could not complete. Fix the issue shown below and try again.' });
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
  document.querySelectorAll('.btn[data-folder-copy]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const choices = [
        ['Copy Windows path', button.dataset.windows],
        ['Copy Windows IP fallback', button.dataset.windowsIp],
        ['Copy Linux/macOS path', button.dataset.smb],
        ['Copy Linux/macOS IP fallback', button.dataset.smbIp],
      ].filter(([, value]) => validCopyValue(value));
      if (choices.length === 0) return PopupManager.showToast('Folder address unavailable', 'error');
      if (choices.length === 1) {
        const ok = await copyToClipboard(choices[0][1]);
        return PopupManager.showToast(ok ? `Copied ${choices[0][1]}` : `Copy unavailable: ${choices[0][1]}`, ok ? 'success' : 'error');
      }
      const body = document.createElement('div');
      body.className = 'copy-choice-list';
      choices.forEach(([label, value]) => {
        const choice = document.createElement('button');
        choice.type = 'button';
        choice.className = 'btn btn--secondary';
        choice.textContent = label;
        choice.addEventListener('click', async () => {
          const ok = await copyToClipboard(value);
          PopupManager.closeModal();
          PopupManager.showToast(ok ? `Copied ${value}` : `Copy unavailable: ${value}`, ok ? 'success' : 'error');
        });
        body.appendChild(choice);
      });
      PopupManager.showModal({ title: 'Copy Game Folders', body: '' });
      const contentNode = document.getElementById('modal-content');
      if (contentNode) { contentNode.textContent = ''; contentNode.appendChild(body); }
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


function bindStorageModals() {
  document.querySelectorAll('[data-storage-modal]').forEach((button) => {
    button.addEventListener('click', (event) => {
      event.preventDefault();
      const key = button.dataset.storageModal;
      const template = document.querySelector(`[data-storage-modal-template="${CSS.escape(key)}"]`);
      if (!template) return PopupManager.showToast('Storage details unavailable', 'error');
      const body = document.createElement('div');
      body.className = 'storage-modal-body';
      body.innerHTML = template.innerHTML;
      bindStorageModalContent(body);
      PopupManager.showModal({ title: button.dataset.storageModalTitle || 'Storage', body, hideDefaultAction: false });
    });
  });
}

function bindStorageModalContent(root) {
  root.querySelectorAll('.btn[data-copy-value]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const value = button.dataset.copyValue || '';
      if (!validCopyValue(value)) return PopupManager.showToast('Address unavailable', 'error');
      const ok = await copyToClipboard(value);
      PopupManager.showToast(ok ? `Copied ${value}` : `Copy unavailable: ${value}`, ok ? 'success' : 'error');
    });
  });
  root.querySelectorAll('.btn[data-folder-copy]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const choices = [
        ['Copy Windows path', button.dataset.windows],
        ['Copy Windows IP fallback', button.dataset.windowsIp],
        ['Copy Linux/macOS path', button.dataset.smb],
        ['Copy Linux/macOS IP fallback', button.dataset.smbIp],
      ].filter(([, value]) => validCopyValue(value));
      if (choices.length === 0) return PopupManager.showToast('Folder address unavailable', 'error');
      if (choices.length === 1) {
        const ok = await copyToClipboard(choices[0][1]);
        return PopupManager.showToast(ok ? `Copied ${choices[0][1]}` : `Copy unavailable: ${choices[0][1]}`, ok ? 'success' : 'error');
      }
      const choiceBody = document.createElement('div');
      choiceBody.className = 'copy-choice-list';
      choices.forEach(([label, value]) => {
        const choice = document.createElement('button');
        choice.type = 'button';
        choice.className = 'btn btn--secondary';
        choice.textContent = label;
        choice.addEventListener('click', async () => {
          const ok = await copyToClipboard(value);
          PopupManager.closeModal();
          PopupManager.showToast(ok ? `Copied ${value}` : `Copy unavailable: ${value}`, ok ? 'success' : 'error');
        });
        choiceBody.appendChild(choice);
      });
      PopupManager.showModal({ title: 'Copy path', body: choiceBody, hideDefaultAction: true });
    });
  });
  root.querySelectorAll('.btn[data-action][data-endpoint]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.action;
      const endpoint = button.dataset.endpoint;
      const body = confirmationFor(action);
      if (body === null) return;
      const original = button.textContent;
      button.disabled = true;
      button.textContent = 'Running...';
      try {
        const data = await postJson(endpoint, body);
        PopupManager.showToast(data.message || (data.ok ? 'Done' : 'Failed'), data.ok ? 'success' : 'error');
        if (data.ok && endpoint.startsWith('/api/storage/')) window.location.reload();
      } catch (_) {
        PopupManager.showToast('Action request failed', 'error');
      } finally {
        button.disabled = false;
        button.textContent = original;
      }
    });
  });
  root.querySelectorAll('[data-create-managed-folder]').forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const path = button.dataset.createManagedFolder || '';
      if (!path || !window.confirm(`Create managed folder?\n${path}`)) return;
      const data = await postJson('/api/storage/create-managed-folder', { path });
      PopupManager.showToast(data.message || (data.ok ? 'Managed folder created' : 'Folder not created'), data.ok ? 'success' : 'error');
      if (data.ok) window.location.reload();
    });
  });
  root.querySelectorAll('[data-nav-target]').forEach((button) => {
    button.addEventListener('click', () => {
      PopupManager.closeModal();
      window.activateArcadiaView?.(button.dataset.navTarget);
    });
  });
}

function validCopyValue(value) {
  return Boolean(value) && !/smb::|smb:\/|\\undefined|smb:\/\/undefined|undefined/i.test(value);
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
  if (root?.dataset.storageBlocked === 'true' || root?.dataset.storageHealth === 'Full') {
    const message = 'Storage is full. Free space before syncing games.';
    setMessage('console-action-message', message, 'error');
    PopupManager.showToast(message, 'error');
    document.querySelector('[data-nav-target="storage"]')?.focus();
    return false;
  }
  if (root?.dataset.storageLow === 'true') {
    PopupManager.showToast('Storage is low. Sync may fail if there is not enough space for artwork or library entries.', 'error');
  }
  return true;
}

function setSyncState(state) {
  const node = document.getElementById('sync-state');
  if (node) {
    node.textContent = state;
    node.dataset.syncState = state.toLowerCase().replace(/\s+/g, '-');
  }
  document.querySelectorAll('.status-badge').forEach((badge) => {
    if (badge.dataset.chipKind !== 'sync') return;
    badge.setAttribute('aria-label', state);
    badge.title = state;
  });
}

const syncSteps = [
  { n: '1', running: 'Checking copied game folders…', complete: 'Complete' },
  { n: '2', running: 'Scanning game folders…', complete: 'Complete' },
  { n: '3', running: 'Fetching artwork…', complete: 'Complete' },
  { n: '4', running: 'Creating GameScope entries…', complete: 'Complete' },
  { n: '5', running: 'Finishing GameScope library…', complete: 'Complete' },
];

function setWorkflowStep(activeIndex, failed = false) {
  document.querySelectorAll('.sync-step').forEach((step, index) => {
    let state = 'waiting';
    if (index < activeIndex) state = 'complete';
    if (index === activeIndex) state = failed ? 'error' : 'running';
    step.dataset.stepState = state;
    const badge = step.querySelector('.sync-step-badge');
    if (badge) badge.textContent = state === 'running' ? 'Running' : state === 'complete' ? 'Complete' : state === 'error' ? 'Error' : 'Waiting';
  });
}

function startSyncProgress() {
  setSyncState('Sync Running');
  const button = document.querySelector('[data-action="sync-games"]');
  if (button) button.textContent = 'Sync Running';
  const progress = document.getElementById('sync-progress-text');
  const log = document.getElementById('sync-output');
  let index = 0;
  const tick = () => {
    const step = syncSteps[Math.min(index, syncSteps.length - 1)];
    setWorkflowStep(Math.min(index, syncSteps.length - 1));
    if (progress) progress.textContent = step.running;
    if (log) log.textContent = step.running;
    index = Math.min(index + 1, syncSteps.length - 1);
  };
  tick();
  return setInterval(tick, 1100);
}

function finishSyncProgress(ok, data = {}) {
  const button = document.querySelector('[data-action="sync-games"]');
  const progress = document.getElementById('sync-progress-text');
  const log = document.getElementById('sync-output');
  const result = document.querySelector('[data-sync-result]');
  const resultCopy = document.getElementById('sync-result-copy');
  if (ok) {
    document.querySelectorAll('.sync-step').forEach((step) => {
      step.dataset.stepState = 'complete';
      const badge = step.querySelector('.sync-step-badge');
      if (badge) badge.textContent = 'Complete';
    });
    setSyncState('Sync Complete');
    if (button) button.textContent = 'Sync Complete';
    const message = data.message || 'Sync complete. Your games are ready in GameScope.';
    if (progress) progress.textContent = message;
    if (result) result.dataset.syncResult = 'success';
    if (resultCopy) resultCopy.textContent = 'Sync complete. Your games are ready in GameScope.';
    if (log) log.textContent = formatActionResult(data);
    markOnboardingFirstSyncComplete(data);
  } else {
    setWorkflowStep(Math.max(0, Array.from(document.querySelectorAll('.sync-step')).findIndex((step) => step.dataset.stepState === 'running')), true);
    setSyncState('Sync Failed');
    if (button) button.textContent = 'Sync Failed';
    const message = data.message || 'Sync could not complete. Fix the issue shown below and try again.';
    if (progress) progress.textContent = message;
    if (result) result.dataset.syncResult = 'error';
    if (resultCopy) resultCopy.textContent = 'Sync could not complete. Fix the issue shown below and try again.';
    if (log) log.textContent = formatActionResult(data);
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
  setMessage('wifi-message', 'Scanning…', 'info');
  try {
    const data = await postJson('/api/network/wifi/scan', {});
    openWifiNetworkPicker(data.state, data.message || 'Wi-Fi scan complete.');
    setMessage('wifi-message', data.message || 'Wi-Fi scan complete.', data.ok ? 'success' : 'error');
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
  setMessage('wifi-message', data.message || 'Network action complete.', data.ok ? 'success' : 'error');
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
  setMessage('ai-message', data.message || label, data.ok ? 'success' : 'error');
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
        else if (action === 'install-recommended') await postAI('/api/ai/models/install-recommended', { id: modelId || 'inharmonia' }, 'Recommended model install');
        else if (action === 'model-select') await postAI('/api/ai/model/select', { modelId }, 'Model selected');
        else if (action === 'model-load') await postAI('/api/ai/model/load', { modelId }, 'Model load requested');
        else if (action === 'model-unload') await postAI('/api/ai/model/unload', {}, 'Model unloaded');
        else if (action === 'model-remove') {
          if (!window.confirm('Remove this model from console storage?\nGames and artwork are not affected.')) return;
          await postAI('/api/ai/models/remove', { modelId, filename, confirm: 'REMOVE_MODEL' }, 'Model removed');
        }
        else if (action === 'inference-enable') await postAI('/api/ai/inference/set-enabled', { enabled: true }, 'Inference enabled');
        else if (action === 'inference-disable') await postAI('/api/ai/inference/set-enabled', { enabled: false }, 'Inference disabled');
        else if (action === 'inference-test') await postAI('/api/ai/inference/test', {}, 'Inference tested');
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
  document.querySelectorAll('[data-ai-logs]').forEach((button) => button.addEventListener('click', async () => {
    try {
      const state = await requestAIState();
      const a = state.activity || {};
      PopupManager.showModal({ title: 'Local AI Logs', body: [a.runtimeUpdateLog, a.modelDownloadLog, a.modelLoadLog, a.inferenceServerLog].filter(Boolean).join('\n\n') || 'No Local AI logs reported.' });
    } catch (_) { PopupManager.showToast('Local AI logs unavailable', 'error'); }
  }));
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
    if (next !== confirm) return setMessage('gui-pin-change-message', 'New PIN confirmation does not match.', 'error');

    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/gui-pin/change', { current_pin: current, new_pin: next });
      form.reset();
      setMessage('gui-pin-change-message', data.message || 'GUI PIN change returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'GUI PIN changed' : 'GUI PIN change failed', data.ok ? 'success' : 'error');
    } catch (_) {
      form.reset();
      setMessage('gui-pin-change-message', 'GUI PIN change request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Change PIN';
    }
  });
}

function bindProviderKeys() {
  const form = document.getElementById('provider-keys-form');
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    clearMessage('provider-keys-message');
    const button = form.querySelector('button[type="submit"]');
    const body = {
      steamgriddb_api_key: form.querySelector('input[name="steamgriddb_api_key"]')?.value || '',
      thegamesdb_api_key: form.querySelector('input[name="thegamesdb_api_key"]')?.value || '',
      screenscraper_api_key: form.querySelector('input[name="screenscraper_api_key"]')?.value || '',
    };
    if (!Object.values(body).some((value) => value.trim())) {
      return setMessage('provider-keys-message', 'Enter at least one optional API key, or leave this section closed.', 'error');
    }
    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/provider-keys/save', body);
      form.reset();
      setMessage('provider-keys-message', data.message || 'Provider key save returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'Optional keys saved' : 'Optional keys not saved', data.ok ? 'success' : 'error');
    } catch (_) {
      setMessage('provider-keys-message', 'API key save request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Save Optional Keys';
    }
  });
}

document.documentElement.dataset.theme = 'dark';
bindNavigation();
bindConsoleActions();
bindStorageModals();
bindProviderKeys();
bindGuiPinUnlock();
bindGuiPinAccess();
bindGuiPinChange();
bindNetworkControls();
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
  if (event.key === 'Escape') PopupManager.closeModal();
});
