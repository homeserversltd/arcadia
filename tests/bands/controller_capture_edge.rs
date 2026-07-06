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
}

#[test]
fn controller_capture_gate_script_kills_ghost_advance_sequence() {
    let helper_start = APP_JS
        .find("function controllerCaptureButtonKey")
        .expect("capture helper starts");
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
const axis2 = {{ control: 'axis 2', binding: '0.61' }};

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
assert(gate.observe({{ pressed: [], axes: [axis2] }}).input === 'axis 2', 'fresh axis edge after absence captures');
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
