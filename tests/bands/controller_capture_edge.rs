use std::process::{Command, Stdio};

const APP_JS: &str = include_str!("../../static/app.js");

#[test]
fn controller_capture_lane_uses_edge_gate_not_level_triggering() {
    for required in [
        "function createControllerCaptureGate",
        "let previousPressed = new Set();",
        "let previousActiveAxes = new Set();",
        "if (!releaseGateOpen && pressed.size === 0 && activeAxes.size === 0)",
        "return key && !previousPressed.has(key);",
        "return key && !previousActiveAxes.has(key);",
        "const capture = captureGate.observe(merged);",
        "const input = capture.input;",
    ] {
        assert!(APP_JS.contains(required), "capture lane missing edge/gate wall: {required}");
    }

    assert!(
        !APP_JS.contains("const captureBinding = () =>"),
        "level-triggered captureBinding helper must stay removed"
    );
}

#[test]
fn controller_capture_rearms_close_gate_and_timer_starts_at_gate_open() {
    for required in [
        "const resetCaptureGate = () => {",
        "captureGate.reset();",
        "bindingStartedAt = 0;",
        "bindingTimeoutShown = false;",
        "onGateOpen: (openedAt) => {",
        "bindingStartedAt = openedAt;",
        "resetCaptureGate();\n    root.querySelectorAll('[data-controller-control], [data-controller-bind-row]')",
        "if (!input && capture.releaseGateOpen && bindingStartedAt",
    ] {
        assert!(APP_JS.contains(required), "capture lane missing rearm/timer wall: {required}");
    }

    assert!(
        !APP_JS.contains("bindingStartedAt = control ? Date.now() : 0"),
        "no-input timer must not start when the still-held press arms the binding"
    );
    assert!(APP_JS.contains("function controllerCaptureAxisInput(axis) {\n  return axis?.binding || null;"));
    assert!(!APP_JS.contains("return `axis ${axis.binding}`"), "axis capture must not prefix raw magnitude fallback");
}

#[test]
fn controller_capture_gate_script_kills_ghost_advance_sequence() {
    let helper_start = APP_JS
        .find("const CONTROLLER_BUTTON_INDEX_LABELS")
        .expect("controller input helpers start");
    let helper_end = APP_JS[helper_start..]
        .find("function hydrateControllerBindings")
        .map(|offset| helper_start + offset)
        .expect("capture helper ends before DOM hydration");
    let helper = &APP_JS[helper_start..helper_end];
    let script = format!(
        r#"
{helper}
let now = 1000;
const gateOpens = [];
const assert = (condition, message) => {{ if (!condition) throw new Error(message); }};
const press0 = {{ control: 'button 0', binding: 'button 0' }};
const press1 = {{ control: 'button 1', binding: 'button 1' }};
const axis2 = {{ control: 'axis 2', binding: 'axis 2', axisValue: 19988 }};
const leftStickY = normalizeControllerInputEvents({{ pressed: [], axes: [{{ control: 'Axis 1', binding: 'axis 1', axisValue: -27917 }}] }});
assert(leftStickY.axes[0].control === 'Axis 1', 'server generic axis label stays generic when unbound');
assert(leftStickY.axes[0].binding === 'axis 1', 'axis binding stays identity, not magnitude');
assert(leftStickY.axes[0].axisValue === -27917, 'axis magnitude rides separately');
const dpadDirections = normalizeControllerInputEvents({{
  pressed: [
    {{ control: 'D-pad Up', binding: 'hat 0 up' }},
    {{ control: 'D-pad Down', binding: 'hat 0 down' }},
    {{ control: 'D-pad Left', binding: 'hat 0 left' }},
    {{ control: 'D-pad Right', binding: 'hat 0 right' }},
  ],
  axes: []
}});
assert(new Set(dpadDirections.pressed.map((item) => item.binding)).size === 4, 'hat directions stay direction-distinct through normalization');
const axis1 = leftStickY.axes[0];
const dpadDown = dpadDirections.pressed[1];

const gate = createControllerCaptureGate({{ now: () => now, onGateOpen: (openedAt) => gateOpens.push(openedAt) }});

// Teach step A is armed while the physical button is already held: no capture until release.
gate.reset();
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === null, 'held press tail must not capture on arm');
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === null, 'level-trigger tick must not recapture');
now = 1600;
let release = gate.observe({{ pressed: [], axes: [] }});
assert(release.input === null, 'release opens the gate without capturing');
assert(release.releaseGateOpen === true, 'release gate opens only on empty tick');
assert(release.gateOpenedAt === 1600, 'gate-open time is captured for the no-input timer');
now = 1660;
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === 'button 0', 'fresh button edge captures after release');
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === null, 'held level after capture cannot ghost-advance');
assert(gate.observe({{ pressed: [press0, press1], axes: [] }}).input === 'button 1', 'new button edge remains capturable');

// Failed bind / next teach step re-arm recloses the gate; the still-held button cannot bind B.
gate.reset();
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === null, 're-arm closes gate against still-held failed-bind press');
assert(gate.observe({{ pressed: [], axes: [] }}).releaseGateOpen === true, 're-arm waits for release before reopening');
assert(gate.observe({{ pressed: [press0], axes: [] }}).input === 'button 0', 'fresh edge after re-arm release captures');

// Axis chatter present before release cannot capture; only absent-then-present after gate-open can bind.
gate.reset();
assert(gate.observe({{ pressed: [], axes: [axis2] }}).input === null, 'resting axis chatter while closed must not capture');
assert(gate.observe({{ pressed: [], axes: [axis2] }}).input === null, 'level axis chatter stays ignored');
assert(gate.observe({{ pressed: [], axes: [] }}).releaseGateOpen === true, 'axis gate opens only after active axis absence');
assert(gate.observe({{ pressed: [], axes: [axis2] }}).input === 'axis 2', 'fresh axis edge after absence captures identity');
assert(gate.observe({{ pressed: [], axes: [axis2] }}).input === null, 'same axis identity with same or changed value does not re-fire');

// Directional teach steps bind their distinct physical tuple, not a sibling.
gate.reset();
assert(gate.observe({{ pressed: [], axes: [axis1] }}).input === null, 'held stick-y edge must not capture on arm');
assert(gate.observe({{ pressed: [], axes: [] }}).releaseGateOpen === true, 'stick-y direction waits for neutral');
assert(gate.observe({{ pressed: [], axes: [axis1] }}).input === 'axis 1', 'fresh stick-y edge captures stable axis identity');
assert(gate.observe({{ pressed: [], axes: [{{ ...axis1, axisValue: -12381 }}] }}).input === null, 'same axis identity with new magnitude does not re-fire');
gate.reset();
assert(gate.observe({{ pressed: [dpadDown], axes: [] }}).input === null, 'held dpad direction must not capture on arm');
assert(gate.observe({{ pressed: [], axes: [] }}).releaseGateOpen === true, 'dpad direction waits for release');
assert(gate.observe({{ pressed: [dpadDown], axes: [] }}).input === 'hat 0 down', 'fresh dpad down captures its own hat direction');
console.log(JSON.stringify({{ ok: true, gateOpens }}));
"#
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node executes controller capture harness");

    assert!(
        output.status.success(),
        "node harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("\"ok\":true"),
        "node harness did not report success"
    );
}

#[test]
fn controller_binding_hydration_keeps_grid_row_names_and_updates_pad_labels() {
    let helper_start = APP_JS
        .find("const CONTROLLER_BUTTON_INDEX_LABELS")
        .expect("controller binding helpers start");
    let helper_end = APP_JS[helper_start..]
        .find("function hydrateControllerGamepad")
        .map(|offset| helper_start + offset)
        .expect("controller binding helpers end before alias");
    let helper = &APP_JS[helper_start..helper_end];
    let script = format!(
        r#"
{helper}
const assert = (condition, message) => {{ if (!condition) throw new Error(message); }};
class FakeNode {{
  constructor(dataset = {{}}, children = []) {{
    this.dataset = dataset;
    this.children = children;
    this.textContent = '';
    this.attributes = {{}};
    children.forEach((child) => child.parent = this);
  }}
  querySelector(selector) {{
    if (selector.includes('[data-controller-binding-label]')) {{
      const explicit = this.children.find((child) => child.dataset.controllerBindingLabel === 'true');
      if (explicit) return explicit;
    }}
    if (selector.includes('s' + 'pan')) return this.children.find((child) => child.kind === 'span') || null;
    if (selector.includes('e' + 'm')) return this.children.find((child) => child.kind === 'em') || null;
    return null;
  }}
  querySelectorAll(selector) {{
    if (selector === '[data-binding-control]') return this.children.filter((child) => child.dataset.bindingControl);
    if (selector === '[data-controller-control]') return this.children.filter((child) => child.dataset.controllerControl);
    return [];
  }}
  closest(selector) {{ return selector === '[data-controller-bind-row]' && this.parent?.dataset.controllerBindRow ? this.parent : null; }}
  setAttribute(name, value) {{ this.attributes[name] = value; }}
}}
const rowName = new FakeNode({{}}, []); rowName.kind = 'span'; rowName.textContent = 'A';
const rowValue = new FakeNode({{ bindingControl: 'A' }}, []); rowValue.kind = 'span'; rowValue.textContent = 'Unbound';
const row = new FakeNode({{ controllerControl: 'A', controllerBindRow: 'true' }}, [rowName, rowValue]);
const padLabel = new FakeNode({{ controllerBindingLabel: 'true' }}, []); padLabel.textContent = 'Unbound';
const padSlot = new FakeNode({{ controllerControl: 'A' }}, [padLabel]);
const root = new FakeNode({{}}, [row, rowValue, padSlot]);
rowValue.parent = row;
hydrateControllerBindings(root, [{{ control: 'A', binding: 'button 0' }}]);
assert(rowName.textContent === 'A', `grid row name was stomped to ${{rowName.textContent}}`);
assert(rowValue.textContent === '#1', `grid row value was not hydrated: ${{rowValue.textContent}}`);
assert(row.attributes['data-controller-binding-state'] === 'bound', 'grid row bound state was not updated');
assert(padLabel.textContent === 'B0', `pad binding label was not hydrated: ${{padLabel.textContent}}`);
console.log(JSON.stringify({{ ok: true, rowName: rowName.textContent, rowValue: rowValue.textContent, padLabel: padLabel.textContent }}));
"#
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node executes controller binding hydration harness");

    assert!(
        output.status.success(),
        "node hydration harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("\"rowName\":\"A\""),
        "node hydration harness did not preserve the canonical grid row name"
    );
}

#[test]
fn indra_observation_reflection_is_default_off_bounded_and_same_origin() {
    let indra = include_str!("../../static/indra-observation.js");
    let script = format!(
        r#"
const vm = require('vm');
const indra = {indra:?};
const assert = (condition, message) => {{ if (!condition) throw new Error(message); }};
function run(search = '', storage = {{}}) {{
  const calls = [];
  const values = new Map(Object.entries(storage));
  const context = {{
    window: {{ location: {{ search }}, fetch: (path, options) => {{ calls.push({{path, options}}); return Promise.resolve({{ok:true, status:204}}); }}, addEventListener: () => {{}}, removeEventListener: () => {{}} }},
    document: {{ body: {{ dataset: {{}} }}, addEventListener: () => {{}}, removeEventListener: () => {{}} }}, localStorage: {{ getItem: (key) => values.get(key) || null }},
    URLSearchParams, URL, Date, Set, String, Object, Array, JSON, Math, Promise, TextEncoder, performance: {{ now: () => 1 }},
    setTimeout, clearTimeout, setInterval, clearInterval,
  }};
  context.window.window = context.window;
  vm.runInNewContext(indra, context);
  return {{ reflection: context.window.IndraObservationReflection, adapters: context.window.ArcadiaObservationAdapters, calls }};
}}
let probe = run();
assert(probe.reflection.emit('runtime', {{secret:'never'}}) === false, 'default debug must be inert');
assert(probe.reflection.begin('runtime') === null, 'default debug must allocate no handle');
assert(probe.calls.length === 0, 'default debug must not fetch');
probe = run('?debug=true');
const root = probe.reflection.begin('runtime', {{event:'boot', attributes:{{token:'no', safe:'yes'}}}});
const child = probe.reflection.child('request', {{event:'begin', pathname:'https://example.test/private?a=1'}}, root);
assert(root && child, 'enabled reflection must create bounded root and child');
assert(probe.reflection.settle(root, {{outcome:'ready'}}), 'first terminal settles');
assert(probe.reflection.settle(root, {{outcome:'duplicate'}}) === false, 'terminal outcome is exactly once');
assert(probe.calls.every((call) => call.path === '/api/debug/emit'), 'only same-origin Arcadia route may receive events');
const events = probe.calls.map((call) => JSON.parse(call.options.body));
assert(events[0].kind === 'runtime' && events[1].parent_span_id === root.span_id, 'parent is emitted before child with bounded lineage');
assert(events.every((event) => !JSON.stringify(event).includes('token') && !JSON.stringify(event).includes('private?a=1')), 'sensitive keys and full URLs are redacted');
probe = run('?debug=false', {{arcadiaDebug:'true'}});
assert(!probe.reflection.enabled('runtime') && probe.calls.length === 0, 'URL false overrides stored selection decisively');
probe = run('?debug=0', {{arcadiaDebug:'true'}});
assert(!probe.reflection.enabled('runtime'), 'URL zero overrides stored selection decisively');
console.log(JSON.stringify({{ok:true}}));
"#
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node executes Indra observation harness");
    assert!(output.status.success(), "node Indra harness failed\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"ok\":true"));
    assert!(indra.contains("const MAX_BYTES = 14 * 1024"));
    assert!(indra.contains("/api/debug/emit"));
    assert!(!indra.contains("/api/v1/hyalos/reflect"));
    assert!(!indra.contains("channel.jsonl"));
    assert!(APP_JS.contains("const ArcadiaObservation = window.ArcadiaObservationAdapters"));
}

#[test]
fn indra_source_census_routes_present_request_stream_action_and_presenter_seams() {
    let indra = include_str!("../../static/indra-observation.js");
    let ui = include_str!("../../src/bands/ui/mod.rs");
    assert!(ui.contains("/static/indra-observation.js"));
    assert!(APP_JS.contains("const fetch = ArcadiaObservation.request"), "ordinary fetch is routed through one request seam");
    assert!(!APP_JS.contains("window.fetch("), "new unclassified direct fetches are refused by this census");
    assert_eq!(APP_JS.matches("new EventSource(").count(), 2, "both present EventSource families must remain classified stream inputs");
    assert!(indra.contains("source_class"));
    assert!(indra.contains("noteDrop(input.attributes?.source_class || 'buffer', 'buffer')"));
    assert!(APP_JS.contains("ArcadiaObservation.navigation(next)"));
    assert!(APP_JS.contains("ArcadiaObservation.action(action, 'invoked')"));
    assert!(APP_JS.contains("ArcadiaObservationAdapters?.presenter"));
    assert!(indra.contains("safePathname(path) === '/api/debug/emit'"), "debug delivery cannot recurse through ordinary request observation");
}

#[test]
fn indra_observation_selection_lifecycle_governor_and_disposal_are_behavioral() {
    let indra = include_str!("../../static/indra-observation.js");
    let script = format!(
        r#"
const vm = require('vm');
const indra = {indra:?};
const assert = (condition, message) => {{ if (!condition) throw new Error(message); }};
function probe(search = '', storage = {{}}, deferred = false) {{
  const calls = [], installed = [], removed = [];
  const values = new Map(Object.entries(storage));
  function listener(owner) {{ return (type, fn, options) => installed.push({{ owner, type, fn, options }}); }}
  const window = {{
    location: {{ search }},
    fetch: (path, options) => {{ calls.push({{ path, options }}); return deferred ? new Promise(() => {{}}) : Promise.resolve({{ ok: true, status: 204 }}); }},
    addEventListener: listener('window'),
    removeEventListener: (type) => removed.push(`window:${{type}}`),
  }};
  const document = {{
    body: {{ dataset: {{}} }},
    addEventListener: listener('document'),
    removeEventListener: (type) => removed.push(`document:${{type}}`),
  }};
  const context = {{ window, document, localStorage: {{ getItem: (key) => values.get(key) || null }}, URLSearchParams, URL, Date, Set, String, Object, Array, JSON, Math, Promise, TextEncoder, performance: {{ now: () => 1 }}, setTimeout, clearTimeout, setInterval, clearInterval }};
  window.window = window;
  vm.runInNewContext(indra, context);
  return {{ reflection: window.IndraObservationReflection, adapters: window.ArcadiaObservationAdapters, calls, installed, removed }};
}}
for (const [search, storage, expected] of [
  ['?debug=', {{}}, ['runtime', 'navigation', 'interaction', 'request', 'ui']],
  ['?debug=true', {{}}, ['runtime', 'navigation', 'interaction', 'request', 'ui']],
  ['?debug=runtime,ui', {{}}, ['runtime', 'ui']],
  ['?debug=unknown', {{}}, []],
  ['?debug=false', {{ arcadiaDebug: 'true' }}, []],
  ['?debug=0', {{ arcadiaDebugKinds: 'runtime' }}, []],
  ['', {{ arcadiaDebug: 'true' }}, ['runtime', 'navigation', 'interaction', 'request', 'ui']],
  ['', {{ arcadiaDebugKinds: 'request,ui' }}, ['request', 'ui']],
]) {{
  const current = probe(search, storage);
  for (const kind of ['runtime', 'navigation', 'interaction', 'request', 'ui']) assert(current.reflection.enabled(kind) === expected.includes(kind), `selection failed for ${{search}}/${{kind}}`);
}}
let current = probe('', {{}}, true);
assert(current.reflection.resources().listeners === 0 && current.calls.length === 0, 'default-off allocation must be zero');
current = probe('?debug=runtime,interaction,request,ui', {{}}, true);
assert(current.reflection.resources().listeners === 6 && current.installed.length === 6, 'enabled listener resources install once');
const actionListener = current.installed.find((item) => item.owner === 'document' && item.type === 'click');
actionListener.fn({{ type: 'click', target: {{ dataset: {{ observationAction: 'controller-device-details' }}, parentElement: null, getAttribute: () => null }} }});
assert(current.calls.length === 1, 'one enabled capture seam reflects a stable action identifier');
for (let i = 0; i < 40; i += 1) current.adapters.currentness('status-refresh', i % 2 ? 'changed' : 'stale');
assert(current.calls.length <= 2, 'in-flight delivery is bounded under deferred fetch');
assert(current.reflection.resources().queue <= 32 && current.reflection.resources().in_flight <= 2, 'queue and in-flight governor bounds hold');
const root = current.reflection.begin('runtime', {{ event: 'boot' }});
const child = current.reflection.child('request', {{ event: 'begin' }}, root);
assert(current.reflection.mark(child, {{ event: 'mark' }}), 'nested mark is emitted');
assert(current.reflection.settle(child, {{ outcome: 'ok' }}), 'nested settle is emitted');
assert(!current.reflection.settle(child, {{ outcome: 'duplicate' }}), 'terminal event remains exactly once');
const lifecycle = current.calls.map((call) => JSON.parse(call.options.body));
assert(child.parent_span_id === root.span_id, 'nested handle preserves parent lineage');
assert(new Set(lifecycle.map((event) => event.sequence)).size === lifecycle.length, 'sequences are unique');
current.reflection.dispose();
current.reflection.dispose();
assert(current.removed.length === 6 && current.reflection.resources().queue === 0, 'dispose removes every installed listener once and clears queue');
const afterDispose = current.calls.length;
actionListener.fn({{ type: 'click', target: {{ dataset: {{ observationAction: 'controller-device-details' }}, parentElement: null, getAttribute: () => null }} }});
assert(current.calls.length === afterDispose, 'disposed observer callbacks are inert');
console.log(JSON.stringify({{ ok: true }}));
"#
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node executes Indra governor harness");
    assert!(
        output.status.success(),
        "node Indra governor harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"ok\":true"));
}
