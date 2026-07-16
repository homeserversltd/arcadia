/* Arcadia's one bounded observation owner. It reflects application state; it never controls it. */
(() => {
  const KINDS = new Set(['runtime', 'navigation', 'interaction', 'request', 'ui']);
  const MAX_BYTES = 14 * 1024;
  const MAX_QUEUE = 32;
  const MAX_IN_FLIGHT = 2;
  const BLOCKED_KEY = /pass(word)?|token|secret|credential|authorization|cookie|pin|capabilit|body|html|text|title|label|url|localstorage/i;
  const SAFE_ID = /^[a-z][a-z0-9:_-]{0,79}$/i;
  // Each declared governor field is enforced in allowSource.
  const POLICIES = Object.freeze({
    'controller-trainer-sse': { max_events: 8, max_unique_fingerprints: 8, lifetime_ms: 30000, coalesce_unchanged: true },
    'home-root-sse': { max_events: 8, max_unique_fingerprints: 8, lifetime_ms: 30000, coalesce_unchanged: true },
    'status-refresh': { max_events: 4, max_unique_fingerprints: 4, lifetime_ms: 30000, coalesce_unchanged: true },
  });
  // This is a source contract: the census test independently reproduces every total.
  const SOURCE_REGISTRY = Object.freeze({
    listeners: Object.freeze({ interactive_action: 90, lifecycle_observer: 8, projection: 2, flow_non_action: 2, total: 102 }),
    requests: Object.freeze({ ordinary_fetch: 9 }),
    streams: Object.freeze({ 'controller-trainer-sse': 1, 'home-root-sse': 1 }),
    presenters: Object.freeze({ callers: 12, declared_requires_data_observation_action: true }),
    runtime: Object.freeze(['shell', 'controller-trainer-sse', 'home-root-sse', 'status-refresh']),
    currentness: Object.freeze(['shell-view', 'controller-trainer-sse', 'home-root-sse', 'status-refresh']),
  });
  const nativeFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;

  function utf8Bytes(value) {
    return new TextEncoder().encode(value).byteLength;
  }

  function safePathname(value) {
    try {
      const url = new URL(String(value), window.location?.origin || 'http://arcadia.local');
      return url.pathname;
    } catch (_) {
      return '/';
    }
  }

  function selectKinds() {
    const params = new URLSearchParams(window.location?.search || '');
    if (params.has('debug')) {
      const value = String(params.get('debug') || '').trim().toLowerCase();
      if (value === '0' || value === 'false') return new Set();
      if (value === '' || value === '1' || value === 'true') return new Set(KINDS);
      return new Set(value.split(',').map((item) => item.trim()).filter((item) => KINDS.has(item)));
    }
    const selected = new Set();
    try {
      for (const key of ['arcadiaDebug', 'arcadiaDebugKinds']) {
        const value = String(localStorage.getItem(key) || '').trim().toLowerCase();
        if (value === '1' || value === 'true') KINDS.forEach((kind) => selected.add(kind));
        for (const item of value.split(',')) if (KINDS.has(item.trim())) selected.add(item.trim());
      }
    } catch (_) {
      // Storage is optional and observation remains inert if it is unavailable.
    }
    return selected;
  }

  function redact(value, depth = 0) {
    if (depth > 3) return '[bounded]';
    if (value === null || value === undefined) return null;
    if (typeof value === 'string') return value.slice(0, 160);
    if (typeof value === 'number' || typeof value === 'boolean') return value;
    if (value instanceof URL) return safePathname(value);
    if (typeof Node !== 'undefined' && value instanceof Node) return '[dom-elided]';
    if (Array.isArray(value)) return value.slice(0, 12).map((item) => redact(item, depth + 1));
    if (typeof value === 'object') {
      const output = {};
      for (const [key, item] of Object.entries(value).slice(0, 24)) {
        if (!BLOCKED_KEY.test(key)) output[key] = redact(item, depth + 1);
      }
      return output;
    }
    return String(value).slice(0, 160);
  }

  function boundedEnvelope(event) {
    const safe = redact(event);
    if (utf8Bytes(JSON.stringify(safe)) <= MAX_BYTES) return safe;
    safe.attributes = { bounded: true };
    if (utf8Bytes(JSON.stringify(safe)) <= MAX_BYTES) return safe;
    return { kind: safe.kind, event: 'bounded', outcome: 'dropped', observed_at: safe.observed_at, attributes: { bounded: true } };
  }

  function stableActionId(target) {
    for (let node = target; node && node !== document; node = node.parentElement) {
      const value = node.dataset?.observationAction || node.dataset?.action || node.id || node.getAttribute?.('name');
      if (value && SAFE_ID.test(value)) return value;
    }
    return '';
  }

  function createReflection() {
    const enabledKinds = selectKinds();
    if (!enabledKinds.size) {
      return Object.freeze({ enabled: () => false, emit: () => false, begin: () => null, mark: () => false, settle: () => false, child: () => null, own: () => null, dispose: () => {}, census: () => SOURCE_REGISTRY, policy: () => POLICIES, resources: () => ({ listeners: 0, timers: 0, observers: 0, queue: 0, in_flight: 0 }) });
    }

    let sequence = 0;
    let disposed = false;
    let inFlight = 0;
    let flushing = false;
    const queue = [];
    const terminal = new Set();
    const cleanup = new Set();
    const drops = new Map();
    const sourceState = new Map();
    const correlationId = `arcadia-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const allocation = { listeners: 0, timers: 0, observers: 0 };

    function enabled(kind) {
      return !disposed && enabledKinds.has(kind);
    }

    function noteDrop(source, reason) {
      const count = (drops.get(source) || 0) + 1;
      drops.set(source, count);
      if (count === 1 || count % 16 === 0) {
        enqueue({ kind: 'runtime', event: 'coalesced', outcome: 'dropped', attributes: { source_class: source, reason, count } }, true);
      }
    }

    function allowSource(input, summary) {
      const source = input.attributes?.source_class;
      const policy = source && POLICIES[source];
      if (!policy || summary) return true;
      const now = Date.now();
      const fingerprint = `${input.event || 'observed'}:${input.outcome || 'observed'}:${input.currentness || ''}:${input.status || ''}`;
      let state = sourceState.get(source);
      if (!state || now - state.started >= policy.lifetime_ms) {
        state = { started: now, count: 0, fingerprints: new Set(), last: '' };
        sourceState.set(source, state);
      }
      if (policy.coalesce_unchanged && state.last === fingerprint) {
        noteDrop(source, 'unchanged');
        return false;
      }
      if (state.count >= policy.max_events || (!state.fingerprints.has(fingerprint) && state.fingerprints.size >= policy.max_unique_fingerprints)) {
        noteDrop(source, 'policy');
        return false;
      }
      state.count += 1;
      state.last = fingerprint;
      state.fingerprints.add(fingerprint);
      return true;
    }

    function enqueue(input, summary = false) {
      if (!enabled(input.kind) || !allowSource(input, summary)) return false;
      const event = boundedEnvelope({
        kind: input.kind,
        event: input.event || 'observed',
        outcome: input.outcome || 'observed',
        observed_at: new Date().toISOString(),
        correlation_id: input.correlation_id || correlationId,
        span_id: input.span_id || `s${++sequence}`,
        parent_span_id: input.parent_span_id,
        sequence: ++sequence,
        duration_ms: Number.isFinite(input.duration_ms) ? Math.max(0, Math.round(input.duration_ms)) : undefined,
        shell_view: document.body?.dataset?.activeView,
        surface_id: input.surface_id,
        surface_class: input.surface_class,
        action_id: input.action_id,
        method: input.method,
        pathname: input.pathname ? safePathname(input.pathname) : undefined,
        status: input.status,
        currentness: input.currentness,
        attributes: input.attributes || {},
      });
      if (queue.length >= MAX_QUEUE) {
        queue.shift();
        noteDrop(input.attributes?.source_class || 'buffer', 'buffer');
      }
      queue.push(event);
      flush();
      return event;
    }

    function flush() {
      if (flushing || disposed || !nativeFetch) return;
      flushing = true;
      while (!disposed && queue.length && inFlight < MAX_IN_FLIGHT) {
        const event = queue.shift();
        inFlight += 1;
        let delivery;
        try {
          delivery = nativeFetch('/api/debug/emit', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(event), keepalive: true });
        } catch (_) {
          delivery = Promise.reject(_);
        }
        Promise.resolve(delivery).then((response) => {
          if (!response?.ok) noteDrop('delivery', 'refused');
        }, () => noteDrop('delivery', 'fault')).finally(() => {
          inFlight -= 1;
          flushing = false;
          flush();
        });
      }
      flushing = false;
    }

    function begin(kind, input = {}, parent = null) {
      if (!enabled(kind)) return null;
      const started = performance?.now ? performance.now() : Date.now();
      const spanId = `s${++sequence}`;
      const parentSpanId = parent?.span_id || input.parent_span_id;
      const event = enqueue({ ...input, kind, event: input.event || 'begin', outcome: 'begin', span_id: spanId, parent_span_id: parentSpanId, correlation_id: parent?.correlation_id || correlationId });
      return event ? { kind, span_id: spanId, parent_span_id: parentSpanId, correlation_id: event.correlation_id, started, terminal: false } : null;
    }

    function mark(handle, input = {}) {
      if (!handle || handle.terminal) return false;
      return enqueue({ ...input, kind: handle.kind, event: input.event || 'mark', outcome: input.outcome || 'observed', span_id: handle.span_id, parent_span_id: handle.parent_span_id, correlation_id: handle.correlation_id });
    }

    function settle(handle, input = {}) {
      if (!handle || handle.terminal || terminal.has(handle.span_id)) return false;
      handle.terminal = true;
      terminal.add(handle.span_id);
      const now = performance?.now ? performance.now() : Date.now();
      return enqueue({ ...input, kind: handle.kind, event: input.event || 'settled', outcome: input.outcome || 'settled', span_id: handle.span_id, parent_span_id: handle.parent_span_id, correlation_id: handle.correlation_id, duration_ms: now - handle.started });
    }

    function emit(kind, payload = {}, correlation = '') {
      const event = enqueue({ ...payload, kind, event: payload.phase || payload.event || 'observed', outcome: payload.outcome || payload.phase || 'observed', attributes: payload.attributes || payload, correlation_id: correlation || undefined });
      return event ? event.correlation_id : false;
    }

    function installListener(target, type, listener, options) {
      target.addEventListener(type, listener, options);
      allocation.listeners += 1;
      cleanup.add(() => target.removeEventListener(type, listener, options));
    }

    function installTimer(timer) {
      allocation.timers += 1;
      cleanup.add(() => clearTimeout(timer));
      return timer;
    }

    function own(resource) {
      if (!resource) return resource;
      if (typeof PerformanceObserver !== 'undefined' && resource instanceof PerformanceObserver) allocation.observers += 1;
      cleanup.add(() => { resource.disconnect?.(); resource.close?.(); clearTimeout(resource); clearInterval(resource); });
      return resource;
    }

    function observeInteraction(event) {
      const actionId = stableActionId(event.target);
      if (!actionId || event.type === 'keydown' && event.key !== 'Enter' && event.key !== ' ') return;
      emit('interaction', { event: 'action', outcome: 'invoked', action_id: actionId, attributes: { source_class: 'interaction-capture', family: event.type } });
    }

    // The action family crosses exactly this capture seam, and only when selected.
    if (enabled('interaction')) {
      installListener(document, 'click', observeInteraction, true);
      installListener(document, 'change', observeInteraction, true);
      installListener(document, 'submit', observeInteraction, true);
      installListener(document, 'keydown', observeInteraction, true);
    }
    if (enabled('runtime')) {
      installListener(window, 'error', () => emit('runtime', { event: 'runtime', outcome: 'fault', attributes: { source_class: 'shell' } }), true);
      installListener(window, 'unhandledrejection', () => emit('runtime', { event: 'runtime', outcome: 'degraded', attributes: { source_class: 'shell' } }), true);
    }

    function dispose() {
      if (disposed) return;
      disposed = true;
      queue.length = 0;
      for (const release of cleanup) { try { release(); } catch (_) {} }
      cleanup.clear();
      terminal.clear();
      sourceState.clear();
      drops.clear();
    }

    return Object.freeze({ enabled, emit, begin, mark, settle, child: begin, own, installTimer, dispose, census: () => SOURCE_REGISTRY, policy: () => POLICIES, resources: () => ({ ...allocation, queue: queue.length, in_flight: inFlight }) });
  }

  const reflection = createReflection();
  const adapters = Object.freeze({
    request(input, init) {
      const path = typeof input === 'string' ? input : input?.url;
      if (safePathname(path) === '/api/debug/emit' || !nativeFetch) return nativeFetch?.(input, init);
      const method = (init?.method || input?.method || 'GET').toUpperCase();
      const handle = reflection.begin('request', { event: 'begin', method, pathname: path });
      return nativeFetch(input, init).then((response) => {
        reflection.settle(handle, { event: 'response', outcome: response.ok ? 'ok' : 'refused', method, pathname: path, status: response.status });
        return response;
      }, (error) => {
        reflection.settle(handle, { event: 'fault', outcome: 'fault', method, pathname: path, attributes: { error: error?.name || 'request-fault' } });
        throw error;
      });
    },
    navigation(view, outcome = 'current') { return reflection.emit('navigation', { event: 'view-change', outcome, attributes: { viewport_id: view } }); },
    presenter(surfaceId, surfaceClass, outcome = 'opened', parentSurfaceId = '') { return reflection.emit('ui', { event: 'presenter', outcome, surface_id: surfaceId, surface_class: surfaceClass, attributes: parentSurfaceId ? { parent_surface_id: parentSurfaceId } : {} }); },
    action(actionId, outcome = 'invoked') { return reflection.emit('interaction', { event: 'action', outcome, action_id: actionId }); },
    currentness(sourceClass, freshness) { return reflection.emit('ui', { event: 'currentness', outcome: freshness, currentness: freshness, attributes: { source_class: sourceClass } }); },
    runtime(outcome, sourceClass = 'shell') { return reflection.emit('runtime', { event: 'runtime', outcome, attributes: { source_class: sourceClass } }); },
    stream(sourceClass, outcome = 'observed') { return reflection.emit('runtime', { event: 'stream', outcome, attributes: { source_class: sourceClass } }); },
    reflection,
  });

  window.IndraObservationReflection = reflection;
  window.ArcadiaObservationAdapters = adapters;
  window.arcadiaDebug = reflection;
})();
