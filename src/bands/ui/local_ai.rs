fn ai_model_view(status: &ConsoleStatus) -> Markup {
    let selected_name = status
        .local_ai
        .selected_model_name
        .as_deref()
        .unwrap_or("No model selected");
    let loaded_name = status
        .local_ai
        .loaded_model_name
        .as_deref()
        .unwrap_or("No model loaded");
    let port = status.local_ai.lan_inference_port.unwrap_or(7777);
    let endpoint = format!(
        "{}:{}",
        status.identity.web_origin.trim_end_matches('/'),
        port
    );
    let base_url = format!("{}/v1", endpoint);
    let api_ready = status.local_ai.lan_inference_enabled;
    let model_count = status.local_ai.available_models.len();
    let library_model_count = status.local_ai.library_models.len();
    let selected_present = status.local_ai.selected_model_id.is_some();
    let model_loaded = matches!(
        status.local_ai.load_state.as_str(),
        "hot" | "loaded" | "running"
    ) || status.local_ai.loaded_model_id.is_some()
        || status.local_ai.loaded_model_name.is_some();
    let model_state =
        local_ai_model_state_label(status, model_loaded, selected_present, model_count);
    let model_state_class =
        local_ai_state_class(&status.local_ai.load_state, model_loaded, model_count);
    let access_state = if api_ready {
        "Trusted LAN enabled"
    } else {
        "Console only"
    };
    let next_action = if model_count == 0 {
        "Import a GGUF model"
    } else if !selected_present {
        "Select a model"
    } else if !model_loaded {
        "Load the selected model"
    } else if !api_ready {
        "Test or enable API access"
    } else {
        "Copy the client endpoint"
    };
    view_shell(
        "local-ai",
        "",
        "",
        "",
        html! {
            section class=(format!("local-ai-hero local-ai-hero--{}", model_state_class)) aria-label="Local AI status" data-ai-auto-refresh="true" data-bind-class="localAiPane.hero.stateClass" data-state=(model_state_class) {
                div class="local-ai-orb" aria-hidden="true" { "◉" }
                div class="local-ai-hero-copy" {
                    span { "Local AI" }
                    strong data-ai-model-state="true" data-bind="localAiPane.hero.headline" { (if model_loaded { "Model loaded and serving" } else { "No model serving" }) }
                    p { span data-bind="localAiPane.hero.nextAction" { (next_action) } " · " span data-bind="localAiPane.hero.endpoint" { (if api_ready { endpoint.as_str() } else { "No active endpoint" }) } }
                }
                div class="local-ai-hero-endpoint" data-bind-class="localAiPane.hero.endpointState" data-state=(if api_ready { "available" } else { "disabled" }) {
                    span { "Client endpoint" }
                    code id="ai-endpoint-readback" data-ai-endpoint=(if api_ready { endpoint.as_str() } else { "" }) data-bind="localAiPane.hero.endpoint" {
                        (if api_ready { endpoint.as_str() } else { "No active endpoint" })
                    }
                    div class="local-ai-actions" {
                        @if api_ready { (copy_button("Copy endpoint", &endpoint)) }
                        @else { button class="btn btn--secondary" type="button" disabled title="Load a model and start the API before copying an endpoint." { "Copy endpoint" } }
                    }
                }
            }

            div class="local-ai-workbench" aria-label="Local AI controls" {
                section class="local-ai-section ai-manager-section local-ai-card local-ai-card--model" aria-label="Model control" data-bind-class="localAiPane.model.stateClass" data-state=(model_state_class) {
                    div class="local-ai-card-head" {
                        strong { "Model control" }
                        span class=(format!("system-status system-status--{}", model_state_class)) data-bind="localAiPane.model.state" { (model_state) }
                    }
                    div class="local-ai-card-body local-ai-state-list" {
                        (ai_state_tile_bound("Selected", selected_name, if selected_present { "Ready to load" } else { "Choose or import a model" }, "selected", "localAiPane.model.selected", "localAiPane.model.selectedDetail"))
                        (ai_state_tile_bound("Serving now", loaded_name, if model_loaded { "Available for client calls" } else { "No model invoked" }, "loaded", "localAiPane.model.servingNow", "localAiPane.model.servingDetail"))
                        (ai_state_tile_bound("Model library", &format!("{} library", library_model_count), if library_model_count == 0 { "Empty" } else { "Plain list ready" }, "library", "localAiPane.model.libraryCount", "localAiPane.model.libraryDetail"))
                        @if let Some(accelerator) = status.local_ai.gpu_memory.as_deref() { (ai_state_tile("Accelerator", accelerator, "Read from backend telemetry", "accelerator")) }
                        @if model_count == 0 {
                            div class="local-ai-empty" { strong { "No GGUF model installed" } p { "Import a model file or fetch a compatible Hugging Face GGUF before enabling client access." } }
                        }
                    }
                    div class="inline-actions inline-actions--compact local-ai-actions local-ai-card-foot" {
                        @if status.local_ai.available_models.is_empty() { (nav_focus_button("Import model", "local-ai", "local-ai-import")) }
                        @else if !selected_present { (nav_focus_button("Choose model", "local-ai", "installed-models")) }
                        @else if !model_loaded { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(status.local_ai.selected_model_id.as_deref().unwrap_or("")) { "Load model" } }
                        @if model_loaded { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } }
                        button class="btn btn--secondary" type="button" data-ai-logs="true" { "Open logs" }
                    }
                }

                section id="local-ai-inference" class="local-ai-section ai-manager-section local-ai-card local-ai-card--access" aria-label="API access" tabindex="-1" data-bind-class="localAiPane.access.stateClass" data-state=(if api_ready { "available" } else { "disabled" }) {
                    div class="local-ai-card-head" {
                        strong { "API access" }
                        span class=(format!("system-status system-status--{}", if api_ready { "available" } else { "disabled" })) data-bind="localAiPane.access.listeningBadge" { (if api_ready { "Listening" } else { "API NOT LISTENING" }) }
                    }
                    div class="local-ai-card-body local-ai-access-grid" {
                        (ai_state_tile_bound("Internal API", if api_ready { "Listening" } else { "Off" }, if api_ready { "Health test can run now" } else { "Load a model before client calls" }, "api", "localAiPane.access.internal", "localAiPane.access.internalDetail"))
                        (ai_state_tile_bound("LAN API", access_state, if api_ready { "Trusted LAN only" } else { "Disabled until explicitly enabled" }, "lan", "localAiPane.access.lan", "localAiPane.access.lanDetail"))
                        (ai_state_tile_bound("Port", &port.to_string(), "Saved HomeConsole Local AI port", "port", "localAiPane.access.port", ""))
                        (ai_state_tile_bound("OpenAI base URL", if api_ready { &base_url } else { "Unavailable" }, if api_ready { "Use this in clients" } else { "No base URL until API listens" }, "endpoint", "localAiPane.access.baseUrl", "localAiPane.access.baseUrlDetail"))
                    }
                    div class="inline-actions inline-actions--compact local-ai-actions local-ai-card-foot" {
                        button class="btn btn--primary" type="button" data-ai-action="inference-enable" disabled[model_count == 0] title=(if model_count == 0 { "Install a GGUF model before enabling API access." } else { "Enable console-local API mode." }) { "API on" }
                        button class="btn btn--secondary" type="button" data-ai-action="inference-disable" { "API off" }
                        button class="btn btn--secondary" type="button" data-ai-action="inference-test" { "Test API" }
                        @if api_ready { (copy_button("Copy base URL", &base_url)) } @else { button class="btn btn--secondary" type="button" disabled title="No API endpoint is reachable yet." { "Copy base URL" } }
                    }
                    p class="local-ai-help" { "Internal mode keeps the service on this console. LAN mode exposes only the saved port to trusted home-network clients." }
                }

                section class="local-ai-section ai-manager-section local-ai-card local-ai-card--config" aria-label="Port management" data-bind-class="localAiPane.port.stateClass" data-state=(if api_ready { "available" } else { "unknown" }) {
                    div class="local-ai-card-head" {
                        strong { "Port management" }
                        span class="system-status system-status--unknown" id="ai-port-state" data-active-port=(port) data-bind="localAiPane.port.activeBadge" { "Active " (port) }
                    }
                    form class="settings-form settings-form--inline local-ai-port-form" id="ai-lan-form" data-active-port=(port) {
                        div class="local-ai-card-body local-ai-port-fields" {
                            label { span { "LAN/API port" } input class="field" name="port" type="number" inputmode="numeric" min="1024" max="65535" value=(port) aria-describedby="ai-port-help"; }
                            label { span { "LAN CIDR" } input class="field" name="lanCidr" value="192.168.123.0/24" autocomplete="off" aria-describedby="ai-port-help"; }
                            p id="ai-port-help" class="local-ai-help" { "Save validates the port without exposing LAN. Enable LAN applies the saved port to trusted-home-LAN access." }
                        }
                        div class="inline-actions inline-actions--compact local-ai-actions local-ai-card-foot" {
                            button class="btn btn--primary" type="submit" data-ai-port-save="true" { "Save port" }
                            button class="btn btn--secondary" type="button" data-ai-port-revert="true" { "Revert" }
                            button class="btn btn--secondary" type="button" data-ai-action="lan-enable" disabled[!model_loaded] title=(if model_loaded { "Expose Local AI on the trusted LAN." } else { "Load a model before exposing LAN access." }) { "Enable LAN" }
                            button class="btn btn--secondary" type="button" data-ai-action="lan-disable" { "Disable LAN" }
                        }
                    }
                }
            }

            section id="local-ai-import" class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Model import" tabindex="-1" {
                form class="settings-form" id="ai-import-form" enctype="multipart/form-data" {
                    label { span { "Import GGUF model" } input class="field" type="file" name="model" accept=".gguf"; }
                    div class="inline-actions local-ai-actions" { button class="btn btn--primary" type="submit" { "Import model" } button class="btn btn--secondary" type="button" data-ai-action="models-rescan" { "Rescan storage" } }
                    progress id="ai-import-progress" max="100" value="0" hidden {}
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Model library" {
                (model_library_plain_dropdown(status))
                div class="model-grid" {
                    @if status.local_ai.available_models.is_empty() {
                        article class="model-card" { strong class="model-name" { "No models installed" } span class="model-filename" { "Import a local .gguf file or download a compatible Hugging Face file." } div class="inline-actions local-ai-actions" { (nav_focus_button("Import model", "local-ai", "local-ai-import")) (nav_focus_button("Get GGUF", "local-ai", "get-models")) } }
                    } @else {
                        @for model in &status.local_ai.available_models {
                            (installed_model_card(model, status))
                        }
                    }
                }
            }

            section id="get-models" class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Get GGUF" tabindex="-1" {
                article class="form-card" data-hf-installer="true" {
                    h3 { "Hugging Face GGUF" }
                    label { span { "Repository" } input class="field" name="repoId" placeholder="TheBloke/example-GGUF" autocomplete="off"; }
                    label { span { "File" } input class="field" name="filename" placeholder="example.Q4_K_M.gguf" autocomplete="off"; }
                    label { span { "Revision" } input class="field" name="revision" placeholder="main" autocomplete="off"; }
                    div class="inline-actions local-ai-actions" { button class="btn btn--secondary" type="button" data-ai-action="hf-list-files" { "Fetch files" } button class="btn btn--primary" type="button" data-ai-action="hf-download" { "Download" } }
                    div id="hf-file-results" class="diagnostics-results" {}
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Client handoff" {
                div class="system-field-grid" {
                    (system_field("Hermes/Pi base URL", if api_ready { &base_url } else { "Enable API first" }))
                    (system_field("Token", "Configured/redacted by backend"))
                    (system_field("Secret receipts", "Redacted"))
                }
                div class="inline-actions local-ai-actions" { button class="btn btn--secondary" type="button" data-ai-action="token-generate" { "Generate token" } button class="btn btn--danger" type="button" data-ai-action="token-revoke" { "Revoke token" } }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Settings" {
                form class="settings-form settings-form--inline" id="ai-settings-form" {
                    label { span { "Context" } input class="field" name="contextSize" type="number" value="4096" min="512" max="262144"; }
                    @if status.local_ai.gpu_memory.is_some() { label { span { "Accelerator layers" } input class="field" name="gpuLayers" type="number" value="-1" min="-1" max="999"; } }
                    label { span { "Threads" } input class="field" name="threads" type="number" value="0" min="0" max="256"; }
                    label { span { "Batch" } input class="field" name="batch" type="number" value="512" min="1" max="8192"; }
                    label class="model-choice" { input type="checkbox" name="startApiOnBoot"; span { "Start API on boot" } }
                    label class="model-choice" { input type="checkbox" name="autoLoadLastModel"; span { "Auto-load last model" } }
                    button class="btn btn--primary" type="submit" { "Save settings" }
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Hardware and storage" {
                @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                    (meter_block("Accelerator", &human_bytes(used), &human_bytes(total), used, total))
                } @else { div class="empty-state" { strong { "No accelerator telemetry source reported" } } }
                (meter_block("AI model storage", &status.storage.ai_models.size, &status.storage.free, status.storage.ai_models.bytes, status.storage.total_bytes.max(1)))
                div class="inline-actions local-ai-actions" { (nav_button("Open Storage", "storage")) }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Diagnostics" {
                div id="ai-activity" class="system-field-grid" { (system_field("Operation", if api_ready { "serving client calls" } else { "idle" })) (system_field("Last error", "Read from /api/ai/state")) }
                details class="collapsible-log" { summary { "llama.cpp update" } pre { code { "Read from backend logs." } } }
                details class="collapsible-log" { summary { "Model import/load" } pre { code { "Read from backend logs." } } }
                details class="collapsible-log" { summary { "Nginx/firewall" } pre { code { "Read from backend receipts." } } }
            }
            div id="ai-message" class="message" hidden {}
        },
    )
}

fn model_library_plain_dropdown(status: &ConsoleStatus) -> Markup {
    html! {
        article class="form-card model-library-plain" data-model-library-plain="true" {
            label {
                span { "All library models" }
                select class="field" name="modelLibraryPlain" data-model-library-select="true" autocomplete="off" {
                    @if status.local_ai.library_models.is_empty() {
                        option value="" { "No model-library entries found" }
                    } @else {
                        @for model in &status.local_ai.library_models {
                            option value=(model.id) {
                                (model.lane) " · " (model.name) " · " (model.status) " · " (model.size)
                            }
                        }
                    }
                }
            }
            div class="model-library-plain-readback" aria-label="Model library count" {
                (status.local_ai.library_models.len()) " total · includes untested and unseated library entries"
            }
        }
    }
}

fn local_ai_model_state_label(
    status: &ConsoleStatus,
    model_loaded: bool,
    selected_present: bool,
    model_count: usize,
) -> &'static str {
    if status.local_ai.load_state == "error" {
        "Backend error"
    } else if model_loaded {
        "Model loaded"
    } else if selected_present {
        "Model selected, not loaded"
    } else if model_count > 0 {
        "Models installed, none selected"
    } else {
        "No model loaded"
    }
}

fn local_ai_state_class(load_state: &str, model_loaded: bool, model_count: usize) -> &'static str {
    if load_state == "error" {
        "error"
    } else if model_loaded {
        "available"
    } else if model_count > 0 {
        "partial"
    } else {
        "disabled"
    }
}

fn ai_state_tile(label: &str, value: &str, detail: &str, kind: &str) -> Markup {
    html! {
        div class="local-ai-state-tile" data-ai-tile=(kind) {
            span { (label) }
            strong { (value) }
            em { (detail) }
        }
    }
}

fn ai_state_tile_bound(label: &str, value: &str, detail: &str, kind: &str, value_bind: &str, detail_bind: &str) -> Markup {
    html! {
        div class="local-ai-state-tile" data-ai-tile=(kind) {
            span { (label) }
            strong data-bind=(value_bind) { (value) }
            @if detail_bind.is_empty() { em { (detail) } }
            @else { em data-bind=(detail_bind) { (detail) } }
        }
    }
}

fn installed_model_card(model: &crate::LocalAiModelStatus, status: &ConsoleStatus) -> Markup {
    let selected = status.local_ai.selected_model_id.as_deref() == Some(model.id.as_str());
    let hot = status.local_ai.loaded_model_id.as_deref() == Some(model.id.as_str());
    html! { article id="installed-models" class=(if selected { "model-card model-card--selected" } else { "model-card" }) {
        strong class="model-name" { (model.name) @if model.is_recommended { " · Recommended" } }
        span class="model-filename" { (model.filename) }
        div class="model-meta-row" {
            (model_meta("Size", &model.size))
            (model_meta("Quantization", model.quantization.as_deref().unwrap_or("Unknown")))
            (model_meta("Use", model.recommended_use.unwrap_or("Balanced")))
            (model_meta("State", if hot { "Hot" } else if selected { "Cold" } else { "Installed" }))
        }
        div class="inline-actions inline-actions--compact local-ai-actions" {
            @if !selected { button class="btn btn--secondary" type="button" data-ai-action="model-select" data-model-id=(model.id) { "Select" } }
            @if !hot { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(model.id) { "Load" } }
            @if hot { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } span class="model-status" { "Unload before removing this model." } }
            @else { button class="btn btn--danger" type="button" data-ai-action="model-remove" data-model-id=(model.id) data-filename=(model.filename) { "Remove" } }
        }
    } }
}

fn meter_block(label: &str, used: &str, total: &str, used_bytes: u64, total_bytes: u64) -> Markup {
    html! { div class="storage-summary ai-meter" { div class="card-head" { h3 { (label) } strong { (used) " / " (total) } } div class="storage-bar" { span class="storage-segment storage-segment--ai" style=(format!("width: {}%", ((used_bytes.saturating_mul(100) / total_bytes.max(1)).min(100)))) {} } } }
}

fn human_bytes(bytes: u64) -> String {
    let gib = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{:.1} GB", gib)
    } else {
        format!("{} MB", bytes / 1024 / 1024)
    }
}

