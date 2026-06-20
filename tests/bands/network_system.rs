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
            "Storage",
            "Network",
            "Local AI",
            "Sync",
            "storage-bar",
        ] {
            assert!(home_html.contains(required), "missing home {required}");
        }
        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "home-action-tile",
            "Add Games",
            "Console Status",
            "Recent Activity",
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
    fn system_view_replaces_advanced_with_structured_support_panel() {
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
            "System",
            "Power",
            "Power &amp; Sessions",
            "Administration",
            "Restart Arcadia",
            "Full system reboot",
            "Power off appliance",
            "Game session only",
            "reboot-console",
            "shutdown-console",
            "Remote Access",
            "SSH service",
            "Hostname",
            "LAN IP address",
            "Username",
            "ssh owner@",
            "Enable SSH",
            "Disable SSH",
            "Copy SSH Command",
            "Authorized public key",
            "Install Public Key",
            "Trust &amp; HTTPS",
            "Root CA bundle",
            "Install Root CA",
            "HTTP Mode",
            "HTTPS with Home Root CA",
            "GameScope",
            "Samba",
            "Game Sync",
            "Local AI",
            "Local AI Inference",
            "Web GUI",
            "Restart",
            "Sync",
            "View",
            "Copy",
            "Download",
            "Root CA",
            "CA path",
            "Arcadia",
        ] {
            assert!(system_html.contains(required), "missing {required}");
        }

        let first_log_group = system_html.find(">Sync<").expect("sync group shown");
        let service_row = system_html.find("GameScope").expect("service row shown");
        assert!(
            service_row < first_log_group,
            "service rows precede event groups"
        );
        assert!(rendered.contains("data-nav-target=\"system\""));
        assert!(APP_JS.contains("if (view === 'advanced') view = 'system';"));
        assert!(APP_JS.contains("Restarting GameScope may close the active game session."));
        assert!(!rendered.contains("data-view=\"advanced\""));
        assert!(!rendered.contains(">Advanced<"));
        let forbidden = [
            ["Power", &format!("{} controls", "Console")].join(" "),
            "Expert Mode".to_string(),
            "Developer".to_string(),
            ["View", "Logs"].join(" "),
            ["Logs help", "diagnose problems"].join(" "),
            ["View technical", "console status"].join(" "),
            ["view", "heading"].join("-"),
            "Networking".to_string(),
            "Health for the console services".to_string(),
            "How the console is reached".to_string(),
            "Open local ports".to_string(),
            "Network status".to_string(),
            "Sync Log".to_string(),
            "Local AI Log".to_string(),
            "System Log".to_string(),
            "SSH is for direct technical access".to_string(),
            "Normal game management does not require SSH".to_string(),
            "Only enable SSH on a trusted home network".to_string(),
            "LAN Inference is intended only for trusted home networks".to_string(),
            "Runs the console gaming session".to_string(),
            "Shares game folders".to_string(),
            "Adds copied games".to_string(),
            "Loads the selected local AI model".to_string(),
            "Lets other home-network devices".to_string(),
            "Runs this management interface".to_string(),
        ];
        for forbidden in forbidden {
            assert!(
                !system_html.contains(&forbidden),
                "forbidden System label survived: {forbidden}"
            );
        }
    }

