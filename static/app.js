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
    setTimeout(() => node.remove(), 2600);
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

ThemeManager.apply(ThemeManager.preferredTheme());

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
