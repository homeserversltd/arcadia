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
                            p data-controller-primary-device="true" { "Every gamepad HomeConsole has seen. Select one, map its buttons, then push that layout to each game system." }
                        }
                        div class="controls-card__head-actions controllers-actions" {
                            span class=(format!("system-status system-status--{}", hero_class)) data-controller-pool-count="true" {
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

                section class="controls-recovery" aria-label="Controller recovery" data-controller-recovery hidden[status.controllers.recovery.state == "connected"] {
                    strong data-controller-recovery-title { (status.controllers.recovery.title) }
                    span data-controller-recovery-detail { (status.controllers.recovery.detail) }
                    em data-controller-recovery-action { (status.controllers.recovery.action) }
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

                template id="controller-tuner-template" {
                    div class="controller-tuner-modal" data-controller-tuner-modal data-controller-broadcast-ms="60" {
                        p class="controller-tuner-intro" {
                            "Adjust deadzone to ignore stick drift, and response for how snappy sticks feel in games. Move your sticks to preview."
                        }
                        p class="controls-live-device controller-tuner-device" data-controller-tuner-device { (status.controllers.primary_device) }
                        div class="controller-tuner-grid" {
                            (controller_tuner_stick_panel("left", "Left stick"))
                            (controller_tuner_stick_panel("right", "Right stick"))
                        }
                        footer class="controller-tuner-footer controllers-actions" {
                            button class="btn btn--primary" type="button" data-controller-tuner-apply { "Apply tuning" }
                            button class="btn btn--secondary" type="button" data-controller-tuner-reset { "Reset defaults" }
                            button class="btn btn--secondary" type="button" data-controller-tuner-cancel { "Cancel" }
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
                                p class="controls-map-recovery" data-controller-programmer-recovery hidden[status.controllers.recovery.state != "receiver-only"] { "Controller receiver found, but the console cannot read it as a gamepad yet. Restarting the console usually fixes this." }
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
                            button class="btn btn--secondary" type="button" data-controller-teach-start { "Teach me" }
                            button class="btn btn--secondary" type="button" data-controller-teach-skip hidden { "Skip" }
                            button class="btn btn--secondary" type="button" data-controller-teach-exit hidden { "Exit teach" }
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
        div class="ux-controller-silhouette ux-gamepad-stage" aria-label="Programmable gamepad face" data-controller-face data-controller-gamepad-programmer data-controller-mapping-editor="default" data-gamepad-layout-version="5" data-gamepad-layout-gap=(format!("{:.3}", GAMEPAD_LAYOUT_MIN_GAP)) {
            div class="ux-gamepad-status-chip" { (if connected { "Connected" } else { title_case_state_like(&status.controllers.state) }) }
            svg class="ux-gamepad-body ux-gamepad-body--anatomical" viewBox="0 0 1000 620" role="img" aria-label="Xbox-style programmable gamepad" {
                path class="ux-gamepad-shell" d="M146 154 C226 98 332 116 386 172 C418 205 582 205 614 172 C668 116 774 98 854 154 C925 205 960 333 943 446 C930 536 873 598 792 598 C733 598 706 548 676 493 C646 438 608 419 500 419 C392 419 354 438 324 493 C294 548 267 598 208 598 C127 598 70 536 57 446 C40 333 75 205 146 154 Z" {}
                path class="ux-gamepad-shell-highlight" d="M178 178 C254 136 334 150 390 204 C424 237 576 237 610 204 C666 150 746 136 822 178 C876 214 904 310 897 402 C880 354 844 321 794 309 C727 292 662 320 627 366 C594 343 553 332 500 332 C447 332 406 343 373 366 C338 320 273 292 206 309 C156 321 120 354 103 402 C96 310 124 214 178 178 Z" {}
                (gamepad_svg_shoulder("shoulder-l2", "L2", "Left trigger", binding_for("L2"), active.contains(&"L2"), "ux-gamepad-trigger ux-gamepad-trigger--left", 213, 105, 116, 42))
                (gamepad_svg_shoulder("shoulder-l1", "L1", "Left shoulder", binding_for("L1"), active.contains(&"L1"), "ux-gamepad-bumper ux-gamepad-bumper--left", 197, 154, 140, 32))
                (gamepad_svg_shoulder("shoulder-r2", "R2", "Right trigger", binding_for("R2"), active.contains(&"R2"), "ux-gamepad-trigger ux-gamepad-trigger--right", 671, 105, 116, 42))
                (gamepad_svg_shoulder("shoulder-r1", "R1", "Right shoulder", binding_for("R1"), active.contains(&"R1"), "ux-gamepad-bumper ux-gamepad-bumper--right", 664, 154, 140, 32))
                (gamepad_svg_stick("stick-left", "Left Stick X", binding_for("Left Stick X"), active.contains(&"Left Stick X") || active.contains(&"Axis 0"), 284, 261, 52, 284, 284))
                (gamepad_svg_dpad(binding_for("D-pad"), active.contains(&"D-pad")))
                (gamepad_svg_system("system-select", "Select", binding_for("Select"), active.contains(&"Select"), 420, 296, 78, 32, 459, 344))
                g class="ux-gamepad-guide" aria-hidden="true" {
                    circle cx="500" cy="248" r="18" {}
                    text x="500" y="253" text-anchor="middle" { "H" }
                }
                (gamepad_svg_system("system-start", "Start", binding_for("Start"), active.contains(&"Start"), 502, 296, 78, 32, 541, 344))
                (gamepad_svg_face("face-y", "Y", "Y button", binding_for("Y"), active.contains(&"Y"), "ux-gamepad-face--y", 732, 241, 732, 289))
                (gamepad_svg_face("face-x", "X", "X button", binding_for("X"), active.contains(&"X"), "ux-gamepad-face--x", 681, 292, 681, 340))
                (gamepad_svg_face("face-b", "B", "B button", binding_for("B"), active.contains(&"B"), "ux-gamepad-face--b", 783, 292, 783, 340))
                (gamepad_svg_face("face-a", "A", "A button", binding_for("A"), active.contains(&"A"), "ux-gamepad-face--a", 732, 343, 732, 391))
                (gamepad_svg_stick("stick-right", "Right Stick X", binding_for("Right Stick X"), active.contains(&"Right Stick X") || active.contains(&"Axis 3"), 608, 370, 52, 608, 393))
            }
        }
    }
}

fn gamepad_slot_class(base: &str, active: bool) -> String {
    format!("ux-gamepad-slot {}{}", base, if active { " is-active" } else { "" })
}

fn gamepad_hit_rect(x: i32, y: i32, width: i32, height: i32) -> Markup {
    html! { rect class="ux-gamepad-hit" x=(x) y=(y) width=(width) height=(height) rx="28" ry="28" {} }
}


fn gamepad_binding_label(binding: &str) -> String {
    binding
        .replace("button ", "B")
        .replace("axis ", "AX")
        .replace("hat 0", "Hat")
}

fn gamepad_svg_face(
    slot_id: &str,
    label: &str,
    name: &str,
    binding: &str,
    active: bool,
    class_name: &str,
    cx: i32,
    cy: i32,
    label_x: i32,
    binding_y: i32,
) -> Markup {
    html! {
        g class=(gamepad_slot_class(&format!("ux-gamepad-face {class_name}"), active)) data-gamepad-slot=(slot_id) data-controller-control=(label) role="button" tabindex="0" aria-label=(format!("{} mapped to {}", name, binding)) {
            (gamepad_hit_rect(cx - 38, cy - 38, 76, 92))
            circle class="ux-gamepad-button-well" cx=(cx) cy=(cy) r="27" {}
            circle class="ux-gamepad-button-cap" cx=(cx) cy=(cy) r="20" {}
            text class="ux-gamepad-label" x=(cx) y=(cy + 6) text-anchor="middle" { (label) }
            text class="ux-gamepad-binding" data-controller-binding-label="true" x=(label_x) y=(binding_y) text-anchor="middle" { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_svg_stick(slot_id: &str, control: &str, binding: &str, active: bool, cx: i32, cy: i32, r: i32, label_x: i32, binding_y: i32) -> Markup {
    let label = control.replace(" X", "").replace(" Y", "");
    html! {
        g class=(gamepad_slot_class("ux-gamepad-stick", active)) data-gamepad-slot=(slot_id) data-controller-control=(control) role="button" tabindex="0" aria-label=(format!("{} mapped to {}", control, binding)) {
            (gamepad_hit_rect(cx - r - 52, cy - r - 52, (r + 52) * 2, (r + 52) * 2 + 34))
            circle class="ux-gamepad-stick-well" cx=(cx) cy=(cy) r=(r) {}
            circle class="ux-gamepad-stick-ring" cx=(cx) cy=(cy) r=(r - 10) {}
            circle class="ux-gamepad-stick-cap" cx=(cx) cy=(cy) r=(r - 21) {}
            text class="ux-gamepad-label ux-gamepad-label--small" x=(cx) y=(cy + 7) text-anchor="middle" { (label) }
            text class="ux-gamepad-binding" data-controller-binding-label="true" x=(label_x) y=(binding_y) text-anchor="middle" { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_svg_dpad(binding: &str, active: bool) -> Markup {
    html! {
        g class=(gamepad_slot_class("ux-gamepad-dpad", active)) data-gamepad-slot="dpad" data-controller-control="D-pad" role="button" tabindex="0" aria-label=(format!("D-pad mapped to {}", binding)) {
            (gamepad_hit_rect(206, 332, 162, 162))
            path class="ux-gamepad-dpad-cross" d="M272 356 H304 V388 H336 V420 H304 V452 H272 V420 H240 V388 H272 Z" {}
            g data-controller-control="D-pad Up" role="button" tabindex="0" aria-label=(format!("D-pad Up mapped to {}", binding)) {
                (gamepad_hit_rect(272, 356, 32, 32))
            }
            g data-controller-control="D-pad Left" role="button" tabindex="0" aria-label=(format!("D-pad Left mapped to {}", binding)) {
                (gamepad_hit_rect(240, 388, 32, 32))
            }
            g data-controller-control="D-pad Right" role="button" tabindex="0" aria-label=(format!("D-pad Right mapped to {}", binding)) {
                (gamepad_hit_rect(304, 388, 32, 32))
            }
            g data-controller-control="D-pad Down" role="button" tabindex="0" aria-label=(format!("D-pad Down mapped to {}", binding)) {
                (gamepad_hit_rect(272, 420, 32, 32))
            }
            text class="ux-gamepad-label ux-gamepad-label--small" x="288" y="410" text-anchor="middle" { "D" }
            text class="ux-gamepad-binding" data-controller-binding-label="true" x="288" y="516" text-anchor="middle" { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_svg_system(slot_id: &str, label: &str, binding: &str, active: bool, x: i32, y: i32, width: i32, height: i32, label_x: i32, binding_y: i32) -> Markup {
    let pill_x = x + (width - 54) / 2;
    let pill_y = y + (height - 22) / 2;
    html! {
        g class=(gamepad_slot_class("ux-gamepad-system", active)) data-gamepad-slot=(slot_id) data-controller-control=(label) role="button" tabindex="0" aria-label=(format!("{} mapped to {}", label, binding)) {
            (gamepad_hit_rect(x - 8, y - 12, width + 16, height + 48))
            rect class="ux-gamepad-pill" x=(pill_x) y=(pill_y) width="54" height="22" rx="11" ry="11" {}
            text class="ux-gamepad-label ux-gamepad-label--tiny" x=(x + width / 2) y=(pill_y + 15) text-anchor="middle" { (label) }
            text class="ux-gamepad-binding" data-controller-binding-label="true" x=(label_x) y=(binding_y) text-anchor="middle" { (gamepad_binding_label(binding)) }
        }
    }
}

fn gamepad_svg_shoulder(slot_id: &str, label: &str, name: &str, binding: &str, active: bool, class_name: &str, x: i32, y: i32, width: i32, height: i32) -> Markup {
    let center_x = x + width / 2;
    let is_trigger = class_name.contains("trigger");
    let label_y = if is_trigger { y + 13 } else { y + height / 2 - 2 };
    let binding_y = if is_trigger { y + 34 } else { y + height + 18 };
    let radius = std::cmp::min(17, height / 2);
    html! {
        g class=(gamepad_slot_class(class_name, active)) data-gamepad-slot=(slot_id) data-controller-control=(label) role="button" tabindex="0" aria-label=(format!("{} mapped to {}", name, binding)) {
            (gamepad_hit_rect(x - 10, y - 12, width + 20, height + 54))
            rect class="ux-gamepad-shoulder-shape" x=(x) y=(y) width=(width) height=(height) rx=(radius) ry=(radius) {}
            text class="ux-gamepad-label ux-gamepad-label--small" x=(center_x) y=(label_y) text-anchor="middle" dominant-baseline="central" { (label) }
            text class="ux-gamepad-binding" data-controller-binding-label="true" x=(center_x) y=(binding_y) text-anchor="middle" { (gamepad_binding_label(binding)) }
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
            button class="controller-pool-card__forget" type="button" data-controller-forget=(entry.id) aria-label=(format!("Forget {}", entry.name)) { "×" }
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
            div class="controller-pool-card__actions" {
                button class="btn btn--secondary controller-pool-card__map" type="button" data-controller-programmer-open data-controller-id=(entry.id) { "Map" }
                button class="btn btn--secondary controller-pool-card__tune" type="button" data-controller-tuner-open data-controller-id=(entry.id) { "Tune" }
            }
        }
    }
}

fn controller_tuner_stick_panel(side: &str, label: &str) -> Markup {
    html! {
        section class="controller-tuner-stick" data-controller-tuner-stick=(side) {
            strong class="controller-tuner-stick__label" { (label) }
            label class="controller-tuner-field" {
                span class="controller-tuner-field__head" {
                    span { "Deadzone" }
                    output class="controller-tuner-field__value" data-controller-tuner-deadzone-value=(side) { "15%" }
                }
                input type="range" class="controller-tuner-range" min="0" max="40" value="15" data-controller-tuner-deadzone=(side) aria-label=(format!("{label} deadzone")) {}
                span class="controller-tuner-field__hint" { "Ignore small drift near center" }
            }
            label class="controller-tuner-field" {
                span class="controller-tuner-field__head" {
                    span { "Response" }
                    output class="controller-tuner-field__value" data-controller-tuner-sensitivity-value=(side) { "100%" }
                }
                input type="range" class="controller-tuner-range" min="10" max="100" value="100" data-controller-tuner-sensitivity=(side) aria-label=(format!("{label} response")) {}
                span class="controller-tuner-field__hint" { "Lower = softer; higher = snappier" }
            }
            div class="controller-tuner-preview" data-controller-tuner-preview=(side) aria-hidden="true" {
                span class="controller-tuner-preview__ring" {}
                span class="controller-tuner-preview__dot" data-controller-tuner-dot=(side) {}
            }
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

