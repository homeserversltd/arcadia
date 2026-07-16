use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    cpu_temperature_celsius, cpu_usage_percent, disk_io_counters, human_size, load_average,
    pressure_avg10_percent, AiModelStorageStatus, ButtonVariant, ConsoleStatus, GameSystemTally,
    StorageCategoryStatus,
};

const VIEWS: [(&str, &str, &str); 9] = [
    ("home", "⌂", "Home"),
    ("sync", "↻", "Sync"),
    ("storage", "▰", "Storage"),
    ("local-ai", "◉", "Local AI"),
    ("controllers", "◈", "Controls"),
    ("network", "◌", "Network"),
    ("access-pin", "●", "Access\nPIN"),
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
include!("access_pin.rs");
include!("updates.rs");
include!("system.rs");
include!("primitives.rs");

pub fn layout(status: &ConsoleStatus) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (status.product) " / Arcadia" }
                script { (theme_boot_script()) }
                link rel="stylesheet" href="/static/app.css";
            }
            body data-ui-schema=(status.ui_contract.schema) data-gui-pin-required=(status.gui_pin.pin_required) {
                (gui_pin_gate(status))
                div id="app" class="app-shell" aria-hidden=(status.gui_pin.pin_required) {
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
                            (access_pin_view(status))
                            (updates_view(status))
                            (system_view(status))
                        }
                    }
                }
                (modal_root())
                script src="/static/indra-observation.js" {}
                script src="/static/app.js" {}
            }
        }
    }
}
