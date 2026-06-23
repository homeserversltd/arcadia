fn access_pin_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "access-pin",
        "Router-style access",
        "Access & PIN",
        "Router-style PIN management for the HomeConsole access gate.",
        html! {
            div class="access-pin-panel" {
                article class="access-pin-card access-pin-card--mode" data-module="gui-pin-access" {
                    div class="access-pin-state" {
                        span class="access-pin-icon" aria-hidden="true" { "●" }
                        div {
                            h3 { "Access mode" }
                            strong data-pin-mode-label="true" { (if status.gui_pin.pin_required { "PIN required" } else { "Open without PIN" }) }
                            p data-pin-mode-copy="true" { (if status.gui_pin.pin_required { "PIN required before accessing HomeConsole." } else { "HomeConsole opens without a PIN." }) }
                        }
                    }
                    label class="pin-toggle" {
                        input type="checkbox" role="switch" name="pin_required" data-pin-required-toggle="true" checked[status.gui_pin.pin_required];
                        span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                        span class="pin-toggle-label" { "Require PIN for console access" }
                    }
                }
                article class="access-pin-card access-pin-card--change" {
                    h3 { "Change access PIN" }
                    p { "Enter the current PIN and choose the new access PIN. Saved PIN values are never shown." }
                    form id="gui-pin-change-form" class="settings-form settings-form--pin" autocomplete="off" {
                        label { span { "Current PIN" } input class="field" type="password" name="current_pin" autocomplete="current-password" required; }
                        div class="pin-form-row" {
                            label { span { "New PIN" } input class="field" type="password" name="new_pin" autocomplete="new-password" required minlength="4"; }
                            label { span { "Confirm new PIN" } input class="field" type="password" name="confirm_pin" autocomplete="new-password" required minlength="4"; }
                        }
                        div id="gui-pin-change-message" class="message" hidden {}
                        button class="btn btn--primary" type="submit" { "Change access PIN" }
                    }
                }
                (settings_action_row(
                    "Default / reset PIN",
                    "Reset restores only the active access PIN to the configured factory/default value. Games, settings, storage, and the operating system stay unchanged.",
                    Some("!"),
                    Some(if status.gui_pin.default_reset_available { ("Default reset available", "system-status--ok") } else { ("Reset helper missing", "system-status--disabled") }),
                    html! {
                        button class="btn btn--danger" type="button" data-gui-pin-reset-default="true" disabled[!status.gui_pin.default_reset_available] { "Reset PIN to default" }
                        div id="gui-pin-reset-message" class="message" hidden {}
                    },
                    true,
                ))
            }
        },
    )
}

