/* Arcadia's one bounded observation owner. It reflects application state; it never controls it. */
(() => {
  const KINDS = new Set(['runtime', 'navigation', 'interaction', 'request', 'ui']);
  const BLOCKED = /pass(word)?|token|secret|credential|authorization|cookie|pin|capabilit|body|html|text|title|label|url|localstorage/i;
  const MAX_BYTES = 14 * 1024;
  const MAX_RING = 32;
  const nativeFetch = window.fetch ? window.fetch.bind(window) : null;

  function selection() {
    const params = new URLSearchParams(window.location?.search || '');
    // A URL value is decisive: ?debug=false and ?debug=0 cannot be revived by storage.
    if (params.has('debug')) {
      const value = String(params.get('debug') || '').trim().toLowerCase();
      if (!value || value === '0' || value === 'false') return new Set();
      if (value === '1' || value === 'true') return new Set(KINDS);
      return new Set(value.split(',').map((item) => item.trim()).filter((item) => KINDS.has(item)));
    }
    const selected = new Set();
    try {
      for (const key of ['arcadiaDebug', 'arcadiaDebugKinds']) {
        const value = String(localStorage.getItem(key) || '').trim().toLowerCase();
        if (value === '1' || value === 'true') KINDS.forEach((kind) => selected.add(kind));
        else value.split(',').map((item) => item.trim()).filter((item) => KINDS.has(item)).forEach((kind) => selected.add(kind));
      }
    } catch (_) {}
    return selected;
  }

  function pathname(value) {
    try { return new URL(String(value), window.location?.origin || 'http://arcadia.local').pathname; } catch (_) { return '/'; }
  }

  function redact(value, depth = 0) {
    if (depth > 3) return '[bounded]';
    if (value === null || value === undefined) return null;
    if (typeof value === 'string') return value.length > 160 ? `${value.slice(0, 160)}…` : value;
    if (typeof value === 'number' || typeof value === 'boolean') return value;
    if (value instanceof URL) return pathname(value);
    if (typeof Node !== 'undefined' && value instanceof Node) return '[dom-elided]';
    if (Array.isArray(value)) return value.slice(0, 12).map((item) => redact(item, depth + 1));
    if (typeof value === 'object') {
      const output = {};
      Object.entries(value).slice(0, 24).forEach(([key, item]) => {
        if (!BLOCKED.test(key)) output[key] = redact(item, depth + 1);
      });
      return output;
    }
    return String(value).slice(0, 160);
  }

  function bounded(event) {
    const safe = redact(event);
    let encoded = JSON.stringify(safe);
    if (encoded.length <= MAX_BYTES) return safe;
    safe.attributes = { bounded: true };
    encoded = JSON.stringify(safe);
    return encoded.length <= MAX_BYTES ? safe : { kind: safe.kind, event: 'bounded', outcome: 'dropped', observed_at: safe.observed_at, attributes: { bounded: true } };
  }

  function create() {
    const enabledKinds = selection();
    if (!enabledKinds.size) {
      const inert = { enabled: () => false, emit: () => false, begin: () => null, mark: () => false, settle: () => false, child: () => inert, dispose: () => {} };
      return inert;
    }

    let sequence = 0;
    let terminal = new Set();
    let ring = [];
    let disposed = false;
    let flushing = false;
    const resources = new Set();
    const dropped = new Map();
    const correlation = `arcadia-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

    function enabled(kind) { return !disposed && enabledKinds.has(kind); }
    function noteDrop(source) {
      const count = (dropped.get(source) || 0) + 1;
      dropped.set(source, count);
      if (count === 1 || count % 16 === 0) enqueue({ kind: 'runtime', event: 'coalesced', outcome: 'dropped', attributes: { source_class: source, count } });
    }
    function enqueue(input) {
      if (!enabled(input.kind)) return false;
      const event = bounded({
        kind: input.kind,
        event: input.event || 'observed',
        outcome: input.outcome || 'observed',
        observed_at: new Date().toISOString(),
        correlation_id: input.correlation_id || correlation,
        span_id: input.span_id || `s${++sequence}`,
        parent_span_id: input.parent_span_id || undefined,
        sequence,
        duration_ms: Number.isFinite(input.duration_ms) ? Math.max(0, Math.round(input.duration_ms)) : undefined,
        shell_view: input.shell_view || document.body?.dataset?.activeView || undefined,
        surface_id: input.surface_id,
        surface_class: input.surface_class,
        action_id: input.action_id,
        method: input.method,
        pathname: input.pathname ? pathname(input.pathname) : undefined,
        status: input.status,
        currentness: input.currentness,
        attributes: input.attributes || {},
      });
      if (ring.length >= MAX_RING) { ring.shift(); noteDrop('buffer'); }
      ring.push(event);
      flush();
      return event;
    }
    function flush() {
      if (flushing || !ring.length || !nativeFetch || disposed) return;
      flushing = true;
      while (ring.length) {
        const event = ring.shift();
        try { Promise.resolve(nativeFetch('/api/debug/emit', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(event), keepalive: true })).catch(() => {}); } catch (_) {}
      }
      flushing = false;
    }
    function begin(kind, input = {}, parent = null) {
      if (!enabled(kind)) return null;
      const started = typeof performance !== 'undefined' && performance.now ? performance.now() : Date.now();
      const event = enqueue({ kind, event: input.event || 'begin', outcome: 'begin', ...input, parent_span_id: parent?.span_id });
      return event ? { kind, span_id: event.span_id, correlation_id: event.correlation_id, started, terminal: false } : null;
    }
    function mark(handle, input = {}) {
      return handle && !handle.terminal && enqueue({ kind: handle.kind, event: input.event || 'mark', outcome: input.outcome || 'observed', ...input, correlation_id: handle.correlation_id, parent_span_id: handle.span_id });
    }
    function settle(handle, input = {}) {
      if (!handle || handle.terminal || terminal.has(handle.span_id)) return false;
      handle.terminal = true; terminal.add(handle.span_id);
      const now = typeof performance !== 'undefined' && performance.now ? performance.now() : Date.now();
      return enqueue({ kind: handle.kind, event: input.event || 'settled', outcome: input.outcome || 'settled', ...input, correlation_id: handle.correlation_id, parent_span_id: handle.span_id, duration_ms: now - handle.started });
    }
    function child(kind, input = {}, parent = null) { return begin(kind, input, parent); }
    function emit(kind, payload = {}, correlationId = '') {
      const event = enqueue({ kind, event: payload.phase || 'observed', outcome: payload.outcome || payload.phase || 'observed', attributes: payload, correlation_id: correlationId || undefined });
      return event ? event.correlation_id : false;
    }
    function own(resource) { if (resource) resources.add(resource); return resource; }
    function dispose() {
      disposed = true; ring.length = 0;
      resources.forEach((resource) => { try { resource.disconnect?.(); resource.close?.(); clearTimeout(resource); clearInterval(resource); } catch (_) {} });
      resources.clear(); terminal.clear(); dropped.clear();
    }
    return { enabled, emit, begin, mark, settle, child, own, noteDrop, dispose };
  }

  const reflection = create();
  const adapters = {
    request(input, init) {
      const path = typeof input === 'string' ? input : input?.url;
      if (pathname(path) === '/api/debug/emit' || !nativeFetch) return nativeFetch(input, init);
      const method = (init?.method || input?.method || 'GET').toUpperCase();
      const handle = reflection.begin('request', { event: 'begin', method, pathname: path });
      return nativeFetch(input, init).then((response) => { reflection.settle(handle, { event: 'response', outcome: response.ok ? 'ok' : 'refused', method, pathname: path, status: response.status }); return response; }, (error) => { reflection.settle(handle, { event: 'fault', outcome: 'fault', method, pathname: path, attributes: { error: error?.name || 'request-fault' } }); throw error; });
    },
    navigation(view, outcome = 'current') { return reflection.emit('navigation', { event: 'view-change', outcome, view, attributes: { viewport_id: view } }); },
    presenter(surfaceId, surfaceClass, outcome = 'opened') { return reflection.emit('ui', { event: 'presenter', outcome, surface_id: surfaceId, surface_class: surfaceClass }); },
    action(actionId, outcome = 'invoked') { return reflection.emit('interaction', { event: 'action', outcome, action_id: actionId }); },
    currentness(sourceClass, freshness) { return reflection.emit('ui', { event: 'currentness', outcome: freshness, currentness: freshness, attributes: { source_class: sourceClass } }); },
    stream(sourceClass, outcome = 'observed') { return reflection.emit('runtime', { event: 'stream', outcome, attributes: { source_class: sourceClass } }); },
    reflection,
  };
  window.IndraObservationReflection = reflection;
  window.ArcadiaObservationAdapters = adapters;
  // Existing consumers receive the same owner, never a parallel lifecycle or buffer.
  window.arcadiaDebug = reflection;
})();
