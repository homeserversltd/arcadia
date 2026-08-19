use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    cpu_temperature_celsius, disk_io_counters, human_size, load_average, memory_usage,
    pressure_avg10_percent, AiModelStorageStatus, ButtonVariant, ConsoleStatus, GameSystemTally,
    StorageCategoryStatus,
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
            body data-ui-schema=(status.ui_contract.schema) data-gui-pin-required="true" data-vault-unlock-required=(status.vault.unlock_required) {
                (gui_pin_gate(status))
                (vault_unlock_gate(status))
                div id="app" class="app-shell" aria-hidden="true" {
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
                script src="/static/vendor/chart.umd.min.js" {}
                script src="/static/indra-observation.js" {}
                script src="/static/app.js" {}
            }
        }
    }
}
