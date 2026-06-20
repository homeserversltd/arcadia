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
            "view-network",
            "view-access-pin",
            "view-updates",
            "view-system",
        ] {
            assert!(rendered.contains(view), "missing {view}");
        }
        for indicator in [
            "GameScope",
            "Storage",
            "Sync",
            "Local AI",
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
            "data-chip-kind=\"games\"",
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
        assert!(!rendered.contains("\\\\HOMECONSOLE"));
        assert!(rendered.contains("Copy IP SMB URL"));
        assert!(rendered.contains("Trust &amp; HTTPS"));

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
        assert!(APP_JS.contains("lanPort: payload.port"));
        assert!(APP_JS.contains("Port must be between 1024 and 65535."));
        assert!(include_str!("../../src/bands/routes_ai_settings.rs").contains("body.lan_port"));
    }
