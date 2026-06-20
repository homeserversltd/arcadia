    #[test]
    fn network_view_and_home_contract_follow_sidebar_boundary() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://arcadia.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-sync\"")
                .expect("sync follows home");
        let home_html = &rendered[home_start..home_end];
        for required in [
            "priority-strip",
            r#"aria-label="Storage""#,
            r#"aria-label="Network""#,
            r#"aria-label="Local AI""#,
            r#"aria-label="Game library""#,
            "storage-bar",
        ] {
            assert!(home_html.contains(required), "missing home {required}");
        }
        assert!(!home_html.contains("<h3"));
        assert!(!home_html.contains("<p"));
        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "home-action-tile",
            "Add Games",
            "Console Status",
            "Recent Activity",
            "data-nav-target=",
            "Manage Network",
            "Manage Storage",
            "Open Sync",
        ] {
            assert!(
                !home_html.contains(forbidden),
                "home duplicated navigation/meta: {forbidden}"
            );
        }

        assert!(rendered.contains("data-view=\"network\""));
        let network_start = rendered
            .find("id=\"view-network\"")
            .expect("network view starts");
        let network_end = network_start
            + rendered[network_start..]
                .find("<section id=\"view-access-pin\"")
                .expect("access follows network");
        let network_html = &rendered[network_start..network_end];
        for required in [
            "Online",
            "Active connection",
            "IP address",
            "Gateway",
            "DNS",
            "LAN",
            "Internet",
            "Wired LAN",
            "Details",
            "IP Settings",
            "Services",
            "Diagnostics",
            "http://arcadia.home.arpa",
        ] {
            assert!(
                network_html.contains(required),
                "missing network {required}"
            );
        }
        for forbidden in ["nmcli", "iwctl", "ip addr", "saved password"] {
            assert!(
                !network_html.contains(forbidden),
                "network leaked raw/secret term: {forbidden}"
            );
        }
        assert!(!network_html.contains("wifi-network-list"));
        assert!(!network_html.contains("data-network-connect-form"));
        assert!(!network_html.contains("Advanced IP Settings"));
        assert!(!network_html.contains("Folders"));
        assert!(!network_html.contains("Game Folders"));
        assert!(!network_html.contains("Test Game Folders"));
        assert!(APP_JS.contains("function openWifiNetworkPicker"));
        assert!(APP_JS.contains("function normalizeWifiNetworks"));
        assert!(APP_JS.contains("input.type = event.target.checked ? 'text' : 'password'"));
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
        assert!(!home_html.contains("Game Library"));
        assert!(!rendered.contains(r#"data-chip-kind="games""#));
        assert!(rendered.contains(r#"data-chip-kind="sync""#));
    }

    #[test]
    fn network_state_payload_matches_appliance_contract() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let payload = network_state(&state);
        let json = serde_json::to_string(&payload).expect("network state serializes");
        for required in [
            "appliance",
            "activeConnection",
            "ethernet",
            "wifi",
            "services",
            "webConsole",
            "lanInference",
            "savedNetworks",
            "scanResults",
        ] {
            assert!(
                json.contains(required),
                "missing network payload field {required}"
            );
        }
        assert!(!json.contains("smb:://"));
        assert!(!json.contains("smb:/homeconsole"));
        assert!(!json.contains("\\undefined"));
        assert!(!json.contains("smb://undefined"));
    }

    #[test]
    fn system_view_is_an_appliance_front_panel_not_a_backend_dump() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let system_start = rendered
            .find("id=\"view-system\"")
            .expect("system view starts");
        let system_html = &rendered[system_start..];

        assert!(rendered.contains("data-view=\"system\""));
        assert!(!rendered.contains("data-view=\"power\""));
        assert!(!rendered.contains("id=\"view-power\""));
        for required in [
            "Power &amp; Sessions",
            "Restart Console",
            "Shut Down",
            "Restart Interface",
            "Restart Game Session",
            "Remote Access",
            "Login policy",
            "Copy Command",
            "Trusted public key",
            "Add Trusted Key",
            "Secure Web Access",
            "Home Root CA bundle",
            "Install Home Root CA",
            "System Health",
            "Game Session",
            "Game Folders",
            "Game Sync",
            "Local AI",
            "HomeConsole Interface",
            "Diagnostics",
            "Open Diagnostics",
        ] {
            assert!(system_html.contains(required), "missing {required}");
        }
        assert!(
            system_html.contains("Turn On Remote Access")
                || system_html.contains("Turn Off Remote Access"),
            "remote access must expose exactly one next action"
        );
        assert!(
            system_html.contains("Allow Password Login")
                || system_html.contains("Require Key Login"),
            "login policy must expose exactly one next action"
        );
        assert!(
            system_html.contains("Enable Secure Web Access")
                || system_html.contains("Use Local HTTP"),
            "trust mode must expose exactly one next action"
        );
        for action in [
            "restart-arcadia",
            "restart-gamescope",
            "enable-ssh",
            "disable-ssh",
            "enable-ssh-password",
            "disable-ssh-password",
            "trust-mode-http",
            "trust-mode-https",
        ] {
            assert!(
                system_html.matches(&format!("data-action=\"{action}\"")).count() <= 1,
                "duplicated System action {action}"
            );
        }
        for forbidden in [
            "Enable SSH",
            "Disable SSH",
            "Enable SSH Password",
            "Disable SSH Password",
            "Trust &amp; HTTPS",
            "HTTP Mode",
            "HTTPS with Home Root CA</button>",
            "GameScope",
            "Samba",
            "Local AI Inference",
            "Web GUI",
            ">Arcadia<",
            ">Restart</button>",
            ">View</button>",
            ">Download</button>",
            "data-modal-body=\"\"",
            "data-copy-value=\"\"",
            "Expert Mode",
            "Developer",
        ] {
            assert!(
                !system_html.contains(forbidden),
                "forbidden System substrate survived: {forbidden}"
            );
        }
        assert!(rendered.contains("data-nav-target=\"system\""));
        assert!(APP_JS.contains("if (view === 'advanced') view = 'system';"));
        assert!(APP_JS.contains("Restarting GameScope may close the active game session."));
        assert!(!rendered.contains("data-view=\"advanced\""));
        assert!(!rendered.contains(">Advanced<"));
    }

