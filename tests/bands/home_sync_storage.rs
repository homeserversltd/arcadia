    #[test]
    fn transient_success_feedback_uses_toasts_not_message_divs() {
        assert!(!APP_CSS.contains(".message--success"));
        for line in APP_JS.lines().filter(|line| line.contains("setMessage(")) {
            assert!(
                !line.contains("success"),
                "successful transient feedback must use PopupManager.showToast, not setMessage: {line}"
            );
        }
    }

    #[test]
    fn human_text_font_sizes_stay_inside_ordinary_bounds() {
        let allowed_large_icon_selectors = [
            ".product-mark",
            ".launcher-icon",
            ".home-action-icon",
            ".access-pin-icon",
        ];

        for (index, line) in APP_CSS.lines().enumerate() {
            if !line.contains("font-size:") {
                continue;
            }
            if allowed_large_icon_selectors
                .iter()
                .any(|selector| line.contains(selector))
            {
                continue;
            }

            let font_size = line.split("font-size:").nth(1).unwrap_or_default();
            for part in font_size.split("px") {
                let value = part
                    .rsplit(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .next()
                    .unwrap_or_default();
                if value.is_empty() {
                    continue;
                }
                let parsed: f32 = value.parse().expect("font-size px value parses");
                assert!(
                    parsed <= 22.0,
                    "human text font-size above 22px on CSS line {}: {}",
                    index + 1,
                    line
                );
            }
        }
    }

    #[test]
    fn home_view_is_operational_surface_without_duplicate_navigation() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        for required in [
            "priority-strip",
            "home-operational-grid",
            "home-operational-grid--dashboard",
            "load-chart-wrap",
            "load-sparkline",
            "load-average-readouts",
            "memory-usage",
            "data-load-card",
            r#"data-load-retry-ms="5000""#,
            "data-load-readout-value",
            "data-memory-bar",
            "data-memory-used-segment",
            "data-memory-used",
            "data-memory-total",
            "data-load-chip-value",
            "CPU temp",
            "GPU",
            "GPU temp",
            "Fan RPM",
            "Storage temp",
            "Read/s",
            "Write/s",
            "storage-home-card",
            "storage-bar--home",
            "storage-segment--games",
            "storage-segment--other",
            "storage-home-details",
            "data-bind=\"home.storage.percentUsed\"",
            "home-detail-row--cat-games",
            "home-detail-row--cat-ai",
            "home-detail-row--cat-else",
            "home-detail-row--cat-free",
            ">Games used:</span>",
            ">AI used:</span>",
            ">Everything else:</span>",
            ">Free:</span>",
            r#"aria-label="Storage""#,
            r#"aria-label="Load""#,

            r#"aria-label="Network""#,
            "home-network-chip-row",
            "home-network-chip",
            "home-network-copy-row",
            "Copy URL",
            "Copy IP",
            "Copy AI",
            r#"aria-label="AI Models""#,
            "local-ai-home-card",
            "local-ai-home-details",
            "local-ai-home-models",
            "data-bind-each=\"home.ai.models\"",
            "data-bind-replace=\"true\"",
            ">Load:</span>",
            ">State:</span>",
            r#"aria-label="Updates""#,
            ">Storage</h3>",
            ">Load</h3>",
            ">Network</h3>",
            ">Updates</h3>",
            ">AI Models</h3>",
        ] {
            assert!(home_html.contains(required), "missing {required}");
        }

        assert!(!home_html.contains("<h1"));
        assert!(!home_html.contains("<h2"));
        assert!(!home_html.contains("<p>") && !home_html.contains("<p "));

        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "Add Games",
            "Console Status",
            "Recent Activity",
            "home-action-tile",
            "data-nav-target=",
            "launcher-button",
            "Manage Network",
            "Manage Storage",
            "Browse Folders",
            "Open Sync",
            "Open Local AI",
            "Open Storage",
            "Review Update",
            ">Synced<",
            ">Current<",
            "playable ROMs ·",
            "Available ROMs",
            "ROMs",
            "data-label=\"Last scan\"",
            "data-label=\"Folder changes\"",
            "data-label=\"Library gap\"",
            "modules ·",
            "Machine ",
            "health-home-card",
            "identity-home-card",
            ">Health</h3>",
            ">Appliance</h3>",
            r#"aria-label="System Health""#,
            r#"aria-label="Appliance""#,
            "home-topology",
            "reachability-row",
            "reachability--ok",
            "Mbps",
            "aria-label=\"Ethernet speed\"",
            "storage-mini-row",
            "storage-mini-rows",
            "aria-label=\"Storage signals\"",
            "data-label=\"Volumes\"",
            "data-label=\"Cleanup\"",
            "data-label=\"Warnings\"",
        ] {
            assert!(
                !home_html.contains(forbidden),
                "duplicated nav/meta survived: {forbidden}"
            );
        }

        assert!(
            !home_html.contains(">Home<"),
            "home title leaked into viewport"
        );
        assert!(APP_CSS.contains(".priority-strip"));
        assert!(APP_CSS.contains(".home-operational-grid"));
        for required in [
            ".view[data-view-panel=\"home\"].is-active { display: grid; grid-template-rows: auto minmax(0, 1fr) auto; gap: 0; }",
            ".view[data-view-panel=\"home\"] .priority-strip { grid-row: 1; }",
            ".view[data-view-panel=\"home\"] .home-operational-grid { grid-row: 2; min-height: 0; }",
            ".view[data-view-panel=\"home\"] .home-warning-strip { grid-row: 3; }",
        ] {
            assert!(APP_CSS.contains(required), "home grid row CSS missing {required}");
        }
        assert!(APP_CSS.contains(".load-home-card"));
        assert!(APP_CSS.contains(".load-telemetry-grid"));
        assert!(APP_JS.contains("function bindHomeLoadSubscription()"));
        assert!(APP_JS.contains("window.arcadiaHomeLoadSubscriptionState = state"));
        assert!(APP_JS.contains("new EventSource('/api/root/events')"));
        assert!(APP_JS.contains("fetchJsonBounded('/api/root/events/renew'"));
        assert!(APP_JS.contains("JSON.stringify({ leaseId: state.lease.leaseId })"));
        assert!(APP_JS.contains("source.addEventListener('stats.tick', () => onPoke('stats.tick'))"));
        assert!(APP_JS.contains("source.addEventListener('state.changed', () => onPoke('state.changed'))"));
        assert!(!APP_JS.contains("source.addEventListener('snapshot'"));
        assert!(!APP_JS.contains("source.addEventListener('root'"));
        assert!(APP_JS.contains("fetchJsonBounded('/api/root/pull'"));
        assert!(APP_JS.contains("fetchJsonBounded('/api/root/state'"));
        assert!(APP_JS.contains("fetch('/api/root/history'"));
        assert!(APP_JS.contains("source.addEventListener('lease', (event) =>"));
        assert!(APP_JS.contains("source.addEventListener('heartbeat', (event) =>"));
        assert!(APP_JS.contains("source.addEventListener('expired'"));
        assert!(APP_JS.contains("fetchSnapshotOnce();"));
        assert!(APP_JS.contains("scheduleRetry();"));
        assert!(APP_JS.contains("arcadia:view-change"));
        assert!(APP_JS.contains("visibilitychange"));
        assert!(APP_JS.contains("document.visibilityState === 'visible'"));
        assert!(APP_JS.contains("[data-view-panel=\"home\"].is-active"));
        assert!(APP_JS.contains("clearTimeout(state.renewalTimer)"));
        assert!(APP_JS.contains("clearTimeout(state.retryTimer)"));
        assert!(APP_JS.contains("state.source.close()"));
        assert!(APP_JS.contains("bindHomeLoadSubscription();"));
        assert!(home_html.contains("<svg class=\"load-sparkline\""));
        assert!(home_html.contains("data-load-history-line"));
        assert!(home_html.contains("data-load-staleness"));
        assert!(APP_JS.contains("appendSnapshot(root)"));
        assert!(APP_JS.contains("ArcadiaProjector.applyOverlay({ home: { telemetry: data } })"));
        assert!(APP_JS.contains("fetch('/api/root/history'"));
        assert!(APP_JS.contains("const drawHistory = () => {"));
        assert!(APP_JS.contains("line.setAttribute('points'"));
        assert!(APP_JS.contains("formatBinding(resolve(node.dataset.bind, state)"));
        assert!(!APP_JS.contains("new window.Chart"));
        assert!(!APP_JS.contains("[data-load-headline]"));
        assert!(!APP_JS.contains("function bindHomeLoadPolling()"));
        assert!(!APP_JS.contains("window.arcadiaHomeLoadPollState"));
        assert!(!APP_JS.contains("setInterval(poll, pollMs)"));
        assert!(!APP_JS.contains("window.arcadiaHomeLoadPolling"));
        assert!(APP_CSS.contains(".view[data-view-panel=\"home\"].is-active"));
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
        for forbidden in ["Game Library", "Detected", ">Synced<", "Folders unavailable"] {
            assert!(!home_html.contains(forbidden), "unbacked home claim survived: {forbidden}");
        }
    }


    #[test]
    fn app_js_has_pane_blind_living_state_projector_and_widget_valve() {
        for required in [
            "const ArcadiaProjector = (() =>",
            "window.ArcadiaProjector = ArcadiaProjector",
            "function resolve(path, root)",
            "[data-bind]",
            "node.textContent = formatBinding(resolve(node.dataset.bind, state), node.dataset.bindFormat || '')",
            "[data-bind-class]",
            "node.setAttribute('data-state', asState(resolve(node.dataset.bindClass, state)))",
            "[data-bind-state]",
            "node.setAttribute('data-state', projected)",
            "function overlayTouches(path, patch)",
            "(_node, path) => overlayTouches(path, nextOverlay)",
            "[data-bind-show]",
            "node.hidden = !visible",
            "[data-bind-each]",
            "host.firstElementChild?.tagName === 'TEMPLATE'",
            "template.content.cloneNode(true)",
            "project(fragment, item, true)",
            "function registerWidget(selectorOrName, fn)",
            "widgets.push({ selectorOrName, fn })",
            "fn(state, widgetContext(selectorOrName))",
            "dispatchWidgets(documentState)",
            "source.addEventListener('state.changed', () => onPoke('state.changed'))",
            "fetchJsonBounded('/api/root/state'",
            "ArcadiaProjector.apply(await response.json())",
            "data-bind-class projects normalized values into data-state",
        ] {
            assert!(APP_JS.contains(required), "projector missing {required}");
        }

        for forbidden in [
            "projector-pane-knowledge",
            "if (!lastDocument) lastDocument = {}",
            "data-view-panel=\"home\"].is-active') && ArcadiaProjector",
            "view-home",
            "view-network",
            "view-controllers",
        ] {
            assert!(!APP_JS.contains(forbidden), "projector leaked pane knowledge: {forbidden}");
        }
    }

    #[test]
    fn pre_document_overlay_is_noop_except_for_load_card_telemetry() {
        let start = APP_JS.find("const ArcadiaProjector = (() =>").expect("projector start");
        let end = APP_JS.find("window.ArcadiaProjector = ArcadiaProjector;").expect("projector export")
            + "window.ArcadiaProjector = ArcadiaProjector;".len();
        let projector = &APP_JS[start..end];
        let script = format!(r#"
const nodes = [];
function select(nodes, selector) {{
  const key = selector.match(/^\[data-([a-z-]+)\]$/)?.[1].replace(/-([a-z])/g, (_, c) => c.toUpperCase());
  return key ? nodes.filter((node) => Object.prototype.hasOwnProperty.call(node.dataset, key)) : [];
}}
const loadCard = {{ nodes: [], querySelectorAll(selector) {{ return select(this.nodes, selector); }} }};
const document = {{
  querySelector(selector) {{ return selector === '[data-load-card]' ? loadCard : null; }},
  querySelectorAll(selector) {{ return select(nodes, selector); }}
}};
const window = {{}};
function node(dataset, textContent = '') {{
  return {{ dataset, textContent, attributes: {{}}, style: {{ setProperty() {{}}, removeProperty() {{}} }},
    setAttribute(name, value) {{ this.attributes[name] = String(value); }},
    removeAttribute(name) {{ delete this.attributes[name]; }}, closest() {{ return null; }}, querySelectorAll() {{ return []; }} }};
}}
const loadStorage = node({{ bind: 'home.storage.percentUsed' }}, 'Load-card Maud');
const loadCpu = node({{ bindState: 'home.telemetry.cpu.temperatureCelsius:gte:82' }}, 'Load CPU Maud');
const otherPaneTelemetry = node({{ bindState: 'home.telemetry.cpu.temperatureCelsius:gte:82' }}, 'Other-pane Maud');
const systemPane = node({{ bind: 'systemPane.adminText' }}, 'Shell Maud');
loadCpu.attributes['data-state'] = 'Maud';
otherPaneTelemetry.attributes['data-state'] = 'Maud';
loadCard.nodes.push(loadStorage, loadCpu);
nodes.push(loadStorage, loadCpu, otherPaneTelemetry, systemPane);
{projector}
let widgets = 0;
window.ArcadiaProjector.registerWidget('home', () => widgets++);
window.ArcadiaProjector.applyOverlay({{ systemPane: {{ adminText: 'Admin' }} }});
if (systemPane.textContent !== 'Shell Maud') throw new Error('unrelated shell overlay changed frame zero');
window.ArcadiaProjector.applyOverlay({{ home: {{ telemetry: {{ cpu: {{ temperatureCelsius: 94 }} }} }} }});
if (loadStorage.textContent !== 'Load-card Maud') throw new Error('telemetry changed unrelated binding inside Load card');
if (loadCpu.attributes['data-state'] !== 'warn') throw new Error('94°C Load chip did not project warn: ' + loadCpu.attributes['data-state']);
if (otherPaneTelemetry.textContent !== 'Other-pane Maud' || otherPaneTelemetry.attributes['data-state'] !== 'Maud') throw new Error('same telemetry path outside Load card changed before document');
if (widgets !== 0) throw new Error('pre-document overlay dispatched widgets');
window.ArcadiaProjector.apply({{ home: {{ storage: {{ percentUsed: 25 }}, telemetry: {{ cpu: {{ temperatureCelsius: 50 }} }} }}, systemPane: {{ adminText: 'Living admin' }} }});
if (loadStorage.textContent !== '25' || otherPaneTelemetry.attributes['data-state'] !== 'warn' || systemPane.textContent !== 'Admin') throw new Error('full living projection did not update every pane');
window.ArcadiaProjector.applyOverlay({{ home: {{ telemetry: {{ cpu: {{ temperatureCelsius: 94 }} }} }} }});
if (loadCpu.attributes['data-state'] !== 'warn' || otherPaneTelemetry.attributes['data-state'] !== 'warn' || widgets !== 2) throw new Error('living overlay lost full projection/widget semantics');
"#);
        let output = std::process::Command::new("node").arg("-e").arg(script).output()
            .expect("node is required for projector behavior regression");
        assert!(output.status.success(), "node projector behavior regression failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    #[test]
    fn load_chips_use_projector_threshold_states_and_css_reads_the_projected_state() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        for rule in [
            "data-bind-state=\"home.telemetry.cpu.temperatureCelsius:gte:82\"",
            "data-bind-state=\"home.telemetry.io.pressureAvg10:gte:10\"",
            "data-bind-state=\"home.telemetry.io.disk.readBytesPerSec:gt:0\"",
            "data-bind-state=\"home.telemetry.io.disk.writeBytesPerSec:gt:0\"",
        ] {
            assert!(home_html.contains(rule), "missing declared Load rule: {rule}");
        }
        assert_eq!(home_html.matches("data-bind-state=\"").count(), 9);
        assert_eq!(home_html.matches("data-state=\"idle\"").count(), 9);
        assert!(!home_html.contains("data-bind-class=\"home.telemetry."));
        assert!(APP_CSS.contains(".load-chip[data-state=\"ok\"]"));
        assert!(APP_CSS.contains(".load-chip[data-state=\"warn\"]"));
        assert!(APP_CSS.contains(".load-chip[data-state=\"idle\"]"));
        assert!(!APP_CSS.contains(".load-chip--ok"));
        assert!(!APP_CSS.contains(".load-chip--warn"));
        assert!(!APP_CSS.contains(".load-chip--idle"));
    }

    #[test]
    fn home_view_updates_available_and_service_states_are_truthful() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.updates.state = "available".to_string();
        status.updates.available_version = Some("arcadia-next".to_string());
        status.updates.suite_ok = true;
        if let Some(first) = status.system.services.get_mut(0) {
            first.state = "running".to_string();
        }
        if let Some(second) = status.system.services.get_mut(1) {
            second.state = "available".to_string();
        }
        status.arcadia.service = "stopped";

        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        assert!(home_html.contains("aria-label=\"Updates\""));
        assert!(home_html.contains("data-bind=\"home.priority.state\""));
        assert!(home_html.contains("data-bind-show=\"home.priority.visible\""));
        assert!(home_html.contains("data-bind=\"home.updates.readinessRatio\""));
        assert!(home_html.contains("data-bind=\"home.updates.pendingUpdates\""));
        assert!(!home_html.contains("update-latest/run.json"));
        assert!(!home_html.contains(">Receipt<"));
        assert!(!home_html.contains("home.session"));
        assert!(!home_html.contains("Game Session"));
        assert!(!home_html.contains(">Unknown<"));
        assert!(!home_html.contains("GameScope"));
        assert!(!home_html.contains(">Arcadia<"));
    }

    #[test]
    fn home_updates_card_surfaces_harmonia_check_and_module_readiness() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        for required in [
            "updates-home-card",
            "updates-home-details",
            "updates-home-actions",
            "data-action=\"check-updates\"",
            "data-endpoint=\"/api/actions/check-updates\"",
            ">Check</button>",
            ">Last ran:</span>",
            ">updates available:</span>",
            "home-detail-row",
        ] {
            assert!(home_html.contains(required), "home updates card missing {required}");
        }
        for forbidden in [
            "update-latest/run.json",
            "data-label=\"Receipt\"",
            "data-label=\"Pressure\"",
            "data-label=\"Ready\"",
            "data-home-update-pressure",
            " on · ",
            "installed",
            "receipt-missing",
            " ago</strong>",
            ">None</strong>",
            ">Last ran:</span><strong>--:--</strong>",
        ] {
            assert!(
                !home_html.contains(forbidden),
                "home updates card leaked internal surface: {forbidden}"
            );
        }

        let last_ran_prefix = "data-bind=\"home.updates.lastRan\">";
        let last_ran_start = home_html
            .find(last_ran_prefix)
            .expect("last ran detail row");
        let last_ran_value_start = last_ran_start + last_ran_prefix.len();
        let last_ran_end = home_html[last_ran_value_start..]
            .find("</strong>")
            .expect("last ran value");
        let last_ran = &home_html[last_ran_value_start..last_ran_value_start + last_ran_end];
        if last_ran != "—" {
            assert!(
                last_ran
                    .split_whitespace()
                    .any(|part| part.len() == 4 && part.chars().all(|c| c.is_ascii_digit())),
                "last ran should include a four-digit year, got: {last_ran}"
            );
        }
    }

    #[test]
    fn home_ai_model_card_surfaces_model_load_and_activity() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.local_ai.load_state = "hot".to_string();
        status.local_ai.loaded_model_name = Some("Hermes 8B".to_string());
        status.local_ai.lan_inference_enabled = true;
        let rendered = ui::layout(&status).into_string();
        let card_start = rendered
            .find("local-ai-home-card")
            .expect("local ai home card");
        let card_end = card_start
            + rendered[card_start..]
                .find("</article>")
                .expect("local ai home card closes");
        let card_html = &rendered[card_start..card_start + card_end];

        for required in [
            ">AI Models</h3>",
            "local-ai-home-models",
            "data-bind-each=\"home.ai.models\"",
            "Loaded:",
            "Hermes 8B",
            "data-bind=\"home.ai.load\">Hot</strong>",
            "data-bind=\"home.ai.activity\">Actively working</strong>",
            "data-home-ai-load-state=\"hot\"",
        ] {
            assert!(card_html.contains(required), "home ai model card missing {required}");
        }
        for forbidden in ["data-label=\"LAN\"", "data-label=\"Models\"", "data-label=\"Accelerator\"", "gpu-bar"] {
            assert!(
                !card_html.contains(forbidden),
                "home ai model card leaked legacy surface: {forbidden}"
            );
        }
    }

    #[test]
    fn home_storage_card_surfaces_games_ai_other_and_free_rows() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.storage.games.size = "12 GB".to_string();
        status.storage.ai_models.size = "48 GB".to_string();
        status.storage.artwork.bytes = 2_000_000_000;
        status.storage.other.bytes = 1_000_000_000;
        status.storage.free = "400 GB".to_string();
        let rendered = ui::layout(&status).into_string();
        let card_start = rendered
            .find("storage-home-card")
            .expect("storage home card");
        let card_end = card_start
            + rendered[card_start..]
                .find("</article>")
                .expect("storage home card closes");
        let card_html = &rendered[card_start..card_start + card_end];

        for required in [
            "storage-bar--home",
            "storage-segment--games",
            "storage-segment--other",
            "storage-segment--free",
            "data-bind=\"home.storage.percentUsed\"",
            "home-detail-row--cat-games",
            "home-detail-row--cat-ai",
            "home-detail-row--cat-else",
            "home-detail-row--cat-free",
            "data-bind=\"home.storage.gamesSize\">12 GB</strong>",
            "data-bind=\"home.storage.aiSize\">48 GB</strong>",
            "data-bind=\"home.storage.everythingElseSize\">3.0 GB</strong>",
            "data-bind=\"home.storage.freeSize\">400 GB</strong>",
        ] {
            assert!(card_html.contains(required), "home storage card missing {required}");
        }
        for forbidden in [
            "storage-mini-row",
            "data-label=\"Artwork\"",
            "data-label=\"Volumes\"",
            "<1%",
        ] {
            assert!(
                !card_html.contains(forbidden),
                "home storage card leaked legacy surface: {forbidden}"
            );
        }
    }

    #[test]
    fn home_network_card_surfaces_copy_actions_and_status_chiplets() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.network.online = true;
        status.network.ip_address = "192.0.2.42".to_string();
        status.network.internet_reachable = Some(true);
        status.network.lan_ai_reachable = true;
        status.local_ai.lan_inference_port = Some(7777);
        let rendered = ui::layout(&status).into_string();
        let card_start = rendered
            .find("network-home-card")
            .expect("network home card");
        let card_end = card_start
            + rendered[card_start..]
                .find("</article>")
                .expect("network home card closes");
        let card_html = &rendered[card_start..card_start + card_end];

        for required in [
            "home-network-chip-row",
            "home-network-chip--ok",
            "home-network-copy-row",
            "Copy URL",
            "Copy IP",
            "Copy AI",
            r#"data-copy-value="http://console.example.com""#,
            r#"data-copy-value="192.0.2.42""#,
            r#"data-copy-value="192.0.2.42:7777""#,
            "aria-label=\"Console Online\"",
            "aria-label=\"AI Online\"",
            "aria-label=\"Internet Online\"",
        ] {
            assert!(card_html.contains(required), "home network card missing {required}");
        }
        for forbidden in [
            "home-network-stack",
            "home-network-node",
            "home-network-link",
            "home-code-line",
            "home-topology",
            "reachability-row",
            "Mbps",
            "→",
            "aria-label=\"Ethernet speed\"",
            "Console URL",
            "data-copy-value=\"\"",
        ] {
            assert!(
                !card_html.contains(forbidden),
                "home network card leaked legacy surface: {forbidden}"
            );
        }
    }

    #[test]
    fn home_view_has_no_idle_readiness_indicator() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.network.online = true;
        status.storage.percent_used = 40;
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        status.updates.state = "current".to_string();
        status.local_ai.load_state = "stopped".to_string();

        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        assert!(home_html.contains("data-bind-show=\"home.priority.visible\""));
        assert!(home_html.contains("hidden"));
        assert!(!home_html.contains("All systems current."));
    }

    #[test]
    fn local_ai_language_replaces_ai_model_jargon() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let local_ai_start = rendered
            .find("id=\"view-local-ai\"")
            .expect("local ai view starts");
        let local_ai_end = local_ai_start
            + rendered[local_ai_start..]
                .find("id=\"view-network\"")
                .expect("network follows local ai");
        let local_ai_html = &rendered[local_ai_start..local_ai_end];

        for required in [
            "llama.cpp",
            "Model control",
            "Model library",
            "All library models",
            "data-model-library-select=\"true\"",
            "includes untested and unseated library entries",
            "Import GGUF model",
            "Hugging Face GGUF",
            "API on",
            "Enable LAN",
            "Hermes/Pi base URL",
            "Copy endpoint",
        ] {
            assert!(local_ai_html.contains(required), "missing {required}");
        }

        for forbidden in [
            "Load AI Model",
            "Model Manager",
            "LLM",
            "Runtime",
            "Loaded Model",
            "Inference",
            "GPU",
            "Estimated VRAM",
            "GPU layers",
            concat!("In", "harmonia"),
            concat!("in", "harmonia"),
        ] {
            assert!(
                !local_ai_html.contains(forbidden),
                "forbidden visible term survived: {forbidden}"
            );
            if !matches!(forbidden, "Runtime" | "Loaded Model" | "Inference") {
                assert!(
                    !APP_JS.contains(forbidden),
                    "forbidden script term survived: {forbidden}"
                );
            }
        }
    }

    fn sync_slice(rendered: &str) -> &str {
        let sync_start = rendered
            .find("<section id=\"view-sync\"")
            .expect("sync view starts");
        let sync_end = sync_start
            + rendered[sync_start..]
                .find("<section id=\"view-storage\"")
                .expect("storage follows sync");
        &rendered[sync_start..sync_end]
    }

    #[test]
    fn sync_view_is_appliance_fsm_without_raw_share_or_path_residue() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);

        for required in [
            r#"data-action="sync-games""#,
            r#"data-endpoint="/api/actions/sync-games""#,
            r#"data-sync-add-games="true""#,
            "Sync games",
            "Add games",
            "sync-running-panel",
            "sync-admission-board",
            "sync-orb-board",
            "sync-orb-stage",
            "ux-sync-orb",
            "sync-orb-lanes",
            r#"data-sync-debt="none""#,
            r#"data-beauty-debt="none""#,
            "sync-system-blades",
            "sync-admitted-shelf",
            "Sync orb",
        ] {
            assert!(sync_html.contains(required), "missing appliance sync marker: {required}");
        }
        for forbidden in [
            "Games current",
            "games waiting",
            "Waiting",
            "Changed",
            "Ejected",
            "Attention",
            ">Admitted<",
            "artwork complete",
            "sync-orb-verdict",
            "sync-orb-subline",
            "ux-sync-orb-glint",
            "sync-folder-source",
            "Samba",
            "Samba for Windows",
            "Samba for Linux",
            r#"\\console.example.com\games"#,
            r#"\\HOMECONSOLE\games"#,
            "smb://console.example.com/games",
            "smb://homeconsole/games",
            "/home/arcadia/Games",
            "games/gba",
            "games/ps2",
            "games/dos",
            ">IP<",
            "Copy Windows path",
            "Copy Linux/macOS path",
            "Copy IP Windows path",
            "Copy IP SMB URL",
            "Open Games Folder",
            "Scan ROM folders",
            "Scanning Samba ROM folders",
            "ROM folders",
            "Games synced. No new games found.",
            "Output</button>",
            "hidden sync output panel",
            "sync-output-store",
            "collapsible-log",
            "Choose files",
            "sync-upload-label",
            r#"data-sync-upload=\"true\""#,
        ] {
            assert!(
                !sync_html.contains(forbidden),
                "raw sync implementation residue survived: {forbidden}"
            );
        }
        assert_eq!(
            sync_html.matches("data-copy-value=").count(),
            0,
            "sync view exposes no raw copy buttons"
        );
        assert!(sync_html.contains("data-storage-health=\"OK\""));
        assert!(VIEWPORT_CSS.contains("prefers-reduced-motion"));
        assert!(APP_CSS.contains("sync-scan-dot"));
        assert!(APP_CSS.contains("sync-orb-stage"));
        assert!(UX_CSS.contains("ux-sync-orb"));
        assert!(UX_CSS.contains("ux-sync-orb-track"));
        assert!(UX_CSS.contains("ux-sync-orb-sweep"));
        assert!(!sync_html.contains("provider-keys-form"));
        assert!(!sync_html.contains("screenscraper_api_key"));
        assert!(APP_JS.contains("openSyncAddGamesModal"));
        assert!(APP_JS.contains("selectSyncGameKind"));
        assert!(APP_JS.contains("Select the game kind first"));
        assert!(APP_JS.contains("Select a kind first"));
        assert!(APP_JS.contains("Add ${label} files"));
        assert!(!APP_JS.contains("Choose files"));
        assert!(APP_JS.contains("form.append('system', gameKind)"));
        assert!(APP_JS.contains("uploadSyncGames"));
        assert!(APP_JS.contains("uploadSyncFiles"));
        assert!(APP_JS.contains("dragover"));
        assert!(APP_JS.contains("dataTransfer"));
        assert!(APP_CSS.contains("sync-add-games-modal"));
        assert!(APP_CSS.contains("sync-kind-grid"));
        assert!(APP_CSS.contains("sync-kind-dropzone"));
        for forbidden in [
            "Scanning Samba ROM folders",
            "Sync complete. Playable GameScope entries were updated from the ROM folders.",
            "Needs attention. Open Output",
            "Sync is reading the games folders now.",
            "Receipt output is stored below",
            "Open Games Folder",
            "Copy IP SMB URL",
            "Choose files",
        ] {
            assert!(!APP_JS.contains(forbidden), "raw sync JS residue survived: {forbidden}");
        }
    }

    #[test]
    fn sync_fsm_renders_before_during_and_after_as_appliance_states() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = false;
        status.library.last_sync_state = "never".to_string();
        status.library.last_sync = "Never".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.gamescope_entries = 0;
        status.library.total_detected_games = 0;
        status.library.total_synced_entries = 0;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;

        let before_rendered = ui::layout(&status).into_string();
        let before = sync_slice(&before_rendered);
        assert!(before.contains("Add games"));
        assert!(before.contains("No games admitted yet"));
        assert!(before.contains("Sync orb"));
        assert!(before.contains(r#"data-sync-debt="none""#));
        assert!(before.contains(r#"data-sync-games-total="0""#));
        assert!(before.contains("Native"));
        assert!(before.contains("Added"));
        assert!(before.contains("Sync games"));
        assert!(!before.contains("/home/arcadia/Games"));
        assert!(!before.contains("Synced"));
        assert!(!before.contains("Completed"));

        status.library.first_sync_completed = true;
        status.library.last_sync_state = "running".to_string();
        status.library.sync_state = "running".to_string();
        status.library.sync_needed = true;
        status.library.unsynced_added = 2;
        status.library.unsynced_changed = 1;
        status.library.unsynced_removed = 0;
        let during_rendered = ui::layout(&status).into_string();
        let during = sync_slice(&during_rendered);
        assert!(during.contains("Sync orb"));
        assert!(during.contains("classify"));
        assert!(during.contains("Syncing"));
        assert!(during.contains(r#"data-sync-orb-state="syncing""#));
        assert!(during.contains("sync-running-panel"));
        assert!(during.contains("disabled"));
        assert!(during.contains("Native"));
        assert!(!during.contains("games folders"));

        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.gamescope_entries = 0;
        status.library.total_detected_games = 3;
        status.library.total_synced_entries = 3;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let after_rendered = ui::layout(&status).into_string();
        let after = sync_slice(&after_rendered);
        assert!(after.contains("Sync orb"));
        assert!(after.contains(r#"<span class="ux-sync-orb-core" data-bind="sync.total">3</span>"#));
        assert!(after.contains("Native"));
        assert!(after.contains("Added"));
        assert!(after.contains("Artwork"));
        assert!(after.contains("Check again"));
        assert!(after.contains(r#"data-sync-debt="none""#));
        assert!(!after.contains("Games synced. No new games found."));
        assert!(!after.contains("0 new · 0 failed"));
        assert!(!after.contains("Receipt ready"));
        assert!(!after.contains("Needs first sync"));
        assert!(!after.contains("Sync failed"));
        assert!(!after.contains("/home/arcadia/Games"));
    }

    #[test]
    fn sync_library_admission_board_is_premium_collection_surface() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.total_detected_games = 4;
        status.library.gamescope_entries = 3;
        status.library.total_synced_entries = 3;
        status.library.artwork_paired_total = 2;
        status.library.artwork_missing = 1;
        status.library.failed_games = 0;
        status.library.skipped_games = 0;
        status.library.game_system_tally = vec![
            GameSystemTally { system: "GBA".to_string(), admitted: 2, artwork_paired: 2, artwork_missing: 0 },
            GameSystemTally { system: "Arcade".to_string(), admitted: 1, artwork_paired: 0, artwork_missing: 1 },
        ];
        status.library.admitted_games = vec![
            AdmittedGameTally {
                title: "Driven".to_string(),
                system: "GBA".to_string(),
                source_file: "/hidden/diagnostic/path/driven.gba".to_string(),
                game_id: "driven".to_string(),
                runner: "RetroArch mGBA".to_string(),
                steam_entry: "Driven (GBA)".to_string(),
                artwork_paired: true,
                artwork_source: "SteamGridDB".to_string(),
            },
            AdmittedGameTally {
                title: "Metal Slug".to_string(),
                system: "Arcade".to_string(),
                source_file: "/hidden/diagnostic/path/mslug.zip".to_string(),
                game_id: "metal-slug".to_string(),
                runner: "MAME / FinalBurn".to_string(),
                steam_entry: "Metal Slug".to_string(),
                artwork_paired: false,
                artwork_source: "missing".to_string(),
            },
        ];

        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        for required in [
            "sync-admission-board",
            "sync-orb-board",
            "sync-orb-stage",
            "ux-sync-orb",
            "ux-sync-orb-track",
            "ux-sync-orb-sweep",
            r#"<span class="ux-sync-orb-core" data-bind="sync.total">4</span>"#,
            "Native",
            "Added",
            "Artwork",
            "2 / 4",
            "Artwork missing",
            r#"data-sync-debt="none""#,
            r#"data-beauty-debt="caveat""#,
            "sync-system-blades",
            "sync-system-blade",
            "sync-admitted-shelf",
            "sync-game-card",
            "Driven",
            "Metal Slug",
            "RetroArch mGBA",
            "MAME / FinalBurn",
            "✓ admitted",
            "art paired",
            "needs cover",
        ] {
            assert!(sync_html.contains(required), "missing admission board marker: {required}");
        }
        for forbidden in ["0 new · 0 failed", "Receipt ready", "Last sync complete", "/hidden/diagnostic/path", ">Ledger<"] {
            assert!(!sync_html.contains(forbidden), "non-appliance board residue survived: {forbidden}");
        }
        for required_css in [
            ".sync-admission-board",
            "inline-size: min(var(--ux-sync-board-max-inline), 100%);",
            "overflow-x: clip;",
            ".sync-orb-stage",
            "grid-template-columns: var(--ux-sync-orb-size) minmax(0, 1fr) minmax(128px, .24fr);",
            "conic-gradient",
            ".sync-system-blade",
            "grid-template-columns: repeat(auto-fit, minmax(var(--ux-sync-system-min-inline), 1fr));",
            ".sync-admitted-shelf",
            "max-block-size: var(--ux-sync-game-list-max-block);",
            "overflow: auto;",
            ".sync-game-card",
            "grid-template-columns: var(--ux-sync-cover-size) minmax(0, 1fr) minmax(118px, auto);",
            ".sync-cover-frame",
            "width: var(--ux-sync-cover-size);",
        ] {
            assert!(APP_CSS.contains(required_css), "missing admission board CSS: {required_css}");
        }
        for required_ux in [
            "--ux-sync-orb-size",
            "--ux-sync-system-min-inline",
            "--ux-sync-game-list-max-block",
            "--ux-sync-cover-size",
        ] {
            assert!(UX_CSS.contains(required_ux), "sync fit token missing: {required_ux}");
        }
        assert!(APP_CSS.contains(".view[data-view-panel=\"sync\"].is-active {"));
        assert!(APP_CSS.contains("overflow: hidden;"));
    }

    #[test]
    fn sync_admitted_game_list_is_dense_full_collection_not_ten_tile_cap() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.total_detected_games = 14;
        status.library.total_synced_entries = 14;
        status.library.artwork_paired_total = 14;
        status.library.artwork_missing = 0;
        status.library.admitted_games = (1..=14)
            .map(|index| AdmittedGameTally {
                title: format!("Pocket Game {index:02}"),
                system: "GBA".to_string(),
                source_file: format!("/hidden/diagnostic/path/pocket-game-{index:02}.gba"),
                game_id: format!("pocket-game-{index:02}"),
                runner: "RetroArch mGBA".to_string(),
                steam_entry: format!("Pocket Game {index:02}"),
                artwork_paired: true,
                artwork_source: "local".to_string(),
            })
            .collect();

        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert_eq!(sync_html.matches("<article class=\"sync-game-card\"").count(), 14);
        assert!(sync_html.contains("Pocket Game 14"));
        assert!(!sync_html.contains("more admitted games"));
        assert!(!include_str!("../../src/bands/ui/sync.rs").contains("admitted_games.iter().take(10)"));
    }

    #[test]
    fn sync_orb_core_shows_library_total_not_admission_debt() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.gamescope_entries = 25;
        status.library.total_detected_games = 100;
        status.library.total_synced_entries = 99;
        status.library.unsynced_added = 1;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(sync_html.contains(r#"data-sync-debt="admission""#));
        assert!(sync_html.contains(r#"<span class="ux-sync-orb-core" data-bind="sync.total">100</span>"#));
        assert!(sync_html.contains(r#"data-sync-games-total="100""#));
        assert!(!sync_html.contains(r#"<span class="ux-sync-orb-core">1</span>"#));
        assert!(!sync_html.contains(r#"<span class="ux-sync-orb-core">125</span>"#));
        assert!(!sync_html.contains("Waiting"));
        assert!(!sync_html.contains("games waiting"));
    }

    #[test]
    fn sync_orb_ring_tracks_artwork_progress_against_added_games() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.total_detected_games = 12;
        status.library.total_synced_entries = 12;
        status.library.artwork_paired_total = 8;
        status.library.artwork_missing = 4;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(sync_html.contains(r#"data-sync-debt="none""#));
        assert!(sync_html.contains(r#"data-beauty-debt="caveat""#));
        assert!(sync_html.contains(r#"data-sync-orb-state="caveat""#));
        assert!(sync_html.contains("8 / 12"));
        assert!(sync_html.contains(r#"data-sync-art-progress="66""#));
        assert!(sync_html.contains("Artwork missing"));
        assert!(sync_html.contains("Check again"));
        assert!(!sync_html.contains("Games current"));
        assert!(!sync_html.contains("artwork complete"));
        assert!(!sync_html.contains("Needs sync"));
        assert!(!sync_html.contains("Sync needed"));
    }

    #[test]
    fn sync_complaint_and_eject_states_are_appliance_actions_not_output_or_path_prompts() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "error".to_string();
        status.library.last_sync = "Failed".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = true;
        status.library.unsynced_added = 1;

        let complaint_rendered = ui::layout(&status).into_string();
        let complaint = sync_slice(&complaint_rendered);
        assert!(complaint.contains(r#"data-sync-state="error""#));
        assert!(complaint.contains(r#"data-sync-result="error""#));
        assert!(complaint.contains("Native"));
        assert!(complaint.contains("Added"));
        assert!(!complaint.contains("Attention"));
        assert!(!complaint.contains("Waiting"));
        assert!(!complaint.contains("Sync failed"));
        assert!(!complaint.contains("Needs attention"));
        assert!(!complaint.contains(">Ledger<"));
        assert!(!complaint.contains("kicked out"));
        assert!(!complaint.contains("Open Output"));
        assert!(!complaint.contains("hidden sync output panel"));
        assert!(!complaint.contains("sync-output-store"));

        status.library.last_sync_state = "success".to_string();
        status.library.sync_needed = true;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 2;
        let eject_rendered = ui::layout(&status).into_string();
        let eject = sync_slice(&eject_rendered);
        assert!(eject.contains("Native"));
        assert!(eject.contains("Added"));
        assert!(!eject.contains("Ejected"));
        assert!(!eject.contains("games waiting"));
        assert!(!eject.contains("Sync needed"));
        assert!(!eject.contains("2 games were ejected and need attention"));
        assert!(!eject.contains("/home/arcadia/Games"));
        assert!(!eject.contains("Folder"));
        assert!(!eject.contains("Path"));
    }

    #[test]
    fn sync_upload_endpoint_accepts_games_and_rejects_unknowns() {
        let source = include_str!("../../src/bands/console_system_actions.rs");
        let main_source = include_str!("../../src/main.rs");
        assert!(main_source.contains("/api/actions/add-games"));
        assert!(main_source.contains("DefaultBodyLimit::max(MAX_SYNC_UPLOAD_TOTAL_BYTES)"));
        assert!(source.contains("async fn action_add_games_upload"));
        assert!(source.contains("arcadia.sync.upload.v1"));
        assert!(source.contains("unsupported game file"));
        assert!(source.contains("Select a game kind before adding files."));
        assert!(source.contains("valid_game_system"));
        assert!(source.contains("supported_game_file"));
        assert!(source.contains("game_system_storage_path(system)"));
        assert!(!source.contains("classify_upload_system"));
        assert!(source.contains("create_new(true)"));
        assert!(source.contains("duplicate name"));
        assert!(source.contains("No game files were selected"));
        assert!(source.contains("MAX_SYNC_UPLOAD_FILES"));
        assert!(source.contains("MAX_SYNC_UPLOAD_BYTES"));
        assert!(!source.contains("\"target\":"));
    }

    #[test]
    fn sync_completed_and_zero_states_are_truthful_fixtures() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.gamescope_entries = 0;
        status.library.total_detected_games = 3;
        status.library.total_synced_entries = 3;
        status.library.artwork_paired_total = 2;
        status.library.artwork_complete = 2;
        status.library.artwork_missing = 1;
        status.library.artwork_status = "2 complete · 1 missing".to_string();

        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(rendered.contains("Games: 3"));
        assert!(!sync_html.contains("Synced"));
        assert!(rendered.contains("Games: 3"));
        assert!(!sync_html.contains("Last sync complete"));
        assert!(sync_html.contains(r#"<span class="ux-sync-orb-core" data-bind="sync.total">3</span>"#));
        assert!(sync_html.contains("Native"));
        assert!(sync_html.contains("Added"));
        assert!(sync_html.contains("Artwork"));
        assert!(sync_html.contains("Artwork missing"));
        assert!(!sync_html.contains("Games current"));
        assert!(!sync_html.contains(">Admitted</em>"));

        status.library.gamescope_entries = 0;
        status.library.total_detected_games = 0;
        status.library.total_synced_entries = 0;
        status.library.admitted_games.clear();
        status.library.game_system_tally.clear();
        status.library.artwork_complete = 0;
        status.library.artwork_missing = 0;
        status.library.artwork_status = "No artwork".to_string();
        let zero = ui::layout(&status).into_string();
        let zero_sync = sync_slice(&zero);
        assert!(zero.contains("Games: 0"));
        assert!(!zero_sync.contains("Synced"));
        assert!(zero_sync.contains("Add games"));
        assert!(zero_sync.contains("sync-shelf-empty"));
        assert!(!zero_sync.contains("0 new · 0 failed"));
        assert!(!zero_sync.contains("/home/arcadia/Games"));
        assert!(!zero_sync.contains("No ROMs"));
        assert!(!zero_sync.contains("No scan has run yet"));
    }

    #[test]
    fn sync_artwork_readback_uses_parenthetical_title_aliases() {
        let candidates = artwork_slug_candidates("Driven (USA) (En,Fr,De,Es,It)");
        assert!(candidates.contains(&"driven-usa-en-fr-de-es-it".to_string()));
        assert!(candidates.contains(&"driven".to_string()));
        let source = include_str!("../../src/bands/status_library.rs");
        assert!(source.contains("artwork_exists_for_title(&file.system, &file.title)"));
        assert!(source.contains("artwork_cache_dir_has_image"));
        assert!(source.contains("grid.png"));
        assert!(source.contains("hero.png"));
        assert!(source.contains("icon.png"));
    }

    #[test]
    fn update_gui_routes_through_caduceus_membrane() {
        let source = include_str!("../../src/bands/console_system_actions.rs");
        assert!(source.contains("/api/v1/gui/update/now"));
        assert!(!source.contains("homeconsole-arcadia-gui-update"));
    }

    #[test]
    fn harmonia_module_toggle_routes_through_caduceus_membrane() {
        let source = include_str!("../../src/bands/console_system_actions.rs");
        assert!(source.contains("/api/v1/profile/module/toggle"));
        assert!(!source.contains("index.json.arcadia-bak"));
    }

    #[test]
    fn sync_action_routes_through_caduceus_membrane() {
        let routes = include_str!("../../src/bands/routes_caduceus.rs");
        let source = include_str!("../../src/bands/console_system_actions.rs");
        assert!(routes.contains("CaduceusAccessClient::default().post_json"));
        assert!(routes.contains("/api/v1/sync/now"));
        assert!(source.contains("run_caduceus_http_mutation"));
        assert!(source.contains("/api/v1/sync/now"));
        assert!(source.contains("/api/v1/update/check"));
        assert!(!source.contains("async fn action_sync_games() -> (StatusCode, Json<ConsoleActionResponse>) {\n    run_console_command(\n        \"sync-games\",\n        CADUCEUS_BIN,"));
    }

    #[test]
    fn caduceus_query_tranche_exposes_same_origin_read_proxy() {
        let constants = include_str!("../../src/bands/constants.rs");
        let routes = include_str!("../../src/bands/routes_caduceus.rs");
        let main_rs = include_str!("../../src/main.rs");
        assert!(constants.contains("const CADUCEUS_STAFF_SOCKET: &str = \"/run/caduceus/staff.sock\";"));
        assert!(routes.contains("/api/v1/identity"));
        assert!(routes.contains("/api/v1/profile"));
        assert!(routes.contains("/api/v1/health"));
        assert!(routes.contains("/api/v1/update/now"));
        assert!(routes.contains("/api/v1/sync/now"));
        assert!(routes.contains("/api/v1/receipts/ledger"));
        assert!(routes.contains("/api/v1/gui/update/now"));
        assert!(routes.contains("/api/v1/local-ai/runtime/update"));
        assert!(routes.contains("/api/v1/profile/module/toggle"));
        assert!(main_rs.contains("/api/caduceus/v1/identity"));
        assert!(main_rs.contains("/api/caduceus/v1/profile"));
        assert!(main_rs.contains("/api/caduceus/v1/health"));
        assert!(main_rs.contains("/api/caduceus/v1/update/now"));
        assert!(main_rs.contains("/api/caduceus/v1/sync/now"));
        assert!(main_rs.contains("/api/caduceus/health"));
    }

    #[test]
    fn api_root_object_is_decomposable_infinite_infinite_tree() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let root = api_root_object(&state);
        let encoded = serde_json::to_value(&root).expect("api root serializes");
        assert_eq!(encoded["schema"], "arcadia.api.root.v1");
        assert_eq!(encoded["kind"], "arcadia-root");
        assert!(encoded["children"].as_array().expect("children array").len() >= 4);

        let storage = encoded["children"]
            .as_array()
            .expect("children")
            .iter()
            .find(|node| node["id"] == "storage")
            .expect("storage node present");
        assert_eq!(storage["route"], "/api/storage/state");
        assert!(storage["children"]
            .as_array()
            .expect("storage children")
            .iter()
            .any(|node| node["id"] == "games"));
        assert!(storage["children"]
            .as_array()
            .expect("storage children")
            .iter()
            .any(|node| node["id"] == "artwork"));

        let telemetry = encoded["children"]
            .as_array()
            .expect("children")
            .iter()
            .find(|node| node["id"] == "telemetry")
            .expect("telemetry node present");
        assert!(telemetry["data"].get("cpu").is_some());
        assert!(telemetry["data"]["cpu"].get("usagePercent").is_some());
        assert!(telemetry["data"].get("load").is_some());
        assert!(telemetry["data"].get("io").is_some());
        assert!(telemetry["data"]["io"].get("pressureAvg10").is_some());
        assert!(telemetry["data"].get("temperature").is_some());
        assert!(telemetry["data"].get("fans").is_some());
        assert!(telemetry["data"].get("gpu").is_some());
        assert!(telemetry["data"].get("history").is_none());
        assert_eq!(telemetry["route"], "/api/root");
        let api = include_str!("../../src/bands/api_root.rs");
        let history_route = api
            .split("async fn api_root_history_route")
            .nth(1)
            .expect("history route implementation");
        assert!(history_route.contains("arcadia.api.root.history.v1"));
        assert!(history_route.contains("\"history\": {\"tiers\": {(tier): entries}"));
        assert!(telemetry["data"]["io"]["disk"].get("readBytesPerSec").is_some());
        assert!(telemetry["data"]["io"]["disk"].get("writeBytesPerSec").is_some());
    }

    #[test]
    fn api_root_routes_are_registered() {
        let source = include_str!("../../src/main.rs");
        assert!(source.contains(".route(\"/api\", get(api_root_route))"));
        assert!(source.contains(".route(\"/api/root\", get(api_root_route))"));
        assert!(source.contains(".route(\"/api/root/pull\", get(api_root_pull_route))"));
        assert!(source.contains(".route(\"/api/root/history\", get(api_root_history_route))"));
        assert!(source.contains(".route(\"/api/root/state\", get(api_root_state_route))"));
        assert!(source.contains(".route(\"/api/root/events\", get(api_root_events_route))"));
        assert!(source.contains(".route(\"/api/root/events/renew\", post(api_root_events_renew_route))"));
        assert!(source.contains("include!(\"bands/api_root.rs\")"));
    }

    #[test]
    fn held_device_and_cached_folder_routes_avoid_live_discovery() {
        let api = include_str!("../../src/bands/api_root.rs");
        let refresh_start = api.find("fn refresh_controller_input").expect("refresh function");
        let refresh_end = api[refresh_start..]
            .find("\nfn refresh_living_state")
            .map(|offset| refresh_start + offset)
            .expect("refresh function end");
        let refresh = &api[refresh_start..refresh_end];
        assert!(!refresh.contains("read_controller_input_fast(None)"));

        let routes = include_str!("../../src/bands/routes_storage.rs");
        let route_start = routes
            .find("async fn storage_rescan_folder_route")
            .expect("folder rescan route");
        let route_end = routes[route_start..]
            .find("\nasync fn storage_game_folders_route")
            .map(|offset| route_start + offset)
            .expect("folder rescan route end");
        let route = &routes[route_start..route_end];
        assert!(!route.contains("folder_storage("));
    }

    #[test]
    fn api_root_events_route_streams_payload_free_stats_and_state_pokes() {
        let source = include_str!("../../src/bands/api_root.rs");
        assert!(source.contains("async fn api_root_events_route"));
        assert!(source.contains("Sse<impl Stream<Item = Result<Event, Infallible>>>"));
        assert!(source.contains("HOME_TELEMETRY_LEASE_COUNTER"));
        assert!(source.contains("HOME_TELEMETRY_LEASES"));
        assert!(source.contains("HOME_TELEMETRY_RENEW_SECONDS"));
        assert!(source.contains("HOME_TELEMETRY_IDLE_TIMEOUT_SECONDS"));
        assert!(source.contains("async fn api_root_events_renew_route"));
        assert!(source.contains("home_telemetry_renew_lease"));
        assert!(source.contains("home_telemetry_lease_status"));
        assert!(source.contains("homeTelemetryLeaseExpired"));
        assert!(source.contains("Event::default().event(\"stats.tick\").data(\"{}\")"));
        assert!(source.contains("Event::default().event(\"state.changed\").data(\"{}\")"));
        assert!(!source.contains(".event(\"snapshot\")"));
        assert!(!source.contains(".event(\"root\")"));
        assert!(!source.contains(".event(\"state\")"));
        assert!(source.contains("last_document_generation != document_generation"));
        assert!(source.contains("last_document_generation = document_generation"));
        assert!(source.contains(".take(60)"));
        assert!(source.contains("Event::default().event(\"lease\")"));
        assert!(source.contains("Event::default().event(\"heartbeat\")"));
        assert!(source.contains("Event::default().event(\"expired\")"));
        assert!(source.contains("const HOME_TELEMETRY_CADENCE_SECONDS: u64 = 1;"));
        assert!(source.contains("tokio::time::interval(Duration::from_secs(HOME_TELEMETRY_CADENCE_SECONDS))"));
        assert!(source.contains("pub struct ApiLivingStateDocument"));
        assert!(source.contains("schema: \"arcadia.api.state.v1\""));
        assert!(source.contains("kind: \"arcadiaLivingState\""));
        assert!(source.contains("pub storage: StorageStatus"));
        assert!(source.contains("pub storage_summary: StorageStatus"));
        assert!(source.contains("pub network: NetworkState"));
        assert!(source.contains("pub ai: LocalAIState"));
        assert!(source.contains("pub controllers: ControllerStatus"));
        assert!(source.contains("pub system: SystemAdminStatus"));
        assert!(source.contains("fn api_telemetry_data"));
        assert!(source.contains("fn api_home_telemetry_data"));
        assert!(source.contains("CaduceusAccessClient::default().get_json(\"/api/v1/appliance/stats\")"));
        assert!(source.contains("CaduceusAccessClient::default().get_json(\"/api/v1/appliance/stats/history\")"));
        for forbidden in [
            "/proc/loadavg", "/proc/stat", "/proc/pressure/io", "/proc/diskstats", "/proc/meminfo",
            "/sys/class/thermal",
        ] {
            assert!(!source.contains(forbidden), "telemetry sampler leaked forbidden path: {forbidden}");
        }
        assert!(source.contains("fn cpu_temperature_celsius"));
        assert!(source.contains("fn load_average"));
        assert!(source.contains("fn pressure_avg10_percent"));
        assert!(source.contains("fn disk_io_counters"));
        assert!(source.contains("fn memory_usage"));
        assert!(!source.contains("fn cpu_usage_percent"));
        assert!(source.contains("storageTemperatureCelsius"));
        assert!(source.contains("someAvg10"));
        assert!(source.contains("\"pressureAvg10\": current.get(\"pressure\")"));
        assert!(source.contains(".and_then(serde_json::Value::as_f64)"));
        assert!(source.contains("usagePercent"));
        assert!(source.contains("readBytesPerSec"));
        assert!(source.contains("writeBytesPerSec"));
        assert!(source.contains("fn api_root_telemetry_tick"));
        assert!(
            !source.contains("let root = api_root_object(&state);"),
            "sse root ticks must not rebuild full console_status each second"
        );
        assert!(source.contains("async fn api_root_pull_route"));
        assert!(source.contains("attendance_cached_validate(&document_for_validation, &attendance)"));
        assert!(source.contains("fn refresh_living_telemetry"));
        assert!(source.contains("KeepAlive::new()"));
        assert!(source.contains("Duration::from_secs(15)"));
    }

    #[test]
    fn api_root_events_dependencies_are_declared() {
        let cargo = include_str!("../../Cargo.toml");
        let main = include_str!("../../src/main.rs");
        assert!(cargo.contains("async-stream"));
        assert!(cargo.contains("futures-core"));
        assert!(cargo.contains("\"time\""));
        assert!(main.contains("sse::{Event, KeepAlive, Sse}"));
        assert!(main.contains("use futures_core::Stream;"));
        assert!(main.contains("convert::Infallible"));
        assert!(main.contains("AtomicU64"));
        assert!(main.contains("Mutex"));
        assert!(main.contains("OnceLock"));
    }

    #[test]
    fn storage_game_roots_follow_homeconsole_runtime_hierarchy() {
        assert_eq!(
            game_system_storage_path("gba").to_string_lossy(),
            "/home/arcadia/Games/roms/gba"
        );
        assert_eq!(
            game_system_storage_path("ps2").to_string_lossy(),
            "/home/arcadia/Games/isos/ps2"
        );
        assert_eq!(
            game_system_storage_path("dos").to_string_lossy(),
            "/home/arcadia/Games/pc/dos"
        );

        let registry = storage_registry(&network_status());
        let gba = registry
            .categories
            .games
            .roots
            .iter()
            .find(|root| root.id == "gba")
            .expect("gba root exists");
        assert_eq!(gba.path, "/home/arcadia/Games/roms/gba");
        assert_ne!(gba.path, "/home/arcadia/Games/gba");
        assert_eq!(gba.samba_share_name, "games/gba");
    }

    #[test]
    fn storage_game_scan_counts_playable_rom_not_extracted_archive_duplicate() {
        let root = std::env::temp_dir().join(format!(
            "arcadia-storage-game-filter-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp game root created");
        fs::write(root.join("Driven (USA) (En,Fr,De,Es,It).gba"), [1u8; 4])
            .expect("gba written");
        fs::write(root.join("Driven (USA) (En,Fr,De,Es,It).zip"), [1u8; 3])
            .expect("zip written");

        let usage = game_path_usage(&root, "gba", 0);
        let largest = largest_game_files(&root, "gba", 3);
        assert_eq!(usage.files, 1);
        assert_eq!(usage.bytes, 4);
        assert_eq!(largest.len(), 1);
        assert_eq!(largest[0].name, "Driven (USA) (En,Fr,De,Es,It).gba");
        assert!(is_playable_game_file(
            &root.join("Driven (USA) (En,Fr,De,Es,It).gba"),
            "gba"
        ));
        assert!(!is_playable_game_file(
            &root.join("Driven (USA) (En,Fr,De,Es,It).zip"),
            "gba"
        ));

        fs::remove_dir_all(&root).expect("temp game root removed");
    }

    #[test]
    fn library_counts_steam_shortcuts_vdf_entries() {
        let root = std::env::temp_dir().join(format!(
            "arcadia-steam-shortcuts-vdf-{}",
            std::process::id()
        ));
        let shortcuts = root.join("75467976/config/shortcuts.vdf");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(shortcuts.parent().expect("shortcut parent")).expect("vdf dir created");
        fs::write(
            &shortcuts,
            b"\x00AppName\x00Driven (GBA)\x00Exe\x00/usr/bin/retroarch\x00LaunchOptions\x00-L /usr/lib/libretro/mgba_libretro.so /home/arcadia/Games/roms/gba/Driven.gba\x00",
        )
        .expect("vdf written");
        let entries = read_steam_shortcuts_file(&shortcuts, "/tmp/steam-userdata", "steam", "75467976");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Driven (GBA)");
        assert_eq!(entries[0].steam_user, "75467976");
        assert_eq!(entries[0].owner_user, "steam");
        assert_eq!(entries[0].executable.as_deref(), Some("/usr/bin/retroarch"));
        assert!(entries[0]
            .launch_options
            .as_deref()
            .unwrap_or_default()
            .contains("Driven.gba"));
        fs::remove_dir_all(&root).expect("temp vdf root removed");
    }

    #[test]
    fn library_lists_gamescope_shortcuts_across_profiles() {
        let bytes = b"\x00\x01AppName\x00Driven (GBA)\x00\x01Exe\x00/usr/bin/retroarch\x00\x01LaunchOptions\x00-L core driven.gba\x00\x01AppName\x00Road Rash (Genesis)\x00\x01Exe\x00/usr/bin/retroarch\x00\x01LaunchOptions\x00-L core \"road-rash.md\"\x00";
        let entries = parse_steam_shortcuts_bytes(
            bytes,
            "/home/steam/.local/share/Steam/userdata",
            "steam",
            "75467976",
            "/home/steam/.local/share/Steam/userdata/75467976/config/shortcuts.vdf",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Driven (GBA)");
        assert_eq!(entries[1].name, "Road Rash (Genesis)");
        assert_eq!(entries[0].steam_user, "75467976");
        assert_eq!(entries[0].owner_user, "steam");
        assert!(entries[1]
            .launch_options
            .as_deref()
            .unwrap_or_default()
            .contains("road-rash.md\""));
    }

    #[test]
    fn storage_view_answers_where_disk_space_went_safely() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let storage_start = rendered
            .find("<section id=\"view-storage\"")
            .expect("storage view starts");
        let storage_end = storage_start
            + rendered[storage_start..]
                .find("<section id=\"view-local-ai\"")
                .expect("local ai follows storage");
        let storage_html = &rendered[storage_start..storage_end];

        for required in [
            "data-view-panel=\"storage\"",
            "view-storage",
            "Storage",
            "Mismatch detected",
            "free",
            "used",
            "storage-appliance",
            "storage-appliance--one-pane",
            "storage-dashboard",
            "storage-command-center",
            "storage-hero-metrics",
            "storage-accounting-grid",
            "storage-category-list",
            "Games",
            "Local AI",
            "Temporary Files",
            "System",
            "Rescan",
            "Managed locations",
            "Filesystem used",
            "Category scan",
            "Other / unclassified",
            "data-storage-modal=\"locations\"",
            "storage-category-row--action",
            "storage-category-chevron",
            "data-storage-modal=\"games\"",
            "data-storage-modal=\"artwork-detail\"",
            "data-storage-modal=\"ai-models-detail\"",
            "data-storage-modal=\"updates-detail\"",
            "data-storage-modal=\"logs-detail\"",
            "data-storage-modal=\"cleanup-review\"",
            "data-storage-modal=\"system-detail\"",
            "data-storage-modal=\"category-other\"",
        ] {
            assert!(storage_html.contains(required), "missing {required}");
        }
        for forbidden in [
            "Artwork &amp; Metadata",
            "Games by Folder",
            "Copy path",
            "Clear Artwork Cache",
            "Clear Partial Downloads",
            "Details",
            "Details / Diagnostics",
            "storage-table--games",
            "cleanup-grid",
            "data-storage-modal=\"category-free\"",
            "/home/arcadia",
        ] {
            assert!(
                !storage_html.contains(forbidden),
                "main storage pane leaked {forbidden}"
            );
        }
        assert!(rendered.contains("/api/storage/rescan-summary"));
        assert!(!rendered.contains("delete-all-games"));
        assert!(!rendered.contains("data-storage-modal-template=\"games\""));
        assert!(!rendered.contains("<details class=\"storage-section"));
    }


    #[test]
    fn restored_campaign_panes_keep_live_projection_hooks() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let required_hooks = [
            ("sync", r#"data-bind="sync.total""#),
            ("sync", r#"data-bind="sync.native""#),
            ("sync", r#"data-bind-each="sync.systems""#),
            ("storage", r#"data-bind="storagePane.hero.free""#),
            ("storage", r#"data-bind-style-var="--storage-seg-pct:storagePane.capacity.segments.free.width""#),
            ("local-ai", r#"data-bind="localAiPane.hero.headline""#),
            ("local-ai", r#"data-bind="localAiPane.hero.endpoint""#),
            ("network", r#"data-bind="networkPane.connection.headline""#),
            ("network", r#"data-bind="networkPane.connection.detail""#),
            ("controllers", r#"data-controller-pool-count="true""#),
            ("controllers", r#"data-controller-primary-device="true""#),
            ("system", r#"data-bind="systemPane.vaultLabel""#),
            ("system", r#"data-bind="systemPane.vaultCopy""#),
            ("system", r#"data-bind="systemPane.pinLabel""#),
            ("system", r#"data-bind="systemPane.pinCopy""#),
            ("system", r#"data-bind="systemPane.adminText""#),
            ("system", r#"data-bind-checked="status.vault.auto_decrypt_enabled""#),
            ("system", r#"data-bind-checked="status.gui_pin.pin_required""#),
        ];
        for (pane, hook) in required_hooks {
            assert!(rendered.contains(hook), "{pane} missing live projection hook {hook}");
        }
        assert!(include_str!("../../src/bands/ui/network.rs").contains(r#"data-bind-copy-value="networkPane.addresses.ip""#));
        assert!(APP_JS.contains("const ArcadiaProjector"));
        assert!(APP_JS.contains("EventSource('/api/root/events')"));
        assert!(APP_JS.contains("EventSource('/api/controllers/trainer/events')"));
    }


    #[test]
    fn inspect_profile_membership_and_module_update_envelope_are_contract_bound() {
        let membership = parse_harmonia_pinned_membership(
            "pinned_module_membership=module-a:unpinned,module-b:unpinned,module-c:unpinned,module-d:unpinned,module-e:unpinned,module-f:unpinned,module-g:unpinned,module-h:unpinned,module-i:unpinned,module-j:unpinned,module-k:unpinned,module-l:gui-face,module-m:runtime-face,module-n:whole-suite",
        );
        assert_eq!(membership.classifications.len(), 14);
        assert_eq!(
            membership
                .classifications
                .values()
                .filter(|classification| classification.as_str() == "unpinned")
                .count(),
            11
        );
        assert_eq!(
            membership
                .classifications
                .values()
                .filter(|classification| classification.as_str() != "unpinned")
                .count(),
            3
        );
        assert!(
            parse_harmonia_pinned_membership(
                "pinned_module_membership=module-a:unpinned,partial-entry"
            )
            .classifications
            .is_empty()
        );
        let envelope = caduceus_staff_envelope_with_target(
            "/api/v1/update/module",
            serde_json::json!({ "module_id": "alpha-runtime" }),
            serde_json::json!({ "apply": true }),
            serde_json::json!({ "module": "alpha-runtime" }),
        );
        assert_eq!(envelope["transition"], "update.module");
        assert_eq!(envelope["target"]["module"], "alpha-runtime");
        assert_eq!(envelope["flags"]["apply"], true);
    }

    #[test]
    fn updates_view_is_harmonia_integration_with_module_controls() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];

        for required in [
            "data-harmonia-updates=\"true\"",
            "data-harmonia-update-controls=\"true\"",
            "data-harmonia-default-grid=\"true\"",
            "data-harmonia-module-pane=\"true\"",
            "data-harmonia-update-pane=\"true\"",
            "Sync",
            "Ledger",
            "Check state",
            "data-harmonia-module-grid=\"true\"",
            "data-harmonia-module=\"identity\"",
            "data-harmonia-module-switch=\"identity\"",
            "data-harmonia-module-switch-row=\"identity\"",
            "type=\"checkbox\"",
            "pin-toggle-track",
            "pin-toggle-thumb",
            "Identity",
            "System Packages",
            "Harmonia Runtime",
            "Keyman Runtime",
            "Homeconsole Sync Runtime",
            "Rust Build Toolchain",
            "Arcadia Gui Runtime",
            "Pinned Artifacts Runtime",
            "ready ·",
            "data-bind=\"stateLabel\"",
            "Version",
            "Readbacks",
            "updatesPane.receipts.suite",
            "updatesPane.receipts.check",
            "updatesPane.receipts.moduleRoot",
            "/var/lib/harmonia/receipts/",
            "update-latest/run.json",
            "/api/actions/check-updates",
            "/api/actions/update-gui",
            "data-harmonia-module-update",
            "data-harmonia-suite-update",
        ] {
            assert!(updates_html.contains(required), "updates view missing {required}");
        }
        assert_eq!(updates_html.matches("data-harmonia-ledger-open=\"true\"").count(), 1);
        assert_eq!(updates_html.matches("/api/actions/check-updates").count(), 1);
        assert_eq!(updates_html.matches("/api/actions/update-gui").count(), 2);
        assert!(updates_html.matches("data-harmonia-module=\"").count() >= 8);
        assert!(updates_html.matches("data-harmonia-module-switch=\"").count() >= 8);
        assert!(updates_html.matches("type=\"checkbox\"").count() >= 8);
        assert!(updates_html.matches("pin-toggle-track").count() >= 8);
        assert!(!updates_html.contains("data-harmonia-module-toggle="));
        assert!(!updates_html.contains(">Disable</button>"));
        assert!(!updates_html.contains(">Enable</button>"));
        assert!(!updates_html.contains("Manual SCP bridge"));
        assert!(!updates_html.contains("Latest available</span><strong>Not checked"));
        assert!(!updates_html.contains("Make harmonious"));
        assert!(!updates_html.contains("data-harmonia-module-menu=\"true\""));
        let updates_source = include_str!("../../src/bands/ui/updates.rs");
        assert!(updates_source.contains("data-update-endpoint=\"/api/actions/update-module\""));
        assert!(updates_source.contains("data-endpoint=\"/api/actions/update-gui\""));
        assert!(APP_JS.contains("updateHarmoniaModuleCard"));
        assert!(APP_JS.contains("updateHarmoniaPinnedGroup"));
    }

    #[test]
    fn updates_view_renders_independent_controls_and_one_pinned_group() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.updates.modules = (0..14)
            .map(|index| {
                let (id, label, membership) = match index {
                    0..=10 => (
                        format!("synthetic-unpinned-{index}"),
                        format!("Synthetic unpinned {index}"),
                        "unpinned",
                    ),
                    11 => (
                        "synthetic-gui-face".to_string(),
                        "Synthetic GUI face".to_string(),
                        "gui-face",
                    ),
                    12 => (
                        "synthetic-runtime-face".to_string(),
                        "Synthetic runtime face".to_string(),
                        "runtime-face",
                    ),
                    _ => (
                        "synthetic-whole-suite".to_string(),
                        "Synthetic whole suite".to_string(),
                        "whole-suite",
                    ),
                };
                HarmoniaModuleStatus {
                    id,
                    label,
                    description: String::new(),
                    enabled: true,
                    present: true,
                    seeking_update: false,
                    state: "enabled".to_string(),
                    receipt_path: "/synthetic/receipt.json".to_string(),
                    pinned_module_membership: Some(membership.to_string()),
                }
            })
            .collect();
        assert_eq!(status.updates.modules.len(), 14);

        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];

        let module_control_attribute = "data-harmonia-module-update=\"";
        let inert_module_control_attribute = "data-harmonia-module-update=\"\"";
        let inert_module_controls = updates_html
            .matches(inert_module_control_attribute)
            .count();
        assert_eq!(inert_module_controls, 1, "the module template is inert");
        let module_controls = updates_html
            .matches(module_control_attribute)
            .count()
            - inert_module_controls;
        assert_eq!(module_controls, 11);

        let pinned_card_marker =
            r#"<article class="updates-pinned-module" data-harmonia-pinned-update="true">"#;
        assert_eq!(updates_html.matches(pinned_card_marker).count(), 2);
        let pinned_card_start = updates_html
            .rfind(pinned_card_marker)
            .expect("server-rendered pinned card");
        let pinned_card_end = pinned_card_start
            + updates_html[pinned_card_start..]
                .find("</article>")
                .expect("pinned card closes")
            + "</article>".len();
        let pinned_card = &updates_html[pinned_card_start..pinned_card_end];
        for member in [
            "Synthetic GUI face",
            "Synthetic runtime face",
            "Synthetic whole suite",
        ] {
            assert_eq!(pinned_card.matches(member).count(), 1, "pinned member label {member}");
        }
        assert_eq!(
            pinned_card
                .matches("data-endpoint=\"/api/actions/update-gui\"")
                .count(),
            1
        );
        for grouped_module in [
            "synthetic-gui-face",
            "synthetic-runtime-face",
            "synthetic-whole-suite",
        ] {
            let grouped_control = format!("data-harmonia-module-update=\"{grouped_module}\"");
            assert!(!updates_html.contains(grouped_control.as_str()));
        }
    }


    #[test]
    fn updates_pane_family_is_in_living_state_and_root_tree() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let living = api_living_state_document(&state);
        let encoded = serde_json::to_value(&living).expect("living state serializes");
        assert_eq!(encoded["updatesPane"]["state"].as_str().unwrap_or(""), living.status.updates.state);
        assert!(encoded["updatesPane"]["lastRan"].as_str().unwrap_or("").len() > 0);
        assert!(encoded["updatesPane"]["pendingUpdates"].as_str().is_some());
        assert!(encoded["updatesPane"]["modules"].as_array().map(|m| !m.is_empty()).unwrap_or(false));
        assert!(encoded["updatesPane"]["receipts"]["suite"].as_str().unwrap_or("").contains("update-latest"));

        let root = api_root_object(&state);
        let root_json = serde_json::to_value(&root).expect("root serializes");
        let children = root_json["children"].as_array().expect("root children");
        assert!(children.iter().any(|node| node["kind"] == "updatesPane" && node["id"] == "updatesPane"));
    }

    #[test]
    fn updates_css_band_is_curated_and_height_budgeted() {
        let views_index = include_str!("../../static/app/views/index.json");
        assert!(views_index.contains("updates.css"));
        for required in [
            ".view[data-view-panel=\"updates\"].is-active",
            ".updates-pane",
            "grid-template-rows: auto minmax(0, 1fr) auto;",
            ".updates-board",
            "grid-template-columns: minmax(0, 1.45fr) minmax(240px, .65fr);",
            ".updates-module-grid",
            "grid-auto-rows: minmax(var(--ux-updates-module-min-height), 1fr);",
            "overflow: hidden;",
        ] {
            assert!(APP_CSS.contains(required), "updates css missing {required}");
        }
    }

    #[test]
    fn updates_module_tiles_use_updates_classes_and_two_row_token_budget() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.updates.modules[0].enabled = false;
        status.updates.modules[0].present = true;
        status.updates.modules[0].state = "disabled".to_string();
        status.updates.modules[1].enabled = true;
        status.updates.modules[1].present = false;
        status.updates.modules[1].state = "missing".to_string();

        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];
        let updates_source = include_str!("../../src/bands/ui/updates.rs");

        for required in [
            r#"class="updates-module""#,
            r#"class="updates-module-switch""#,
            r#"class="updates-module-copy""#,
            "Update needed",
        ] {
            assert!(updates_html.contains(required), "rendered updates tile missing {required}");
            assert!(updates_source.contains(required), "updates source missing {required}");
        }
        for required in [
            "data-harmonia-module=\"identity\"",
            "data-harmonia-module-switch=\"identity\"",
        ] {
            assert!(updates_html.contains(required), "rendered updates hook missing {required}");
        }
        for legacy in [
            r#"class="harmonia-module""#,
            r#"class="harmonia-module-switch""#,
            r#"class="harmonia-module-copy""#,
            "harmonia-module--",
        ] {
            assert!(!updates_html.contains(legacy), "rendered updates tile retained {legacy}");
            assert!(!updates_source.contains(legacy), "updates source retained {legacy}");
        }
        for legacy_selector in [
            ".harmonia-module {",
            ".harmonia-module-switch",
            ".harmonia-module-copy",
            ".harmonia-module--",
        ] {
            assert!(!APP_CSS.contains(legacy_selector), "composed CSS retained {legacy_selector}");
        }
        for required_css in [
            "grid-template-columns: minmax(var(--ux-updates-module-version-min-inline), 1fr) minmax(var(--ux-updates-module-status-min-inline), var(--ux-updates-module-status-max-inline));",
            "grid-template-areas: \"switch switch\" \"version action\" \"status status\";",
            "grid-area: switch;",
            "grid-area: version;",
            "grid-area: status;",
            ".updates-module-copy > span { display: none; }",
            ".updates-module-version em { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }",
            ".updates-module-version strong { min-width: 0; color: var(--cream); font-size: var(--ux-text-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }",
            "padding: var(--ux-updates-module-status-padding);",
            "white-space: normal;",
            "overflow-wrap: anywhere;",
            ".updates-module-switch input:checked + .pin-toggle-track",
            ".updates-module-switch:focus-within",
        ] {
            assert!(APP_CSS.contains(required_css), "updates tile CSS missing {required_css}");
        }
        for required_token in [
            "--ux-updates-module-column-gap: 4px;",
            "--ux-updates-module-gap: 2px;",
            "--ux-updates-module-padding: 2px 4px;",
            "--ux-updates-module-radius: 15px;",
            "--ux-updates-module-switch-gap: 4px;",
            "--ux-updates-module-toggle-width: 36px;",
            "--ux-updates-module-toggle-height: 20px;",
            "--ux-updates-module-toggle-thumb-size: 14px;",
            "--ux-updates-module-toggle-thumb-shift: 16px;",
            "--ux-updates-module-version-min-inline: 0px;",
            "--ux-updates-module-status-min-inline: 66px;",
            "--ux-updates-module-status-max-inline: 72px;",
            "--ux-updates-module-status-padding: 2px 4px;",
        ] {
            assert!(UX_CSS.contains(required_token), "updates tile token missing {required_token}");
        }
    }

    #[test]
    fn updates_readbacks_ride_living_state_not_obsolete_dom_menu() {
        for required in [
            "data-bind-checked",
            "data-bind-value",
            "data-bind-attr-id",
            "host.dataset.bindReplace === 'true'",
            "document.addEventListener('change', (event) =>",
            "fetchJsonBounded('/api/root/state'",
            "ArcadiaProjector.apply(await response.json())",
            "source.addEventListener('state.changed', () => onPoke('state.changed'))",
        ] {
            assert!(APP_JS.contains(required), "projector missing updates support {required}");
        }
        for forbidden in [
            "function openHarmoniaModuleMenu()",
            "data-harmonia-module-toggle",
            "window.setInterval(poll, 650)",
            "setInterval(poll, pollMs)",
        ] {
            assert!(!APP_JS.contains(forbidden), "obsolete updates readback/poll remains: {forbidden}");
        }
    }

    #[test]
    fn updates_view_exposes_paginated_harmonia_ledger() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];
        for required in [
            "data-harmonia-ledger-open=\"true\"",
            "Ledger",
            "data-harmonia-updates=\"true\"",
        ] {
            assert!(updates_html.contains(required), "updates view missing {required}");
        }
        for required in [
            "function ledgerValue(value)",
            "/api/harmonia/ledger?page=",
            "harmonia-ledger-list",
            "harmonia-ledger-pager",
            "Entry JSON",
            "Previous",
            "Next",
        ] {
            assert!(APP_JS.contains(required) || APP_CSS.contains(required), "ledger modal missing {required}");
        }
    }

    #[test]
    fn updates_view_collapses_available_side_when_zero_updates() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        for module in &mut status.updates.modules {
            module.enabled = true;
            module.present = true;
            module.state = "enabled".to_string();
        }
        status.updates.pending_updates = 0;
        status.updates.check_ok = true;
        status.updates.suite_ok = true;
        status.updates.check_changed = false;
        status.updates.suite_changed = false;
        status.updates.check_missing_signal = "none".to_string();
        status.updates.first_missing_signal = "none".to_string();

        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];

        assert!(updates_html.contains("data-zero-updates=\"true\""));
        assert!(updates_html.contains("Zero updates available"));
        assert!(!updates_html.contains("data-harmonia-update-tiles=\"true\""));
    }

    #[test]
    fn updates_view_surfaces_available_update_tiles_right_of_modules() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.updates.pending_updates = 3;
        status.updates.check_ok = false;
        status.updates.check_changed = true;
        status.updates.suite_ok = false;
        status.updates.suite_changed = true;
        status.updates.check_missing_signal = "packages-stale".to_string();
        status.updates.first_missing_signal = "packages-stale".to_string();
        status.updates.state = "available".to_string();
        status.updates.available_version = Some("arcadia-next".to_string());
        if let Some(module) = status.updates.modules.first_mut() {
            module.enabled = true;
            module.present = false;
            module.state = "missing".to_string();
        }

        let rendered = ui::layout(&status).into_string();
        let updates_start = rendered
            .find("<section id=\"view-updates\"")
            .expect("updates view starts");
        let updates_end = updates_start
            + rendered[updates_start..]
                .find("<section id=\"view-system\"")
                .expect("system follows updates");
        let updates_html = &rendered[updates_start..updates_end];
        let modules_pos = updates_html.find("data-harmonia-module-pane=\"true\"").expect("module pane");
        let tiles_pos = updates_html.find("data-harmonia-update-tiles=\"true\"").expect("update tiles");

        assert!(modules_pos < tiles_pos, "module pane should precede right-side update tiles");
        for required in [
            "data-update-kind=\"identity\"",
            "data-update-kind=\"system-packages\"",
            "data-update-kind=\"harmonia-runtime\"",
            "8 modules need update",
            "Update needed",
            "Press Sync to update this module",
        ] {
            assert!(updates_html.contains(required), "updates tile surface missing {required}");
        }
        assert!(!updates_html.contains("Zero updates available"));
        for forbidden in ["State check", "System suite", "Needs repair", "Repair pending", "Drift", "Suite stale", "No Harmonia receipt", "Missing"] {
            assert!(!updates_html.contains(forbidden), "updates view leaked customer-hostile text: {forbidden}");
        }
    }



    #[test]
    fn storage_system_bindings_resolve_against_serialized_living_state() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();
        let living = serde_json::to_value(api_living_state_document(&state)).expect("living state serializes");
        let storage_start = rendered.find("<section id=\"view-storage\"").expect("storage view");
        let storage_end = rendered[storage_start..]
            .find("<section id=\"view-local-ai\"")
            .map(|offset| storage_start + offset)
            .expect("local ai view follows storage");
        let system_start = rendered.find("<section id=\"view-system\"").expect("system view");
        let system_end = rendered[system_start..]
            .find("</main>")
            .map(|offset| system_start + offset)
            .expect("containing main closes after system view");
        assert!(storage_start < storage_end && storage_end <= system_start);
        assert!(system_start < system_end);
        let scopes = [&rendered[storage_start..storage_end], &rendered[system_start..system_end]];
        let binding_names = [
            "data-bind", "data-bind-copy-value", "data-bind-checked", "data-bind-enabled",
            "data-bind-value", "data-bind-attr-id", "data-bind-class", "data-bind-show",
            "data-bind-aria-label",
        ];
        let mut absolute = 0usize;
        let mut relative = 0usize;
        let mut each_count = 0usize;
        let mut style_count = 0usize;
        for scope in scopes {
            let mut rest = scope;
            struct Frame {
                name: String,
                each_host: bool,
                each_template: bool,
            }
            let mut frames: Vec<Frame> = Vec::new();
            let void_tags = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];
            while let Some(start) = rest.find('<') {
                let tag_end = rest[start..].find('>').expect("tag closes") + start;
                let tag = &rest[start..=tag_end];
                rest = &rest[tag_end + 1..];
                if tag.starts_with("<!--") || tag.starts_with("<!") || tag.starts_with("<?") { continue; }
                let closing = tag.starts_with("</");
                let tag_name = tag.trim_start_matches(|character| character == '<' || character == '/').split_whitespace().next().unwrap_or_default().trim_end_matches('>').to_ascii_lowercase();
                if closing {
                    let frame = frames.pop().expect("closing tag has an open frame");
                    assert_eq!(frame.name, tag_name, "closing tag must match its opening frame");
                    continue;
                }
                let attr = |name: &str| -> Option<&str> {
                    let needle = format!(" {}=\"", name);
                    let begin = tag.find(&needle)? + needle.len();
                    let end = tag[begin..].find('"')? + begin;
                    Some(&tag[begin..end])
                };
                let each_template = tag_name == "template" && frames.iter().any(|frame| frame.each_host);
                if let Some(path) = attr("data-bind-each") {
                    each_count += 1;
                    let mut value = &living;
                    for key in path.split('.') {
                        value = if let Ok(index) = key.parse::<usize>() { value.get(index) } else { value.get(key) }
                            .unwrap_or_else(|| panic!("unresolved each binding {path} at {key}"));
                    }
                    assert!(value.is_array(), "each binding is not an array: {path}");
                }
                for name in binding_names {
                    if name == "data-bind" && attr("data-bind-each").is_some() { continue; }
                    if let Some(path) = attr(name) {
                        let is_relative = path == "." || (!path.contains('.') && !path.contains('['));
                        if is_relative {
                            assert!(
                                frames.iter().any(|frame| frame.each_template),
                                "single-segment binding outside active each host template: {path}"
                            );
                            relative += 1;
                        } else {
                            let mut value = &living;
                            for key in path.split('.') {
                                value = if let Ok(index) = key.parse::<usize>() { value.get(index) } else { value.get(key) }
                                    .unwrap_or_else(|| panic!("unresolved binding {path} at {key}"));
                            }
                            absolute += 1;
                        }
                    }
                }
                if let Some(bindings) = attr("data-bind-style-var") {
                    for binding in bindings.split(',') {
                        let path = binding.split_once(':').map(|(_, path)| path.trim()).unwrap_or_default();
                        assert!(!path.is_empty(), "style-var binding path is empty");
                        style_count += 1;
                        let mut value = &living;
                        for key in path.split('.') {
                            value = if let Ok(index) = key.parse::<usize>() { value.get(index) } else { value.get(key) }
                                .unwrap_or_else(|| panic!("unresolved style-var binding {path} at {key}"));
                        }
                    }
                }
                let self_closing = tag.ends_with("/>") || void_tags.contains(&tag_name.as_str());
                if !self_closing {
                    frames.push(Frame {
                        name: tag_name,
                        each_host: attr("data-bind-each").is_some(),
                        each_template,
                    });
                }
            }
            assert!(frames.is_empty(), "binding census ended with unclosed HTML frames");
        }
        assert!(absolute > 0, "census did not exercise document paths");
        assert!(relative > 0, "census did not exercise item-relative template paths");
        assert!(each_count > 0, "census omitted data-bind-each");
        assert!(style_count > 0, "census omitted data-bind-style-var");
        assert!(!rendered.contains("data-bind=\"attendance."));
        assert_eq!(living["systemPane"]["adminText"], "Guest");
        assert!(rendered.contains("data-bind-each=\"storage.aiModelFiles\""));
        assert!(rendered.contains("data-bind-zero-dash=\"true\""));
        assert!(rendered.contains("data-bind-enabled=\"storage.cleanup.artworkBytesClearable\""));
        assert!(rendered.contains("data-bind-each=\"storage.registry.categories.aiModels.roots\""));
        assert!(rendered.contains("data-bind-each=\"storage.diagnostics.warnings\""));
    }

    #[test]
    fn storage_and_system_presenters_retain_actions_without_building_html() {
        let storage = include_str!("../../src/bands/ui/storage.rs");
        let system = include_str!("../../src/bands/ui/system.rs");
        for source in [storage, system] {
            for forbidden in ["innerHTML", "createElement", "insertAdjacentHTML"] {
                assert!(!source.contains(forbidden), "presenter retained forbidden HTML builder: {forbidden}");
            }
        }
        for path in [
            "/api/storage/rescan-summary", "/api/storage/cleanup/artwork", "/api/storage/cleanup/temporary",
            "/api/storage/cleanup/old-updates", "/api/storage/cleanup/logs", "/api/storage/cleanup/partial-ai-downloads",
        ] { assert!(storage.contains(path), "storage action path missing: {path}"); }
        for path in ["/api/actions/reboot-console", "/api/actions/shutdown-console", "/api/system/ssh/service"] {
            assert!(system.contains(path), "system action path missing: {path}");
        }
        assert!(APP_JS.contains("/api/v1/exousia/invalidate"));
        assert!(APP_JS.contains("x-caduceus-attendance"));
        assert!(APP_JS.contains("applyOverlay"));
        assert!(APP_JS.contains("value === 0 || value === '0' ? '—'"));
        assert!(APP_JS.contains("node.setAttribute('aria-disabled', String(!enabled))"));
        assert!(!APP_JS.contains("[data-admin-projection]').forEach((node) => { node.textContent"));
    }

    #[test]
    fn module_update_receipts_use_run_mode_and_ignore_malformed_steps() {
        let root = std::env::temp_dir().join(format!("arcadia-home-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let module_dir = root.join("modules").join("fixture");
        std::fs::create_dir_all(&module_dir).unwrap();
        let write = |name: &str, value: &str| std::fs::write(module_dir.join(name), value).unwrap();
        let set_mode = |mode: Option<&str>| {
            let path = root.join("run.json");
            if let Some(mode) = mode {
                std::fs::write(path, format!(r#"{{"mode":"{mode}"}}"#)).unwrap();
            } else {
                let _ = std::fs::remove_file(path);
            }
        };

        // Apply (and an absent mode) treats only failed receipts as pending.
        set_mode(Some("apply"));
        write("ready.json", r#"{"changed":true,"ok":true,"final_state":"current","diff_decision":"replace"}"#);
        assert!(!module_seeking_update(&root, "fixture"));
        write("failed.json", r#"{"changed":false,"ok":false,"final_state":"blocked"}"#);
        assert!(module_seeking_update(&root, "fixture"));
        std::fs::remove_file(module_dir.join("failed.json")).unwrap();
        write("blocked.json", r#"{"changed":true,"ok":true,"final_state":"blocked"}"#);
        assert!(!module_seeking_update(&root, "fixture"));
        std::fs::remove_file(module_dir.join("blocked.json")).unwrap();
        set_mode(None);
        assert!(!module_seeking_update(&root, "fixture"));

        // Check/plan receipts retain diff evidence; receipt scanning never reads per-step mode.
        std::fs::remove_file(module_dir.join("ready.json")).unwrap();
        write("changed.json", r#"{"changed":true,"ok":true,"final_state":"converged","diff_decision":"empty","mode":"apply"}"#);
        set_mode(Some("check"));
        assert!(module_seeking_update(&root, "fixture"));
        std::fs::remove_file(module_dir.join("changed.json")).unwrap();
        write("diff.json", r#"{"changed":false,"ok":true,"diff_decision":"replace","mode":"apply"}"#);
        assert!(module_seeking_update(&root, "fixture"));
        std::fs::remove_file(module_dir.join("diff.json")).unwrap();
        set_mode(Some("plan"));
        write("plan.json", r#"{"changed":true,"ok":true,"diff_decision":"empty"}"#);
        assert!(module_seeking_update(&root, "fixture"));
        std::fs::remove_file(module_dir.join("plan.json")).unwrap();
        write("empty.json", r#"{"changed":false,"ok":true,"diff_decision":"empty"}"#);
        write("broken.json", "{");
        assert!(!module_seeking_update(&root, "fixture"));
        assert!(!module_seeking_update(&root, "missing"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn home_updates_projects_enabled_flagged_first_then_label_and_id() {
        let mk = |id: &str, label: &str, enabled: bool, seeking_update: bool| HarmoniaModuleStatus {
            id: id.to_string(), label: label.to_string(), description: String::new(), enabled,
            present: true, seeking_update, state: "enabled".to_string(), receipt_path: String::new(),
            pinned_module_membership: None,
        };
        let modules = vec![
            mk("zeta", "Alpha", true, true),
            mk("disabled", "Aardvark", false, true),
            mk("beta", "Alpha", true, true),
            mk("ready", "Beta", true, false),
        ];
        let ordered = enabled_home_update_modules(&modules);
        assert_eq!(ordered.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["beta", "zeta", "ready"]);
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".into(),
            product: "HomeConsole".into(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let mut status = console_status(&state);
        status.updates.modules = modules;
        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync view follows home");
        let html = &rendered[home_start..home_end];
        for expected in [
            "data-bind-each=\"home.updates.modules\"",
            "data-bind-replace=\"true\"",
            "data-bind-show=\"seekingUpdate\"",
            "data-bind=\"label\"",
            "check-updates",
            "Enabled modules",
            "Beta",
            "Alpha",
        ] {
            assert!(html.contains(expected), "home Updates markup missing {expected}");
        }
        assert!(html.find("check-updates").unwrap() < html.find("Enabled modules").unwrap());
        let rows = html
            .split("class=\"updates-home-module-row\"")
            .skip(2)
            .map(|row| row.split("</div>").next().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 3, "three enabled rows render after the projector template");
        assert!(rows[0].contains(">Alpha</span>") && rows[0].contains("↑"));
        assert!(rows[1].contains(">Alpha</span>") && rows[1].contains("↑"));
        assert!(rows[2].contains(">Beta</span>") && !rows[2].contains("↑"));
        assert!(!html.contains("Aardvark"), "disabled modules are omitted from the server render");
        let projected = api_home_state(&status);
        assert_eq!(projected.updates.modules.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["beta", "zeta", "ready"]);
        assert_eq!(projected.updates.modules.iter().map(|m| m.seeking_update).collect::<Vec<_>>(), [true, true, false]);
        let serialized = serde_json::to_value(projected).unwrap();
        assert_eq!(serialized["updates"]["modules"][0]["seekingUpdate"], true);
    }
