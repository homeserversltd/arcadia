/* Arcadia's one bounded observation owner. It reflects application state; it never controls it. */
(() => {
  const KINDS = new Set(['runtime', 'navigation', 'interaction', 'request', 'ui']);
  const BLOCKED = /pass(word)?|token|secret|credential|authorization|cookie|pin|capabilit|body|html|text|title|label|url|localstorage/i;
  const MAX_BYTES = 14 * 1024;
  const MAX_QUEUE = 32;
  const MAX_IN_FLIGHT = 2;
  const POLICIES = Object.freeze({
    'controller-trainer-sse': { rate: 8, cardinality: 8, lifetime_ms: 30000, coalesce: true },
    'home-root-sse': { rate: 8, cardinality: 8, lifetime_ms: 30000, coalesce: true },
    'status-refresh': { rate: 4, cardinality: 4, lifetime_ms: 30000, coalesce: true },
  });
  const nativeFetch = window.fetch ? window.fetch.bind(window) : null;
  const utf8 = (value) => new TextEncoder().encode(value).byteLength;

  function selection() {
    const params = new URLSearchParams(window.location?.search || '');
    if (params.has('debug')) {
      const value = String(params.get('debug') || '').trim().toLowerCase();
      if (value === '0' || value === 'false') return new Set();
      if (!value || value === '1' || value === 'true') return new Set(KINDS);
      return new Set(value.split(',').map((item) => item.trim()).filter((item) => KINDS.has(item)));
    }
    const selected = new Set();
    try { for (const key of ['arcadiaDebug', 'arcadiaDebugKinds']) {
      const value = String(localStorage.getItem(key) || '').trim().toLowerCase();
      if (value === '1' || value === 'true') KINDS.forEach((kind) => selected.add(kind));
      else value.split(',').map((item) => item.trim()).filter((item) => KINDS.has(item)).forEach((kind) => selected.add(kind));
    }} catch (_) {}
    return selected;
  }
  function pathname(value) { try { return new URL(String(value), window.location?.origin || 'http://arcadia.local').pathname; } catch (_) { return '/'; } }
  function redact(value, depth = 0) {
    if (depth > 3) return '[bounded]';
    if (value === null || value === undefined) return null;
    if (typeof value === 'string') return value.length > 160 ? `${value.slice(0, 160)}…` : value;
    if (typeof value === 'number' || typeof value === 'boolean') return value;
    if (value instanceof URL) return pathname(value);
    if (typeof Node !== 'undefined' && value instanceof Node) return '[dom-elided]';
    if (Array.isArray(value)) return value.slice(0, 12).map((item) => redact(item, depth + 1));
    if (typeof value === 'object') { const output = {}; Object.entries(value).slice(0, 24).forEach(([key, item]) => { if (!BLOCKED.test(key)) output[key] = redact(item, depth + 1); }); return output; }
    return String(value).slice(0, 160);
  }
  function bounded(event) {
    const safe = redact(event); let rendered = JSON.stringify(safe);
    if (utf8(rendered) <= MAX_BYTES) return safe;
    safe.attributes = { bounded: true }; rendered = JSON.stringify(safe);
    return utf8(rendered) <= MAX_BYTES ? safe : { kind: safe.kind, event: 'bounded', outcome: 'dropped', observed_at: safe.observed_at, attributes: { bounded: true } };
  }
  function create() {
    const enabledKinds = selection();
    if (!enabledKinds.size) { const inert = { enabled: () => false, emit: () => false, begin: () => null, mark: () => false, settle: () => false, child: () => null, own: () => null, dispose: () => {} }; return inert; }
    let sequence = 0, disposed = false, flushing = false, inFlight = 0;
    const queue = [], terminal = new Set(), resources = new Set(), drops = new Map(), lastBySource = new Map();
    const correlation = `arcadia-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const enabled = (kind) => !disposed && enabledKinds.has(kind);
    function noteDrop(source, reason = 'buffer') { const count = (drops.get(source) || 0) + 1; drops.set(source, count); if (count === 1 || count % 16 === 0) enqueue({ kind: 'runtime', event: 'coalesced', outcome: 'dropped', attributes: { source_class: source, reason, count } }, true); }
    function enqueue(input, summary = false) {
      if (!enabled(input.kind)) return false;
      const source = input.attributes?.source_class;
      const policy = source && POLICIES[source]; const fingerprint = source && `${input.event}:${input.outcome}:${input.currentness || ''}:${input.status || ''}`;
      if (!summary && policy && policy.coalesce && lastBySource.get(source) === fingerprint) { noteDrop(source, 'coalesced'); return false; }
      if (source) lastBySource.set(source, fingerprint);
      const event = bounded({ kind: input.kind, event: input.event || 'observed', outcome: input.outcome || 'observed', observed_at: new Date().toISOString(), correlation_id: input.correlation_id || correlation, span_id: input.span_id || `s${++sequence}`, parent_span_id: input.parent_span_id, sequence: ++sequence, duration_ms: Number.isFinite(input.duration_ms) ? Math.max(0, Math.round(input.duration_ms)) : undefined, shell_view: input.shell_view || document.body?.dataset?.activeView, surface_id: input.surface_id, surface_class: input.surface_class, action_id: input.action_id, method: input.method, pathname: input.pathname ? pathname(input.pathname) : undefined, status: input.status, currentness: input.currentness, attributes: input.attributes || {} });
      if (queue.length >= MAX_QUEUE) { queue.shift(); noteDrop(source || 'buffer', 'buffer'); }
      queue.push(event); flush(); return event;
    }
    function flush() {
      if (flushing || disposed || !nativeFetch) return; flushing = true;
      while (queue.length && inFlight < MAX_IN_FLIGHT) { const event = queue.shift(); inFlight += 1;
        let delivered; try { delivered = nativeFetch('/api/debug/emit', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(event), keepalive: true }); } catch (_) { delivered = Promise.reject(_); }
        Promise.resolve(delivered).then((response) => { if (!response?.ok) noteDrop('delivery', 'refused'); }, () => noteDrop('delivery', 'fault')).finally(() => { inFlight -= 1; flushing = false; flush(); });
      } flushing = false;
    }
    function begin(kind, input = {}, parent = null) { if (!enabled(kind)) return null; const started = performance?.now ? performance.now() : Date.now(); const span_id = `s${++sequence}`; const event = enqueue({ kind, event: input.event || 'begin', outcome: 'begin', ...input, span_id, parent_span_id: parent?.span_id, correlation_id: parent?.correlation_id || correlation }); return event ? { kind, span_id, correlation_id: event.correlation_id, started, terminal: false } : null; }
    function mark(handle, input = {}) { return handle && !handle.terminal && enqueue({ kind: handle.kind, event: input.event || 'mark', outcome: input.outcome || 'observed', ...input, span_id: handle.span_id, correlation_id: handle.correlation_id }); }
    function settle(handle, input = {}) { if (!handle || handle.terminal || terminal.has(handle.span_id)) return false; handle.terminal = true; terminal.add(handle.span_id); const now = performance?.now ? performance.now() : Date.now(); return enqueue({ kind: handle.kind, event: input.event || 'settled', outcome: input.outcome || 'settled', ...input, span_id: handle.span_id, correlation_id: handle.correlation_id, duration_ms: now - handle.started }); }
    function emit(kind, payload = {}, correlationId = '') { const event = enqueue({ kind, event: payload.phase || payload.event || 'observed', outcome: payload.outcome || payload.phase || 'observed', ...payload, attributes: payload.attributes || payload, correlation_id: correlationId || undefined }); return event ? event.correlation_id : false; }
    function own(resource) { if (resource) resources.add(resource); return resource; }
    function dispose() { disposed = true; queue.length = 0; resources.forEach((resource) => { try { resource.disconnect?.(); resource.close?.(); clearTimeout(resource); clearInterval(resource); } catch (_) {} }); resources.clear(); terminal.clear(); drops.clear(); lastBySource.clear(); }
    return { enabled, emit, begin, mark, settle, child: begin, own, noteDrop, dispose, policy: () => POLICIES };
  }
  const reflection = create();
  const adapters = {
    request(input, init) { const path = typeof input === 'string' ? input : input?.url; if (pathname(path) === '/api/debug/emit' || !nativeFetch) return nativeFetch(input, init); const method = (init?.method || input?.method || 'GET').toUpperCase(); const handle = reflection.begin('request', { event: 'begin', method, pathname: path }); return nativeFetch(input, init).then((response) => { reflection.settle(handle, { event: 'response', outcome: response.ok ? 'ok' : 'refused', method, pathname: path, status: response.status }); return response; }, (error) => { reflection.settle(handle, { event: 'fault', outcome: 'fault', method, pathname: path, attributes: { error: error?.name || 'request-fault' } }); throw error; }); },
    navigation(view, outcome = 'current') { return reflection.emit('navigation', { event: 'view-change', outcome, attributes: { viewport_id: view } }); },
    presenter(surfaceId, surfaceClass, outcome = 'opened', parentSurfaceId = '') { return reflection.emit('ui', { event: 'presenter', outcome, surface_id: surfaceId, surface_class: surfaceClass, attributes: parentSurfaceId ? { parent_surface_id: parentSurfaceId } : {} }); },
    action(actionId, outcome = 'invoked') { return reflection.emit('interaction', { event: 'action', outcome, action_id: actionId }); },
    currentness(sourceClass, freshness) { return reflection.emit('ui', { event: 'currentness', outcome: freshness, currentness: freshness, attributes: { source_class: sourceClass } }); },
    runtime(outcome, sourceClass = 'shell') { return reflection.emit('runtime', { event: 'runtime', outcome, attributes: { source_class: sourceClass } }); },
    stream(sourceClass, outcome = 'observed') { return reflection.emit('runtime', { event: 'stream', outcome, attributes: { source_class: sourceClass } }); }, reflection,
  };
  window.IndraObservationReflection = reflection; window.ArcadiaObservationAdapters = adapters; window.arcadiaDebug = reflection;
})();
