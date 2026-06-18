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
  const button = () => document.getElementById('theme-toggle');

  function preferredTheme() {
    const stored = localStorage.getItem(storageKey);
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function apply(theme) {
    const next = theme === 'dark' ? 'dark' : 'light';
    document.body.dataset.theme = next;
    const toggle = button();
    if (toggle) {
      toggle.textContent = next === 'dark' ? 'Light' : 'Dark';
      toggle.setAttribute('aria-pressed', String(next === 'dark'));
      toggle.setAttribute('aria-label', `Switch to ${next === 'dark' ? 'light' : 'dark'} theme`);
    }
  }

  function toggle() {
    const current = document.body.dataset.theme === 'dark' ? 'dark' : 'light';
    const next = current === 'dark' ? 'light' : 'dark';
    localStorage.setItem(storageKey, next);
    apply(next);
    PopupManager.showToast(`${next[0].toUpperCase()}${next.slice(1)} theme`, 'info');
  }

  return { apply, preferredTheme, toggle };
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
    button.textContent = 'Changing...';
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
      button.textContent = 'Change Vault Password';
    }
  });
}

function bindVaultPasswordReset() {
  const form = document.getElementById('vault-password-reset-form');
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    clearMessage('vault-password-reset-message');
    const confirm = form.querySelector('input[name="confirm"]')?.value || '';
    const button = form.querySelector('button[type="submit"]');
    if (confirm !== 'RESET') return setMessage('vault-password-reset-message', 'Type RESET to restore the default vault password.', 'error');

    button.disabled = true;
    button.textContent = 'Resetting...';
    try {
      const data = await postJson('/api/vault/password/reset-default', { confirm });
      form.reset();
      setMessage('vault-password-reset-message', data.message || 'Vault password reset returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'Vault password reset to default' : 'Vault password reset failed', data.ok ? 'success' : 'error');
    } catch (_) {
      form.reset();
      setMessage('vault-password-reset-message', 'Vault password reset request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Reset to Default';
    }
  });
}

ThemeManager.apply(ThemeManager.preferredTheme());
bindVaultPasswordChange();
bindVaultPasswordReset();

document.addEventListener('click', (event) => {
  const themeToggle = event.target.closest('[data-theme-toggle]');
  if (themeToggle) {
    event.stopPropagation();
    ThemeManager.toggle();
    return;
  }

  const navTile = event.target.closest('[data-nav-action]');
  if (navTile) {
    document.querySelectorAll('.nav-tile').forEach((tile) => tile.classList.remove('nav-tile--active'));
    navTile.classList.add('nav-tile--active');
    if (navTile.dataset.navAction !== 'vault-password') {
      PopupManager.showToast(`${navTile.innerText.split('\n')[0]} panel is staged`, 'info');
    }
    return;
  }

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
