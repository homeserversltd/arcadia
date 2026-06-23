fn controllers_view(status: &ConsoleStatus) -> Markup {
    let connected = status.controllers.detected_count > 0;
    let hero_class = if connected {
        "available"
    } else if status.controllers.state == "receiver-only" {
        "starting"
    } else {
        "disabled"
    };
    let device_glyph = if connected {
        status.controllers.devices[0].glyph.as_str()
    } else {
        "◈"
    };
    let transport = if connected {
        status.controllers.devices[0].transport.as_str()
    } else {
        "Waiting for gamepad"
    };
    view_shell(
        "controllers",
        "",
        "",
        "",
        html! {
            div class=(format!("controls-hub controllers-command-deck controllers-command-deck--{}", hero_class)) data-controller-state=(status.controllers.state) {
                header class=(format!("controls-status controls-status--{}", hero_class)) aria-label="Controller manager" {
                    div class="controls-status__identity" {
                        div class="controls-status__glyph" aria-hidden="true" { (device_glyph) }
                        div class="controls-status__copy" {
                            span class="controls-status__eyebrow" { "Gamepad" }
                            strong class="controls-status__title" { (status.controllers.primary_device) }
                            p class="controls-status__detail" {
                                (transport)
                                " · "
                                (status.controllers.recovery.detail)
                            }
                        }
                    }
                    div class="controls-status__badge" {
                        span class=(format!("system-status system-status--{}", hero_class)) {
                            (if connected { "Connected" } else { title_case_state_like(&status.controllers.state) })
                        }
                    }
                    div class="controls-status__actions controllers-actions" {
                        (action_button(ButtonVariant::Secondary, "Scan", "controllers-rescan", "/api/actions/controllers-rescan"))
                        @if connected {
                            (action_button(ButtonVariant::Secondary, "Test buttons", "controllers-test", "/api/actions/controllers-test"))
                        } @else {
                            button class="btn btn--secondary" type="button" disabled title="Connect a gamepad before testing buttons." { "Test buttons" }
                        }
                        (action_button(ButtonVariant::Primary, "Push to all games", "controllers-ramrod-all", "/api/actions/controllers-ramrod-all"))
                    }
                }

                div class="controls-main" {
                    section class="controls-card controls-card--layout" aria-label="Button layout" {
                        div class="controls-card__head" {
                            div {
                                strong { "Button layout" }
                                p { "One map for every game. Tap a control, then press it on your gamepad." }
                            }
                            button class="btn btn--primary" type="button" data-controller-programmer-open { "Map on gamepad" }
                        }
                        (controller_bindings_grid(&status.controllers.profile.bindings, connected))
                    }

                    section class="controls-card controls-card--live controllers-live-test" aria-label="Live input" data-controller-live-input {
                        div class="controls-card__head" {
                            strong { "Live input" }
                            span class=(format!("system-status system-status--{}", if status.controllers.live_input.state == "active" { "available" } else { "starting" })) data-controller-input-state {
                                (title_case_state_like(&status.controllers.live_input.state))
                            }
                        }
                        div class="controls-live-meter" aria-hidden="true" {
                            span class="controls-live-meter__ring" {}
                            span class="controls-live-meter__core" {}
                        }
                        p class="controls-live-device" data-controller-input-device=(status.controllers.live_input.device) {
                            (status.controllers.live_input.device)
                        }
                        div class="controller-axis-strip controls-live-feed" data-controller-axes {
                            @if status.controllers.live_input.axes.is_empty() && status.controllers.live_input.pressed.is_empty() {
                                span class="controls-live-idle" { "Move a stick or press a button to see activity here." }
                            }
                            @for axis in &status.controllers.live_input.axes {
                                span class="controller-axis-pill" { (axis.control) " " (axis.binding) }
                            }
                            @for pressed in &status.controllers.live_input.pressed {
                                span class="controller-axis-pill controller-button-dot--active" { (pressed.control) }
                            }
                        }
                        div class="controls-presets" aria-label="Layout presets" {
                            span class="controls-presets__label" { "Layout style" }
                            div class="controller-profile-cards controls-preset-grid" {
                                @for preset in &status.controllers.profile_presets {
                                    (controller_profile_preset_card(preset, connected))
                                }
                            }
                        }
                        div class="controls-card__actions controllers-actions" {
                            (action_button(ButtonVariant::Primary, "Save layout", "controllers-save-profile", "/api/actions/controllers-save-profile"))
                            (modal_button(ButtonVariant::Secondary, "Help", "How controller mapping works", "HomeConsole remembers one button layout for your gamepad.\n\n1. Choose a layout style (Xbox, Nintendo, PlayStation, or Arcade).\n2. Tap Map on gamepad or any button row, then press the matching control.\n3. Save layout, then Push to all games so RetroArch, Dolphin, and other emulators stay in sync."))
                        }
                    }
                }

                @if !connected {
                    aside class="controls-recovery" data-controller-recovery=(status.controllers.recovery.state) {
                        strong { (status.controllers.recovery.title) }
                        span { (status.controllers.recovery.detail) }
                        em { (status.controllers.recovery.action) }
                    }
                } @else if status.controllers.devices.len() > 1 {
                    section class="controls-devices" aria-label="Detected controllers" {
                        div class="controls-card__head" { strong { "Detected controllers" } }
                        div class="controller-device-list" {
                            @for device in &status.controllers.devices {
                                (controller_device_card(device))
                            }
                        }
                    }
                }

                section class="controls-card controls-card--games" aria-label="Game systems" {
                    div class="controls-card__head controls-card__head--games" {
                        div {
                            strong { "Game systems" }
                            p { "Your saved layout is translated for each emulator HomeConsole uses." }
                        }
                        (action_button(ButtonVariant::Primary, "Push to all games", "controllers-ramrod-all", "/api/actions/controllers-ramrod-all"))
                    }
                    div class="emulator-controller-grid controls-games-grid" {
                        @for emulator in &status.controllers.emulators {
                            (emulator_controller_card(emulator, connected))
                        }
                    }
                }

                template id="controller-programmer-template" {
                    div class="controller-programmer-modal" data-controller-programmer-modal data-controller-broadcast-ms="60" data-controller-rapid-fire-ms="60" {
                        (controller_silhouette(status, connected))
                        div class="controller-programmer-side" {
                            div class="controls-card__head" {
                                strong { "Live preview" }
                                span class="system-status system-status--starting" data-controller-programmer-state { "Listening" }
                            }
                            p class="controls-live-device" data-controller-programmer-device { (status.controllers.live_input.device) }
                            div class="controller-axis-strip" data-controller-programmer-axes {
                                @for axis in &status.controllers.live_input.axes {
                                    span class="controller-axis-pill" { (axis.control) " " (axis.binding) }
                                }
                            }
                            p class="controls-map-hint" { "Select a button on the gamepad, then press the matching control on your physical controller." }
                            div class="controls-card__actions" {
                                button class="btn btn--secondary" type="button" data-controller-broadcast-toggle { "Pause live preview" }
                                output class="controls-map-readout" data-controller-broadcast-readout { "Live" }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn friendly_binding_label(binding: &str) -> String {
    if binding.starts_with("button ") {
        format!("Button {}", binding.trim_start_matches("button "))
    } else if binding.starts_with("axis ") {
        format!("Stick axis {}", binding.trim_start_matches("axis "))
    } else if binding.contains("hat") {
        "D-pad".to_string()
    } else {
        binding.to_string()
    }
}

fn controller_bindings_grid(bindings: &[crate::ControllerBindingStatus], connected: bool) -> Markup {
    html! {
        div class="controls-bindings" data-controller-mapping-editor="default" {
            @if bindings.is_empty() {
                p class="controls-bindings__empty" { "No button layout saved yet. Connect a gamepad and tap Map on gamepad to begin." }
            } @else {
                @for binding in bindings {
                    button class="controls-binding-row" type="button" data-controller-control=(binding.control) data-controller-bind-row="true" aria-label=(format!("{} mapped to {}", binding.control, binding.binding)) {
                        span class="controls-binding-name" { (binding.control) }
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
        div class="ux-controller-silhouette ux-gamepad-stage" aria-label="Programmable gamepad face" data-controller-face data-controller-gamepad-programmer data-controller-mapping-editor="default" {
            div class="ux-gamepad-body" {
                div class="ux-gamepad-shell ux-gamepad-shell--left" aria-hidden="true" {}
                div class="ux-gamepad-shell ux-gamepad-shell--right" aria-hidden="true" {}
                div class="ux-gamepad-top-row" aria-label="Shoulder and trigger mapping" {
                    (gamepad_control("L2", "Left trigger", binding_for("L2"), active.contains(&"L2"), "ux-gamepad-shoulder ux-gamepad-shoulder--trigger"))
                    (gamepad_control("L1", "Left shoulder", binding_for("L1"), active.contains(&"L1"), "ux-gamepad-shoulder"))
                    div class="ux-gamepad-status" { (if connected { "Connected" } else { title_case_state_like(&status.controllers.state) }) }
                    (gamepad_control("R1", "Right shoulder", binding_for("R1"), active.contains(&"R1"), "ux-gamepad-shoulder"))
                    (gamepad_control("R2", "Right trigger", binding_for("R2"), active.contains(&"R2"), "ux-gamepad-shoulder ux-gamepad-shoulder--trigger"))
                }
                div class="ux-gamepad-left" {
                    (gamepad_stick("Left Stick X", binding_for("Left Stick X"), active.contains(&"Left Stick X") || active.contains(&"Axis 0")))
                    (gamepad_dpad(active.contains(&"D-pad"), connected))
                }
                div class="ux-gamepad-center" {
                    (gamepad_control("Select", "Select", binding_for("Select"), active.contains(&"Select"), "ux-gamepad-system"))
                    div class="ux-gamepad-home" aria-hidden="true" { "⌂" }
                    (gamepad_control("Start", "Start", binding_for("Start"), active.contains(&"Start"), "ux-gamepad-system"))
                }
                div class="ux-gamepad-right" {
                    div class="ux-gamepad-face-diamond" aria-label="Face button mapping" {
                        (gamepad_control("Y", "Y button", binding_for("Y"), active.contains(&"Y"), "ux-gamepad-face ux-gamepad-face--y"))
                        (gamepad_control("X", "X button", binding_for("X"), active.contains(&"X"), "ux-gamepad-face ux-gamepad-face--x"))
                        (gamepad_control("B", "B button", binding_for("B"), active.contains(&"B"), "ux-gamepad-face ux-gamepad-face--b"))
                        (gamepad_control("A", "A button", binding_for("A"), active.contains(&"A"), "ux-gamepad-face ux-gamepad-face--a"))
                    }
                    (gamepad_stick("Right Stick X", binding_for("Right Stick X"), active.contains(&"Right Stick X") || active.contains(&"Axis 3")))
                }
            }
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
        button class=(if preset.state == "active" { "controller-profile-card controls-preset controller-profile-card--active" } else { "controller-profile-card controls-preset" }) type="button" data-controller-profile=(preset.name) data-controller-profile-action="apply" aria-label=(format!("Apply {} layout", preset.name)) {
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
                (action_button(ButtonVariant::Secondary, if connected { "Sync one" } else { "Prepare" }, &action, &endpoint))
            }
        }
    }
}

