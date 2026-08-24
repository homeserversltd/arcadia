    #[test]
    fn provider_env_uses_screenscraper_api_key_only() {
        let mut lines = Vec::new();
        let mut written = Vec::new();

        push_env_value(
            &mut lines,
            &mut written,
            "SCREENSCRAPER_API_KEY",
            Some("  scrape-secret  ".to_string()),
        );

        assert_eq!(written, vec!["SCREENSCRAPER_API_KEY"]);
        assert_eq!(lines, vec!["SCREENSCRAPER_API_KEY=\"scrape-secret\""]);
        let joined = lines.join("\n");
        assert!(!joined.contains("SCREENSCRAPER_USER"));
        assert!(!joined.contains("SCREENSCRAPER_PASSWORD"));
    }

    #[test]
    fn provider_status_response_is_redacted() {
        let response = ProviderKeysStatusResponse {
            ok: true,
            action: "provider-keys-status",
            path: PROVIDER_KEYS_PATH,
            providers: PROVIDER_KEY_NAMES
                .iter()
                .map(|(id, key)| ProviderKeyStatus {
                    id: *id,
                    env_key: *key,
                    configured: true,
                })
                .collect(),
            message: "Provider key status loaded without exposing secret values.".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("STEAMGRIDDB"));
        assert!(!json.contains("scrape-secret"));
        assert!(!json.to_ascii_lowercase().contains("password"));
    }

    #[test]
    fn network_status_parses_ip_addr_global_fallback() {
        let text =
            "2: enp1s0    inet 192.0.2.54/24 brd 192.0.2.255 scope global dynamic enp1s0\n";
        assert_eq!(
            parse_global_ipv4_address(text),
            Some("192.0.2.54".to_string())
        );
    }

    #[test]
    fn provider_keys_ui_exposes_api_key_not_username_password() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        assert!(!rendered.contains(r#"data-provider-keys-open="true""#));
        assert!(!rendered.contains(r#"id="provider-keys-form""#));
        assert!(!rendered.contains(r#"name="screenscraper_api_key""#));
        assert!(APP_JS.contains("ScreenScraper API key"));
        assert!(APP_JS.contains("screenscraper_api_key"));
        assert!(APP_JS.contains("/api/provider-keys/status"));
        assert!(APP_JS.contains("PopupManager.trapFocus"));
        assert!(!rendered.contains("screenscraper_user"));
        assert!(!rendered.contains("screenscraper_password"));
        assert!(!rendered.contains("ScreenScraper user"));
    }

    #[test]
    fn provider_keys_script_posts_screenscraper_api_key_only() {
        assert!(APP_JS.contains("screenscraper_api_key"));
        assert!(!APP_JS.contains("screenscraper_user"));
        assert!(!APP_JS.contains("screenscraper_password"));
    }

    #[test]
    fn ux_library_owns_viewport_contract() {
        assert!(UX_CSS.contains("Arcadia UX library"));
        assert!(UX_CSS.contains("--ux-shell-padding"));
        assert!(UX_CSS.contains("--ux-view-min-height"));
        assert!(UX_CSS.contains("var(--ux-header-min-height)"));
        assert!(VIEWPORT_CSS.contains("Arcadia viewport contract"));
        assert!(VIEWPORT_CSS.contains("@media (max-width: 980px)"));
        assert!(VIEWPORT_CSS.contains("@media (max-width: 720px)"));
        assert!(VIEWPORT_CSS.contains("overflow-y: auto"));
        assert!(
            APP_CSS
                .lines()
                .all(|line| !line.trim_start().starts_with("@media")),
            "static/app.css must not own viewport media bands"
        );
        assert!(APP_CSS.contains("height: 100vh"));
        assert!(APP_CSS.contains("Desktop appliance fit"));
        assert!(APP_CSS.contains(".ai-manager-section--desktop-detail"));
    }

    #[test]
    fn desktop_fit_defers_heavy_detail_from_default_panes() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();

        for marker in [
            "ai-manager-section--desktop-detail",
            "system-redo",
            "sync-desktop-detail",
        ] {
            assert!(
                rendered.contains(marker),
                "missing desktop deferral marker {marker}"
            );
            assert!(
                APP_CSS.contains(marker),
                "desktop CSS does not defer {marker}"
            );
            assert!(
                VIEWPORT_CSS.contains(marker),
                "mobile CSS does not restore {marker}"
            );
        }
    }

    #[test]
    fn theme_system_is_rendered_and_served_through_unified_assets() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();
        let css_bundle = format!("{}\n{}\n{}\n{}", THEME_CSS, UX_CSS, APP_CSS, VIEWPORT_CSS);
        let js_bundle = format!("{}\n{}", THEME_JS, APP_JS);

        for theme in [
            "crown-noir",
            "ember-aubergine",
            "forge-slate",
            "orchard-terminal",
        ] {
            assert!(THEME_CSS.contains(&format!("data-theme=\"{}\"", theme)));
            assert!(THEME_JS.contains(&format!("name: {:?}", theme)));
        }
        assert!(rendered.contains("data-theme-cycle=\"true\""));
        assert!(rendered.contains("class=\"theme-name\""));
        assert!(APP_JS.contains("function initializeArcadiaTheme"));
        assert!(APP_JS.contains("localStorage.getItem('arcadia-theme')"));
        assert!(APP_CSS.contains("--aubergine-0: var(--theme-bg-base)"));
        assert!(APP_CSS.contains("--orange: var(--theme-accent"));
        assert!(
            css_bundle.find("generated from static/themes").unwrap()
                < css_bundle.find("Arcadia UX library").unwrap()
        );
        assert!(
            css_bundle.find("Arcadia UX library").unwrap()
                < css_bundle.find("Arcadia appliance shell").unwrap()
        );
        assert!(
            css_bundle.find("Arcadia appliance shell").unwrap()
                < css_bundle.find("Arcadia viewport contract").unwrap()
        );
        assert!(
            js_bundle.find("window.ARCADIA_THEMES").unwrap()
                < js_bundle.find("function arcadiaThemeNames").unwrap()
        );
    }

    #[test]
    fn header_currentness_chips_have_lucide_icons_tooltips_and_theme_control() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();

        for marker in [
            "data-chip-kind=\"network\"",
            "data-chip-kind=\"sync\"",
            "data-chip-kind=\"updates\"",
            "data-chip-kind=\"uptime\"",
            "data-chip-kind=\"local-ai\"",
            "data-chip-kind=\"pin\"",
        ] {
            assert!(rendered.contains(marker), "missing header marker {marker}");
        }
        assert!(rendered.matches("class=\"chip-icon\"").count() >= 7);
        assert!(rendered.contains("Toggle theme, current theme Ember Aubergine"));
        assert!(rendered.contains("data-tooltip=\"Toggle theme"));
        assert!(rendered.contains("<svg viewBox=\"0 0 24 24\""));
        assert!(APP_CSS.contains(".status-badge[data-tooltip]::before"));
        assert!(APP_CSS.contains(".status-badge--action:hover"));
        assert!(APP_JS.contains("button.dataset.tooltip = `Toggle theme; current theme"));
    }

    #[test]
    fn toasts_are_clickable_dismiss_controls() {
        assert!(APP_JS.contains("document.createElement('button')"));
        assert!(APP_JS.contains("dismiss notification"));
        assert!(APP_JS.contains("node.addEventListener('click', () => node.remove())"));
        assert!(APP_JS.contains("toast-dismiss"));
        assert!(APP_CSS.contains(".toast:focus-visible"));
        assert!(APP_CSS.contains("cursor: pointer"));
    }

    #[test]
    fn gui_pin_access_uses_toast_not_inline_success_popup() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.example.com/".to_string(),
            product: "HomeConsole".to_string(),
            living: Arc::new(ArcadiaLivingMachine::new()),
        };
        let rendered = ui::layout(&console_status(&state)).into_string();
        assert!(!rendered.contains("gui-pin-access-message"));
        assert!(!APP_JS.contains("setMessage('gui-pin-access-message'"));
        assert!(!APP_JS.contains("clearMessage('gui-pin-access-message'"));
        assert!(APP_JS
            .contains("PopupManager.showToast(data.message || (data.ok ? 'GUI PIN setting saved'"));
    }

    #[test]
    fn system_view_contains_the_single_access_section() {
        let state = AppState { started_unix: 0, canonical_url: "http://console.example.com/".to_string(), product: "HomeConsole".to_string(), living: Arc::new(ArcadiaLivingMachine::new()) };
        let rendered = ui::layout(&console_status(&state)).into_string();
        let system_start = rendered.find("<section id=\"view-system\"").expect("system view starts");
        let system_html = &rendered[system_start..];
        for required in ["system-access-panel", "Vault status", "Access mode", "Require PIN for console access", "Change access PIN", "Default / reset PIN", "Reset PIN to default", "Caduceus action", "data-pin-required-toggle", "role=\"switch\""] {
            assert!(system_html.contains(required), "missing {required}");
        }
        assert!(!rendered.contains("view-access-pin"));
        assert!(!rendered.contains("view-vault"));
        assert_eq!(system_html.matches("data-pin-required-toggle").count(), 1);
    }

    #[test]
    fn access_pin_script_persists_toggle_validates_change_and_confirms_reset() {
        assert!(APP_JS.contains("postJson('/api/gui-pin/access', { pin_required: nextRequired })"));
        assert!(APP_JS.contains("renderGuiPinMode(data.pin_required !== false)"));
        assert!(APP_JS.contains("New PIN confirmation does not match."));
        assert!(APP_JS.contains("New PIN must be at least 4 characters."));
        assert!(APP_JS
            .contains("postJson('/api/gui-pin/change', { current_pin: current, new_pin: next })"));
        assert!(APP_JS.contains(
            "window.confirm('Reset the access PIN to the console factory/default value?"
        ));
        assert!(APP_JS.contains("postJson('/api/gui-pin/reset-default', { confirm: 'RESET' })"));
    }

