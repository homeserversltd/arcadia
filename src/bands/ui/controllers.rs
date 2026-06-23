fn controllers_view(status: &ConsoleStatus) -> Markup {
    let connected = status.controllers.detected_count > 0;
    let hero_class = if connected {
        "available"
    } else if status.controllers.state == "receiver-only" {
        "starting"
    } else {
        "disabled"
    };
    view_shell(
        "controllers",
        "",
        "",
        "",
        html! {
            div class=(format!("controls-hub controllers-command-deck controllers-command-deck--{}", hero_class)) data-controller-state=(status.controllers.state) data-active-controller-id=(status.controllers.active_controller_id) {
                section class="controls-card controls-card--pool" aria-label="Your controllers" data-controller-pool {
                    div class="controls-card__head controls-card__head--pool" {
                        div {
                            strong { "Your controllers" }
                            p { "Every gamepad HomeConsole has seen. Select one, map its buttons, then push that layout to each game system." }
                        }
                        div class="controls-card__head-actions controllers-actions" {
                            span class=(format!("system-status system-status--{}", hero_class)) {
                                (format!("{} saved", status.controllers.controller_pool.len()))
                            }
                            (action_button(ButtonVariant::Secondary, "Scan", "controllers-rescan", "/api/actions/controllers-rescan"))
                        }
                    }
                    @if status.controllers.controller_pool.is_empty() {
                        p class="controls-pool-empty" {
                            (status.controllers.recovery.detail)
                            " Scan to add the first gamepad to your library."
                        }
                    } @else {
                        div class="controller-pool-scroll" {
                            @for entry in &status.controllers.controller_pool {
                                (controller_pool_card(entry))
                            }
                        }
                    }
                }

                section class="controls-card controls-card--games" aria-label="Game systems" {
                    div class="controls-card__head controls-card__head--games" {
                        div {
                            strong { "Game systems" }
                            p { "Push the selected controller's mapping into each emulator's live mapper." }
                        }
                    }
                    div class="emulator-controller-grid controls-games-grid" {
                        @for emulator in &status.controllers.emulators {
                            (emulator_controller_card(emulator, connected))
                        }
                    }
                }

                template id="controller-programmer-template" {
                    div class="controller-programmer-modal" data-controller-programmer-modal data-controller-id=(status.controllers.active_controller_id) data-controller-broadcast-ms="60" data-controller-rapid-fire-ms="60" {
                        div class="controller-map-stage" {
                            div class="controller-map-stage__pad" {
                                (controller_silhouette(status, connected))
                            }
                        }
                        div class="controller-map-workbench" {
                            section class="controller-map-bindings" aria-label="Your mappings" {
                                div class="controls-card__head controls-card__head--compact" {
                                    strong { "Your mappings" }
                                }
                                (controller_bindings_grid(&status.controllers.profile.bindings, connected))
                            }
                            section class="controller-map-presets" aria-label="Button label style" {
                                div class="controls-presets__head" {
                                    span class="controls-presets__label" { "Button label style" }
                                    p class="controls-presets__hint" { "How face buttons are named on the virtual pad. Your saved mappings stay put." }
                                }
                                div class="controller-profile-cards controls-preset-grid" {
                                    @for preset in &status.controllers.profile_presets {
                                        (controller_profile_preset_card(preset, connected))
                                    }
                                }
                            }
                            section class="controller-map-live" aria-label="Live preview" {
                                span class="system-status system-status--starting controller-map-bind-state" data-controller-programmer-state { "Tap a control on the gamepad to begin" }
                                p class="controls-live-device" data-controller-programmer-device { (status.controllers.live_input.device) }
                                div class="controller-axis-strip" data-controller-programmer-axes {}
                                p class="controls-map-hint" { "Press your controller to light up controls here." }
                                div class="controls-card__actions controllers-actions" {
                                    button class="btn btn--secondary" type="button" data-controller-broadcast-toggle { "Pause live preview" }
                                    output class="controls-map-readout" data-controller-broadcast-readout { "Live" }
                                }
                            }
                        }
                        footer class="controller-map-footer controllers-actions" {
                            (action_button(ButtonVariant::Primary, "Save layout", "controllers-save-profile", "/api/actions/controllers-save-profile"))
                            (modal_button(ButtonVariant::Secondary, "Help", "How controller mapping works", "HomeConsole remembers every gamepad it has seen.\n\n1. Pick a controller from Your controllers.\n2. Tap Map, choose a control on the virtual pad, then press the matching button on your real controller.\n3. Check Your mappings on the right — that is the before/after readout.\n4. Save layout, then use Push mapping on each game system."))
                        }
                    }
                }
            }
        },
    )
}

fn friendly_binding_label(binding: &str) -> String {
    if binding == "Waiting" || binding == "Tap to bind" {
        return binding.to_string();
    }
    if let Some(index) = binding.strip_prefix("button ") {
        let number = index.trim().parse::<u32>().unwrap_or(0).saturating_add(1);
        return format!("#{number}");
    }
    if let Some(index) = binding.strip_prefix("axis ") {
        return format!("AX{index}");
    }
    if binding.contains("hat") {
        return "D-pad".to_string();
    }
    binding.to_string()
}

fn controller_bindings_grid(bindings: &[crate::ControllerBindingStatus], connected: bool) -> Markup {
    html! {
        div class="controls-bindings" data-controller-mapping-editor="default" {
            @if bindings.is_empty() {
                p class="controls-bindings__empty" { "No mappings yet. Tap a control on the gamepad, then press the matching button on your controller." }
            } @else {
                @for binding in bindings {
                    button class="controls-binding-row" type="button" data-controller-control=(binding.control) data-controller-bind-row="true" aria-label=(format!("{} mapped to {}", binding.control, friendly_binding_label(&binding.binding))) {
                        span class="controls-binding-name" { (binding.control) }
                        span class="controls-binding-arrow" aria-hidden="true" { "→" }
                        span class="controls-binding-value" data-binding-control=(binding.control) {
                            (friendly_binding_label(if connected { &binding.binding } else { "Waiting" }))
                        }
                    }
                }
            }
        }
    }
}

fn controller_silhouette(status: &ConsoleStatus, connected: bool) -> Markup {
    let active = status
        .controllers
        .live_input
        .pressed
        .iter()
        .map(|b| b.control.as_str())
        .collect::<Vec<_>>();
    let binding_for = |control: &str| -> &str {
        status
            .controllers
            .profile
            .bindings
            .iter()
            .find(|binding| binding.control == control)
            .map(|binding| binding.binding.as_str())
            .unwrap_or(if connected { "Tap to bind" } else { "Waiting" })
    };
    html! {
        div class="ux-controller-silhouette ux-gamepad-stage" aria-label="Programmable gamepad face" data-controller-face data-controller-gamepad-programmer data-controller-mapping-editor="default" data-gamepad-layout-version="3" data-gamepad-layout-gap=(format!("{:.3}", GAMEPAD_LAYOUT_MIN_GAP)) data-gamepad-cublet-cols=(GAMEPAD_CUBLET_COLS) data-gamepad-cublet-rows=(GAMEPAD_CUBLET_ROWS) {
            div class="ux-gamepad-body ux-gamepad-body--cublet-grid" {
                div class="ux-gamepad-wing ux-gamepad-wing--left" aria-hidden="true" {}
                div class="ux-gamepad-wing ux-gamepad-wing--right" aria-hidden="true" {}
                div class="ux-gamepad-bridge" aria-hidden="true" {
                    span class="ux-gamepad-bridge__seam" aria-hidden="true" {}
                    span class="ux-gamepad-bridge__logo" aria-hidden="true" { "H" }
                }
                (gamepad_cublet("shoulder-l2", "ux-gamepad-cublet--l2", gamepad_control("L2", "Left trigger", binding_for("L2"), active.contains(&"L2"), "ux-gamepad-shoulder ux-gamepad-shoulder--trigger")))
                (gamepad_cublet("shoulder-l1", "ux-gamepad-cublet--l1", gamepad_control("L1", "Left shoulder", binding_for("L1"), active.contains(&"L1"), "ux-gamepad-shoulder")))
                div class="ux-gamepad-cublet ux-gamepad-cublet--status" aria-hidden="true" {
                    span class="ux-gamepad-status" { (if connected { "Connected" } else { title_case_state_like(&status.controllers.state) }) }
                }
                (gamepad_cublet("shoulder-r1", "ux-gamepad-cublet--r1", gamepad_control("R1", "Right shoulder", binding_for("R1"), active.contains(&"R1"), "ux-gamepad-shoulder")))
                (gamepad_cublet("shoulder-r2", "ux-gamepad-cublet--r2", gamepad_control("R2", "Right trigger", binding_for("R2"), active.contains(&"R2"), "ux-gamepad-shoulder ux-gamepad-shoulder--trigger")))
                (gamepad_cublet("stick-left", "ux-gamepad-cublet--lstk", gamepad_stick("Left Stick X", binding_for("Left Stick X"), active.contains(&"Left Stick X") || active.contains(&"Axis 0"))))
                (gamepad_cublet("dpad", "ux-gamepad-cublet--dpad", gamepad_dpad(active.contains(&"D-pad"), connected)))
                (gamepad_cublet("system-select", "ux-gamepad-cublet--select", gamepad_control("Select", "Select", binding_for("Select"), active.contains(&"Select"), "ux-gamepad-system")))
                div class="ux-gamepad-cublet ux-gamepad-cublet--home" aria-hidden="true" {
                    span class="ux-gamepad-home" { "⌂" }
                }
                (gamepad_cublet("system-start", "ux-gamepad-cublet--start", gamepad_control("Start", "Start", binding_for("Start"), active.contains(&"Start"), "ux-gamepad-system")))
                (gamepad_cublet("face-y", "ux-gamepad-cublet--face-y", gamepad_control("Y", "Y button", binding_for("Y"), active.contains(&"Y"), "ux-gamepad-face ux-gamepad-face--y")))
                (gamepad_cublet("face-x", "ux-gamepad-cublet--face-x", gamepad_control("X", "X button", binding_for("X"), active.contains(&"X"), "ux-gamepad-face ux-gamepad-face--x")))
                (gamepad_cublet("face-b", "ux-gamepad-cublet--face-b", gamepad_control("B", "B button", binding_for("B"), active.contains(&"B"), "ux-gamepad-face ux-gamepad-face--b")))
                (gamepad_cublet("face-a", "ux-gamepad-cublet--face-a", gamepad_control("A", "A button", binding_for("A"), active.contains(&"A"), "ux-gamepad-face ux-gamepad-face--a")))
                (gamepad_cublet("stick-right", "ux-gamepad-cublet--rstk", gamepad_stick("Right Stick X", binding_for("Right Stick X"), active.contains(&"Right Stick X") || active.contains(&"Axis 3"))))
            }
        }
    }
}

fn gamepad_cublet(slot_id: &str, area_class: &str, content: Markup) -> Markup {
    html! {
        div class=(format!("ux-gamepad-cublet {}", area_class)) data-gamepad-slot=(slot_id) {
            (content)
        }
    }
}

fn gamepad_binding_label(binding: &str) -> String {
    binding
        .replace("button ", "B")
        .replace("axis ", "AX")
        .replace("hat 0", "Hat")
}

fn gamepad_control(
    label: &str,
    name: &str,
    binding: &str,
    active: bool,
    class_name: &str,
) -> Markup {
    html! {
        button class=(format!("ux-gamepad-control {}{}", class_name, if active { " is-active" } else { "" })) type="button" data-controller-control=(label) aria-label=(format!("{} mapped to {}", name, binding)) {
            strong { (label) }
            span { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_stick(label: &str, binding: &str, active: bool) -> Markup {
    let display = label.replace(" X", "").replace(" Y", "");
    html! {
        button class=(if active { "ux-gamepad-stick is-active" } else { "ux-gamepad-stick" }) type="button" data-controller-control=(label) aria-label=(format!("{} mapped to {}", label, binding)) {
            span class="ux-gamepad-stick-cap" aria-hidden="true" {}
            strong { (display) }
            em { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_dpad(active: bool, connected: bool) -> Markup {
    let binding = if connected { "hat 0" } else { "Waiting" };
    html! {
        div class=(if active { "ux-gamepad-dpad is-active" } else { "ux-gamepad-dpad" }) aria-label=(format!("D-pad mapped to {}", binding)) data-controller-control="D-pad" {
            button type="button" class="ux-gamepad-dpad-arm ux-gamepad-dpad-arm--up" data-controller-control="D-pad Up" { "▲" }
            button type="button" class="ux-gamepad-dpad-arm ux-gamepad-dpad-arm--left" data-controller-control="D-pad Left" { "◀" }
            button type="button" class="ux-gamepad-dpad-center" tabindex="-1" { "D" }
            button type="button" class="ux-gamepad-dpad-arm ux-gamepad-dpad-arm--right" data-controller-control="D-pad Right" { "▶" }
            button type="button" class="ux-gamepad-dpad-arm ux-gamepad-dpad-arm--down" data-controller-control="D-pad Down" { "▼" }
            span { (gamepad_binding_label(binding)) }
        }
    }
}

fn controller_pool_card(entry: &crate::ControllerPoolEntry) -> Markup {
    let state_class = match entry.state.as_str() {
        "connected" => "available",
        _ => "starting",
    };
    let state_label = match entry.state.as_str() {
        "connected" => "Connected",
        _ => "Remembered",
    };
    html! {
        article class=(if entry.selected { "controller-pool-card controller-pool-card--selected" } else { "controller-pool-card" }) data-controller-id=(entry.id) data-controller-state=(entry.state) {
            button class="controller-pool-card__select" type="button" data-controller-select=(entry.id) aria-label=(format!("Select {}", entry.name)) {
                div class="controller-pool-card__head" {
                    span class="controller-pool-card__glyph" aria-hidden="true" { (entry.glyph) }
                    span class=(format!("controller-pool-card__status system-status system-status--{}", state_class)) { (state_label) }
                }
                strong class="controller-pool-card__name" { (entry.name) }
                span class="controller-pool-card__meta" {
                    (entry.transport) " · " (entry.layout_style) " · " (entry.tuple_count) " mapped"
                }
            }
            button class="btn btn--secondary controller-pool-card__map" type="button" data-controller-programmer-open data-controller-id=(entry.id) { "Map" }
        }
    }
}

fn controller_device_card(device: &crate::ControllerDeviceStatus) -> Markup {
    html! {
        button class="btn controller-device-card controller-device-card--action" type="button" data-controller-device=(device.handler) data-modal-title=(format!("{} input details", device.name)) data-modal-body=(format!("Handler: {}\nPath: {}", device.handler, device.path)) aria-label=(format!("Open {} controller details", device.name)) {
            div class="controller-device-glyph" aria-hidden="true" { (device.glyph) }
            div { strong { (device.name) } span { (device.transport) " · " (device.kind) " · " (device.state) } }
            span class="controller-card-action" { "Details" }
        }
    }
}

fn controller_profile_preset_card(
    preset: &crate::ControllerProfilePresetStatus,
    _enabled: bool,
) -> Markup {
    html! {
        button class=(if preset.state == "active" { "controller-profile-card controls-preset controller-profile-card--active" } else { "controller-profile-card controls-preset" }) type="button" data-controller-profile=(preset.name) data-controller-profile-action="apply" title=(preset.description) data-tooltip=(preset.description) aria-label=(format!("Use {} label style: {}", preset.name, preset.description)) {
            strong { (preset.name) }
            span { (preset.layout) }
            em { (if preset.state == "active" { "Active" } else { "Use" }) }
        }
    }
}

fn emulator_action_slug(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn emulator_short_tag(emulator: &str) -> &'static str {
    match emulator {
        "RetroArch" => "RA",
        "Dolphin" => "GC",
        "DuckStation" => "PS1",
        "PCSX2" => "PS2",
        "PPSSPP" => "PSP",
        _ => "EMU",
    }
}

fn emulator_controller_card(emulator: &crate::EmulatorControllerStatus, connected: bool) -> Markup {
    let state_class = match emulator.state.as_str() {
        "configured" => "available",
        "needs setup" => "starting",
        "not installed" => "disabled",
        _ => "disabled",
    };
    let status_label = match emulator.state.as_str() {
        "configured" => "Ready",
        "needs setup" => "Needs setup",
        "not installed" => "Not installed",
        other => other,
    };
    html! {
        article class="emulator-controller-card controls-game-card" data-emulator=(emulator.command) data-state=(emulator.state) {
            div class="controls-game-card__top" {
                span class="controls-game-card__tag" aria-hidden="true" { (emulator_short_tag(&emulator.emulator)) }
                div class="controls-game-card__copy" {
                    strong { (emulator.emulator) }
                    span { (emulator.profile) }
                }
                span class=(format!("controls-game-card__status system-status system-status--{}", state_class)) { (status_label) }
            }
            div class="emulator-controller-meter" aria-hidden="true" { span class=(format!("emulator-controller-meter-fill emulator-controller-meter-fill--{}", state_class)) {} }
            p class="emulator-profile-line" title=(format!("Config: {} · Mapping: {}", emulator.config_path, emulator.mapping_path)) { (emulator.profile) }
            div class="controls-game-card__actions controllers-actions" {
                @let action = format!("controllers-assign-{}", emulator_action_slug(&emulator.emulator));
                @let endpoint = format!("/api/actions/{}", action);
                @if emulator.state == "not installed" {
                    button class="btn btn--secondary" type="button" disabled data-action=(action) data-endpoint=(endpoint) title="This emulator is not installed on this console." { "Not installed" }
                } @else {
                    (action_button(ButtonVariant::Primary, "Push mapping", &action, &endpoint))
                }
            }
        }
    }
}

