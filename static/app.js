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
    title().textContent = modalTitle || 'Arcadia';
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
    setTimeout(() => node.remove(), 3200);
  }

  return { showModal, closeModal, showToast };
})();

const ThemeManager = (() => {
  const storageKey = 'arcadia-theme';
  function preferredTheme() {
    let stored = null;
    try { stored = localStorage.getItem(storageKey); } catch (_) { stored = null; }
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function apply(theme) {
    const next = theme === 'light' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    document.body.dataset.theme = next;
  }

  return { apply, preferredTheme };
})();

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

function bindVaultPasswordChange() {
  const form = document.getElementById('vault-password-change-form');
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    clearMessage('vault-password-change-message');
    const current = form.querySelector('input[name="current_password"]')?.value || '';
    const next = form.querySelector('input[name="new_password"]')?.value || '';
    const confirm = form.querySelector('input[name="confirm_password"]')?.value || '';
    const button = form.querySelector('button[type="submit"]');

    if (!current || !next) return setMessage('vault-password-change-message', 'Current password and new password are required.', 'error');
    if (next !== confirm) return setMessage('vault-password-change-message', 'New password confirmation does not match.', 'error');

    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/vault/password/change', { current_password: current, new_password: next });
      form.reset();
      setMessage('vault-password-change-message', data.message || 'Vault password change returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'Vault password changed' : 'Vault password change failed', data.ok ? 'success' : 'error');
    } catch (_) {
      form.reset();
      setMessage('vault-password-change-message', 'Vault password change request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Change Vault password';
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
      screenscraper_user: form.querySelector('input[name="screenscraper_user"]')?.value || '',
      screenscraper_password: form.querySelector('input[name="screenscraper_password"]')?.value || '',
    };
    if (!Object.values(body).some((value) => value.trim())) {
      return setMessage('provider-keys-message', 'Enter at least one API key.', 'error');
    }
    button.disabled = true;
    button.textContent = 'Saving...';
    try {
      const data = await postJson('/api/provider-keys/save', body);
      form.reset();
      setMessage('provider-keys-message', data.message || 'Provider key save returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'API keys saved' : 'API keys not saved', data.ok ? 'success' : 'error');
    } catch (_) {
      setMessage('provider-keys-message', 'API key save request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Save API keys';
    }
  });
}

ThemeManager.apply(ThemeManager.preferredTheme());
bindProviderKeys();
bindVaultPasswordChange();

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
