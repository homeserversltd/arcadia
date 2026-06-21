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
            "Controller manager",
            "Controller",
            "Profiles",
            "RetroArch",
            "Dolphin",
            "DuckStation",
            "PCSX2",
            "PPSSPP",
            "data-action=\"controllers-rescan\"",
            "data-action=\"controllers-test\"",
            "data-action=\"controllers-save-profile\"",
            "data-action=\"controllers-assign-retroarch\"",
            "data-action=\"controllers-assign-dolphin\"",
            "data-action=\"controllers-assign-duckstation\"",
            "data-action=\"controllers-assign-pcsx2\"",
            "data-action=\"controllers-assign-ppsspp\"",
            "data-controller-live-input",
            "data-controller-mapping-editor=\"default\"",
            "data-controller-face",
            "data-controller-gamepad-programmer",
            "ux-gamepad-body",
            "ux-gamepad-stick",
            "ux-gamepad-dpad",
            "ux-gamepad-face-diamond",
            "ux-gamepad-shoulder",
            "controller-profile-card",
            "Live input",
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
        ] {
            assert!(!controllers_html.contains(forbidden), "lazy controller pane prose survived: {forbidden}");
        }
        assert!(APP_JS.contains("bindControllerLiveInput"));
        assert!(APP_JS.contains("/api/controllers/input"));
        assert!(APP_CSS.contains(".controllers-command-deck"));
        assert!(APP_CSS.contains(".emulator-controller-grid"));
        assert!(APP_CSS.contains(".controller-button-dot--active"));
        assert!(UX_CSS.contains(".ux-controller-silhouette"));
        assert!(UX_CSS.contains(".ux-gamepad-body"));
        assert!(UX_CSS.contains(".ux-gamepad-face-diamond"));
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
        assert!(controller_backend.contains("ControllerBindRequest"));
        assert!(APP_JS.contains("bindControllerProgramming"));
        assert!(APP_JS.contains("/api/actions/controllers-bind"));
    }


    #[test]
    fn controller_profile_writers_stage_every_known_emulator() {
        let root = std::env::temp_dir().join(format!("arcadia-controller-writer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut bindings = default_controller_bindings();
        upsert_binding(&mut bindings, "A", "button 9");
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
        let _ = std::fs::remove_dir_all(&root);
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
