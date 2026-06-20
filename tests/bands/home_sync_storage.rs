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
            "Storage",
            "storage-bar",
            "Artwork",
            "AI Models",
            "Other",
            "Network",
            "Manage Network",
            "Local AI",
            "Model",
            "LAN",
            "Sync",
            "Available ROMs",
            "0 available ROMs",
            "Last scan",
            "Manage Storage",
            "Browse Folders",
        ] {
            assert!(home_html.contains(required), "missing {required}");
        }

        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "Add Games",
            "Console Status",
            "Recent Activity",
            "home-action-tile",
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
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
        for forbidden in ["Game Library", "Detected", ">Synced<", "GPU", "Folders unavailable"] {
            assert!(!home_html.contains(forbidden), "unbacked home claim survived: {forbidden}");
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

    #[test]
    fn sync_view_is_reasonable_samba_ip_and_queue_surface() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let sync_start = rendered
            .find("<section id=\"view-sync\"")
            .expect("sync view starts");
        let sync_end = sync_start
            + rendered[sync_start..]
                .find("<section id=\"view-storage\"")
                .expect("storage follows sync");
        let sync_html = &rendered[sync_start..sync_end];

        for required in [
            r#"data-action="sync-games""#,
            "Sync now",
            "sync-folder-source",
            "Samba for Windows",
            "Samba for Linux",
            ">IP<",
            r#"\\console.home.arpa\games"#,
            "smb://console.home.arpa/games",
            "Queued",
            "Available ROMs",
            "GameScope",
            "/home/owner/Games",
            "games/gba",
            "sync-running-panel",
            "Queued",
        ] {
            assert!(sync_html.contains(required), "missing {required}");
        }
        for forbidden in [
            "home console",
            "smb://homeconsole/games",
            r#"\\HOMECONSOLE\games"#,
            "Sync ROMs to GameScope",
            "Scan ROM folders",
            "Open Games Folder",
            "Copy Windows path",
            "Copy Linux/macOS path",
            "Copy IP Windows path",
            "Copy IP SMB URL",
            "Tools and troubleshooting",
            "Configure Scrapers",
            "Output</button>",
            "Storage</button>",
            "<details",
            "Ready to scan",
            "Scanning Samba ROM folders",
            "Needs attention. Open Output",
            "Scan complete. GameScope entries and artwork were updated from the ROM folders.",
        ] {
            assert!(
                !sync_html.contains(forbidden),
                "rejected sync surface survived: {forbidden}"
            );
        }
        assert_eq!(
            sync_html.matches("data-copy-value=").count(),
            3,
            "sync view exposes exactly three copy buttons"
        );
        assert!(sync_html.contains("data-storage-health=\"OK\""));
        assert!(VIEWPORT_CSS.contains("prefers-reduced-motion"));
        assert!(APP_CSS.contains("sync-scan-dot"));
        assert!(!sync_html.contains("provider-keys-form"));
        assert!(!sync_html.contains("screenscraper_api_key"));
        assert!(APP_JS.contains("Scanning"));
        assert!(!APP_JS.contains("Scanning Samba ROM folders"));
        assert!(!APP_JS.contains("Sync complete. Playable GameScope entries were updated from the ROM folders."));
        assert!(!APP_JS.contains("Needs attention. Open Output"));
    }


    #[test]
    fn sync_no_history_never_claims_completed_or_synced() {
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

        let rendered = ui::layout(&status).into_string();
        let sync_start = rendered.find("<section id=\"view-sync\"").expect("sync view starts");
        let sync_end = sync_start + rendered[sync_start..].find("<section id=\"view-storage\"").expect("storage follows sync");
        let sync_html = &rendered[sync_start..sync_end];

        assert!(rendered.contains("Games: 0 ROMs"));
        assert!(rendered.contains("Not scanned · 0 ROMs"));
        assert!(rendered.contains("0 available ROMs"));
        assert!(sync_html.contains("Needs first sync"));
        assert!(sync_html.contains("First sync waiting"));
        assert!(sync_html.contains("No sync receipt yet."));
        assert!(sync_html.contains("/home/owner/Games"));
        assert!(!sync_html.contains("Completed"));
        assert!(!sync_html.contains("Scan complete"));
        assert!(!rendered.contains("Games: Synced"));
        assert!(!rendered.contains(r#"data-chip-kind="games""#));
    }

    #[test]
    fn sync_completed_and_zero_rom_states_are_truthful_fixtures() {
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
        assert!(rendered.contains("Games: 3 ROMs"));
        assert!(rendered.contains("Idle · 3 ROMs"));
        assert!(rendered.contains("3 available ROMs"));
        assert!(rendered.contains("Last sync complete"));
        assert!(rendered.contains("Available ROMs"));
        assert!(rendered.contains("GameScope"));
        assert!(rendered.contains("2 complete · 1 missing"));

        status.library.total_detected_games = 0;
        status.library.total_synced_entries = 0;
        status.library.artwork_complete = 0;
        status.library.artwork_missing = 0;
        status.library.artwork_status = "No artwork".to_string();
        let zero = ui::layout(&status).into_string();
        assert!(zero.contains("Games: 0 ROMs"));
        assert!(zero.contains("Idle · 0 ROMs"));
        assert!(zero.contains("0 available ROMs"));
        assert!(zero.contains("Last sync complete"));
        assert!(zero.contains("0 queued changes under /home/owner/Games."));
        assert!(zero.contains("Available ROMs"));
        assert!(!zero.contains("No ROMs"));
        assert!(!zero.contains("No scan has run yet"));
    }

    #[test]
    fn sync_action_uses_installed_harmonia_module_path() {
        assert_eq!(HOMECONSOLE_SYNC_MODULE, "/etc/harmonia/modules/homeconsole/sync/index.json");
        let source = include_str!("../../src/bands/console_system_actions.rs");
        assert!(source.contains("\"--module\",\n            HOMECONSOLE_SYNC_MODULE"));
        assert!(!source.contains("profiles/homeconsole/modules/sync/index.json"));
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
        let bytes = b"\x00\x01AppName\x00Driven (GBA)\x00\x01Exe\x00/usr/bin/retroarch\x00\x01LaunchOptions\x00-L core driven.gba\x00\x01AppName\x00Road Rash (Genesis)\x00\x01Exe\x00/usr/bin/retroarch\x00\x01LaunchOptions\x00-L core road-rash.md\x00";
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
            .contains("road-rash.md"));
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
            "AI Models",
            "Temporary Files",
            "System",
            "Rescan",
            "Managed locations",
            "Filesystem used",
            "Category scan",
            "Other / unclassified",
            "data-storage-modal=\"locations\"",
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

