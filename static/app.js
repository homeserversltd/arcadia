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
    document.body.classList.add('transmuting');
    setTimeout(() => document.body.classList.remove('transmuting'), 760);
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

const ThemeManager = (() => {
  const storageKey = 'arcadia-theme';
  const button = () => document.getElementById('theme-toggle');

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
    try { localStorage.setItem(storageKey, next); } catch (_) {}
    apply(next);
    ArcadiaAether.pulseCenter(button());
    PopupManager.showToast(`${next[0].toUpperCase()}${next.slice(1)} transmutation`, 'info');
  }

  return { apply, preferredTheme, toggle };
})();

const ArcadiaAether = (() => {
  const commands = [
    'distill --vault /vault --sigil green',
    'bind --surface console.home.arpa --holo true',
    'transmute --games visible --receipts alive',
    'listen --smb HOMECONSOLE --runes molten',
    'seal --operator local --noise zero'
  ];
  const lore = [
    'Amber retorts breathe in the lower deck. Each intent brick is a sealed vessel: heat, pressure, proof.',
    'A digital golem watches the uplink and opens nothing until the local membrane sings back.',
    'The arcade stack is a neon athanor: local metal, local secrets, local play, one pane of command.',
    'Data-sparks leak from the command rail whenever intent touches the glass. Nothing here is passive.',
    'The console remembers the old city as rain, phosphorus, copper, and the soft click of a mounted vault.'
  ];
  let commandIndex = 0;
  let loreIndex = 0;

  function setPointerGlow(event) {
    document.body.style.setProperty('--spark-x', `${event.clientX}px`);
    document.body.style.setProperty('--spark-y', `${event.clientY}px`);
  }

  function rotateCommand() {
    const target = document.getElementById('command-feed');
    if (!target) return;
    target.textContent = commands[commandIndex % commands.length];
    commandIndex += 1;
  }

  function rotateLore() {
    const target = document.getElementById('lore-feed');
    if (!target) return;
    target.textContent = lore[loreIndex % lore.length];
    loreIndex += 1;
  }

  function spark(x, y, count = 5) {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    for (let i = 0; i < count; i += 1) {
      const node = document.createElement('span');
      node.className = 'essence-spark';
      node.style.left = `${x + (Math.random() * 18 - 9)}px`;
      node.style.top = `${y + (Math.random() * 18 - 9)}px`;
      node.style.setProperty('--spark-dx', `${Math.random() * 80 - 40}px`);
      document.body.appendChild(node);
      setTimeout(() => node.remove(), 1200);
    }
  }

  function pulseCenter(element) {
    if (!element) return;
    const rect = element.getBoundingClientRect();
    spark(rect.left + rect.width / 2, rect.top + rect.height / 2, 8);
    document.body.classList.add('transmuting');
    setTimeout(() => document.body.classList.remove('transmuting'), 760);
  }

  function init() {
    rotateCommand();
    rotateLore();
    setInterval(rotateCommand, 5200);
    setInterval(rotateLore, 7600);
    window.addEventListener('pointermove', setPointerGlow, { passive: true });
    document.addEventListener('click', (event) => {
      const active = event.target.closest('.btn, .intent-module, .glyph');
      if (active) pulseCenter(active);
    });
  }

  return { init, spark, pulseCenter };
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
    button.textContent = 'Distilling...';
    ArcadiaAether.pulseCenter(button);
    try {
      const data = await postJson('/api/vault/password/change', { current_password: current, new_password: next });
      form.reset();
      setMessage('vault-password-change-message', data.message || 'Vault password change returned no message.', data.ok ? 'success' : 'error');
      PopupManager.showToast(data.ok ? 'Vault password transmuted' : 'Vault password change failed', data.ok ? 'success' : 'error');
    } catch (_) {
      form.reset();
      setMessage('vault-password-change-message', 'Vault password change request failed.', 'error');
    } finally {
      button.disabled = false;
      button.textContent = 'Change Vault password';
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
    button.textContent = 'Calcining...';
    ArcadiaAether.pulseCenter(button);
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
      button.textContent = 'Reset Vault password';
    }
  });
}

ThemeManager.apply(ThemeManager.preferredTheme());
ArcadiaAether.init();
bindVaultPasswordChange();
bindVaultPasswordReset();

document.addEventListener('click', (event) => {
  const themeToggle = event.target.closest('[data-theme-toggle]');
  if (themeToggle) {
    event.stopPropagation();
    ThemeManager.toggle();
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
