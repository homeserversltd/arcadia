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
