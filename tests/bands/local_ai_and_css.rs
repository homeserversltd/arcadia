    #[test]
    fn local_ai_state_payload_matches_manager_contract() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let payload = local_ai_state(&state);
        let json = serde_json::to_string(&payload).expect("local ai state serializes");
        for required in [
            "runtime",
            "loadedModel",
            "installedModels",
            "recommendedModels",
            "downloads",
            "inference",
            "hardware",
        ] {
            assert!(json.contains(required), "missing local ai field {required}");
        }
        assert!(!json.to_ascii_lowercase().contains("password"));
        assert!(!json.to_ascii_lowercase().contains("api_key"));
    }

    #[test]
    fn appliance_shell_renders_eight_viewports_and_system_access() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        for view in [
            "view-home",
            "view-sync",
            "view-storage",
            "view-local-ai",
            "view-controllers",
            "view-network",
            "view-updates",
            "view-system",
        ] {
            assert!(rendered.contains(view), "missing {view}");
        }
        for indicator in [
            "Storage",
            "Sync",
            "Local AI",
            "Controls",
            "Updates",
        ] {
            assert!(rendered.contains(indicator), "missing {indicator}");
        }
        let header_start = rendered
            .find("<header class=\"top-header\"")
            .expect("top header rendered");
        let header_end = rendered
            .find("<div class=\"workspace\"")
            .expect("workspace follows header");
        let header_html = &rendered[header_start..header_end];
        for required in [
            "HomeConsole",
            "Network",
            "Games",
            "Updates",
            "Uptime",
            "AI",
            "Lock",
            "data-chip-kind=\"network\"",
            "data-chip-kind=\"sync\"",
            "data-chip-kind=\"updates\"",
            "data-chip-kind=\"uptime\"",
            "data-chip-kind=\"local-ai\"",
            "data-chip-kind=\"pin\"",
            "Vault",
            "data-chip-kind=\"vault\"",
        ] {
            assert!(
                header_html.contains(required),
                "missing header currentness item: {required}"
            );
        }
        for forbidden in [
            "Arcadia Console",
            "GameScope",
            "Storage",
            "UI contract",
        ] {
            assert!(
                !header_html.contains(forbidden),
                "developer/proof header item survived: {forbidden}"
            );
        }
        assert!(!rendered.contains("smb:://"));
        assert!(rendered.contains("HTTPS bundle"));
        assert!(rendered.contains("system-access-panel"));
        assert!(rendered.contains("Caduceus attendance is required"));
        assert!(!rendered.contains("view-access-pin"));
        assert!(!rendered.contains("view-vault"));

        assert!(
            !rendered.contains("data-view=\"games\""),
            "left launcher games section survived"
        );
        assert!(
            !rendered.contains("id=\"view-games\""),
            "dedicated games viewport survived"
        );
        assert!(
            !rendered.contains("aria-controls=\"view-games\""),
            "games launcher control survived"
        );
        assert!(!rendered.contains(r#"data-view="lan-inference""#));
        assert!(!rendered.contains("view-lan-inference"));
        assert!(APP_JS.contains("view === 'ai-model' || view === 'lan-inference'"));
    }

    #[test]
    fn appliance_css_uses_desktop_density_not_jumbo_display_type() {
        for forbidden in [
            "font-size: clamp(34px",
            "font-size: clamp(24px",
            "font-size: 64px",
            "font-size: 38px",
            "font-size: 36px",
            "font-size: 34px",
            "font-size: 32px",
            "font-size: 28px",
            "font-size: 26px",
        ] {
            assert!(
                !APP_CSS.contains(forbidden),
                "oversized CSS survived: {forbidden}"
            );
        }
        let forbidden = [
            [".view", "heading"].join("-"),
            ["min-height:", "230px"].join(" "),
            ["min-height:", "66px"].join(" "),
            ["font-size:", "28px"].join(" "),
            ["font-size:", "26px"].join(" "),
        ];
        for forbidden in forbidden {
            assert!(
                !APP_CSS.contains(&forbidden),
                "oversized/header CSS survived: {forbidden}"
            );
        }
        assert!(APP_CSS.contains(".path-card code { display: block; margin-top: 7px; color: var(--orange-strong); font-size: 20px;"));
        assert!(APP_CSS.contains(".status-card strong, .active-model strong { display: block; margin: 5px 0 7px; font-size: 18px;"));
    }


    #[test]
    fn modal_presenter_uses_responsive_viewport_grid() {
        assert!(UX_CSS.contains("--ux-modal-max-width: 720px;"));
        assert!(UX_CSS.contains("--ux-modal-viewport-gap: 18px;"));
        assert!(APP_CSS.contains(".modal-overlay {"));
        assert!(APP_CSS.contains("overflow: hidden;"));
        assert!(APP_CSS.contains("grid-template-rows: auto minmax(0, 1fr) auto;"));
        assert!(APP_CSS.contains("inline-size: min(calc(100vw - (var(--ux-modal-viewport-gap) * 2)), var(--ux-modal-max-width));"));
        assert!(APP_CSS.contains("max-block-size: calc(100dvh - (var(--ux-modal-viewport-gap) * 2));"));
        assert!(APP_CSS.contains(".modal-content { min-height: 0; overflow: auto;"));
        assert!(APP_CSS.contains("overscroll-behavior: contain;"));
        assert!(!APP_CSS.contains("transform: translate(550px, -8px)"));
        assert!(VIEWPORT_CSS.contains(".modal-overlay { align-items: stretch; justify-items: stretch; padding: 8px; }"));
        assert!(VIEWPORT_CSS.contains("max-block-size: calc(100dvh - 16px);"));
        assert!(VIEWPORT_CSS.contains(".sync-kind-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }"));
        assert!(UX_CSS.contains("--ux-modal-fullscreen-padding:"));
        assert!(APP_CSS.contains(".modal-card.modal-card--fullscreen {"));
        let modal_card_pos = APP_CSS.find(".modal-card {").expect("base modal card");
        let fullscreen_pos = APP_CSS
            .find(".modal-card.modal-card--fullscreen {")
            .expect("fullscreen modal card");
        assert!(
            fullscreen_pos > modal_card_pos,
            "fullscreen modal rules must follow base .modal-card so capped width does not win"
        );
        assert!(APP_CSS.contains("max-inline-size: none;"));
    }

    #[test]
    fn local_ai_buttons_use_shared_consistent_sizing() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let local_ai_start = rendered
            .find("id=\"view-local-ai\"")
            .expect("local ai view starts");
        let local_ai_end = rendered[local_ai_start..]
            .find("id=\"view-controllers\"")
            .map(|offset| local_ai_start + offset)
            .expect("controllers view follows local ai");
        let local_ai_html = &rendered[local_ai_start..local_ai_end];

        assert!(
            local_ai_html.contains("local-ai-actions"),
            "Local AI action rows must opt into the shared equal button track"
        );
        assert!(APP_CSS.contains(".view[data-view-panel=\"local-ai\"] .btn { width: 128px; min-height: var(--ux-button-min-height);"));
        assert!(APP_CSS.contains(".local-ai-actions { display: grid; grid-template-columns: repeat(auto-fit, minmax(128px, 128px));"));
        assert!(APP_CSS.contains(".local-ai-actions .btn { width: 100%; }"));
        assert!(APP_CSS.contains("padding: var(--ux-button-padding-block) var(--ux-button-padding-inline);"));
        assert!(APP_CSS.contains("white-space: nowrap;"));
        assert!(UX_CSS.contains("--ux-button-min-height: 34px;"));
        assert!(UX_CSS.contains("--ux-button-padding-block: 7px;"));
        assert!(UX_CSS.contains("--ux-button-padding-inline: 10px;"));
    }


    #[test]
    fn controller_recovery_splits_idle_wake_from_missing_gamepad_surface() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let idle = recovery_status_for_receiver_only_by_id(
            "usb-8BitDo_IDLE_2D377104CC-hidraw",
        )
        .expect("idle receiver fixture classified");
        let missing = recovery_status_for_receiver_only_by_id(
            "usb-8BitDo_8BitDo_Ultimate_2C_Wireless_Controller_2D377104CC-if02-hidraw",
        )
        .expect("full controller fixture classified");

        for (recovery, required, forbidden, expected_title) in [
            (
                idle,
                "Controller is asleep. Press any button on it to wake it.",
                "Restart the console",
                "8BitDo receiver",
            ),
            (
                missing,
                "Receiver is awake; no gamepad event surface is exposed yet. Restart the console to reload controller support.",
                "Wake the controller",
                "8BitDo Ultimate 2C Wireless Controller receiver",
            ),
        ] {
            let mut status = console_status(&state);
            status.controllers.detected_count = 0;
            status.controllers.state = recovery.state.clone();
            status.controllers.recovery = recovery;

            let rendered = ui::layout(&status).into_string();
            let controllers_start = rendered.find("id=\"view-controllers\"").expect("controllers view starts");
            let controllers_end = rendered[controllers_start..]
                .find("id=\"view-network\"")
                .map(|offset| controllers_start + offset)
                .expect("network follows controllers");
            let controllers_html = &rendered[controllers_start..controllers_end];

            for required in [
                "class=\"controls-recovery\"",
                "data-controller-recovery",
                "data-controller-recovery-title",
                "data-controller-recovery-detail",
                "data-controller-recovery-action",
                "data-controller-programmer-recovery",
                required,
                expected_title,
            ] {
                assert!(controllers_html.contains(required), "missing truthful recovery surface: {required}");
            }
            assert!(!controllers_html.contains(forbidden), "recovery family leaked wrong guidance: {forbidden}");
            assert!(!controllers_html.contains("/dev/input"), "main controls pane leaked raw device path");
            assert!(!controllers_html.contains("2D377104CC"), "main controls pane leaked receiver serial");
            assert!(!controllers_html.contains("if02"), "main controls pane leaked receiver interface suffix");
            assert!(!controllers_html.contains("hidraw"), "main controls pane leaked raw hidraw detail");
            assert!(!controllers_html.contains("8BitDo 8BitDo"), "main controls pane leaked doubled vendor word");
            assert!(!controllers_html.contains("event0"), "main controls pane leaked raw event number");
            assert!(!controllers_html.contains("event1"), "main controls pane leaked raw event number");
        }
        assert!(APP_CSS.contains(".controls-recovery { display: grid;"));
        assert!(APP_CSS.contains(".controls-recovery[hidden] { display: none; }"));
        assert!(APP_CSS.contains(".controls-map-recovery[hidden] { display: none; }"));
        assert!(APP_JS.contains("function updateControllerRecoverySurfaces"));
        assert!(APP_JS.contains("controllerRecoveryNeedsRestart"));
        assert!(APP_JS.contains("state === 'gamepad-surface-missing' || detail.includes('no gamepad event surface')"));
        assert!(APP_JS.contains("controllerRecoveryModalLine"));
        assert!(APP_JS.contains("state === 'receiver-idle' || controllerRecoveryNeedsRestart(recovery)"));
        assert!(APP_JS.contains("line.textContent = recovery.detail || ''"));
        assert!(APP_JS.contains("line.hidden = !show"));
    }

    #[test]
    fn nominal_controller_recovery_keeps_empty_chrome_hidden() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.controllers.recovery.state = "connected".to_string();
        let rendered = ui::layout(&status).into_string();
        let controllers_start = rendered.find("id=\"view-controllers\"").expect("controllers view starts");
        let controllers_end = rendered[controllers_start..]
            .find("id=\"view-network\"")
            .map(|offset| controllers_start + offset)
            .expect("network follows controllers");
        let controllers_html = &rendered[controllers_start..controllers_end];
        assert!(controllers_html.contains("data-controller-recovery hidden"));
        assert!(controllers_html.contains("data-controller-programmer-recovery hidden"));
        assert!(APP_JS.contains("normalizedState === 'connected' || normalizedState === 'nominal'"));
    }

    #[test]
    fn controllers_view_is_single_pane_for_controller_mapping() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.controllers.live_input.pressed.push(ControllerBindingStatus {
            control: "A".to_string(),
            binding: "button 0".to_string(),
            pressed: true,
            axis_value: None,
        });
        let rendered = ui::layout(&status).into_string();
        let controllers_start = rendered
            .find("id=\"view-controllers\"")
            .expect("controllers view starts");
        let controllers_end = rendered[controllers_start..]
            .find("id=\"view-network\"")
            .map(|offset| controllers_start + offset)
            .expect("network follows controllers");
        let controllers_html = &rendered[controllers_start..controllers_end];

        for required in [
            "data-view=\"controllers\"",
            "data-view-panel=\"controllers\"",
            "Your controllers",
            "data-controller-pool",
            "data-controller-select",
            "controls-card__head--pool",
            "controls-hub",
            "controller-map-stage__pad",
            "Your mappings",
            "Button label style",
            "data-tooltip",
            "Game systems",
            "Push mapping",
            "data-controller-programmer-open",
            "data-controller-tuner-open",
            "controller-tuner-template",
            "Apply tuning",
            "data-controller-tuner-sensitivity",
            "data-controller-tuner-apply",
            "data-controller-tuner-reset",
            ">Tune<",
            "RetroArch",
            "Dolphin",
            "DuckStation",
            "PCSX2",
            "PPSSPP",
            "data-action=\"controllers-rescan\"",
            "data-action=\"controllers-save-profile\"",
            "data-action=\"controllers-assign-retroarch\"",
            "data-action=\"controllers-assign-dolphin\"",
            "data-action=\"controllers-assign-duckstation\"",
            "data-action=\"controllers-assign-pcsx2\"",
            "data-action=\"controllers-assign-ppsspp\"",
            "data-controller-mapping-editor=\"default\"",
            "data-controller-face",
            "data-controller-gamepad-programmer",
            "data-gamepad-layout-version=\"5\"",
            "data-gamepad-slot=\"face-a\"",
            "data-gamepad-slot=\"system-start\"",
            "data-gamepad-slot=\"face-x\"",
            "data-gamepad-slot=\"stick-left\"",
            "data-gamepad-slot=\"dpad\"",
            "data-controller-control=\"D-pad Up\"",
            "data-controller-binding-label=\"true\"",
            "ux-gamepad-body--anatomical",
            "viewBox=\"0 0 1000 620\"",
            "ux-gamepad-slot",
            "ux-gamepad-system",
            "ux-gamepad-face--x",
            "ux-gamepad-stick",
            "ux-gamepad-dpad",
            "ux-gamepad-shoulder",
            "ux-gamepad-shell",
            "ux-gamepad-guide",
            "controller-profile-card",
            "Teach me",
            "data-controller-teach-start",
            "data-controller-teach-skip",
            "data-controller-teach-exit",
        ] {
            assert!(controllers_html.contains(required) || rendered.contains(required), "missing controller manager surface: {required}");
        }
        for forbidden in [
            "Controller tutorial",
            "Developer",
            "Debug",
            "Arcadia scaffold",
            "controller-bind-flow",
            "controllers-panel--binding",
            "data-bind-step",
            "Guided bind flow",
            ">Bind<",
            "Live input",
            "data-controller-live-input",
            "Controller manager",
            "controls-status",
            "Push to all games",
            "Test buttons",
            "controller-map-header",
            "controller-map-instruction",
            "Button pairs",
            "tuple pairs",
            "Linux signal",
            "Game control",
            "Your button",
            "Physical button",
            "Each row is a game control",
        ] {
            assert!(!controllers_html.contains(forbidden) && !rendered.contains(forbidden), "lazy controller pane prose survived: {forbidden}");
        }

        assert!(!controllers_html.contains("ux-gamepad-body--cublet-grid"));
        assert!(!controllers_html.contains("ux-gamepad-cublet"));
        assert_eq!(controllers_html.matches("data-gamepad-slot=\"").count(), 13);
        assert!(controllers_html.contains("<svg"));
        assert!(controllers_html.contains("ux-gamepad-face ux-gamepad-face--a is-active"));
        assert!(APP_JS.contains("bindControllerLiveInput"));
        assert!(APP_JS.contains("const panel = document.querySelector('[data-view-panel=\"controllers\"]')"));
        assert!(APP_JS.contains("openControllerModal"));
        assert!(APP_JS.contains("openControllerTunerModal"));
        assert!(APP_JS.contains("/api/actions/controllers-save-tuning"));
        assert!(APP_JS.contains("/api/controllers/input"));
        assert!(APP_CSS.contains(".controller-pool-card__tune"));
        assert!(APP_CSS.contains(".controls-hub"));
        assert!(APP_CSS.contains(".controls-status"));
        assert!(APP_CSS.contains(".view[data-view-panel=\"controllers\"].is-active"));
        assert!(APP_CSS.contains("grid-template-rows: minmax(0, 1fr) auto"));
        assert!(APP_CSS.contains("--ux-controls-gap"));
        assert!(APP_CSS.contains("repeat(var(--ux-controls-binding-cols)"));
        assert!(VIEWPORT_CSS.contains(".view[data-view-panel=\"controllers\"].is-active"));
        assert!(VIEWPORT_CSS.contains("controls-bindings"));
        assert!(APP_CSS.contains(".emulator-controller-grid"));
        assert!(APP_CSS.contains(".controller-button-dot--active"));
        assert!(UX_CSS.contains(".ux-controller-silhouette"));
        assert!(UX_CSS.contains(".ux-controller-silhouette.ux-gamepad-stage"));
        assert!(UX_CSS.contains(".ux-gamepad-body"));
        assert!(APP_JS.contains("compactGamepadBinding"));
        assert!(APP_JS.contains("auditGamepadControlLayout"));
        assert!(APP_CSS.contains(".controller-map-bindings .controls-binding-row"));
        assert!(include_str!("../../src/bands/ui/gamepad_layout.rs").contains("GAMEPAD_LAYOUT_MIN_GAP"));
        assert!(include_str!("../../src/bands/ui/gamepad_layout.rs").contains("gamepad_layout_overlap_report"));
        assert!(UX_CSS.contains(".ux-gamepad-slot"));
        assert!(UX_CSS.contains("[data-gamepad-slot]"));
        assert!(UX_CSS.contains(".ux-gamepad-body--anatomical"));
        assert!(!UX_CSS.contains("grid-template-areas:"));
        assert!(UX_CSS.contains(".ux-gamepad-system"));
        assert!(UX_CSS.contains(".ux-gamepad-face--x"));
        assert!(APP_CSS.contains("max-height: none"));
        assert!(UX_CSS.contains(".ux-gamepad-shell"));
        assert!(include_str!("../../src/bands/ui/gamepad_layout.rs").contains("viewBox=\"0 0 1000 620\""));
        assert!(APP_CSS.contains(".controller-pool-scroll"));
        assert!(APP_CSS.contains("repeat(auto-fill, minmax(168px, 1fr))"));
        assert!(UX_CSS.contains("--ux-controller-pool-card-min-height:"));
        assert!(APP_CSS.contains("min-height: var(--ux-controller-pool-card-min-height)"));
        assert!(APP_CSS.contains("min-height: var(--ux-controller-pool-action-min-height)"));
        assert!(!UX_CSS.contains("cublet"));
        assert!(APP_CSS.contains("min-height: min(62dvh, 520px)"));
        assert!(UX_CSS.contains(".ux-gamepad-hit"));
        assert!(APP_CSS.contains(".controller-map-stage__pad .ux-gamepad-body--anatomical"));
        assert!(UX_CSS.contains(".ux-gamepad-shell-highlight"));
        assert!(APP_JS.contains("host.querySelectorAll('[data-gamepad-slot]')"));
        assert!(APP_JS.contains("[data-gamepad-slot][data-controller-control]"));
        assert!(APP_JS.contains("event.key === 'Enter' || event.key === ' '"));
        assert!(APP_JS.contains("node.querySelector('[data-controller-binding-label]')"));
        assert!(!APP_JS.contains("[data-controller-binding-label], span, em"));
        assert!(APP_CSS.contains("modal-card--fullscreen"));
        assert!(APP_CSS.contains("modal-overlay--fullscreen"));
        assert!(APP_JS.contains("clearOverlayVariants"));
        assert!(APP_JS.contains("setOverlayVariant"));
        assert!(APP_JS.contains("variant: 'fullscreen'"));
        assert!(APP_JS.contains("modal-fullscreen-open"));
        assert!(UX_CSS.contains("--ux-spinner-size-md:"));
        assert!(APP_CSS.contains(".ux-arcadia-spinner__ring"));
        assert!(APP_JS.contains("const ArcadiaLoading"));
        assert!(APP_JS.contains("Opening controller map"));
        let controllers_backend = include_str!("../../src/bands/status_controllers.rs");
        assert!(controllers_backend.contains("fn controller_status_api()"));
        assert!(controllers_backend.contains("Json(controller_status_api())"));
        assert!(controllers_backend.contains("async fn controllers_input_route()"));
        assert!(controllers_backend.contains("read_controller_input(active_device.as_ref().or(devices.first()))"));
        assert!(!controllers_backend.contains("console_status(&state)"));
        assert!(APP_JS.contains("setBindingListenState"));
        let controller_backend = include_str!("../../src/bands/status_controllers.rs");
        assert!(controller_backend.contains("/proc/bus/input/devices"));
        assert!(controller_backend.contains("Keyboard") || controller_backend.contains("keyboard"));
        assert!(controller_backend.contains("event-joystick"));
        assert!(controller_backend.contains("action_controllers_assign_retroarch"));
        assert!(controller_backend.contains("action_controllers_assign_dolphin"));
        assert!(controller_backend.contains("action_controllers_assign_duckstation"));
        assert!(controller_backend.contains("action_controllers_assign_pcsx2"));
        assert!(controller_backend.contains("action_controllers_assign_ppsspp"));
        assert!(controller_backend.contains("write_emulator_profile"));
        assert!(controller_backend.contains("action_controllers_ramrod_all"));
        assert!(controller_backend.contains("ramrod_controller_profiles"));
        assert!(include_str!("../../src/bands/controller_writers/retroarch.rs").contains("input_l_x_plus_axis"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-ramrod-all"));
        assert!(controller_backend.contains("ControllerBindRequest"));
        assert!(controller_backend.contains("ControllerProfileApplyRequest"));
        assert!(controller_backend.contains("action_controllers_apply_profile"));
        assert!(controller_backend.contains("action_controllers_ramrod_all"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-ramrod-all"));
        assert!(APP_JS.contains("controllers-assign-"));
        assert!(APP_JS.contains("controllerId: activeControllerId()"));
        assert!(APP_JS.contains("bindControllerProgramming"));
        assert!(APP_JS.contains("readBrowserGamepadInput"));
        assert!(APP_JS.contains("mergeControllerInput"));
        assert!(APP_JS.contains("navigator.getGamepads"));
        assert!(APP_JS.contains("normalizeControllerInputEvents"));
        assert!(APP_JS.contains("Pause or quit your game first"));
        assert!(APP_JS.contains("/api/actions/controllers-bind"));
        assert!(APP_JS.contains("/api/actions/controllers-apply-profile"));
        assert!(APP_JS.contains("EventSource('/api/controllers/trainer/events')"));
        assert!(APP_JS.contains("ArcadiaControllerTrainerStream.subscribe"));
        assert!(APP_JS.contains("/api/controllers/trainer/events"));
        assert!(APP_JS.contains("data-controller-programmer-modal"));
        assert!(APP_JS.contains("data-controller-broadcast-toggle"));
        assert!(APP_JS.contains("/api/actions/controllers-select"));
        assert!(APP_JS.contains("controllerId"));
        assert!(APP_JS.contains("hydrateControllerBindings"));
        assert!(APP_JS.contains("controllerBindingMap"));
        assert!(APP_JS.contains("CONTROLLER_CANONICAL_CONTROL_SET"));
        assert!(APP_JS.contains("CONTROLLER_DEFAULT_BINDING_TO_CONTROL"));
        assert!(APP_JS.contains("node.closest('[data-controller-bind-row]')?.setAttribute('data-controller-binding-state'"));
        assert!(APP_JS.contains("updateControllerPoolSelection"));
        assert!(APP_JS.contains("openControllerModal"));
        assert!(VIEWPORT_CSS.contains(".controller-pool-scroll"));
        assert!(VIEWPORT_CSS.contains("max-height: 280px"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("arcadia.controller_library.v1"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("controller_library"));
        assert!(controller_backend.contains("action_controllers_select"));
        assert!(controller_backend.contains("action_controllers_save_tuning"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-select"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-save-tuning"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("save_tuning_for_controller"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("forget_controller_from_library"));
        assert!(APP_JS.contains("showConfirm"));
        assert!(APP_JS.contains("confirmationBodyFor"));
        assert!(APP_JS.contains("data-controller-forget"));
        assert!(APP_JS.contains("/api/actions/controllers-forget"));
        assert!(APP_JS.contains("removeControllerPoolCard"));
        assert!(APP_CSS.contains(".controller-pool-card__forget"));
        assert!(APP_CSS.contains(".modal-confirm__actions"));
        assert!(controller_backend.contains("action_controllers_forget"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-forget"));
    }

    #[test]
    fn controller_teach_sweep_order_and_motion_walls_are_present() {
        let order_start = APP_JS.find("const TEACH_SWEEP_ORDER = [").expect("teach order const");
        let order_end = APP_JS[order_start..].find("];
").expect("teach order end") + order_start;
        let order = &APP_JS[order_start..order_end];
        let expected = [
            "A", "B", "X", "Y", "L1", "R1", "L2", "R2", "Select", "Start",
            "Left Stick X", "Left Stick Y", "Right Stick X", "Right Stick Y",
            "D-pad Up", "D-pad Down", "D-pad Left", "D-pad Right",
        ];
        assert_eq!(order.matches("{ control:").count(), expected.len());
        let tuple_source = include_str!("../../src/bands/controller_writers/tuple.rs");
        let mut cursor = 0;
        for control in expected {
            let needle = format!("control: '{control}'");
            let next = order[cursor..].find(&needle).unwrap_or_else(|| panic!("missing teach control {control}"));
            cursor += next + needle.len();
            assert!(
                tuple_source.contains(&format!("(\"{control}\",")),
                "teach control {control} must exist in canonical binding vocabulary"
            );
        }
        for required in [
            "highlightSlot: 'stick-left'",
            "highlightSlot: 'stick-right'",
            "highlightSlot: 'dpad'",
            "Push the left stick straight left (11 of 18)",
            "Push the left stick straight up (12 of 18)",
            "Press right on the D-pad (18 of 18)",
        ] {
            assert!(order.contains(required), "teach order missing {required}");
        }
        assert!(APP_JS.contains("root.dataset.teachControl = step.control"));
        assert!(APP_JS.contains("const highlightSelector = step.highlightSlot"));
        assert!(APP_JS.contains("setBindingListenState(root, step.control, { text: step.prompt"));
        assert!(APP_JS.contains("if (!result.ok && teachIndex >= 0)"));
        assert!(APP_JS.contains("setTeachStep(root, teachIndex)"));
        assert!(APP_JS.contains("clearTeachMode(root)"));
        assert!(APP_JS.contains("event.key !== 'Escape'"));
        assert!(APP_JS.contains("advanceTeachStep(root)"));
        assert!(APP_JS.contains("Teaching complete — save your layout"));
        assert!(APP_JS.contains("controller-save-layout--cue"));
        assert!(UX_CSS.contains("@keyframes ux-gamepad-teach-pulse"));
        assert!(UX_CSS.contains(".ux-gamepad-slot.is-teach-target"));
        assert!(UX_CSS.contains("@media (prefers-reduced-motion: reduce)"));
        assert!(UX_CSS.contains("animation: none;"));
        assert!(UX_CSS.contains(".controller-save-layout--cue { animation: none; }"));
    }

    #[test]
    fn gamepad_mock_render_seats_labels_and_catches_trigger_stack_collisions() {
        fn attr_i32(tag: &str, name: &str) -> i32 {
            let needle = format!("{name}=\"");
            let start = tag.find(&needle).unwrap_or_else(|| panic!("missing {name} in {tag}")) + needle.len();
            let rest = &tag[start..];
            let end = rest.find('\"').unwrap_or_else(|| panic!("unterminated {name} in {tag}"));
            rest[..end].parse::<i32>().unwrap_or_else(|_| panic!("invalid {name} in {tag}"))
        }

        fn group<'a>(html: &'a str, slot: &str) -> &'a str {
            let needle = format!("data-gamepad-slot=\"{slot}\"");
            let start = html.find(&needle).unwrap_or_else(|| panic!("missing slot {slot}"));
            let end = html[start..].find("</g>").unwrap_or_else(|| panic!("unclosed slot {slot}"));
            &html[start..start + end]
        }

        fn text_tag<'a>(group: &'a str, class: &str) -> &'a str {
            let start = group.find(class).unwrap_or_else(|| panic!("missing text class {class}"));
            let tag_start = group[..start].rfind("<text").unwrap_or(start);
            let tag_end = group[start..].find('>').unwrap_or_else(|| panic!("unclosed text tag {class}"));
            &group[tag_start..start + tag_end]
        }

        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();

        let l2 = group(&rendered, "shoulder-l2");
        let l2_label_y = attr_i32(text_tag(l2, "ux-gamepad-label"), "y");
        let l2_binding_y = attr_i32(text_tag(l2, "ux-gamepad-binding"), "y");
        assert_eq!((l2_label_y, l2_binding_y), (118, 139));
        assert!(
            l2_label_y + 16 <= l2_binding_y,
            "mock-render trigger stack collision: L2 label baseline/central text overlaps binding"
        );

        for (slot, expected_x, expected_y) in [
            ("stick-left", 284, 284),
            ("stick-right", 608, 393),
            ("system-select", 459, 344),
            ("system-start", 541, 344),
            ("face-y", 732, 289),
            ("face-x", 681, 340),
            ("face-b", 783, 340),
            ("face-a", 732, 391),
        ] {
            let binding = text_tag(group(&rendered, slot), "ux-gamepad-binding");
            assert_eq!((attr_i32(binding, "x"), attr_i32(binding, "y")), (expected_x, expected_y), "{slot} binding anchor drifted");
        }
    }





    #[test]
    fn controller_mappings_panel_renders_control_major_full_vocabulary() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();
        let panel_start = rendered.find("data-controller-mapping-major=\"control\"").expect("control-major mapping panel rendered");
        let panel_end = rendered[panel_start..].find("</section>").map(|offset| panel_start + offset).expect("mapping panel closes");
        let panel = &rendered[panel_start..panel_end];
        let expected = [
            "A", "B", "X", "Y", "L1", "R1", "L2", "R2", "L3", "R3", "Select", "Start",
            "D-pad Up", "D-pad Down", "D-pad Left", "D-pad Right",
            "Left Stick X", "Left Stick Y", "Right Stick X", "Right Stick Y",
        ];
        assert_eq!(panel.matches("data-controller-bind-row=\"true\"").count(), expected.len());
        for control in expected {
            assert!(panel.contains(&format!("data-controller-control=\"{control}\"")), "missing control-major row for {control}");
        }
        for required in ["AX0", "AX1", "AX3", "AX4", "D-pad"] {
            assert!(panel.contains(required), "missing friendly binding value {required}");
        }
        assert!(include_str!("../../src/bands/ui/controllers.rs").contains("\"Unbound\".to_string()"));
    }

    #[test]
    fn controller_profile_writers_stage_every_known_emulator() {
        let root = std::env::temp_dir().join(format!("arcadia-controller-writer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut bindings = default_controller_bindings();
        upsert_binding(&mut bindings, "A", "button 9");
        let nintendo = controller_bindings_for_profile("Nintendo");
        assert!(nintendo.iter().any(|binding| binding.control == "A" && binding.binding == "button 1"));
        let profile = root.join("default.json");
        write_controller_profile(&profile, "Virtual Arcadia Gamepad", "virtual0", &bindings)
            .expect("profile writes");
        let profile_text = std::fs::read_to_string(&profile).expect("profile text");
        assert!(profile_text.contains("arcadia.controller_profile.v1"));
        assert!(profile_text.contains("\"tuples\""));
        assert!(profile_text.contains("\"input\":\"button 9\""));
        assert!(!profile_text.contains("\"bindings\""));

        let tuning = ControllerTuningStatus::defaults();
        for emulator in ["RetroArch", "Dolphin", "DuckStation", "PCSX2", "PPSSPP"] {
            let path = write_emulator_profile(
                emulator,
                "Virtual Arcadia Gamepad",
                "virtual0",
                &bindings,
                &tuning,
                false,
                &root,
            )
                .unwrap_or_else(|error| panic!("{emulator} writer failed: {error}"));
            let body = std::fs::read_to_string(&path).expect("emulator profile text");
            assert!(body.contains("Virtual Arcadia Gamepad"));
            assert!(body.contains("mode=staged-for-install"));
            assert!(body.contains("button_9") || body.contains("button 9") || body.contains("9"));
        }
        let retroarch_body = std::fs::read_to_string(
            root.join("emulators/RetroArch/default-profile.txt"),
        )
        .expect("retroarch staged profile");
        assert!(retroarch_body.contains("input_l_x_plus_axis = \"+0\""));
        assert!(retroarch_body.contains("input_l_y_plus_axis = \"+1\""));
        assert!(retroarch_body.contains("analog_dpad_mode = \"0\""));
        assert!(retroarch_body.contains("input_axis_threshold"));

        let autoconfig = retroarch_autoconfig_from_bindings("Virtual Arcadia Gamepad", &bindings, &tuning);
        assert!(autoconfig.contains("input_axis_sensitivity"));
        assert!(autoconfig.contains("input_l3_btn"));
        assert!(!autoconfig.contains("input_l_x_plus_axis = \"axis 0\""));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn controller_ramrod_spine_writes_all_emulator_strata() {
        let root = std::env::temp_dir().join(format!(
            "arcadia-controller-ramrod-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::env::set_var("ARCADIA_CONTROLLER_PROFILE_ROOT", &root);
        let bindings = default_controller_bindings();
        let emulators = vec![
            EmulatorControllerStatus {
                emulator: "RetroArch".to_string(),
                command: "retroarch".to_string(),
                state: "configured".to_string(),
                config_path: String::new(),
                mapping_path: String::new(),
                profile: String::new(),
            },
            EmulatorControllerStatus {
                emulator: "Dolphin".to_string(),
                command: "dolphin-emu".to_string(),
                state: "not installed".to_string(),
                config_path: String::new(),
                mapping_path: String::new(),
                profile: String::new(),
            },
            EmulatorControllerStatus {
                emulator: "DuckStation".to_string(),
                command: "duckstation-qt".to_string(),
                state: "not installed".to_string(),
                config_path: String::new(),
                mapping_path: String::new(),
                profile: String::new(),
            },
            EmulatorControllerStatus {
                emulator: "PCSX2".to_string(),
                command: "pcsx2-qt".to_string(),
                state: "not installed".to_string(),
                config_path: String::new(),
                mapping_path: String::new(),
                profile: String::new(),
            },
            EmulatorControllerStatus {
                emulator: "PPSSPP".to_string(),
                command: "PPSSPPSDL".to_string(),
                state: "not installed".to_string(),
                config_path: String::new(),
                mapping_path: String::new(),
                profile: String::new(),
            },
        ];
        let receipt = ramrod_controller_profiles(
            "Virtual Arcadia Gamepad",
            "virtual0",
            &bindings,
            &ControllerTuningStatus::defaults(),
            &emulators,
        )
        .expect("ramrod spine");
        assert_eq!(receipt.entries.len(), 5);
        assert!(receipt.entries.iter().any(|entry| entry.emulator == "RetroArch" && entry.deployed));
        assert!(receipt.entries.iter().any(|entry| entry.emulator == "Dolphin" && !entry.deployed));
        let _ = std::fs::remove_dir_all(&root);
        std::env::remove_var("ARCADIA_CONTROLLER_PROFILE_ROOT");
    }


    #[test]
    fn local_ai_runtime_actions_call_real_harmonia_and_reject_fake_green() {
        let routes = include_str!("../../src/bands/routes_ai_models.rs");
        assert!(
            routes.contains("/api/v1/local-ai/runtime/update"),
            "runtime update must route through Caduceus HTTP"
        );
        assert!(
            !routes.contains("homeconsole-local-ai-update"),
            "runtime update must not call Harmonia directly from Arcadia"
        );
        assert!(
            include_str!("../../src/bands/routes_ai_models.rs").contains("runtime.installed"),
            "runtime update must prove installed state after Harmonia runs"
        );
        let runtime_routes = include_str!("../../src/bands/routes_ai_runtime.rs");
        assert!(runtime_routes.contains("llama.cpp is not installed. Run Update llama.cpp before enabling inference."));
        assert!(runtime_routes.contains("llama.cpp is not installed. Run Update llama.cpp before exposing Local AI on LAN."));
    }


    #[test]
    fn local_ai_version_readback_uses_stderr_aware_command() {
        let runtime = include_str!("../../src/bands/local_ai_runtime.rs");
        assert!(runtime.contains("command_combined_output(LLAMA_SERVER_BIN"));
        assert!(runtime.contains("String::from_utf8_lossy(&output.stderr)"));
        assert!(runtime.contains("(Some(_), None) => \"unknown\""));
    }


    #[test]
    fn local_ai_inference_controls_require_model_and_listener_before_green() {
        let runtime_routes = include_str!("../../src/bands/routes_ai_runtime.rs");
        assert!(runtime_routes.contains("No GGUF model is installed. Import or download a model before enabling inference."));
        assert!(runtime_routes.contains("Local inference server is not running. Load a model before exposing Local AI on LAN."));
        assert!(runtime_routes.contains("ai.installed_models.is_empty()"));
        assert!(runtime_routes.contains("tcp_port_listening(cfg_now.lan_port)"));
        assert!(runtime_routes.contains("let bind_host = if cfg.lan_enabled { \"0.0.0.0\" } else { \"127.0.0.1\" };"));
        assert!(include_str!("../../src/bands/local_ai_config.rs").contains("mode=direct-llama-server-lan-bind"));
        assert!(APP_JS.contains("lanPort: payload.port"));
        assert!(APP_JS.contains("Port must be between 1024 and 65535."));
        assert!(include_str!("../../src/bands/routes_ai_settings.rs").contains("body.lan_port"));
    }
