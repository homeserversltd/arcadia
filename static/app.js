const PopupManager = (() => {
  const overlay = () => document.getElementById('modal-overlay');
  const title = () => document.getElementById('modal-title');
  const content = () => document.getElementById('modal-content');
  const actions = () => document.querySelector('#modal-overlay .modal-actions');
  const toasts = () => document.getElementById('toast-container');
  let previousFocus = null;

  function clearOverlayVariants(el) {
    for (const cls of [...el.classList]) {
      if (cls.startsWith('modal-overlay--')) el.classList.remove(cls);
    }
  }

  function setOverlayVariant(el, variant) {
    clearOverlayVariants(el);
    if (variant) el.classList.add(`modal-overlay--${variant}`);
  }

  function setCardVariant(card, variant) {
    card.className = 'modal-card';
    if (variant) card.classList.add(`modal-card--${variant}`);
  }

  let pendingConfirmResolve = null;
  let openSurface = null;

  function showModal({ title: modalTitle, body, hideDefaultAction = false, variant = '', surfaceId, parentSurfaceId = '' }) {
    const el = overlay();
    if (!el) return;
    previousFocus = document.activeElement;
    title().textContent = modalTitle || 'Arcadia Console';
    setOverlayVariant(el, variant);
    const card = el.querySelector('.modal-card');
    if (card) setCardVariant(card, variant);
    document.body.classList.toggle('modal-fullscreen-open', variant === 'fullscreen');
    const target = content();
    target.textContent = '';
    if (body instanceof Node) target.appendChild(body);
    else target.textContent = body || '';
    actions()?.toggleAttribute('hidden', hideDefaultAction);
    el.hidden = false;
    openSurface = { id: surfaceId || 'modal:unclassified', class: variant || 'modal', parent: parentSurfaceId };
    window.ArcadiaObservationAdapters?.presenter(openSurface.id, openSurface.class, 'opened', openSurface.parent);
    document.dispatchEvent(new CustomEvent('arcadia:modal-open', { detail: { title: modalTitle || 'Arcadia Console', variant } }));
    const focusable = el.querySelector('button, [href], input, select, textarea, details, [tabindex]:not([tabindex="-1"])');
    focusable?.focus();
  }

  function closeModalSurface() {
    const el = overlay();
    if (!el) return;
    el.hidden = true;
    setOverlayVariant(el, '');
    const card = el.querySelector('.modal-card');
    if (card) setCardVariant(card, '');
    document.body.classList.remove('modal-fullscreen-open');
    content().textContent = '';
    actions()?.removeAttribute('hidden');
    if (openSurface) window.ArcadiaObservationAdapters?.presenter(openSurface.id, openSurface.class, 'closed', openSurface.parent);
    openSurface = null;
    document.dispatchEvent(new CustomEvent('arcadia:modal-close'));
    if (previousFocus && typeof previousFocus.focus === 'function') previousFocus.focus();
  }

  function finishConfirm(accepted) {
    const resolver = pendingConfirmResolve;
    pendingConfirmResolve = null;
    window.ArcadiaObservationAdapters?.presenter('modal:confirm', 'confirm', accepted ? 'confirmed' : 'refused');
    closeModalSurface();
    if (resolver) resolver(accepted);
  }

  function closeModal() {
    if (pendingConfirmResolve) {
      finishConfirm(false);
      return;
    }
    closeModalSurface();
  }

  function showConfirm({
    title = 'Are you sure?',
    message = '',
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    danger = false,
    variant = 'confirm',
  } = {}) {
    return new Promise((resolve) => {
      pendingConfirmResolve = resolve;
      const body = document.createElement('div');
      body.className = 'modal-confirm';
      const messageNode = document.createElement('p');
      messageNode.className = 'modal-confirm__message';
      messageNode.textContent = message;
      const confirmActions = document.createElement('footer');
      confirmActions.className = 'modal-confirm__actions';
      const cancel = document.createElement('button');
      cancel.type = 'button';
      cancel.className = 'btn btn--secondary';
      cancel.textContent = cancelLabel;
      cancel.addEventListener('click', () => finishConfirm(false));
      const confirm = document.createElement('button');
      confirm.type = 'button';
      confirm.className = danger ? 'btn btn--danger' : 'btn btn--primary';
      confirm.textContent = confirmLabel;
      confirm.addEventListener('click', () => finishConfirm(true));
      confirmActions.append(cancel, confirm);
      body.append(messageNode, confirmActions);
      showModal({ title, body, hideDefaultAction: true, variant, surfaceId: 'modal:confirm' });
      cancel.focus();
    });
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

  return { showModal, closeModal, showConfirm, showToast, trapFocus };
})();

// The single Indra owner is loaded before this adapter. All ordinary app requests cross its same-origin seam.
const ArcadiaDebug = window.IndraObservationReflection;
const ArcadiaObservation = window.ArcadiaObservationAdapters;
const fetch = ArcadiaObservation.request;
window.arcadiaDebug = ArcadiaDebug;

const ArcadiaLoading = (() => {
  let overlaySeq = 0;

  function spinner({ label = 'Loading', size = 'md' } = {}) {
    const node = document.createElement('div');
    node.className = `ux-arcadia-spinner ux-arcadia-spinner--${size}`;
    node.setAttribute('role', 'status');
    node.setAttribute('aria-live', 'polite');
    node.innerHTML = '<div class="ux-arcadia-spinner__visual"><span class="ux-arcadia-spinner__ring" aria-hidden="true"></span><span class="ux-arcadia-spinner__mark" aria-hidden="true">H</span></div><span class="ux-arcadia-spinner__label"></span>';
    node.querySelector('.ux-arcadia-spinner__label').textContent = label;
    return node;
  }

  function showIn(target, { label = 'Loading', size = 'lg' } = {}) {
    if (!target) return () => {};
    const previous = target.innerHTML;
    target.classList.add('ux-loading-host');
    target.replaceChildren(spinner({ label, size }));
    return () => {
      target.classList.remove('ux-loading-host');
      target.innerHTML = previous;
    };
  }

  function showOverlay({ label = 'Loading', size = 'lg' } = {}) {
    const id = ++overlaySeq;
    const host = document.createElement('div');
    host.className = 'ux-loading-overlay';
    host.dataset.loadingOverlay = String(id);
    host.appendChild(spinner({ label, size }));
    document.body.appendChild(host);
    return () => host.remove();
  }

  async function during(promise, options = {}) {
    const hide = options.target
      ? showIn(options.target, options)
      : options.overlay
        ? showOverlay(options)
        : () => {};
    try {
      return await promise;
    } finally {
      hide();
    }
  }

  return { spinner, showIn, showOverlay, during };
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

async function checkVaultStatus() {
  const res = await fetch('/api/vault/status', { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error('Vault status request failed');
  return await res.json();
}

function setVaultIndicator(mounted) {
  document.querySelectorAll('[data-chip-kind="vault"]').forEach((badge) => {
    badge.setAttribute('aria-label', mounted ? 'Vault: Unlocked' : 'Vault: Locked');
    badge.classList.toggle('status-badge--good', mounted);
    badge.classList.toggle('status-badge--warn', !mounted);
    const value = badge.querySelector('strong');
    if (value) value.textContent = mounted ? 'Unlocked' : 'Locked';
  });
}

function renderVaultMode({ mounted, auto_decrypt_enabled: autoDecrypt }) {
  document.body.dataset.vaultMounted = String(Boolean(mounted));
  document.body.classList.toggle('vault-open', Boolean(mounted));
  updateArcadiaShellVisibility();
  setVaultIndicator(Boolean(mounted));
  document.querySelectorAll('[data-vault-auto-decrypt-toggle]').forEach((toggle) => { toggle.checked = Boolean(autoDecrypt); });
  document.querySelectorAll('[data-vault-mode-label]').forEach((node) => { node.textContent = mounted ? 'Unlocked' : 'Locked'; });
  document.querySelectorAll('[data-vault-mode-copy]').forEach((node) => { node.textContent = autoDecrypt ? 'Automatically decrypts when the console starts.' : 'Unlock required after the console starts.'; });
}

async function initializeVaultGate() {
  try { renderVaultMode(await checkVaultStatus()); }
  catch (_) { renderVaultMode({ mounted: true, auto_decrypt_enabled: true }); PopupManager.showToast('Vault status is unavailable. HomeConsole remains open.', 'error'); }
}

function bindVaultUnlockForm(id, messageId) {
  const form = document.getElementById(id);
  if (!form) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = form.querySelector('input[name="password"]');
    const button = form.querySelector('button[type="submit"]');
    const message = document.getElementById(messageId);
    if (!input?.value) { if (message) { message.textContent = 'Vault password is required'; message.hidden = false; } return; }
    if (message) message.hidden = true;
    button.disabled = true; button.textContent = 'Unlocking...';
    try {
      const data = await postJson('/api/vault/unlock', { password: input.value });
      input.value = '';
      if (data.success) { renderVaultMode({ mounted: true, auto_decrypt_enabled: document.querySelector('[data-vault-auto-decrypt-toggle]')?.checked ?? true }); PopupManager.showToast(data.message || 'Vault unlocked', 'success'); }
      else if (message) { message.textContent = data.message || 'Vault could not be unlocked.'; message.hidden = false; }
    } catch (_) { input.value = ''; if (message) { message.textContent = 'Vault unlock request failed.'; message.hidden = false; } }
    finally { button.disabled = false; button.textContent = 'Unlock Vault'; }
  });
}

function bindVaultAccess() {
  document.querySelectorAll('[data-vault-auto-decrypt-toggle]').forEach((toggle) => {
    toggle.addEventListener('change', async () => {
      const enabled = Boolean(toggle.checked);
      toggle.disabled = true;
      try {
        const data = await postJson('/api/vault/auto-decrypt', { enabled });
        if (data.success) { renderVaultMode({ mounted: document.body.dataset.vaultMounted !== 'false', auto_decrypt_enabled: Boolean(data.auto_decrypt_enabled) }); PopupManager.showToast(data.message || 'Vault startup setting saved', 'success'); }
        else { toggle.checked = !enabled; PopupManager.showToast(data.message || 'Vault startup setting not saved', 'error'); }
      } catch (_) { toggle.checked = !enabled; PopupManager.showToast('Vault startup request failed.', 'error'); }
      finally { toggle.disabled = false; }
    });
  });
}

function randomUuid() {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

const caduceusDocument = `arcadia-${randomUuid()}`;
let caduceusAttendance = null;

function caduceusAttendanceHeaders() {
  const headers = { 'x-caduceus-document': caduceusDocument };
  if (caduceusAttendance) headers['x-caduceus-attendance'] = caduceusAttendance;
  return headers;
}

function clearCaduceusAttendance() {
  caduceusAttendance = null;
}

function invalidateCaduceusAttendance() {
  if (!caduceusAttendance) return;
  fetch('/api/v1/attendance/invalidate', {
    method: 'POST',
    headers: { accept: 'application/json', ...caduceusAttendanceHeaders() },
    keepalive: true,
  }).catch(() => {});
  clearCaduceusAttendance();
}

function updateArcadiaShellVisibility() {
  const pinRequired = document.body.dataset.guiPinRequired === 'true';
  const vaultMounted = document.body.dataset.vaultMounted !== 'false';
  document.getElementById('app')?.toggleAttribute('aria-hidden', pinRequired || !vaultMounted);
}

function openArcadia() {
  document.body.classList.add('pin-open');
  document.body.dataset.guiPinRequired = 'false';
  updateArcadiaShellVisibility();
}

function keepGuiPinGate() {
  document.body.classList.remove('pin-open');
  document.body.dataset.guiPinRequired = 'true';
  updateArcadiaShellVisibility();
  document.querySelector('.field--pin')?.focus();
}

async function initializeGuiPinGate() {
  try {
    const status = await checkGuiPinStatus();
    if (status.pin_required) keepGuiPinGate(); else openArcadia();
    setPinIndicator(Boolean(status.pin_required));
  } catch (_) {
    keepGuiPinGate();
    PopupManager.showToast('PIN attendance status unavailable. Arcadia remains locked.', 'error');
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
    let activePanel;
    panels.forEach((panel) => {
      const active = panel.dataset.viewPanel === next;
      panel.classList.toggle('is-active', active);
      panel.hidden = !active;
      if (active) activePanel = panel;
    });
    if (activePanel) requestAnimationFrame(() => activePanel.focus({ preventScroll: true }));
    try { localStorage.setItem('arcadia-active-view', next); } catch (_) {}
    ArcadiaObservation.navigation(next);
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


const ArcadiaProjector = (() => {
  const widgets = [];
  let lastDocument = null;

  function resolve(path, root) {
    if (!path) return root;
    return String(path).split('.').reduce((value, key) => {
      if (value == null || key === '') return undefined;
      return value[key];
    }, root);
  }

  function asText(value) {
    if (value == null) return '';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  }

  function asState(value) {
    return asText(value).trim().toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-|-$/g, '') || 'unknown';
  }

  function boundNodes(root, selector, includeGenerated) {
    return Array.from(root.querySelectorAll(selector)).filter((node) => includeGenerated || !node.closest('[data-projector-generated="true"]'));
  }

  function projectScalarBindings(root, state, includeGenerated = false) {
    boundNodes(root, '[data-bind]', includeGenerated).forEach((node) => {
      node.textContent = asText(resolve(node.dataset.bind, state));
    });
    boundNodes(root, '[data-bind-copy-value]', includeGenerated).forEach((node) => {
      const value = asText(resolve(node.dataset.bindCopyValue, state));
      node.dataset.copyValue = value;
      node.setAttribute('data-copy-value', value);
    });
    boundNodes(root, '[data-bind-enabled]', includeGenerated).forEach((node) => {
      const enabled = Boolean(resolve(node.dataset.bindEnabled, state));
      node.disabled = !enabled;
      node.setAttribute('aria-disabled', String(!enabled));
    });
    boundNodes(root, '[data-bind-checked]', includeGenerated).forEach((node) => {
      node.checked = Boolean(resolve(node.dataset.bindChecked, state));
    });
    boundNodes(root, '[data-bind-value]', includeGenerated).forEach((node) => {
      const value = asText(resolve(node.dataset.bindValue, state));
      node.value = value;
      node.dataset.harmoniaModuleSwitch = value;
      node.setAttribute('data-harmonia-module-switch', value);
      node.setAttribute('aria-label', `${value} module enabled`);
    });
    boundNodes(root, '[data-bind-attr-id]', includeGenerated).forEach((node) => {
      const value = asText(resolve(node.dataset.bindAttrId, state));
      if (value) {
        node.dataset.harmoniaModule = value;
        node.setAttribute('data-harmonia-module', value);
        node.querySelectorAll('[data-harmonia-module-switch-row=""]').forEach((row) => row.setAttribute('data-harmonia-module-switch-row', value));
      }
    });
    boundNodes(root, '[data-bind-class]', includeGenerated).forEach((node) => {
      node.setAttribute('data-state', asState(resolve(node.dataset.bindClass, state)));
    });
    boundNodes(root, '[data-bind-show]', includeGenerated).forEach((node) => {
      const visible = Boolean(resolve(node.dataset.bindShow, state));
      node.hidden = !visible;
      node.setAttribute('aria-hidden', String(!visible));
    });
    boundNodes(root, '[data-bind-style-var]', includeGenerated).forEach((node) => {
      String(node.dataset.bindStyleVar || '').split(',').forEach((binding) => {
        const [name, path] = binding.split(':').map((part) => part && part.trim());
        if (!name || !path || !name.startsWith('--')) return;
        node.style.setProperty(name, asText(resolve(path, state)));
      });
    });
  }

  function projectEachBindings(root, state, includeGenerated = false) {
    boundNodes(root, '[data-bind-each]', includeGenerated).forEach((host) => {
      const template = host.firstElementChild?.tagName === 'TEMPLATE' ? host.firstElementChild : null;
      if (!template) return;
      if (host.dataset.bindReplace === 'true') {
        Array.from(host.children).forEach((node) => { if (node !== template) node.remove(); });
      } else {
        host.querySelectorAll(':scope > [data-projector-generated="true"]').forEach((node) => node.remove());
      }
      const items = resolve(host.dataset.bindEach, state);
      if (!Array.isArray(items)) return;
      items.forEach((item) => {
        const fragment = template.content.cloneNode(true);
        Array.from(fragment.children).forEach((node) => { node.dataset.projectorGenerated = 'true'; });
        project(fragment, item, true);
        host.appendChild(fragment);
      });
    });
  }

  function widgetContext(selectorOrName) {
    const elements = /^[.#\[]/.test(selectorOrName) ? Array.from(document.querySelectorAll(selectorOrName)) : [];
    return { selectorOrName, elements };
  }

  function dispatchWidgets(state) {
    widgets.forEach(({ selectorOrName, fn }) => {
      try {
        fn(state, widgetContext(selectorOrName));
      } catch (error) {
        console.warn('ArcadiaProjector widget failed', selectorOrName, error);
      }
    });
  }

  function project(root, state, includeGenerated = false) {
    projectEachBindings(root, state, includeGenerated);
    projectScalarBindings(root, state, includeGenerated);
  }

  function apply(state) {
    lastDocument = state || {};
    if (window.arcadiaControllerTrainerStreamState) window.arcadiaControllerTrainerStreamState.lastLivingState = lastDocument;
    project(document, lastDocument);
    dispatchWidgets(lastDocument);
  }

  function registerWidget(selectorOrName, fn) {
    if (typeof selectorOrName !== 'string' || typeof fn !== 'function') return false;
    widgets.push({ selectorOrName, fn });
    if (lastDocument) fn(lastDocument, widgetContext(selectorOrName));
    return true;
  }

  // data-bind-class projects normalized values into data-state="<value>"; CSS may target that stable state attribute.
  // data-bind-style-var="--var:path" projects a document value into a CSS custom property without pane knowledge.
  // data-bind-copy-value and data-bind-enabled keep presenter-owned controls stable while values change.
  return { apply, registerWidget, resolve };
})();
window.ArcadiaProjector = ArcadiaProjector;


const ArcadiaControllerTrainerStream = (() => {
  const subscribers = new Map();
  const state = { source: null, events: 0, lease: null, lastInput: null, lastEvent: '', fallbackSnapshots: 0 };
  window.arcadiaControllerTrainerStreamState = state;

  function hasWatchers() { return subscribers.size > 0; }
  function close() {
    if (state.source) { state.source.close(); ArcadiaObservation.stream('controller-trainer-sse', 'closed'); }
    state.source = null;
  }
  async function fallbackSnapshot() {
    try {
      const serverData = await getJson('/api/controllers/input');
      state.fallbackSnapshots += 1;
      publish(serverData, 'snapshot');
    } catch (_) {}
  }
  function publish(serverData, eventName = 'input') {
    const merged = mergeControllerInput(serverData, readBrowserGamepadInput());
    state.lastInput = merged;
    state.lastEvent = eventName;
    state.events += 1;
    ArcadiaObservation.currentness('controller-trainer-sse', state.events === 1 ? 'current' : 'changed');
    subscribers.forEach((fn, key) => {
      try { fn(merged, { eventName, key, lease: state.lease }); }
      catch (error) { console.warn('controller trainer subscriber failed', key, error); }
    });
  }
  function connect() {
    if (!hasWatchers() || state.source || document.visibilityState !== 'visible') return;
    if (!window.EventSource) { fallbackSnapshot(); return; }
    const source = new EventSource('/api/controllers/trainer/events');
    ArcadiaObservation.stream('controller-trainer-sse', 'opened');
    state.source = source;
    source.addEventListener('lease', (event) => {
      try { state.lease = JSON.parse(event.data); } catch (_) {}
    });
    ['snapshot', 'input'].forEach((name) => {
      source.addEventListener(name, (event) => {
        try { publish(JSON.parse(event.data), name); } catch (_) {}
      });
    });
    source.addEventListener('heartbeat', (event) => { state.lastEvent = 'heartbeat'; try { state.lease = JSON.parse(event.data); } catch (_) {} });
    source.addEventListener('expired', () => { close(); if (hasWatchers()) setTimeout(connect, 300); });
    source.onerror = () => {
      ArcadiaObservation.runtime('degraded', 'controller-trainer-sse');
      ArcadiaObservation.currentness('controller-trainer-sse', 'stale');
      close();
      if (hasWatchers()) setTimeout(connect, 800);
    };
  }
  function subscribe(key, fn) {
    if (!key || typeof fn !== 'function') return () => {};
    subscribers.set(key, fn);
    connect();
    if (state.lastInput) fn(state.lastInput, { eventName: state.lastEvent || 'cached', key, lease: state.lease });
    return () => {
      subscribers.delete(key);
      if (!hasWatchers()) close();
    };
  }
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState !== 'visible') close();
    else connect();
  });
  return { subscribe, close, state };
})();

function controllerPaneIsActive() {
  return Boolean(document.querySelector('[data-view-panel="controllers"].is-active')) && document.visibilityState === 'visible';
}

function controllerRecoveryNeedsRestart(recovery = {}) {
  const state = String(recovery.state || '').toLowerCase();
  const detail = String(recovery.detail || '').toLowerCase();
  return state === 'gamepad-surface-missing' || detail.includes('no gamepad event surface');
}

function controllerRecoveryModalLine(recovery = {}) {
  const state = String(recovery.state || '').toLowerCase();
  return state === 'receiver-idle' || controllerRecoveryNeedsRestart(recovery);
}

function updateControllerRecoverySurfaces(root, recovery = {}) {
  const normalizedState = String(recovery.state || '').toLowerCase();
  const nominal = !normalizedState || normalizedState === 'connected' || normalizedState === 'nominal';
  root.querySelectorAll('[data-controller-recovery]').forEach((card) => {
    card.hidden = nominal;
    card.setAttribute('aria-hidden', String(nominal));
    const title = card.querySelector('[data-controller-recovery-title]');
    const detail = card.querySelector('[data-controller-recovery-detail]');
    const action = card.querySelector('[data-controller-recovery-action]');
    if (title) title.textContent = recovery.title || 'Controller recovery';
    if (detail) detail.textContent = recovery.detail || '';
    if (action) action.textContent = recovery.action || '';
  });
  root.querySelectorAll('[data-controller-programmer-recovery]').forEach((line) => {
    const show = controllerRecoveryModalLine(recovery);
    line.hidden = !show;
    line.setAttribute('aria-hidden', String(!show));
    if (show) line.textContent = recovery.detail || '';
  });
}

let controllerPaneUnsubscribe = null;
function controllerPaneWidget(state) {
  const controllers = state?.controllers || {};
  const panel = document.querySelector('[data-view-panel="controllers"]');
  if (!panel) return;
  if (Array.isArray(controllers.controllerPool)) {
    panel.querySelectorAll('[data-controller-pool-count]').forEach((node) => { node.textContent = String(controllers.controllerPool.length); });
  }
  panel.querySelectorAll('[data-controller-primary-device]').forEach((node) => { node.textContent = controllers.primaryDevice || 'No controller detected'; });
  updateControllerRecoverySurfaces(panel, controllers.recovery || {});
  const programmer = document.querySelector('[data-controller-programmer-modal]');
  if (programmer) updateControllerRecoverySurfaces(programmer, controllers.recovery || {});
  if (!document.querySelector('[data-controller-programmer-modal]')) {
    hydrateControllerBindings(panel, controllers.profile?.bindings || []);
  }
  const shouldStream = controllerPaneIsActive();
  if (shouldStream && !controllerPaneUnsubscribe) {
    controllerPaneUnsubscribe = ArcadiaControllerTrainerStream.subscribe('controllers-pane', (input) => {
      if (!controllerPaneIsActive() || document.querySelector('[data-controller-programmer-modal], [data-controller-tuner-modal]')) return;
      updateControllerLiveInput(input);
    });
  } else if (!shouldStream && controllerPaneUnsubscribe) {
    controllerPaneUnsubscribe();
    controllerPaneUnsubscribe = null;
  }
}
ArcadiaProjector.registerWidget('controllersPane', controllerPaneWidget);

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
    chart: null,
    chartLabels: [],
    chartCpuUsage: [],
    chartCpuTemperature: [],
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
  const fmtLoadAvgPct = (value, cores) => value == null ? '—' : `${((value / Math.max(1, cores)) * 100).toFixed(1)}%`;
  const fmtTemp = (value) => value == null ? '—' : `${value.toFixed(1)}°C`;
  const fmtPressure = (value) => value == null ? '—' : `${value.toFixed(1)}%`;
  const formatBytes = (value) => {
    if (value == null || value < 0) return '—';
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`;
    return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  };
  const updateChart = (usage, temperature) => {
    const canvas = card.querySelector('#loadChart');
    if (!canvas || typeof window.Chart !== 'function') return;
    state.chartLabels.push(new Date().toLocaleTimeString([], { minute: '2-digit', second: '2-digit' }));
    state.chartCpuUsage.push(usage);
    state.chartCpuTemperature.push(temperature);
    while (state.chartLabels.length > 60) { state.chartLabels.shift(); state.chartCpuUsage.shift(); state.chartCpuTemperature.shift(); }
    if (!state.chart) {
      state.chart = new window.Chart(canvas, {
        type: 'line',
        data: {
          labels: state.chartLabels,
          datasets: [
            {
              label: 'CPU usage',
              data: state.chartCpuUsage,
              borderColor: '#5dc9ff',
              backgroundColor: 'rgba(93, 201, 255, 0.12)',
              tension: 0.25,
              pointRadius: 0,
              yAxisID: 'usage',
            },
            {
              label: 'CPU temperature',
              data: state.chartCpuTemperature,
              borderColor: '#f5a65b',
              backgroundColor: 'rgba(245, 166, 91, 0.12)',
              tension: 0.25,
              pointRadius: 0,
              yAxisID: 'temperature',
            },
          ],
        },
        options: {
          animation: false,
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } },
          scales: {
            usage: {
              position: 'left',
              min: 0,
              max: 100,
              ticks: { callback: (value) => `${value}%` },
            },
            temperature: {
              position: 'right',
              ticks: { callback: (value) => `${value}°C` },
              grid: { drawOnChartArea: false },
            },
            x: { display: false },
          },
        },
      });
    } else state.chart.update('none');
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
    const usage = number(data.cpu?.usagePercent);
    const temp = number(data.cpu?.temperatureCelsius);
    setText('[data-load-readout-value="oneMinute"]', fmtLoadAvgPct(one, cores));
    setText('[data-load-readout-value="fiveMinute"]', fmtLoadAvgPct(five, cores));
    setText('[data-load-readout-value="fifteenMinute"]', fmtLoadAvgPct(fifteen, cores));
    updateChart(usage, temp);
    const memory = data.memory || {};
    const memoryPercent = number(memory.usedPercent);
    const memoryUsed = number(memory.usedBytes);
    const memoryTotal = number(memory.totalBytes);
    setText('[data-memory-used]', formatBytes(memoryUsed));
    setText('[data-memory-total]', memoryTotal == null ? '' : ` / ${formatBytes(memoryTotal)}`);
    const boundedMemoryPercent = memoryPercent == null ? null : Math.max(0, Math.min(100, memoryPercent));
    const memoryUsedSegment = card.querySelector("[data-memory-used-segment]");
    const memoryFreeSegment = card.querySelector("[data-memory-free-segment]");
    const memoryProgress = card.querySelector("[data-memory-bar]");
    if (memoryUsedSegment && boundedMemoryPercent != null) memoryUsedSegment.style.width = String(boundedMemoryPercent) + "%";
    if (memoryFreeSegment && boundedMemoryPercent != null) memoryFreeSegment.style.width = String(100 - boundedMemoryPercent) + "%";
    if (memoryProgress && boundedMemoryPercent != null) memoryProgress.setAttribute("aria-valuenow", String(Math.round(boundedMemoryPercent)));
    const pressure = number(io.pressureAvg10);
    setChip('cpu', fmtTemp(temp), temp == null ? 'idle' : (temp >= 82 ? 'warn' : 'ok'));
    setChip('io', fmtPressure(pressure), pressure == null ? 'idle' : (pressure >= 10 ? 'warn' : 'ok'));
    const readRate = number(disk.readBytesPerSec);
    const writeRate = number(disk.writeBytesPerSec);
    setChip('read', readRate == null ? '—' : formatTransferRate(readRate), readRate > 0 ? 'ok' : 'idle');
    setChip('write', writeRate == null ? '—' : formatTransferRate(writeRate), writeRate > 0 ? 'ok' : 'idle');
  };
  const clearRenewal = () => { if (state.renewalTimer) clearTimeout(state.renewalTimer); state.renewalTimer = null; };
  const clearRetry = () => { if (state.retryTimer) clearTimeout(state.retryTimer); state.retryTimer = null; };
  const stopEvents = () => {
    if (state.source) { state.source.close(); ArcadiaObservation.stream('home-root-sse', 'closed'); }
    state.source = null;
    clearRenewal();
    clearRetry();
  };
  const fetchSnapshotOnce = async () => {
    if (!homeIsActive() || state.inFlight) return;
    state.inFlight = true;
    try {
      const res = await fetch('/api/root', { headers: { Accept: 'application/json' }, cache: 'no-store' });
      if (res.ok) {
        apply(await res.json());
        state.fallbackSnapshots += 1;
        ArcadiaObservation.currentness('status-refresh', state.fallbackSnapshots === 1 ? 'recovered' : 'changed');
      } else {
        ArcadiaObservation.runtime('degraded', 'status-refresh');
        ArcadiaObservation.currentness('status-refresh', 'stale');
      }
    } catch (_) {
      ArcadiaObservation.runtime('fault', 'status-refresh');
      ArcadiaObservation.currentness('status-refresh', 'stale');
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
      ArcadiaObservation.stream('home-root-sse', 'opened');
      state.source = source;
      const onRoot = (event) => {
        if (!homeIsActive()) return stopEvents();
        try {
          apply(JSON.parse(event.data));
          ArcadiaObservation.currentness('home-root-sse', state.events ? 'changed' : 'current');
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
      const onLivingState = (event) => {
        try {
          ArcadiaProjector.apply(JSON.parse(event.data));
        } catch (_) {
          // Ignore malformed living-state payloads and keep the last projected document.
        }
      };
      source.addEventListener('snapshot', onRoot);
      source.addEventListener('root', onRoot);
      source.addEventListener('state', onLivingState);
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
        ArcadiaObservation.runtime('degraded', 'home-root-sse');
        ArcadiaObservation.currentness('home-root-sse', 'stale');
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
      const data = await postJson('/api/v1/attendance/open', { pin: input.value }, { attendance: false });
      input.value = '';
      if (data.ok && typeof data.attendance === 'string') {
        caduceusAttendance = data.attendance;
        openArcadia();
        PopupManager.showToast('Arcadia opened', 'success');
      } else {
        clearCaduceusAttendance();
        error.textContent = data.firstMissingSignal || 'PIN attendance refused.';
        error.hidden = false;
      }
    } catch (_) {
      input.value = '';
      clearCaduceusAttendance();
      error.textContent = 'PIN attendance is unavailable. Arcadia remains locked.';
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

async function confirmationBodyFor(action) {
  if (action === 'controllers-ramrod-all') {
    const ok = await PopupManager.showConfirm({
      title: 'Push mapping to all games?',
      message: 'Push your saved button layout to every game emulator on this console?',
      confirmLabel: 'Push to all',
    });
    return ok ? {} : null;
  }
  return confirmationFor(action);
}

function bindConsoleActions() {
  document.querySelectorAll('.btn[data-action][data-endpoint]').forEach((button) => {
    if (button.dataset.pinRequired !== undefined) return;
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const action = button.dataset.action;
      const endpoint = button.dataset.endpoint;
      ArcadiaObservation.action(action, 'invoked');
      if (action === 'sync-games' && !prepareSyncStart()) return;
      const original = button.textContent;
      if (action === 'storage-rescan') {
        button.disabled = true;
        button.textContent = 'Scanning...';
        try { await postJson(endpoint, {}); window.location.reload(); }
        catch (_) { PopupManager.showToast('Rescan failed', 'error'); button.disabled = false; button.textContent = original; }
        return;
      }
      const body = await confirmationBodyFor(action);
      if (body === null) { ArcadiaObservation.action(action, 'refused'); return; }
      clearMessage('console-action-message');
      button.disabled = true;
      button.textContent = action === 'sync-games' ? 'Sync Running' : 'Running...';
      try {
        const assignBody = action.startsWith('controllers-assign-')
          ? { ...body, controllerId: activeControllerId() }
          : body;
        const data = await postJson(endpoint, assignBody);
        const variant = data.ok ? 'success' : 'error';
        setMessage('console-action-message', formatActionResult(data), variant);
        PopupManager.showToast(data.message || (data.ok ? 'Done' : 'Failed'), variant);
        ArcadiaObservation.action(action, data.ok ? 'settled' : 'refused');
        if (action === 'sync-games' && data.ok !== false) markOnboardingFirstSyncComplete(data);
      } catch (_) {
        ArcadiaObservation.action(action, 'fault');
        setMessage('console-action-message', 'Action request failed.', 'error');
        PopupManager.showToast('Action request failed', 'error');
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
  PopupManager.showModal({ title, body, hideDefaultAction: true, surfaceId: 'modal:storage' });
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

const CONTROLLER_BUTTON_INDEX_LABELS = [
  'A', 'B', 'X', 'Y', 'L1', 'R1', 'L2', 'R2', 'L3', 'R3', 'Select', 'Start',
];

function controlLabelForButtonIndex(index) {
  return CONTROLLER_BUTTON_INDEX_LABELS[index] || `Button ${index}`;
}

function normalizeControllerInputEvents(data) {
  if (!data) return data;
  const normalize = (item) => {
    const match = String(item.control || '').match(/^(Button|Axis)\s+(\d+)$/i);
    if (!match) return item;
    return {
      ...item,
      binding: item.binding || `${match[1].toLowerCase()} ${match[2]}`,
    };
  };
  return {
    ...data,
    pressed: (data.pressed || []).map(normalize),
    axes: (data.axes || []).map(normalize),
  };
}

function readBrowserGamepadInput() {
  const getGamepads = navigator.getGamepads?.bind(navigator);
  if (!getGamepads) return null;
  const pad = getGamepads().find((item) => item && item.connected) || null;
  if (!pad) {
    return {
      state: 'waiting',
      device: 'No browser gamepad',
      samplePath: 'navigator.getGamepads',
      pressed: [],
      axes: [],
    };
  }
  const pressed = [];
  const axes = [];
  pad.buttons.forEach((button, index) => {
    if (!(button.pressed || button.value > 0.45)) return;
    pressed.push({
      control: controlLabelForButtonIndex(index),
      binding: `button ${index}`,
      pressed: true,
    });
  });
  const axisLabels = ['Left Stick X', 'Left Stick Y', '', 'Right Stick X', 'Right Stick Y'];
  pad.axes.forEach((value, index) => {
    if (Math.abs(value) < 0.55) return;
    axes.push({
      control: axisLabels[index] || `Axis ${index}`,
      binding: `axis ${index}`,
      pressed: true,
      axisValue: Math.round(value * 32767),
    });
  });
  return {
    state: pressed.length || axes.length ? 'active' : 'listening',
    device: pad.id || 'Browser gamepad',
    samplePath: 'navigator.getGamepads',
    pressed,
    axes,
  };
}

function mergeControllerInput(serverData, browserData) {
  const server = normalizeControllerInputEvents(serverData) || {
    state: 'listening',
    device: '',
    samplePath: '',
    pressed: [],
    axes: [],
  };
  if (!browserData) return server;
  const browser = normalizeControllerInputEvents(browserData);
  const serverActive = (server.pressed?.length || 0) + (server.axes?.length || 0);
  const browserActive = (browser.pressed?.length || 0) + (browser.axes?.length || 0);
  if (serverActive > 0) return server;
  if (browserActive > 0) return browser;
  return {
    ...server,
    device: browser.device && browser.device !== 'No browser gamepad' ? browser.device : server.device,
    samplePath: browser.samplePath || server.samplePath,
    state: browser.state || server.state,
  };
}

function updateControllerLiveInput(data) {
  const root = document.querySelector('[data-controller-live-input]');
  if (!data) return;
  const state = root?.querySelector('[data-controller-input-state]');
  if (state) state.textContent = titleCase((data.state || 'listening').replace(/-/g, ' '));
  const device = root?.querySelector('[data-controller-input-device]');
  if (device) device.textContent = data.device || 'No controller detected';
  const pressed = new Set((data.pressed || []).map((item) => item.control));
  document.querySelectorAll('[data-controller-control]').forEach((pill) => {
    const active = pressed.has(pill.dataset.controllerControl || '');
    pill.classList.toggle('controller-button-dot--active', active);
    pill.classList.toggle('is-active', active);
  });
  const axes = root?.querySelector('[data-controller-axes]');
  if (axes) {
    const values = data.axes || [];
    axes.innerHTML = '';
    if (!values.length) {
      const idle = document.createElement('span');
      idle.textContent = 'Move a stick or press a button to see activity here.';
      axes.appendChild(idle);
    } else {
      values.forEach((axis) => {
        const pill = document.createElement('span');
        pill.className = 'controller-axis-pill';
        pill.textContent = `${axis.control || 'Axis'} ${axis.binding || ''}${axis.axisValue === undefined || axis.axisValue === null ? '' : ` ${axis.axisValue}`}`.trim();
        axes.appendChild(pill);
      });
    }
  }
}


function auditGamepadControlLayout(root, gapPx = 2) {
  const host = root || document.querySelector('[data-controller-gamepad-programmer]');
  if (!host) return { ok: false, error: 'gamepad programmer not found', overlaps: [], boxes: [] };
  const nodes = [...host.querySelectorAll('[data-gamepad-slot]')].filter((node) => {
    const rect = node.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  });
  const boxes = nodes.map((node) => ({
    slot: node.dataset.gamepadSlot || '',
    control: node.dataset.controllerControl || node.getAttribute('aria-label') || '',
    rect: node.getBoundingClientRect(),
  }));
  const overlaps = [];
  for (let i = 0; i < boxes.length; i += 1) {
    for (let j = i + 1; j < boxes.length; j += 1) {
      const a = boxes[i].rect;
      const b = boxes[j].rect;
      const separated = a.right + gapPx <= b.left
        || b.right + gapPx <= a.left
        || a.bottom + gapPx <= b.top
        || b.bottom + gapPx <= a.top;
      if (!separated) overlaps.push({ a: boxes[i], b: boxes[j] });
    }
  }
  return { ok: overlaps.length === 0, gapPx, overlaps, boxes };
}

function compactGamepadBinding(binding) {
  const raw = String(binding || '').trim();
  if (!raw || raw === 'Waiting' || raw === 'Tap to bind') return raw || '—';
  if (raw.startsWith('button ')) {
    const index = Number.parseInt(raw.slice('button '.length), 10);
    return Number.isFinite(index) ? `B${index}` : raw;
  }
  if (raw.startsWith('axis ')) return `AX${raw.slice('axis '.length)}`;
  if (raw.includes('hat')) return 'Hat';
  return raw;
}

function formatControllerBinding(binding) {
  const raw = String(binding || '').trim();
  if (!raw || raw === 'Waiting' || raw === 'Tap to bind') return raw || 'Not set';
  if (raw.startsWith('button ')) {
    const index = Number.parseInt(raw.slice('button '.length), 10);
    return Number.isFinite(index) ? `#${index + 1}` : raw;
  }
  if (raw.startsWith('axis ')) return `AX${raw.slice('axis '.length)}`;
  if (raw.includes('hat')) return 'D-pad';
  return raw;
}

function activeControllerId() {
  const hub = document.querySelector('[data-controller-state]');
  return hub?.dataset.activeControllerId || '';
}

function scopedControllerBody(root, extra = {}) {
  const controllerId = root?.dataset?.controllerId || activeControllerId();
  return controllerId ? { controllerId, ...extra } : { ...extra };
}

function controllerCaptureButtonKey(item) {
  return item?.binding || item?.input || item?.control || '';
}

function controllerCaptureAxisKey(axis) {
  return axis?.binding || axis?.input || axis?.control || '';
}

function controllerCaptureAxisInput(axis) {
  return axis?.binding || null;
}

function createControllerCaptureGate({ now = () => Date.now(), onGateOpen = () => {} } = {}) {
  let previousPressed = new Set();
  let previousActiveAxes = new Set();
  let releaseGateOpen = false;
  let gateOpenedAt = 0;

  const reset = () => {
    previousPressed = new Set();
    previousActiveAxes = new Set();
    releaseGateOpen = false;
    gateOpenedAt = 0;
  };

  const observe = (merged) => {
    const pressedItems = merged?.pressed || [];
    const axisItems = merged?.axes || [];
    const pressed = new Set(pressedItems.map(controllerCaptureButtonKey).filter(Boolean));
    const activeAxes = new Set(axisItems.map(controllerCaptureAxisKey).filter(Boolean));
    let input = null;

    if (!releaseGateOpen && pressed.size === 0 && activeAxes.size === 0) {
      releaseGateOpen = true;
      gateOpenedAt = now();
      onGateOpen(gateOpenedAt);
    }

    if (releaseGateOpen) {
      const button = pressedItems.find((item) => {
        const key = controllerCaptureButtonKey(item);
        return key && !previousPressed.has(key);
      });
      if (button) {
        input = button.binding || button.input || null;
      } else {
        const axis = axisItems.find((item) => {
          const key = controllerCaptureAxisKey(item);
          return key && !previousActiveAxes.has(key);
        });
        input = controllerCaptureAxisInput(axis);
      }
    }

    previousPressed = pressed;
    previousActiveAxes = activeAxes;
    return { input, releaseGateOpen, gateOpenedAt };
  };

  return { reset, observe };
}

const CONTROLLER_CANONICAL_CONTROLS = [
  'A', 'B', 'X', 'Y', 'L1', 'R1', 'L2', 'R2', 'L3', 'R3', 'Select', 'Start',
  'D-pad Up', 'D-pad Down', 'D-pad Left', 'D-pad Right',
  'Left Stick X', 'Left Stick Y', 'Right Stick X', 'Right Stick Y',
];
const CONTROLLER_CANONICAL_CONTROL_SET = new Set(CONTROLLER_CANONICAL_CONTROLS);
const CONTROLLER_DEFAULT_BINDING_TO_CONTROL = new Map([
  ['button 0', 'A'], ['button 1', 'B'], ['button 2', 'X'], ['button 3', 'Y'],
  ['button 4', 'L1'], ['button 5', 'R1'], ['axis 2', 'L2'], ['axis 5', 'R2'],
  ['button 8', 'L3'], ['button 9', 'R3'], ['button 6', 'Select'], ['button 7', 'Start'],
  ['hat 0 up', 'D-pad Up'], ['hat 0 down', 'D-pad Down'], ['hat 0 left', 'D-pad Left'], ['hat 0 right', 'D-pad Right'],
  ['axis 0', 'Left Stick X'], ['axis 1', 'Left Stick Y'], ['axis 3', 'Right Stick X'], ['axis 4', 'Right Stick Y'],
]);

function normalizeControllerBindingLabel(value) {
  const raw = String(value || '').trim();
  const lowered = raw.toLowerCase();
  if (lowered.startsWith('button ') || lowered.startsWith('axis ') || lowered.startsWith('hat 0 ')) return lowered;
  if (lowered.startsWith('#')) {
    const parsed = Number.parseInt(lowered.slice(1), 10);
    return Number.isFinite(parsed) && parsed > 0 ? `button ${parsed - 1}` : '';
  }
  if (/^b\d+$/.test(lowered)) return `button ${lowered.slice(1)}`;
  if (/^ax\d+$/.test(lowered)) return `axis ${lowered.slice(2)}`;
  return '';
}

function canonicalControllerControlForBindingLabel(value) {
  const binding = normalizeControllerBindingLabel(value);
  return binding ? CONTROLLER_DEFAULT_BINDING_TO_CONTROL.get(binding) || '' : '';
}

function canonicalControllerBindingControl(item) {
  const raw = String(item?.control || '').trim();
  if (CONTROLLER_CANONICAL_CONTROL_SET.has(raw)) return raw;
  return canonicalControllerControlForBindingLabel(raw) || canonicalControllerControlForBindingLabel(item?.binding);
}

function controllerBindingMap(bindings) {
  const map = new Map();
  (bindings || []).forEach((item) => {
    const control = canonicalControllerBindingControl(item);
    if (control) map.set(control, item.binding);
  });
  return map;
}

function hydrateControllerBindings(root, bindings) {
  if (!root) return;
  const map = controllerBindingMap(bindings);
  root.querySelectorAll('[data-binding-control]').forEach((node) => {
    const control = node.dataset.bindingControl || '';
    const binding = map.get(control);
    node.textContent = binding ? formatControllerBinding(binding) : 'Unbound';
    node.closest('[data-controller-bind-row]')?.setAttribute('data-controller-binding-state', binding ? 'bound' : 'unbound');
  });
  root.querySelectorAll('[data-controller-control]').forEach((node) => {
    const control = node.dataset.controllerControl || '';
    const binding = map.get(control);
    if (!binding) return;
    const label = node.querySelector('[data-controller-binding-label]');
    if (label) label.textContent = compactGamepadBinding(binding);
  });
}

function hydrateControllerGamepad(root, bindings) {
  hydrateControllerBindings(root, bindings);
}

function updateControllerPoolSelection(controllerId) {
  const hub = document.querySelector('[data-controller-state]');
  if (hub && controllerId) hub.dataset.activeControllerId = controllerId;
  document.querySelectorAll('[data-controller-pool] .controller-pool-card').forEach((card) => {
    const id = card.dataset.controllerId || '';
    card.classList.toggle('controller-pool-card--selected', Boolean(controllerId) && id === controllerId);
  });
}

function updateControllerPoolCount() {
  const pool = document.querySelector('[data-controller-pool]');
  if (!pool) return 0;
  const remaining = pool.querySelectorAll('.controller-pool-card').length;
  const badge = pool.querySelector('.controls-card__head-actions .system-status');
  if (badge) badge.textContent = `${remaining} saved`;
  return remaining;
}

function removeControllerPoolCard(controllerId) {
  const card = document.querySelector(`[data-controller-pool] .controller-pool-card[data-controller-id="${CSS.escape(controllerId)}"]`);
  if (card) card.remove();
  const remaining = updateControllerPoolCount();
  if (remaining === 0) window.location.reload();
}

function controllerProgrammerRoot() {
  return document.querySelector('[data-controller-programmer-modal]') || document.querySelector('[data-view-panel="controllers"]');
}

function bindControllerProgramming() {
  const panel = document.querySelector('[data-view-panel="controllers"]');
  if (!panel) return;
  const TEACH_SWEEP_ORDER = [
    { control: 'A', highlightSlot: 'face-a', prompt: 'Press A (1 of 18)' },
    { control: 'B', highlightSlot: 'face-b', prompt: 'Press B (2 of 18)' },
    { control: 'X', highlightSlot: 'face-x', prompt: 'Press X (3 of 18)' },
    { control: 'Y', highlightSlot: 'face-y', prompt: 'Press Y (4 of 18)' },
    { control: 'L1', highlightSlot: 'shoulder-l1', prompt: 'Press the left shoulder (5 of 18)' },
    { control: 'R1', highlightSlot: 'shoulder-r1', prompt: 'Press the right shoulder (6 of 18)' },
    { control: 'L2', highlightSlot: 'shoulder-l2', prompt: 'Pull the left trigger (7 of 18)' },
    { control: 'R2', highlightSlot: 'shoulder-r2', prompt: 'Pull the right trigger (8 of 18)' },
    { control: 'Select', highlightSlot: 'system-select', prompt: 'Press Select (9 of 18)' },
    { control: 'Start', highlightSlot: 'system-start', prompt: 'Press Start (10 of 18)' },
    { control: 'Left Stick X', highlightSlot: 'stick-left', prompt: 'Push the left stick straight left (11 of 18)' },
    { control: 'Left Stick Y', highlightSlot: 'stick-left', prompt: 'Push the left stick straight up (12 of 18)' },
    { control: 'Right Stick X', highlightSlot: 'stick-right', prompt: 'Push the right stick straight left (13 of 18)' },
    { control: 'Right Stick Y', highlightSlot: 'stick-right', prompt: 'Push the right stick straight up (14 of 18)' },
    { control: 'D-pad Up', highlightSlot: 'dpad', prompt: 'Press up on the D-pad (15 of 18)' },
    { control: 'D-pad Down', highlightSlot: 'dpad', prompt: 'Press down on the D-pad (16 of 18)' },
    { control: 'D-pad Left', highlightSlot: 'dpad', prompt: 'Press left on the D-pad (17 of 18)' },
    { control: 'D-pad Right', highlightSlot: 'dpad', prompt: 'Press right on the D-pad (18 of 18)' },
  ];
  let selected = null;
  let bindingStartedAt = 0;
  let bindingTimeoutShown = false;
  let programmerTimer = null;
  let programmerPaused = false;
  let teachIndex = -1;
  const intervalMs = 60;
  const bindingTimeoutMs = 3000;
  const captureGate = createControllerCaptureGate({
    onGateOpen: (openedAt) => {
      bindingStartedAt = openedAt;
      bindingTimeoutShown = false;
    },
  });
  const resetCaptureGate = () => {
    captureGate.reset();
    bindingStartedAt = 0;
    bindingTimeoutShown = false;
  };

  const refreshModalBindings = async (root) => {
    try {
      const state = await getJson('/api/controllers/state');
      const activeId = root.dataset.controllerId || state.activeControllerId || '';
      const entry = (state.controllerPool || []).find((item) => item.id === activeId);
      hydrateControllerBindings(root, entry?.bindings?.length ? entry.bindings : (state.profile?.bindings || []));
      updateControllerRecoverySurfaces(root, state.recovery || {});
      const device = root.querySelector('[data-controller-programmer-device]');
      if (device) device.textContent = entry?.name || state.primaryDevice || state.liveInput?.device || 'No controller detected';
    } catch (_) {}
  };

  const bindProfileCards = (root) => {
    root.querySelectorAll('button[data-controller-profile-action="apply"]:not([data-controller-profile-bound])').forEach((button) => {
      button.dataset.controllerProfileBound = 'true';
      button.addEventListener('click', async (event) => {
        event.preventDefault();
        const profile = button.dataset.controllerProfile || '';
        button.disabled = true;
        try {
          const data = await postJson('/api/actions/controllers-apply-profile', scopedControllerBody(root, { profile }));
          root.querySelectorAll('[data-controller-profile-action="apply"]').forEach((node) => node.classList.toggle('controller-profile-card--active', node === button));
          const state = button.querySelector('em');
          if (state) state.textContent = data.ok ? 'Active' : 'Apply';
          if (data.ok) await refreshModalBindings(root);
          PopupManager.showToast(data.message || `${profile} profile applied`, data.ok ? 'success' : 'error');
        } catch (_) {
          PopupManager.showToast(`${profile} profile failed`, 'error');
        } finally {
          button.disabled = false;
        }
      });
    });
  };

  const clearTeachTarget = (root) => {
    root.classList.remove('is-teaching');
    delete root.dataset.teachControl;
    root.querySelectorAll('.is-teach-target').forEach((node) => node.classList.remove('is-teach-target'));
    root.querySelector('[data-controller-teach-skip]')?.setAttribute('hidden', '');
    root.querySelector('[data-controller-teach-exit]')?.setAttribute('hidden', '');
    root.querySelector('[data-action="controllers-save-profile"]')?.classList.remove('controller-save-layout--cue');
  };

  const selectProgrammerControl = (root, control) => {
    selected = control;
    resetCaptureGate();
    root.querySelectorAll('[data-controller-control], [data-controller-bind-row]').forEach((node) => {
      node.classList.toggle('is-selected', node.dataset.controllerControl === control);
    });
  };

  const setBindingListenState = (root, control, options = {}) => {
    root.classList.toggle('is-binding', Boolean(control));
    if (control) root.dataset.bindingTarget = control;
    else delete root.dataset.bindingTarget;
    bindingStartedAt = 0;
    bindingTimeoutShown = false;
    const state = root.querySelector('[data-controller-programmer-state]');
    if (!state) return;
    if (control) {
      state.textContent = options.text || `Now press ${control} on your controller`;
      state.classList.add('controller-map-bind-state--active');
    } else {
      state.textContent = options.text || 'Tap a control on the gamepad to begin';
      state.classList.toggle('controller-map-bind-state--active', Boolean(options.active));
    }
  };

  const clearTeachMode = (root, message = 'Tap a control on the gamepad to begin') => {
    teachIndex = -1;
    selected = null;
    resetCaptureGate();
    clearTeachTarget(root);
    root.querySelectorAll('[data-controller-control], [data-controller-bind-row]').forEach((node) => node.classList.remove('is-selected'));
    setBindingListenState(root, null, { text: message });
  };

  const setTeachStep = (root, index) => {
    if (index >= TEACH_SWEEP_ORDER.length) {
      teachIndex = -1;
      selected = null;
      resetCaptureGate();
      clearTeachTarget(root);
      const save = root.querySelector('[data-action="controllers-save-profile"]');
      if (save) {
        save.classList.add('controller-save-layout--cue');
        save.addEventListener('animationend', () => save.classList.remove('controller-save-layout--cue'), { once: true });
      }
      root.querySelectorAll('[data-controller-control], [data-controller-bind-row]').forEach((node) => node.classList.remove('is-selected'));
      setBindingListenState(root, null, { text: 'Teaching complete — save your layout', active: true });
      return;
    }
    teachIndex = index;
    clearTeachTarget(root);
    const step = TEACH_SWEEP_ORDER[teachIndex];
    root.classList.add('is-teaching');
    root.dataset.teachControl = step.control;
    root.querySelector('[data-controller-teach-skip]')?.removeAttribute('hidden');
    root.querySelector('[data-controller-teach-exit]')?.removeAttribute('hidden');
    const highlightSelector = step.highlightSlot
      ? `[data-gamepad-slot="${CSS.escape(step.highlightSlot)}"]`
      : `[data-gamepad-slot][data-controller-control="${CSS.escape(step.control)}"], [data-gamepad-slot] [data-controller-control="${CSS.escape(step.control)}"]`;
    root.querySelectorAll(highlightSelector).forEach((node) => {
      (node.closest('[data-gamepad-slot]') || node).classList.add('is-teach-target');
    });
    selectProgrammerControl(root, step.control);
    setBindingListenState(root, step.control, { text: step.prompt || `Press ${step.control} on your controller (${teachIndex + 1} of ${TEACH_SWEEP_ORDER.length})` });
  };

  const advanceTeachStep = (root) => {
    if (teachIndex < 0) return;
    setTeachStep(root, teachIndex + 1);
  };

  const bindControlButtons = (root) => {
    root.querySelectorAll('[data-gamepad-slot][data-controller-control]:not([data-controller-control-bound]), [data-gamepad-slot] [data-controller-control]:not([data-controller-control-bound])').forEach((button) => {
      button.dataset.controllerControlBound = 'true';
      const activateControl = (event) => {
        event.preventDefault();
        const control = button.dataset.controllerControl || '';
        if (!control || control === 'D-pad') return;
        if (teachIndex >= 0) clearTeachMode(root);
        selectProgrammerControl(root, control);
        setBindingListenState(root, control);
        PopupManager.showToast(`Tap detected — now press ${control} on your real controller`, 'info');
      };
      button.addEventListener('click', activateControl);
      button.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') activateControl(event);
      });
    });
    root.querySelectorAll('[data-controller-bind-row]:not([data-controller-bind-row-bound])').forEach((button) => {
      button.dataset.controllerBindRowBound = 'true';
      button.addEventListener('click', (event) => {
        event.preventDefault();
        const control = button.dataset.controllerControl || '';
        if (!control) return;
        if (teachIndex >= 0) clearTeachMode(root);
        selectProgrammerControl(root, control);
        setBindingListenState(root, control);
        PopupManager.showToast(`Now press ${control} on your real controller`, 'info');
      });
    });
  };

  const ingestProgrammerInput = async (root, data) => {
    const merged = normalizeControllerInputEvents(data);
    updateControllerLiveInput(merged);
    const pressed = new Set((merged.pressed || []).map((item) => item.control));
    root.querySelectorAll('[data-controller-control]').forEach((pill) => {
      const active = pressed.has(pill.dataset.controllerControl || '');
      pill.classList.toggle('controller-button-dot--active', active);
      pill.classList.toggle('is-active', active);
    });
    const device = root.querySelector('[data-controller-programmer-device]');
    if (device) device.textContent = merged.device || 'No controller detected';
    const axes = root.querySelector('[data-controller-programmer-axes]');
    if (axes) {
      axes.textContent = '';
      (merged.axes || []).forEach((axis) => {
        const pill = document.createElement('span');
        pill.className = 'controller-axis-pill';
        pill.textContent = `${axis.control || 'Axis'} ${axis.binding || ''}${axis.axisValue === undefined || axis.axisValue === null ? '' : ` ${axis.axisValue}`}`.trim();
        axes.appendChild(pill);
      });
    }
    const capture = captureGate.observe(merged);
    if (selected) {
      const input = capture.input;
      if (!input && capture.releaseGateOpen && bindingStartedAt && !bindingTimeoutShown && Date.now() - bindingStartedAt >= bindingTimeoutMs) {
        bindingTimeoutShown = true;
        PopupManager.showToast('No button detected. Pause or quit your game first — running games often keep the controller.', 'info');
      }
      if (input) {
        const capturedControl = selected;
        const result = await postJson('/api/actions/controllers-bind', scopedControllerBody(root, { control: capturedControl, binding: input }));
        root.querySelectorAll(`[data-controller-control="${CSS.escape(capturedControl)}"], [data-binding-control="${CSS.escape(capturedControl)}"]`).forEach((node) => {
          if (node.dataset.bindingControl) node.textContent = formatControllerBinding(result.stdout || input);
          const label = node.querySelector?.('[data-controller-binding-label]') || node.closest?.('[data-gamepad-slot]')?.querySelector?.('[data-controller-binding-label]');
          if (label && result.stdout) label.textContent = formatControllerBinding(result.stdout);
        });
        PopupManager.showToast(result.message || `${capturedControl} mapped`, result.ok ? 'success' : 'error');
        if (!result.ok && teachIndex >= 0) {
          setTeachStep(root, teachIndex);
          return;
        }
        selected = null;
        setBindingListenState(root, null);
        root.querySelectorAll('[data-controller-control], [data-controller-bind-row]').forEach((node) => node.classList.remove('is-selected'));
        if (result.ok && teachIndex >= 0) advanceTeachStep(root);
      }
    }
  };

  const bindModalActions = (root) => {
    root.querySelectorAll('.btn[data-action][data-endpoint]:not([data-modal-action-bound])').forEach((button) => {
      button.dataset.modalActionBound = 'true';
      button.addEventListener('click', async (event) => {
        event.preventDefault();
        const action = button.dataset.action;
        const endpoint = button.dataset.endpoint;
        const original = button.textContent;
        const body = confirmationFor(action);
        if (body === null) return;
        button.disabled = true;
        button.textContent = 'Running...';
        try {
          const data = await postJson(endpoint, { ...body, ...scopedControllerBody(root) });
          const variant = data.ok ? 'success' : 'error';
          PopupManager.showToast(data.message || (data.ok ? 'Done' : 'Failed'), variant);
        } catch (_) {
          PopupManager.showToast('Action request failed', 'error');
        } finally {
          button.disabled = false;
          button.textContent = original;
        }
      });
    });
  };

  const openControllerModal = async (controllerId) => {
    const targetId = controllerId || activeControllerId();
    const template = document.getElementById('controller-programmer-template');
    const body = template?.content?.firstElementChild?.cloneNode(true);
    if (!body) return;
    const loadingHost = document.createElement('div');
    loadingHost.className = 'ux-loading-host ux-loading-host--fullscreen-modal';
    loadingHost.appendChild(ArcadiaLoading.spinner({ label: 'Opening controller map…', size: 'lg' }));
    PopupManager.showModal({ title: 'Map buttons', body: loadingHost, hideDefaultAction: true, variant: 'fullscreen', surfaceId: 'modal:controller-map' });
    try {
      const currentId = activeControllerId();
      if (targetId && targetId !== currentId) {
        const data = await postJson('/api/actions/controllers-select', { controllerId: targetId });
        if (!data.ok) {
          PopupManager.closeModal();
          PopupManager.showToast(data.message || 'Could not select controller', 'error');
          return;
        }
        updateControllerPoolSelection(targetId);
      }
      const state = await getJson('/api/controllers/state');
      programmerPaused = false;
      const activeId = targetId || state.activeControllerId || body.dataset.controllerId || '';
      body.dataset.controllerId = activeId;
      const entry = (state.controllerPool || []).find((item) => item.id === activeId);
      hydrateControllerBindings(body, entry?.bindings?.length ? entry.bindings : (state.profile?.bindings || []));
      hydrateControllerGamepad(body, entry?.bindings?.length ? entry.bindings : (state.profile?.bindings || []));
      const content = document.getElementById('modal-content');
      if (content) {
        content.textContent = '';
        content.appendChild(body);
      }
      const root = document.querySelector('[data-controller-programmer-modal]');
      if (root) {
        updateControllerRecoverySurfaces(root, state.recovery || {});
        const device = root.querySelector('[data-controller-programmer-device]');
        if (device) device.textContent = entry?.name || state.primaryDevice || state.liveInput?.device || 'No controller detected';
        startProgrammerLoop(root);
      }
    } catch (_) {
      PopupManager.closeModal();
      PopupManager.showToast('Could not open controller mapping', 'error');
    }
  };

  const startProgrammerLoop = (root) => {
    if (programmerTimer) { programmerTimer(); programmerTimer = null; }
    resetCaptureGate();
    bindProfileCards(root);
    bindControlButtons(root);
    bindModalActions(root);
    const teach = root.querySelector('[data-controller-teach-start]:not([data-controller-teach-bound])');
    if (teach) {
      teach.dataset.controllerTeachBound = 'true';
      teach.addEventListener('click', (event) => {
        event.preventDefault();
        clearTeachMode(root);
        setTeachStep(root, 0);
      });
    }
    const skip = root.querySelector('[data-controller-teach-skip]:not([data-controller-teach-skip-bound])');
    if (skip) {
      skip.dataset.controllerTeachSkipBound = 'true';
      skip.addEventListener('click', (event) => {
        event.preventDefault();
        advanceTeachStep(root);
      });
    }
    const exit = root.querySelector('[data-controller-teach-exit]:not([data-controller-teach-exit-bound])');
    if (exit) {
      exit.dataset.controllerTeachExitBound = 'true';
      exit.addEventListener('click', (event) => {
        event.preventDefault();
        clearTeachMode(root);
      });
    }
    if (!root.dataset.controllerTeachEscBound) {
      root.dataset.controllerTeachEscBound = 'true';
      root.addEventListener('keydown', (event) => {
        if (event.key !== 'Escape' || teachIndex < 0) return;
        event.preventDefault();
        clearTeachMode(root);
      });
    }
    const readout = root.querySelector('[data-controller-broadcast-readout]');
    if (readout) readout.textContent = 'Live';
    programmerTimer = ArcadiaControllerTrainerStream.subscribe('controller-programmer', async (merged) => {
      if (!document.querySelector('[data-controller-programmer-modal]')) {
        if (programmerTimer) programmerTimer();
        programmerTimer = null;
        return;
      }
      if (programmerPaused) return;
      await ingestProgrammerInput(root, merged);
    });
    const toggle = root.querySelector('[data-controller-broadcast-toggle]:not([data-controller-broadcast-bound])');
    if (toggle) {
      toggle.dataset.controllerBroadcastBound = 'true';
      toggle.addEventListener('click', (event) => {
        programmerPaused = !programmerPaused;
        event.currentTarget.textContent = programmerPaused ? 'Resume live preview' : 'Pause live preview';
      });
    }
  };

  panel.querySelectorAll('[data-controller-forget]:not([data-controller-forget-bound])').forEach((button) => {
    button.dataset.controllerForgetBound = 'true';
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      event.stopPropagation();
      const controllerId = button.dataset.controllerForget || '';
      if (!controllerId) return;
      const card = button.closest('.controller-pool-card');
      const name = card?.querySelector('.controller-pool-card__name')?.textContent?.trim() || 'this controller';
      const ok = await PopupManager.showConfirm({
        title: 'Forget controller?',
        message: `Remove ${name} from your controllers? Saved button mappings for this gamepad will be forgotten.`,
        confirmLabel: 'Forget',
        cancelLabel: 'Keep',
        danger: true,
      });
      if (!ok) return;
      const original = button.textContent;
      button.disabled = true;
      button.textContent = 'Removing…';
      try {
        const data = await postJson('/api/actions/controllers-forget', { controllerId });
        PopupManager.showToast(data.message || (data.ok ? 'Controller forgotten' : 'Could not forget controller'), data.ok ? 'success' : 'error');
        if (data.ok) {
          removeControllerPoolCard(controllerId);
          const nextId = (data.stdout || '').trim();
          if (nextId) updateControllerPoolSelection(nextId);
          else updateControllerPoolSelection('');
        }
      } catch (_) {
        PopupManager.showToast('Could not forget controller', 'error');
      } finally {
        button.disabled = false;
        button.textContent = original;
      }
    });
  });

  panel.querySelectorAll('[data-controller-select]:not([data-controller-select-bound])').forEach((button) => {
    button.dataset.controllerSelectBound = 'true';
    button.addEventListener('click', async (event) => {
      event.preventDefault();
      const controllerId = button.dataset.controllerSelect || '';
      if (!controllerId) return;
      try {
        const data = await postJson('/api/actions/controllers-select', { controllerId });
        if (data.ok) updateControllerPoolSelection(controllerId);
        PopupManager.showToast(data.message || (data.ok ? 'Controller selected' : 'Selection failed'), data.ok ? 'success' : 'error');
      } catch (_) {
        PopupManager.showToast('Could not select controller', 'error');
      }
    });
  });

  const tuningPercent = (ratio) => `${Math.round(Number(ratio || 0) * 100)}%`;
  const tuningFromPercent = (percent, minRatio, maxRatio) => {
    const ratio = Number(percent) / 100;
    return Math.min(maxRatio, Math.max(minRatio, ratio));
  };
  const defaultTuning = () => ({
    leftStickDeadzone: 0.15,
    rightStickDeadzone: 0.15,
    leftStickSensitivity: 1,
    rightStickSensitivity: 1,
  });
  const hydrateTunerSliders = (root, tuning = {}) => {
    const apply = (side, deadzone, sensitivity) => {
      const deadzoneInput = root.querySelector(`[data-controller-tuner-deadzone="${side}"]`);
      const sensitivityInput = root.querySelector(`[data-controller-tuner-sensitivity="${side}"]`);
      const deadzoneOut = root.querySelector(`[data-controller-tuner-deadzone-value="${side}"]`);
      const sensitivityOut = root.querySelector(`[data-controller-tuner-sensitivity-value="${side}"]`);
      if (deadzoneInput) {
        deadzoneInput.value = String(Math.round((deadzone ?? 0.15) * 100));
        if (deadzoneOut) deadzoneOut.textContent = tuningPercent(deadzone ?? 0.15);
      }
      if (sensitivityInput) {
        sensitivityInput.value = String(Math.round((sensitivity ?? 1) * 100));
        if (sensitivityOut) sensitivityOut.textContent = tuningPercent(sensitivity ?? 1);
      }
    };
    apply('left', tuning.leftStickDeadzone, tuning.leftStickSensitivity);
    apply('right', tuning.rightStickDeadzone, tuning.rightStickSensitivity);
  };
  const readTunerPayload = (root) => ({
    leftStickDeadzone: tuningFromPercent(root.querySelector('[data-controller-tuner-deadzone="left"]')?.value, 0, 0.4),
    rightStickDeadzone: tuningFromPercent(root.querySelector('[data-controller-tuner-deadzone="right"]')?.value, 0, 0.4),
    leftStickSensitivity: tuningFromPercent(root.querySelector('[data-controller-tuner-sensitivity="left"]')?.value, 0.1, 1),
    rightStickSensitivity: tuningFromPercent(root.querySelector('[data-controller-tuner-sensitivity="right"]')?.value, 0.1, 1),
  });
  const normalizeStickAxis = (value) => {
    const n = Number(value);
    if (!Number.isFinite(n)) return 0;
    if (Math.abs(n) <= 1.05) return n;
    return Math.max(-1, Math.min(1, n / 32767));
  };
  const applyStickFeel = (x, y, deadzone, sensitivity) => {
    const shape = (v) => {
      const abs = Math.abs(v);
      if (abs <= deadzone) return 0;
      const scaled = (abs - deadzone) / Math.max(0.01, 1 - deadzone);
      return Math.sign(v) * Math.min(1, scaled * sensitivity);
    };
    return { x: shape(x), y: shape(y) };
  };
  const updateTunerPreview = (root, merged) => {
    const payload = readTunerPayload(root);
    const axisValue = (index) => {
      const match = (merged.axes || []).find((axis) => String(axis.control || '').includes(String(index)));
      return normalizeStickAxis(match?.binding);
    };
    const browser = readBrowserGamepadInput();
    const browserAxis = (index) => normalizeStickAxis(browser.axes?.find((axis) => axis.index === index)?.value);
    const left = applyStickFeel(
      browserAxis(0) || axisValue(0),
      browserAxis(1) || axisValue(1),
      payload.leftStickDeadzone,
      payload.leftStickSensitivity,
    );
    const right = applyStickFeel(
      browserAxis(3) || axisValue(3),
      browserAxis(4) || axisValue(4),
      payload.rightStickDeadzone,
      payload.rightStickSensitivity,
    );
    const paint = (side, vector) => {
      const dot = root.querySelector(`[data-controller-tuner-dot="${side}"]`);
      if (!dot) return;
      dot.style.transform = `translate(${vector.x * 42}%, ${vector.y * 42}%)`;
    };
    paint('left', left);
    paint('right', right);
  };
  let tunerTimer = null;
  const bindTunerControls = (root) => {
    root.querySelectorAll('[data-controller-tuner-deadzone], [data-controller-tuner-sensitivity]').forEach((input) => {
      if (input.dataset.controllerTunerInputBound) return;
      input.dataset.controllerTunerInputBound = 'true';
      input.addEventListener('input', () => {
        const side = input.dataset.controllerTunerDeadzone || input.dataset.controllerTunerSensitivity;
        const deadzoneOut = root.querySelector(`[data-controller-tuner-deadzone-value="${side}"]`);
        const sensitivityOut = root.querySelector(`[data-controller-tuner-sensitivity-value="${side}"]`);
        const deadzoneInput = root.querySelector(`[data-controller-tuner-deadzone="${side}"]`);
        const sensitivityInput = root.querySelector(`[data-controller-tuner-sensitivity="${side}"]`);
        if (deadzoneOut && deadzoneInput) deadzoneOut.textContent = `${deadzoneInput.value}%`;
        if (sensitivityOut && sensitivityInput) sensitivityOut.textContent = `${sensitivityInput.value}%`;
      });
    });
    const reset = root.querySelector('[data-controller-tuner-reset]:not([data-controller-tuner-reset-bound])');
    if (reset) {
      reset.dataset.controllerTunerResetBound = 'true';
      reset.addEventListener('click', () => {
        hydrateTunerSliders(root, defaultTuning());
      });
    }
    const apply = root.querySelector('[data-controller-tuner-apply]:not([data-controller-tuner-apply-bound])');
    if (apply) {
      apply.dataset.controllerTunerApplyBound = 'true';
      apply.addEventListener('click', async () => {
        apply.disabled = true;
        try {
          const data = await postJson('/api/actions/controllers-save-tuning', {
            ...scopedControllerBody(root),
            ...readTunerPayload(root),
          });
          PopupManager.showToast(data.message || 'Controller tuning applied', data.ok ? 'success' : 'error');
          if (data.ok) PopupManager.closeModal();
        } catch (_) {
          PopupManager.showToast('Could not apply controller tuning', 'error');
        } finally {
          apply.disabled = false;
        }
      });
    }
    const cancel = root.querySelector('[data-controller-tuner-cancel]:not([data-controller-tuner-cancel-bound])');
    if (cancel) {
      cancel.dataset.controllerTunerCancelBound = 'true';
      cancel.addEventListener('click', () => PopupManager.closeModal());
    }
  };
  const startTunerLoop = (root) => {
    if (tunerTimer) { tunerTimer(); tunerTimer = null; }
    bindTunerControls(root);
    tunerTimer = ArcadiaControllerTrainerStream.subscribe('controller-tuner', (merged) => {
      if (!document.querySelector('[data-controller-tuner-modal]')) {
        if (tunerTimer) tunerTimer();
        tunerTimer = null;
        return;
      }
      updateTunerPreview(root, merged);
    });
  };
  const openControllerTunerModal = async (controllerId) => {
    const targetId = controllerId || activeControllerId();
    const template = document.getElementById('controller-tuner-template');
    const body = template?.content?.firstElementChild?.cloneNode(true);
    if (!body) return;
    try {
      const currentId = activeControllerId();
      if (targetId && targetId !== currentId) {
        const data = await postJson('/api/actions/controllers-select', { controllerId: targetId });
        if (!data.ok) {
          PopupManager.showToast(data.message || 'Could not select controller', 'error');
          return;
        }
        updateControllerPoolSelection(targetId);
      }
      const state = await getJson('/api/controllers/state');
      const activeId = targetId || state.activeControllerId || '';
      body.dataset.controllerId = activeId;
      const entry = (state.controllerPool || []).find((item) => item.id === activeId);
      hydrateTunerSliders(body, entry?.tuning || state.profile?.tuning || {});
      const device = body.querySelector('[data-controller-tuner-device]');
      if (device) device.textContent = entry?.name || state.primaryDevice || 'No controller selected';
      PopupManager.showModal({ title: 'Tune controller', body, hideDefaultAction: true, surfaceId: 'modal:controller-tune' });
      const root = document.querySelector('[data-controller-tuner-modal]');
      if (root) startTunerLoop(root);
    } catch (_) {
      PopupManager.showToast('Could not open controller tuning', 'error');
    }
  };

  panel.querySelectorAll('[data-controller-tuner-open]:not([data-controller-tuner-bound])').forEach((button) => {
    button.dataset.controllerTunerBound = 'true';
    button.addEventListener('click', async () => {
      if (button.dataset.loading === 'true') return;
      const controllerId = button.dataset.controllerId || activeControllerId();
      const original = button.textContent;
      button.dataset.loading = 'true';
      button.disabled = true;
      button.textContent = 'Opening…';
      try {
        await openControllerTunerModal(controllerId);
      } finally {
        button.disabled = false;
        button.textContent = original;
        delete button.dataset.loading;
      }
    });
  });

  panel.querySelectorAll('[data-controller-programmer-open]:not([data-controller-programmer-bound])').forEach((button) => {
    button.dataset.controllerProgrammerBound = 'true';
    button.addEventListener('click', async () => {
      if (button.dataset.loading === 'true') return;
      const controllerId = button.dataset.controllerId || activeControllerId();
      const original = button.textContent;
      button.dataset.loading = 'true';
      button.disabled = true;
      button.textContent = 'Opening…';
      try {
        await openControllerModal(controllerId);
      } finally {
        button.disabled = false;
        button.textContent = original;
        delete button.dataset.loading;
      }
    });
  });

  if (!window.arcadiaGamepadListenerBound) {
    window.arcadiaGamepadListenerBound = true;
    window.addEventListener('gamepadconnected', (event) => {
      const label = event.gamepad?.id || 'Gamepad';
      if (document.querySelector('[data-controller-programmer-modal]')) {
        PopupManager.showToast(`${label} ready in browser`, 'success');
      }
    });
  }

  window.arcadiaControllerProgramming = {
    selectedControl: () => selected,
    broadcastMs: () => intervalMs,
    modalOpen: () => Boolean(document.querySelector('[data-controller-programmer-modal]')),
    readBrowserGamepadInput,
    mergeControllerInput,
    auditGamepadControlLayout,
  };
}

function bindControllerLiveInput() {
  const panel = document.querySelector('[data-view-panel="controllers"]');
  if (!panel) return;
  bindControllerProgramming();
  const refresh = () => controllerPaneWidget(window.arcadiaControllerTrainerStreamState?.lastLivingState || {});
  document.addEventListener('arcadia:view-change', refresh);
  document.addEventListener('visibilitychange', refresh);
  document.addEventListener('arcadia:modal-close', refresh);
  refresh();
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
  if (syncStateIsRunning() || ['scanning', 'syncing'].includes(stateNode?.dataset.syncState)) {
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

function syncStateIsRunning() {
  const root = document.querySelector('[data-sync-root]');
  return root?.dataset.state === 'running' || root?.dataset.syncState === 'running';
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
  PopupManager.showModal({ title: 'Add games', body, hideDefaultAction: true, surfaceId: 'modal:add-games' });
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
  PopupManager.showToast(`${files.length} game file${files.length === 1 ? '' : 's'} entering the machine.`, 'info');
  try {
    const response = await fetch(endpoint, { method: 'POST', body: form });
    const data = await response.json().catch(() => ({}));
    if (!response.ok || data.ok === false) {
      const message = data.message || 'Some games were rejected. Open the ledger for the reason and fix action.';
      PopupManager.showToast(message, 'error');
      return;
    }
    const message = data.message || 'Games staged. Press Sync games.';
    PopupManager.showToast(message, 'success');
    PopupManager.closeModal();
  } catch (_) {
    const message = 'The machine could not accept those games.';
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
    openWifiNetworkPicker(data.state);
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

function wifiSignalBars(percent) {
  if (!Number.isFinite(percent) || percent <= 0) return 1;
  if (percent < 25) return 1;
  if (percent < 50) return 2;
  if (percent < 75) return 3;
  return 4;
}

function wifiSignalMeter(percent) {
  const meter = document.createElement('span');
  meter.className = 'wifi-signal-bars';
  meter.dataset.bars = String(wifiSignalBars(percent));
  meter.setAttribute('aria-label', wifiSignalText(percent));
  for (let i = 0; i < 4; i += 1) {
    const bar = document.createElement('span');
    bar.className = 'wifi-signal-bars__bar';
    meter.appendChild(bar);
  }
  return meter;
}

function wifiNetworkMeta(group) {
  return [securityLabel(group.security), group.known ? 'Saved' : 'Nearby', wifiSignalText(group.bestSignalPercent)]
    .filter(Boolean)
    .join(' · ');
}

function wifiNetworkCard(group) {
  const card = document.createElement('article');
  card.className = `wifi-network-card${group.connected ? ' wifi-network-card--connected' : ''}`;
  const main = document.createElement('button');
  main.type = 'button';
  main.className = 'wifi-network-card__main';
  const body = document.createElement('div');
  body.className = 'wifi-network-card__body';
  const title = document.createElement('strong');
  title.textContent = group.ssid;
  const meta = document.createElement('span');
  meta.className = 'wifi-network-card__meta';
  meta.textContent = wifiNetworkMeta(group);
  body.append(title, meta);
  main.append(wifiSignalMeter(group.bestSignalPercent), body);
  if (group.security && group.security !== 'open') {
    const lock = document.createElement('span');
    lock.className = 'wifi-network-card__lock';
    lock.setAttribute('aria-hidden', 'true');
    lock.textContent = '🔒';
    main.appendChild(lock);
  }
  if (group.connected) {
    const badge = document.createElement('span');
    badge.className = 'wifi-network-card__badge';
    badge.textContent = 'Connected';
    main.appendChild(badge);
  }
  main.addEventListener('click', () => {
    if (group.connected) return;
    openWifiConnectModal(group.ssid, group.security !== 'open', group.known, group.bestSignalPercent, group.security);
  });
  card.appendChild(main);
  if (group.connected || group.known) {
    const actions = document.createElement('div');
    actions.className = 'wifi-network-card__actions';
    if (group.connected) {
      const disconnect = document.createElement('button');
      disconnect.type = 'button';
      disconnect.className = 'btn btn--secondary';
      disconnect.textContent = 'Disconnect';
      disconnect.addEventListener('click', () => postNetworkAction('/api/network/wifi/disconnect', {}, 'wifi-disconnect'));
      actions.appendChild(disconnect);
    }
    const forget = document.createElement('button');
    forget.type = 'button';
    forget.className = 'btn btn--secondary';
    forget.textContent = 'Forget';
    forget.addEventListener('click', () => postNetworkAction('/api/network/wifi/forget', { ssid: group.ssid }, 'wifi-forget'));
    actions.appendChild(forget);
    card.appendChild(actions);
  }
  return card;
}

function openWifiNetworkPicker(state) {
  const body = document.createElement('div');
  body.className = 'wifi-modal wifi-modal--picker';
  const hero = document.createElement('header');
  hero.className = 'wifi-modal__hero';
  hero.innerHTML = `
    <span class="wifi-modal__glyph" aria-hidden="true">◌</span>
    <div class="wifi-modal__copy">
      <p class="wifi-modal__eyebrow">Wireless</p>
      <p class="wifi-modal__lede">Pick a network to join</p>
    </div>`;
  const scan = document.createElement('button');
  scan.type = 'button';
  scan.className = 'btn btn--primary wifi-modal__scan';
  scan.textContent = 'Scan';
  scan.addEventListener('click', () => requestWifiScan(true));
  hero.appendChild(scan);
  body.appendChild(hero);
  const list = document.createElement('div');
  list.className = 'wifi-modal__list';
  const groups = normalizeWifiNetworks(state || {});
  if (groups.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'wifi-modal__empty';
    empty.innerHTML = '<strong>No networks in range</strong><p>Scan again or join a hidden network.</p>';
    list.appendChild(empty);
  } else {
    const connected = groups.filter((group) => group.connected);
    const others = groups.filter((group) => !group.connected);
    if (connected.length > 0) {
      const label = document.createElement('p');
      label.className = 'wifi-modal__section-label';
      label.textContent = 'Connected';
      list.appendChild(label);
      connected.forEach((group) => list.appendChild(wifiNetworkCard(group)));
    }
    if (others.length > 0) {
      const label = document.createElement('p');
      label.className = 'wifi-modal__section-label';
      label.textContent = connected.length > 0 ? 'Nearby' : 'Networks';
      list.appendChild(label);
      others.forEach((group) => list.appendChild(wifiNetworkCard(group)));
    }
  }
  body.appendChild(list);
  const foot = document.createElement('footer');
  foot.className = 'wifi-modal__foot';
  const hidden = document.createElement('button');
  hidden.type = 'button';
  hidden.className = 'btn btn--secondary';
  hidden.textContent = 'Join hidden network';
  hidden.addEventListener('click', () => openHiddenNetworkModal());
  foot.appendChild(hidden);
  body.appendChild(foot);
  PopupManager.showModal({ title: 'Wi-Fi', body, hideDefaultAction: true, surfaceId: 'modal:wifi' });
}

function wifiJoinSteps() {
  return ['Joining network', 'Authenticating', 'Getting IP address', 'Testing LAN', 'Testing Internet'];
}

function animateWifiJoinSteps(root) {
  const steps = Array.from(root.querySelectorAll('.wifi-join-step'));
  let index = 0;
  const tick = () => {
    steps.forEach((step, i) => {
      step.classList.toggle('is-active', i === index);
      step.classList.toggle('is-done', i < index);
    });
    if (index < steps.length - 1) {
      index += 1;
      window.setTimeout(tick, 650);
    }
  };
  tick();
}

function securityLabel(value) {
  if (value === 'open') return 'Open';
  if (value === 'wpa2') return 'WPA2';
  if (value === 'wpa3') return 'WPA3';
  if (value === 'wpa-wpa2') return 'WPA/WPA2';
  if (!value) return '';
  return String(value).toUpperCase();
}

function openWifiConnectModal(ssid = '', secured = true, saved = false, signalPercent = null, security = '') {
  const form = document.createElement('form');
  form.className = 'wifi-modal wifi-modal--join';
  form.autocomplete = 'off';
  const hero = document.createElement('div');
  hero.className = 'wifi-join-hero';
  const heroCopy = document.createElement('div');
  const heroTitle = document.createElement('strong');
  heroTitle.textContent = ssid || 'Hidden network';
  const heroMeta = document.createElement('span');
  heroMeta.className = 'wifi-join-hero__meta';
  heroMeta.textContent = [
    securityLabel(security) || (secured ? 'Secured' : 'Open'),
    Number.isFinite(signalPercent) ? wifiSignalText(signalPercent) : '',
    saved ? 'Saved network' : '',
  ].filter(Boolean).join(' · ');
  heroCopy.append(heroTitle, heroMeta);
  hero.append(wifiSignalMeter(signalPercent ?? 0), heroCopy);
  form.appendChild(hero);
  if (!ssid) {
    const ssidField = document.createElement('label');
    ssidField.className = 'wifi-join-field';
    ssidField.innerHTML = '<span>Network name</span><input class="field" name="ssid" autocomplete="off" placeholder="Enter network name">';
    form.appendChild(ssidField);
  }
  if (secured) {
    const passwordField = document.createElement('label');
    passwordField.className = 'wifi-join-field';
    passwordField.innerHTML = '<span>Password</span><input class="field" type="password" name="password" autocomplete="new-password" placeholder="Network password">';
    form.appendChild(passwordField);
    const show = document.createElement('label');
    show.className = 'wifi-join-show';
    show.innerHTML = '<input type="checkbox" data-toggle-modal-password="password">Show password';
    form.appendChild(show);
  }
  const steps = document.createElement('div');
  steps.className = 'wifi-join-steps';
  steps.hidden = true;
  wifiJoinSteps().forEach((label, index) => {
    const step = document.createElement('div');
    step.className = `wifi-join-step${index === 0 ? ' is-active' : ''}`;
    step.textContent = label;
    steps.appendChild(step);
  });
  form.appendChild(steps);
  const msg = document.createElement('p');
  msg.className = 'wifi-join-message message';
  msg.hidden = true;
  form.appendChild(msg);
  const actions = document.createElement('footer');
  actions.className = 'wifi-join-actions';
  actions.innerHTML = `
    <button class="btn btn--primary wifi-join-submit" type="submit">Connect</button>
    <button class="btn btn--secondary" type="button" data-modal-cancel>Cancel</button>
    ${saved ? '<button class="btn btn--secondary" type="button" data-modal-forget>Forget</button>' : ''}`;
  form.appendChild(actions);
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
    if (!chosenSsid.trim()) {
      msg.textContent = 'Wi-Fi network name is required.';
      msg.className = 'wifi-join-message message message--error';
      msg.hidden = false;
      return;
    }
    const button = form.querySelector('button[type="submit"]');
    const old = button.textContent;
    button.disabled = true;
    button.textContent = 'Connecting…';
    steps.hidden = false;
    msg.hidden = true;
    animateWifiJoinSteps(steps);
    try {
      const data = await postJson('/api/network/wifi/connect', { ssid: chosenSsid, password });
      if (passwordInput) passwordInput.value = '';
      steps.querySelectorAll('.wifi-join-step').forEach((step) => {
        step.classList.remove('is-active');
        step.classList.add('is-done');
      });
      if (data.ok) {
        msg.hidden = true;
        PopupManager.showToast(data.message || 'Wi-Fi connect complete.', 'success');
        window.setTimeout(() => PopupManager.closeModal(), 700);
      } else {
        msg.textContent = data.message || 'Wi-Fi connect failed.';
        msg.className = 'wifi-join-message message message--error';
        msg.hidden = false;
        PopupManager.showToast(data.message || 'Wi-Fi connect failed.', 'error');
      }
    } catch (_) {
      if (passwordInput) passwordInput.value = '';
      steps.hidden = true;
      msg.textContent = 'Wi-Fi connect request failed.';
      msg.className = 'wifi-join-message message message--error';
      msg.hidden = false;
    } finally {
      button.disabled = false;
      button.textContent = old;
    }
  });
  PopupManager.showModal({ title: ssid ? 'Connect' : 'Hidden network', body: form, hideDefaultAction: true, surfaceId: ssid ? 'modal:wifi-connect' : 'modal:wifi-hidden' });
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
      ['Link', e.connected ? (e.speedMbps ? `${e.speedMbps} Mbps negotiated` : 'Link up') : 'No cable'],
      ['IP address', e.ip || a.ip || 'Unavailable'],
      ['MAC address', e.macAddress || 'Unknown'],
      ['Mode', e.dhcp ? 'DHCP' : 'Manual'],
      ['Gateway', e.gateway || a.gateway || 'Unknown'],
      ['DNS servers', (e.dnsServers || a.dnsServers || []).join(', ') || 'Unknown'],
      ['Hostname', state.appliance?.hostname || 'homeconsole'],
      ['Web console URL', state.appliance?.webOrigin || 'http://console.example.com'],
    ];
    rows.forEach(([label, value]) => body.appendChild(detailRow(label, value, label.includes('URL') || label.includes('address') || label === 'Gateway')));
    PopupManager.showModal({ title: 'Wired LAN Details', body, surfaceId: 'modal:wired-details' });
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
    <label><span>IP address</span><input class="field" name="ip" inputmode="numeric" placeholder="192.0.2.54"></label>
    <label><span>Subnet prefix</span><input class="field" name="prefixLength" inputmode="numeric" placeholder="24"></label>
    <label><span>Gateway</span><input class="field" name="gateway" inputmode="numeric" placeholder="192.0.2.1"></label>
    <label><span>DNS servers</span><input class="field" name="dnsServers" placeholder="192.0.2.1 1.1.1.1"></label>
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
    PopupManager.showModal({ title: data.ok ? 'Network settings changed' : 'Network settings not applied', body: data.message || '', surfaceId: 'modal:network-result' });
  });
  PopupManager.showModal({ title: 'IP Settings', body: form, hideDefaultAction: true, surfaceId: 'modal:ip-settings' });
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
      if (action === 'speed-test') {
        const root = document.getElementById('speed-test-results');
        const old = button.textContent;
        button.disabled = true;
        button.textContent = 'Testing…';
        if (root) {
          root.hidden = false;
          root.className = 'network-speed-result';
          root.textContent = 'Measuring download speed…';
        }
        try {
          const data = await postJson('/api/network/speed-test', {});
          if (root) {
            root.className = `network-speed-result ${data.ok ? 'network-speed-result--ok' : 'network-speed-result--error'}`;
            root.textContent = data.ok
              ? `${Number(data.downloadMbps || 0).toFixed(1)} Mbps down · ${data.durationMs || 0} ms`
              : (data.message || 'Speed test failed.');
          }
          PopupManager.showToast(data.message || 'Speed test complete.', data.ok ? 'success' : 'error');
        } catch (error) {
          if (root) {
            root.className = 'network-speed-result network-speed-result--error';
            root.textContent = 'Speed test failed.';
          }
          PopupManager.showToast('Speed test failed.', 'error');
        } finally {
          button.disabled = false;
          button.textContent = old;
        }
        return;
      }
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
      PopupManager.showModal({ title: 'Local AI Logs', body: [a.runtimeUpdateLog, a.inferenceServerLog].filter(Boolean).join('\n\n') || 'No Local AI logs reported.', surfaceId: 'modal:local-ai-logs' });
    } catch (_) { PopupManager.showToast('Local AI logs unavailable', 'error'); }
  }));
}

function localAIPortPayload() {
  const form = document.getElementById('ai-lan-form');
  const rawPort = form?.querySelector('input[name="port"]')?.value.trim() || '';
  if (!/^\d+$/.test(rawPort)) throw new Error('Port must be a whole number.');
  const port = Number(rawPort);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Port must be between 1024 and 65535.');
  if ([22, 80, 443, 445, 8080].includes(port)) throw new Error('That port is reserved for HomeConsole services.');
  return { port };
}

async function saveLocalAIPort() {
  let payload;
  try { payload = localAIPortPayload(); }
  catch (error) {
    setMessage('ai-message', error.message, 'error');
    PopupManager.showToast(error.message, 'error');
    return null;
  }
  return await postAI('/api/ai/settings', { lanPort: payload.port }, 'Local AI port saved');
}

async function applyLocalAIPort(enableLan) {
  let payload;
  try { payload = localAIPortPayload(); }
  catch (error) {
    setMessage('ai-message', error.message, 'error');
    PopupManager.showToast(error.message, 'error');
    return null;
  }
  return await postAI('/api/ai/inference/set-lan-access', { enabled: Boolean(enableLan), port: payload.port }, 'LAN access applied');
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

function formatTransferRate(bytesPerSec) {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return '0 B/s';
  if (bytesPerSec < 1024) return `${Math.round(bytesPerSec)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${Math.round(bytesPerSec / 1024)} KB/s`;
  const mb = bytesPerSec / 1024 / 1024;
  return mb >= 10 ? `${Math.round(mb)} MB/s` : `${mb.toFixed(1)} MB/s`;
}

async function postJson(url, body, { attendance = true } = {}) {
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      accept: 'application/json',
      ...(attendance ? caduceusAttendanceHeaders() : { 'x-caduceus-document': caduceusDocument }),
    },
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
  PopupManager.showModal({ title: 'Scraper API Keys', body, hideDefaultAction: true, surfaceId: 'modal:scraper-keys' });
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
  document.querySelectorAll('[data-harmonia-ledger-open]').forEach((button) => button.addEventListener('click', openHarmoniaLedger));
  document.addEventListener('change', (event) => {
    const input = event.target.closest?.('[data-harmonia-module-switch]');
    if (!input) return;
    toggleHarmoniaModule(input.dataset.harmoniaModuleSwitch, input.checked, null);
  });
}

function bindSystemTrustAndAccessForms() {
  const trustPanel = document.querySelector('.system-ca-panel');
  if (trustPanel && !trustPanel.querySelector('[data-household-trust]')) {
    const card = document.createElement('section');
    card.className = 'household-trust-card';
    card.dataset.householdTrust = 'true';
    card.setAttribute('aria-label', 'Household trust');
    card.innerHTML = `
      <div class="household-trust-card__head"><span>Household trust</span><b class="system-status system-status--unknown" data-household-trust-state>Checking</b></div>
      <p class="household-trust-card__copy">Fetch the HomeServer certificate bundle and install it on this console.</p>
      <div class="household-trust-card__action">
        <label for="household-trust-server">HomeServer address</label>
        <div class="household-trust-card__controls">
          <input class="field" id="household-trust-server" data-household-trust-server inputmode="numeric" autocomplete="off" placeholder="HomeServer IP address">
          <button class="btn btn--primary" type="button" data-household-trust-fetch>Fetch &amp; Install</button>
        </div>
      </div>
      <div class="household-trust-card__fields">
        <span class="system-field"><em>CA bundle</em><strong data-household-trust-installed>—</strong></span>
        <span class="system-field system-field--anchor"><em>Fingerprint</em><strong data-household-trust-fingerprint>—</strong></span>
        <span class="system-field"><em>Role</em><strong data-household-trust-role>—</strong></span>
      </div>
      <p class="household-trust-card__error" data-household-trust-error hidden></p>`;
    const rootCaFallback = trustPanel.querySelector('.system-manual-install');
    trustPanel.insertBefore(card, rootCaFallback || null);

    const field = (name) => card.querySelector(`[data-household-trust-${name}]`);
    const receipt = (data, fallback) => data.first_missing_signal || data.firstMissingSignal || data.refusal_reason || data.refusalReason || data.message || fallback;
    const fingerprint = (data) => data.ca_fingerprint || data.fingerprint || data.bundle_fingerprint || '—';
    const presentReceipt = (data) => {
      const error = field('error');
      if (data.ok === false) {
        error.textContent = receipt(data, 'HomeServer request was refused');
        error.hidden = false;
        return;
      }
      const shownFingerprint = fingerprint(data);
      field('installed').textContent = 'Installed';
      field('state').textContent = 'Installed';
      field('state').className = 'system-status system-status--available';
      if (shownFingerprint !== '—') field('fingerprint').textContent = String(shownFingerprint).slice(0, 20);
      error.hidden = true;
    };
    const refresh = async () => {
      let data;
      try {
        data = await getJson('/api/caduceus/v1/cert/status');
      } catch (_) {
        data = { ok: false, first_missing_signal: 'caduceus-http-unreachable' };
      }
      const error = field('error');
      const state = field('state');
      if (data.ok === false) {
        field('installed').textContent = 'Unavailable';
        field('fingerprint').textContent = '—';
        field('role').textContent = data.role || data.profile || '—';
        state.textContent = 'Unavailable';
        state.className = 'system-status system-status--error';
        error.textContent = receipt(data, 'Caduceus is unavailable');
        error.hidden = false;
        return data;
      }
      const installed = data.bundle_installed === true;
      field('installed').textContent = installed ? 'Installed' : 'Not installed';
      field('fingerprint').textContent = String(fingerprint(data)).slice(0, 20);
      field('role').textContent = data.role || data.profile || '—';
      state.textContent = installed ? 'Installed' : 'Not installed';
      state.className = `system-status system-status--${installed ? 'available' : 'unknown'}`;
      error.hidden = true;
      return data;
    };
    const prefillGateway = async () => {
      try {
        const network = await requestNetworkState();
        const gateway = network.activeConnection?.gateway;
        if (gateway) field('server').value = gateway;
      } catch (_) {}
    };

    card.querySelector('[data-household-trust-fetch]').addEventListener('click', async (event) => {
      const button = event.currentTarget;
      const server = field('server').value.trim();
      if (!server) {
        field('error').textContent = 'Enter the HomeServer address.';
        field('error').hidden = false;
        field('server').focus();
        return;
      }
      const label = button.textContent;
      button.disabled = true;
      button.textContent = 'Fetching…';
      let response;
      try {
        response = await postJson('/api/caduceus/v1/cert/trust-fetch', { server });
        presentReceipt(response);
        PopupManager.showToast(response.ok ? `Installed${fingerprint(response) === '—' ? '' : ` · ${fingerprint(response)}`}` : receipt(response, 'HomeServer request was refused'), response.ok ? 'success' : 'error');
      } catch (_) {
        field('error').textContent = 'HomeServer request failed.';
        field('error').hidden = false;
        PopupManager.showToast('HomeServer request failed', 'error');
      } finally {
        button.disabled = false;
        button.textContent = label;
        await refresh();
        if (response?.ok) presentReceipt(response);
      }
    });
    prefillGateway();
    refresh();
  }

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
      const file = caForm.querySelector('[name="ca_bundle_file"]')?.files?.[0] || null;
      let ca = caForm.querySelector('[name="ca_bundle"]')?.value || '';
      if (file && !ca.trim()) {
        ca = await file.text();
      }
      if (!ca.includes('-----BEGIN CERTIFICATE-----') || !ca.includes('-----END CERTIFICATE-----')) {
        return setMessage('root-ca-message', 'Upload or paste a PEM certificate bundle.', 'error');
      }
      if (!window.confirm('Install this HTTPS bundle as the active Arcadia trust bundle?')) return;
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
bindVaultUnlockForm('vault-unlock-form', 'vault-auth-error');
bindVaultUnlockForm('vault-settings-unlock-form', 'vault-settings-unlock-message');
bindVaultAccess();
bindGuiPinChange();
bindGuiPinResetDefault();
bindNetworkControls();
bindSystemTrustAndAccessForms();
bindLocalAIControls();
initializeOnboarding();
initializeGuiPinGate();
initializeVaultGate();
window.addEventListener('pagehide', invalidateCaduceusAttendance);
ArcadiaObservation.runtime('boot', 'shell');
ArcadiaObservation.currentness('shell-view', 'current');
ArcadiaObservation.runtime('ready', 'shell');

document.addEventListener('click', (event) => {
  const close = event.target.closest('#modal-close, [data-action="modal-ok"]');
  if (close) return PopupManager.closeModal();
  if (event.target.id === 'modal-overlay') return PopupManager.closeModal();

  const button = event.target.closest('.btn[data-modal-title]');
  if (!button) return;
  event.stopPropagation();
  const declaredId = button.dataset.observationAction;
  if (!declaredId || !/^[a-z][a-z0-9:_-]{0,79}$/i.test(declaredId)) return;
  PopupManager.showModal({ title: button.dataset.modalTitle, body: button.dataset.modalBody, surfaceId: `modal:declared:${declaredId}` });
});

document.addEventListener('keydown', (event) => {
  if (PopupManager.trapFocus(event)) return;
  if (event.key === 'Escape') PopupManager.closeModal();
});
