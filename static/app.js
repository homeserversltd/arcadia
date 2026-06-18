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
    setTimeout(() => node.remove(), 4200);
  }

  return { showModal, closeModal, showToast };
})();

async function checkVaultStatus() {
  const res = await fetch('/api/vault/status', { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error('Vault status request failed');
  return await res.json();
}

function openPortals() {
  document.body.classList.add('vault-open');
  document.body.dataset.vaultMounted = 'true';
  document.getElementById('app')?.removeAttribute('aria-hidden');
}
function keepVaultGate() {
  document.body.classList.remove('vault-open');
  document.body.dataset.vaultMounted = 'false';
  document.querySelector('.field--password')?.focus();
}
async function initializeArcadia() {
  try {
    const vault = await checkVaultStatus();
    if (vault.mounted) openPortals(); else keepVaultGate();
  } catch (err) {
    keepVaultGate();
    PopupManager.showToast('Vault status check failed', 'error');
  }
}

document.addEventListener('click', (event) => {
  const close = event.target.closest('#modal-close, [data-action="modal-ok"]');
  if (close) return PopupManager.closeModal();
  if (event.target.id === 'modal-overlay') return PopupManager.closeModal();

  const button = event.target.closest('.btn[data-modal-title]');
  if (button) {
    event.stopPropagation();
    PopupManager.showModal({ title: button.dataset.modalTitle, body: button.dataset.modalBody });
    return;
  }

  const tile = event.target.closest('.tile--portal');
  if (!tile) return;
  if (tile.dataset.action === 'network') {
    window.location.href = tile.dataset.url || '/api/status';
    return;
  }
  PopupManager.showModal({ title: tile.dataset.modalTitle || 'Arcadia portal', body: tile.dataset.modalBody || '' });
});

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') PopupManager.closeModal();
  if ((event.key === 'Enter' || event.key === ' ') && event.target.matches('.tile--portal')) {
    event.preventDefault();
    event.target.click();
  }
});

const unlockForm = document.getElementById('vault-unlock-form');
if (unlockForm) {
  unlockForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = unlockForm.querySelector('input[name="password"]');
    const error = document.getElementById('vault-auth-error');
    const button = unlockForm.querySelector('button[type="submit"]');
    if (!input?.value) { error.textContent = 'Vault password is required'; error.hidden = false; return; }
    error.hidden = true; button.disabled = true; button.textContent = 'Unlocking...';
    try {
      const res = await fetch('/pre-unlock', { method: 'POST', headers: { 'content-type': 'application/json', accept: 'application/json' }, body: JSON.stringify({ password: input.value }) });
      input.value = '';
      const data = await res.json();
      if (data.ok || data.mounted) { openPortals(); PopupManager.showToast('Vault unlocked', 'success'); }
      else { error.textContent = data.message || 'Failed to unlock vault.'; error.hidden = false; }
    } catch (err) {
      input.value = ''; error.textContent = 'Vault unlock request failed.'; error.hidden = false;
    } finally {
      button.disabled = false; button.textContent = 'Unlock Vault';
    }
  });
}

initializeArcadia();
