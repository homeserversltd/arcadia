const PopupManager = (() => {
  const overlay = () => document.getElementById('modal-overlay');
  const title = () => document.getElementById('modal-title');
  const content = () => document.getElementById('modal-content');
  const toasts = () => document.getElementById('toast-container');
  let previousFocus = null;

  function showModal({ title: modalTitle, body }) {
    const el = overlay();
    if (!el) return;
    previousFocus = document.activeElement;
    title().textContent = modalTitle || 'Arcadia Console';
    content().textContent = body || '';
    el.hidden = false;
    document.querySelector('#modal-overlay [data-action="modal-ok"]')?.focus();
  }

  function closeModal() {
    const el = overlay();
    if (!el) return;
    el.hidden = true;
    if (previousFocus && typeof previousFocus.focus === 'function') previousFocus.focus();
  }

  function showToast(message, variant = 'info') {
    const root = toasts();
    if (!root) return;
    const node = document.createElement('div');
    node.className = `toast ${variant}`;
    node.textContent = message;
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
    const label = badge.querySelector('span')?.textContent?.trim().toLowerCase();
    if (label !== 'pin') return;
    badge.querySelector('strong').textContent = required ? 'Required' : 'Open';
    badge.classList.toggle('status-badge--warn', required);
    badge.classList.toggle('status-badge--idle', !required);
  });
}

function bindNavigation() {
  const buttons = Array.from(document.querySelectorAll('.launcher-button[data-view]'));
  const panels = Array.from(document.querySelectorAll('[data-view-panel]'));
  const activate = (view) => {
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
  buttons.forEach((button) => button.addEventListener('click', () => activate(button.dataset.view)));
  document.querySelectorAll('[data-nav-target]').forEach((button) => button.addEventListener('click', () => activate(button.dataset.navTarget)));
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
  if (action === 'restart-gamescope') return window.confirm('Restart GameScope now? The TV game session will close and reopen.') ? { confirm: 'RESTART_GAMESCOPE' } : null;
  if (action === 'clear-artwork-cache') return window.confirm('Clear artwork cache? This does not delete games. Artwork can be downloaded again during Sync.') ? { confirm: 'CLEAR_ARTWORK' } : null;
  if (action === 'remove-ai-model') return window.confirm('Remove this local AI model file from console storage? This does not affect games.') ? { confirm: 'REMOVE_MODEL' } : null;
  if (action === 'clean-temporary-files') return window.confirm('Clean safe temporary files? This will not remove games, artwork intentionally kept, or installed AI models.') ? { confirm: 'CLEAN_TEMPORARY' } : null;
  return {};
}

function bindConsoleActions() {
  document.querySelectorAll('.btn[data-action][data-endpoint]').forEach((button) => {
    if (button.dataset.pinRequired !== undefined) return;
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.action;
      const endpoint = button.dataset.endpoint;
      const body = confirmationFor(action);
      if (body === null) return;
      clearMessage('console-action-message');
      const original = button.textContent;
      button.disabled = true;
      button.textContent = action === 'sync-games' ? 'Sync running...' : 'Running...';
      if (action === 'sync-games') setSyncState('Running');
      try {
        const data = await postJson(endpoint, body);
        const variant = data.ok ? 'success' : 'error';
        setMessage('console-action-message', formatActionResult(data), variant);
        PopupManager.showToast(data.message || (data.ok ? 'Done' : 'Failed'), variant);
        if (action === 'sync-games') setSyncState(data.ok ? 'Complete' : 'Error');
      } catch (_) {
        setMessage('console-action-message', 'Action request failed.', 'error');
        PopupManager.showToast('Action request failed', 'error');
        if (action === 'sync-games') setSyncState('Error');
      } finally {
        button.disabled = false;
        button.textContent = original;
      }
    });
  });
  document.querySelectorAll('.btn[data-url]').forEach((button) => {
    button.addEventListener('click', (event) => {
      event.preventDefault();
      window.location.href = button.dataset.url;
    });
  });
}

function setSyncState(state) {
  const node = document.getElementById('sync-state');
  if (node) node.textContent = state;
  document.querySelectorAll('.status-badge').forEach((badge) => {
    const label = badge.querySelector('span')?.textContent?.trim().toLowerCase();
    if (label !== 'sync') return;
    badge.querySelector('strong').textContent = state;
  });
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
bindProviderKeys();
bindGuiPinUnlock();
bindGuiPinAccess();
bindGuiPinChange();
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
