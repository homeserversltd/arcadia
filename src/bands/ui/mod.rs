use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    cpu_temperature_celsius, disk_io_counters, enabled_home_update_modules, human_size,
    load_average, memory_usage, pressure_avg10_percent, AiModelStorageStatus, ButtonVariant,
    ConsoleStatus, GameSystemTally, StorageCategoryStatus,
};

const VIEWS: [(&str, &str, &str); 8] = [
    ("home", "⌂", "Home"),
    ("sync", "↻", "Sync"),
    ("storage", "▰", "Storage"),
    ("local-ai", "◉", "Local AI"),
    ("controllers", "◈", "Controls"),
    ("network", "◌", "Network"),
    ("updates", "⬆", "Updates"),
    ("system", "⚙", "System"),
];

include!("shell.rs");
include!("home.rs");
include!("sync.rs");
include!("storage.rs");
include!("local_ai.rs");
include!("gamepad_layout.rs");
include!("controllers.rs");
include!("network.rs");
include!("updates.rs");
include!("system.rs");
include!("primitives.rs");

pub fn layout(status: &ConsoleStatus) -> Markup {
    face_document("console", Some(status))
}

pub fn face_document(stage: &str, status: Option<&ConsoleStatus>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "HomeConsole / Arcadia" }
                script { (theme_boot_script()) }
                link rel="stylesheet" href="/static/app.css";
            }
            body data-face-stage=(stage)
                data-ui-schema=(status.map(|status| status.ui_contract.schema).unwrap_or(""))
                data-gui-pin-required=(status.is_some_and(|status| status.gui_pin.pin_required))
                data-vault-unlock-required=(status.is_some_and(|status| status.vault.unlock_required)) {
                (face_stage_root(stage, status))
                script src="/static/indra-observation.js" {}
                script src="/static/app.js" {}
            }
        }
    }
}

pub fn face_fragment(stage: &str, status: Option<&ConsoleStatus>) -> Markup {
    face_stage_root(stage, status)
}

fn face_stage_root(stage: &str, status: Option<&ConsoleStatus>) -> Markup {
    html! {
        div id="face-stage-root" data-face-stage=(stage)
            data-ui-schema=(status.map(|status| status.ui_contract.schema).unwrap_or(""))
            data-gui-pin-required=(status.is_some_and(|status| status.gui_pin.pin_required))
            data-vault-unlock-required=(status.is_some_and(|status| status.vault.unlock_required)) {
            @if stage == "vault" {
                (vault_unlock_gate())
            } @else if stage == "pin" {
                (gui_pin_gate())
            } @else if let Some(status) = status {
                (console_surface(status))
            }
        }
    }
}

fn console_surface(status: &ConsoleStatus) -> Markup {
    html! {
        div id="app" class="app-shell" {
            (header(status))
            div class="workspace" {
                (sidebar_launcher())
                main class="viewport" aria-live="polite" {
                    (home_view(status))
                    (sync_view(status))
                    (storage_view(status))
                    (ai_model_view(status))
                    (controllers_view(status))
                    (network_view(status))
                    (updates_view(status))
                    (system_view(status))
                }
            }
        }
        (modal_root())
    }
}
