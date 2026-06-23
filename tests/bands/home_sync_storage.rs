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
            canonical_url: "http://console.home.arpa/".to_string(),
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
                .expect("sync view follows home");
        let home_html = &rendered[home_start..home_end];

        for required in [
            "priority-strip",
            "home-operational-grid",
            "home-operational-grid--dashboard",
            "load-orb",
            "load-spark-bank",
            "data-load-card",
            r#"data-load-retry-ms="5000""#,
            "data-load-orb",
            "data-load-headline",
            "data-load-spark-value",
            "data-load-chip-value",
            "CPU",
            "I/O",
            "storage-bar",
            r#"aria-label="Storage""#,
            r#"aria-label="Load""#,
            r#"aria-label="Game library""#,
            r#"aria-label="Network""#,
            "home-topology",
            "Console URL",
            r#"aria-label="AI Model""#,
            "local-ai-home-card",
            "local-ai-home-details",
            ">Model:</span>",
            ">Load:</span>",
            ">State:</span>",
            r#"data-label="Available ROMs""#,
            r#"aria-label="Updates""#,
            ">Storage</h3>",
            ">Load</h3>",
            ">Games</h3>",
            ">Network</h3>",
            ">Updates</h3>",
            ">AI Model</h3>",
        ] {
            assert!(home_html.contains(required), "missing {required}");
        }

        assert!(!home_html.contains("<h1"));
        assert!(!home_html.contains("<h2"));
        assert!(!home_html.contains("<p"));

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
            "modules ·",
            "Machine ",
            "health-home-card",
            "identity-home-card",
            ">Health</h3>",
            ">Appliance</h3>",
            r#"aria-label="System Health""#,
            r#"aria-label="Appliance""#,
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
        assert!(APP_CSS.contains(".load-home-card"));
        assert!(APP_CSS.contains(".load-orb"));
        assert!(APP_CSS.contains(".load-spark"));
        assert!(APP_CSS.contains(".load-telemetry-grid"));
        assert!(APP_CSS.contains("conic-gradient"));
        assert!(APP_JS.contains("function bindHomeLoadSubscription()"));
        assert!(APP_JS.contains("window.arcadiaHomeLoadSubscriptionState = state"));
        assert!(APP_JS.contains("new EventSource('/api/root/events')"));
        assert!(APP_JS.contains("fetch('/api/root/events/renew'"));
        assert!(APP_JS.contains("JSON.stringify({ leaseId: state.lease.leaseId })"));
        assert!(APP_JS.contains("source.addEventListener('snapshot', onRoot)"));
        assert!(APP_JS.contains("source.addEventListener('root', onRoot)"));
        assert!(APP_JS.contains("source.addEventListener('lease', onLease)"));
        assert!(APP_JS.contains("source.addEventListener('heartbeat', onHeartbeat)"));
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
        assert!(!APP_JS.contains("function bindHomeLoadPolling()"));
        assert!(!APP_JS.contains("window.arcadiaHomeLoadPollState"));
        assert!(!APP_JS.contains("setInterval(poll, pollMs)"));
        assert!(!APP_JS.contains("window.arcadiaHomeLoadPolling"));
        assert!(APP_CSS.contains(".view[data-view-panel=\"home\"].is-active"));
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
        for forbidden in ["Game Library", "Detected", ">Synced<", "GPU", "Folders unavailable"] {
            assert!(!home_html.contains(forbidden), "unbacked home claim survived: {forbidden}");
        }
    }

    #[test]
    fn home_view_updates_available_and_service_states_are_truthful() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
        assert!(home_html.contains("Update available"));
        assert!(!home_html.contains("homeconsole-latest/run.json"));
        assert!(!home_html.contains(">Receipt<"));
        assert!(home_html.contains(">✓<"));
        assert!(home_html.contains("aria-label=\"Game Session\""));
        assert!(home_html.contains(">!<"));
        assert!(!home_html.contains(">Unknown<"));
        assert!(!home_html.contains("GameScope"));
        assert!(!home_html.contains(">Arcadia<"));
    }

    #[test]
    fn home_updates_card_surfaces_harmonia_check_and_module_readiness() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
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
            "homeconsole-latest/run.json",
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

        let last_ran_prefix = ">Last ran:</span><strong>";
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
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
            ">AI Model</h3>",
            ">Model:</span><strong>Hermes 8B</strong>",
            ">Load:</span><strong>Hot</strong>",
            ">State:</span><strong>Actively working</strong>",
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
    fn home_view_has_no_idle_readiness_indicator() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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

        assert!(!home_html.contains("priority-strip"));
        assert!(!home_html.contains(">Ready<"));
        assert!(!home_html.contains("All systems current."));
    }

    #[test]
    fn local_ai_language_replaces_ai_model_jargon() {
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
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
            "sync-folder-source",
            "Samba",
            "Samba for Windows",
            "Samba for Linux",
            r#"\\console.home.arpa\games"#,
            r#"\\HOMECONSOLE\games"#,
            "smb://console.home.arpa/games",
            "smb://homeconsole/games",
            "/home/owner/Games",
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
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = false;
        status.library.last_sync_state = "never".to_string();
        status.library.last_sync = "Never".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
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
        assert!(before.contains("Add games"));
        assert!(before.contains("Sync games"));
        assert!(!before.contains("/home/owner/Games"));
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
        assert!(!during.contains("GameScope"));
        assert!(!during.contains("games folders"));

        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.total_detected_games = 3;
        status.library.total_synced_entries = 3;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let after_rendered = ui::layout(&status).into_string();
        let after = sync_slice(&after_rendered);
        assert!(after.contains("Games current"));
        assert!(after.contains("Sync orb"));
        assert!(after.contains("3 admitted"));
        assert!(after.contains("Check again"));
        assert!(after.contains(r#"data-sync-debt="none""#));
        assert!(!after.contains("Games synced. No new games found."));
        assert!(!after.contains("0 new · 0 failed"));
        assert!(!after.contains("Receipt ready"));
        assert!(!after.contains("Needs first sync"));
        assert!(!after.contains("Sync failed"));
        assert!(!after.contains("/home/owner/Games"));
    }

    #[test]
    fn sync_library_admission_board_is_premium_collection_surface() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.total_detected_games = 4;
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
            "Games current",
            "3 admitted · 1 missing artwork",
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
            ".sync-orb-stage",
            "conic-gradient",
            ".sync-system-blade",
            ".sync-admitted-shelf",
            ".sync-game-card",
            ".sync-cover-frame",
        ] {
            assert!(APP_CSS.contains(required_css), "missing admission board CSS: {required_css}");
        }
    }

    #[test]
    fn sync_orb_waiting_number_is_actual_admission_debt() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.total_detected_games = 100;
        status.library.total_synced_entries = 99;
        status.library.unsynced_added = 1;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(sync_html.contains("1 games waiting"));
        assert!(sync_html.contains(r#"data-sync-orb-state="waiting""#));
        assert!(sync_html.contains(r#"data-sync-debt="admission""#));
        assert!(sync_html.contains(r#"<span class="ux-sync-orb-core">1</span>"#));
        assert!(!sync_html.contains(r#"<span class="ux-sync-orb-core">100</span>"#));
    }

    #[test]
    fn sync_orb_separates_admission_debt_from_artwork_caveat() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.total_synced_entries = 12;
        status.library.artwork_paired_total = 8;
        status.library.artwork_missing = 4;
        status.library.unsynced_added = 0;
        status.library.unsynced_changed = 0;
        status.library.unsynced_removed = 0;
        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(sync_html.contains("Games current"));
        assert!(sync_html.contains("12 admitted · 4 missing artwork"));
        assert!(sync_html.contains(r#"data-sync-debt="none""#));
        assert!(sync_html.contains(r#"data-beauty-debt="caveat""#));
        assert!(sync_html.contains(r#"data-sync-orb-state="caveat""#));
        assert!(sync_html.contains("Check again"));
        assert!(!sync_html.contains("Needs sync"));
        assert!(!sync_html.contains("Sync needed"));
    }

    #[test]
    fn sync_complaint_and_eject_states_are_appliance_actions_not_output_or_path_prompts() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
        assert!(complaint.contains("1 games waiting"));
        assert!(complaint.contains("Attention"));
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
        assert!(eject.contains("games waiting"));
        assert!(eject.contains("Ejected"));
        assert!(!eject.contains("Sync needed"));
        assert!(!eject.contains("2 games were ejected and need attention"));
        assert!(!eject.contains("/home/owner/Games"));
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
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let mut status = console_status(&state);
        status.library.first_sync_completed = true;
        status.library.last_sync_state = "success".to_string();
        status.library.last_sync = "Receipt found".to_string();
        status.library.sync_state = "idle".to_string();
        status.library.sync_needed = false;
        status.library.total_detected_games = 3;
        status.library.total_synced_entries = 3;
        status.library.artwork_complete = 2;
        status.library.artwork_missing = 1;
        status.library.artwork_status = "2 complete · 1 missing".to_string();

        let rendered = ui::layout(&status).into_string();
        let sync_html = sync_slice(&rendered);
        assert!(rendered.contains("Games: 3"));
        assert!(!sync_html.contains("Synced"));
        assert!(rendered.contains("Games: 3"));
        assert!(!sync_html.contains("Last sync complete"));
        assert!(sync_html.contains("Games current"));
        assert!(sync_html.contains("3 admitted"));
        assert!(sync_html.contains("Admitted"));
        assert!(sync_html.contains("Artwork missing"));

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
        assert!(!zero_sync.contains("/home/owner/Games"));
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
    fn sync_action_routes_through_caduceus_membrane() {
        let constants = include_str!("../../src/bands/constants.rs");
        let source = include_str!("../../src/bands/console_system_actions.rs");
        assert!(constants.contains("const CADUCEUS_BIN: &str = \"/usr/local/bin/caduceus\";"));
        assert!(source.contains("CADUCEUS_BIN"));
        assert!(source.contains("&[\"sync\", \"now\"]"));
        assert!(!source.contains("async fn action_sync_games() -> (StatusCode, Json<ConsoleActionResponse>) {\n    run_console_command(\n        \"sync-games\",\n        HARMONIA_BIN,"));
    }

    #[test]
    fn api_root_object_is_decomposable_infinite_infinite_tree() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
        assert!(telemetry["data"].get("load").is_some());
        assert!(telemetry["data"].get("io").is_some());
        assert!(telemetry["data"]["io"].get("pressureAvg10").is_some());
        let telemetry_metrics = telemetry["metrics"].as_array().expect("metrics");
        assert!(telemetry_metrics
            .iter()
            .any(|metric| metric["id"] == "cpuTemperatureCelsius"));
        assert!(telemetry_metrics
            .iter()
            .any(|metric| metric["id"] == "ioPressureAvg10"));
    }

    #[test]
    fn api_root_routes_are_registered() {
        let source = include_str!("../../src/main.rs");
        assert!(source.contains(".route(\"/api\", get(api_root_route))"));
        assert!(source.contains(".route(\"/api/root\", get(api_root_route))"));
        assert!(source.contains(".route(\"/api/root/events\", get(api_root_events_route))"));
        assert!(source.contains(".route(\"/api/root/events/renew\", post(api_root_events_renew_route))"));
        assert!(source.contains("include!(\"bands/api_root.rs\")"));
    }

    #[test]
    fn api_root_events_route_streams_sse_root_payloads() {
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
        assert!(source.contains("Event::default()\n            .event(\"snapshot\")"));
        assert!(source.contains("Event::default()\n                .event(\"root\")"));
        assert!(source.contains("Event::default().event(\"lease\")"));
        assert!(source.contains("Event::default().event(\"heartbeat\")"));
        assert!(source.contains("Event::default().event(\"expired\")"));
        assert!(source.contains("const HOME_TELEMETRY_CADENCE_SECONDS: u64 = 1;"));
        assert!(source.contains("tokio::time::interval(Duration::from_secs(HOME_TELEMETRY_CADENCE_SECONDS))"));
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
            "/home/owner/Games/roms/gba"
        );
        assert_eq!(
            game_system_storage_path("ps2").to_string_lossy(),
            "/home/owner/Games/isos/ps2"
        );
        assert_eq!(
            game_system_storage_path("dos").to_string_lossy(),
            "/home/owner/Games/pc/dos"
        );

        let registry = storage_registry(&network_status());
        let gba = registry
            .categories
            .games
            .roots
            .iter()
            .find(|root| root.id == "gba")
            .expect("gba root exists");
        assert_eq!(gba.path, "/home/owner/Games/roms/gba");
        assert_ne!(gba.path, "/home/owner/Games/gba");
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
            b"\x00AppName\x00Driven (GBA)\x00Exe\x00/usr/bin/retroarch\x00LaunchOptions\x00-L /usr/lib/libretro/mgba_libretro.so /home/owner/Games/roms/gba/Driven.gba\x00",
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
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
            "/home/owner",
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
    fn updates_view_is_harmonia_integration_with_module_controls() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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
            "Check state",
            "Make harmonious",
            "data-harmonia-module-menu=\"true\"",
            "data-harmonia-module-grid=\"true\"",
            "data-harmonia-module=\"identity\"",
            "/var/lib/harmonia/receipts/homeconsole-latest/run.json",
            "/api/actions/check-updates",
            "/api/actions/update-gui",
        ] {
            assert!(updates_html.contains(required), "updates view missing {required}");
        }
        assert!(!updates_html.contains("Manual SCP bridge"));
        assert!(!updates_html.contains("Latest available</span><strong>Not checked"));
    }

    #[test]
    fn updates_view_exposes_paginated_harmonia_ledger() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
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

