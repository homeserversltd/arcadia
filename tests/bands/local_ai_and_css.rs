    #[test]
    fn local_ai_state_payload_matches_manager_contract() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
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
    fn appliance_shell_renders_required_viewports_and_no_vault_indicator() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
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
            "view-access-pin",
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
            "Access\nPIN",
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
            "Vault status",
        ] {
            assert!(
                !header_html.contains(forbidden),
                "developer/proof header item survived: {forbidden}"
            );
        }
        assert!(!rendered.contains("smb:://"));
        assert!(!rendered.contains("Vault"));
        assert!(rendered.contains("Secure Web Access"));

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
            canonical_url: "http://console.home.arpa/".to_string(),
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
    fn controllers_view_is_single_pane_for_controller_mapping() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
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
            "Game control",
            "Your button",
            "Button label style",
            "data-tooltip",
            "Game systems",
            "Push mapping",
            "data-controller-programmer-open",
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
            "ux-gamepad-body",
            "ux-gamepad-stick",
            "ux-gamepad-dpad",
            "ux-gamepad-face-diamond",
            "ux-gamepad-shoulder",
            "ux-gamepad-wing",
            "ux-gamepad-bridge",
            "ux-gamepad-left__stick",
            "ux-gamepad-right__stick",
            "controller-profile-card",
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
        ] {
            assert!(!controllers_html.contains(forbidden) && !rendered.contains(forbidden), "lazy controller pane prose survived: {forbidden}");
        }
        assert!(APP_JS.contains("bindControllerLiveInput"));
        assert!(APP_JS.contains("const panel = document.querySelector('[data-view-panel=\"controllers\"]')"));
        assert!(APP_JS.contains("openControllerModal"));
        assert!(APP_JS.contains("/api/controllers/input"));
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
        assert!(UX_CSS.contains(".ux-gamepad-body"));
        assert!(UX_CSS.contains("aspect-ratio: 2.35 / 1"));
        assert!(UX_CSS.contains(".ux-gamepad-wing--left"));
        assert!(UX_CSS.contains(".ux-gamepad-face-diamond"));
        assert!(APP_CSS.contains(".controller-pool-scroll"));
        assert!(APP_CSS.contains("repeat(auto-fill, minmax(168px, 1fr))"));
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
        assert!(APP_JS.contains("window.setInterval(async ()"));
        assert!(APP_JS.contains("}, intervalMs);"));
        assert!(APP_JS.contains("const intervalMs = 60;"));
        assert!(APP_JS.contains("data-controller-programmer-modal"));
        assert!(APP_JS.contains("data-controller-broadcast-toggle"));
        assert!(APP_JS.contains("/api/actions/controllers-select"));
        assert!(APP_JS.contains("controllerId"));
        assert!(APP_JS.contains("hydrateControllerBindings"));
        assert!(APP_JS.contains("updateControllerPoolSelection"));
        assert!(APP_JS.contains("openControllerModal"));
        assert!(VIEWPORT_CSS.contains(".controller-pool-scroll"));
        assert!(VIEWPORT_CSS.contains("max-height: 280px"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("arcadia.controller_library.v1"));
        assert!(include_str!("../../src/bands/controller_writers/tuple.rs").contains("controller_library"));
        assert!(controller_backend.contains("action_controllers_select"));
        assert!(include_str!("../../src/main.rs").contains("/api/actions/controllers-select"));
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

        for emulator in ["RetroArch", "Dolphin", "DuckStation", "PCSX2", "PPSSPP"] {
            let path = write_emulator_profile(emulator, "Virtual Arcadia Gamepad", "virtual0", &bindings, false, &root)
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

        let autoconfig = retroarch_autoconfig_from_bindings("Virtual Arcadia Gamepad", &bindings);
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
        assert!(
            include_str!("../../src/bands/routes_ai_models.rs").contains("homeconsole-local-ai-update"),
            "runtime update must call the real Harmonia Local AI transition"
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
