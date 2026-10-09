fn ai_model_view(status: &ConsoleStatus) -> Markup {
    // ConsoleStatus may carry discovery heuristics; persisted selection is localAiPane.model.selectedModelId.
    let selected_name = "Reading saved selection";
    let loaded_name = "Reading reported model";
    let port = status.local_ai.lan_inference_port;
    let port_label = port
        .map(|value| value.to_string())
        .unwrap_or_else(|| "Reading saved port".to_string());
    let model_count = status.local_ai.available_models.len();
    let library_model_count = status.local_ai.library_models.len();
    let selected_present = false;
    let model_loaded = false;
    let model_state_class = "unknown";
    let listener_state = "Reading saved access state";

    view_shell(
        "local-ai",
        "",
        "",
        "",
        html! {
            main id="local-ai-title" class="local-ai-page" aria-label="Local AI" data-ai-auto-refresh="true" {
                section class="local-ai-overview-grid ux-grid-3" aria-label="Local AI overview" {
                    article class="local-ai-card local-ai-card--model" aria-labelledby="local-ai-model-title" {
                        div class="local-ai-card-head" {
                            h2 id="local-ai-model-title" { "Model" }
                            span class="system-status" title="Backend-reported load state; this is not an API health check." aria-description="Backend-reported load state; this is not an API health check." data-bind="localAiPane.model.loadBadge" data-bind-class="localAiPane.model.stateClass" data-state=(model_state_class) { "Reading model state" }
                        }
                        dl class="local-ai-model-identity" aria-label="Selected and loaded models" {
                            div class="local-ai-model-identity-row" { dt { "Selected" } dd title="Persisted Local AI selection." aria-description="Persisted Local AI selection." data-bind="localAiPane.model.selected" { (selected_name) } }
                            div class="local-ai-model-identity-row" { dt { "Loaded" } dd title="Backend-reported loaded model; API health is not tested." aria-description="Backend-reported loaded model; API health is not tested." data-bind="localAiPane.model.servingNow" { (loaded_name) } }
                        }
                        div class="local-ai-actions ux-row-actions" {
                            button class="btn btn--primary" type="button" data-ai-action="model-load" data-bind-attr="data-model-id:localAiPane.model.selectedModelId" data-bind-enabled="localAiPane.model.hasSelectedModel" disabled[!selected_present] { "Load selected" }
                            button class="btn btn--secondary" type="button" data-ai-action="model-unload" data-bind-enabled="localAiPane.model.modelLoaded" disabled[!model_loaded] { "Unload" }
                            button class="btn btn--secondary" type="button" data-ai-modal="models" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Library" }
                        }
                    }

                    article class="local-ai-card local-ai-card--access" aria-labelledby="local-ai-access-title" {
                        div class="local-ai-card-head" {
                            h2 id="local-ai-access-title" { "API access" }
                            span class="system-status" title="TCP listener observation only; API health and remote reachability are not tested." aria-description="TCP listener observation only; API health and remote reachability are not tested." data-bind="localAiPane.access.state" data-bind-class="localAiPane.access.stateClass" data-state="unknown" { (listener_state) }
                        }
                        div class="local-ai-endpoint" data-bind-show="localAiPane.hero.endpointAvailable" hidden {
                            span class="local-ai-kicker" title="Configured URL only; remote reachability and API health are not tested." aria-description="Configured URL only; remote reachability and API health are not tested." { "Configured LAN URL" }
                            code data-bind="localAiPane.hero.endpoint" { "No configured LAN URL" }
                            button class="btn btn--secondary" type="button" data-copy-value="" data-bind-copy-value="localAiPane.access.baseUrl" data-bind-enabled="localAiPane.hero.endpointAvailable" disabled { "Copy base URL" }
                        }
                        div class="local-ai-actions ux-row-actions" {
                            button class="btn btn--primary" type="button" data-ai-action="inference-enable" { "API on" }
                            button class="btn btn--secondary" type="button" data-ai-action="inference-disable" { "API off" }
                            button class="btn btn--secondary" type="button" data-ai-action="inference-test" { "Check listener" }
                            button class="btn btn--secondary" type="button" data-ai-modal="access" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Configure access" }
                        }
                    }

                    article class="local-ai-card local-ai-card--library" aria-labelledby="local-ai-library-title" {
                        div class="local-ai-card-head" {
                            h2 id="local-ai-library-title" { "Library & settings" }
                            span class="local-ai-count" data-bind="localAiPane.model.libraryCount" { (format!("{} library", library_model_count)) }
                        }
                        div class="local-ai-actions ux-row-actions" {
                            button class="btn btn--primary" type="button" data-ai-modal="add" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Add model" }
                            button class="btn btn--secondary" type="button" data-ai-modal="customize" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Preferences" }
                        }
                    }
                }

                nav class="local-ai-support-rail ux-appliance-command-rail" aria-label="Additional Local AI details" {
                    button class="btn btn--secondary" type="button" data-ai-modal="resources" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Resources & engines" }
                    button class="btn btn--secondary" type="button" data-ai-modal="lanes" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Serving lanes" }
                    button class="btn btn--secondary" type="button" data-ai-logs="true" { "Activity & logs" }
                }
                div id="ai-message" class="message" hidden {}
            }

            div id="local-ai-modal-owner" class="local-ai-modal-owner" hidden {
                section class="local-ai-modal-section" data-ai-modal-section="models" aria-labelledby="local-ai-models-dialog-title" hidden {
                    h2 id="local-ai-models-dialog-title" data-modal-title hidden aria-hidden="true" { "Installed models" }
                    div class="local-ai-model-list" data-bind-each="localAiPane.installedModels" data-bind-replace="true" aria-label="Installed GGUF models" {
                        template {
                            article class="local-ai-model-row" data-bind-attr="data-selected:selected" {
                                div class="local-ai-model-copy" {
                                    strong class="local-ai-model-name" data-bind="name" {}
                                    span class="local-ai-model-filename" data-bind="filename" {}
                                    div class="local-ai-model-meta" {
                                        span { "Size " strong data-bind="size" {} }
                                        span { "Quantization " strong data-bind="quantization" {} }
                                        span { "Use " strong data-bind="recommendedUse" {} }
                                    }
                                }
                                div class="local-ai-actions ux-row-actions" {
                                    button class="btn btn--secondary" type="button" data-ai-action="model-select" data-bind-attr="data-model-id:id" data-bind-show="canSelect" { "Select" }
                                    button class="btn btn--primary" type="button" data-ai-action="model-load" data-bind-attr="data-model-id:id" data-bind-show="canLoad" { "Load" }
                                    button class="btn btn--secondary" type="button" data-ai-action="model-unload" data-bind-show="canUnload" { "Unload" }
                                    button class="btn btn--danger" type="button" data-ai-action="model-remove" data-bind-attr="data-model-id:id,data-filename:filename" data-bind-show="canRemove" { "Remove" }
                                }
                            }
                        }
                        @for model in &status.local_ai.available_models { (installed_model_card(model, status)) }
                    }
                    div class="local-ai-empty" data-bind-show="localAiPane.model.noInstalledModels" hidden[model_count != 0] {
                        strong { "No installed models" }
                        p { "Add a local GGUF or fetch a compatible file to begin." }
                    }
                    section class="local-ai-library-section" aria-labelledby="local-ai-library-list-title" {
                        h3 id="local-ai-library-list-title" { "Library entries" }
                        ul class="local-ai-library-list" data-bind-each="ai.libraryModels" data-bind-replace="true" aria-label="All model library entries" {
                            template {
                                li class="local-ai-library-row" {
                                    strong data-bind="name" {}
                                    span { span data-bind="lane" {} " · " span data-bind="status" {} " · " span data-bind="size" {} }
                                }
                            }
                            @for model in &status.local_ai.library_models {
                                li class="local-ai-library-row" {
                                    strong { (model.name) }
                                    span { (model.lane) " · " (model.status) " · " (model.size) }
                                }
                            }
                        }
                        div class="local-ai-empty" data-bind-show="localAiPane.model.noLibraryModels" hidden[library_model_count != 0] {
                            p { "No library entries are currently reported." }
                        }
                    }
                    div class="local-ai-actions ux-row-actions" {
                        button class="btn btn--primary" type="button" data-ai-modal="add" aria-expanded="false" aria-controls="local-ai-modal-owner" { "Add a model" }
                        button class="btn btn--secondary" type="button" data-ai-action="models-rescan" { "Rescan storage" }
                    }
                }

                section class="local-ai-modal-section" data-ai-modal-section="add" aria-labelledby="local-ai-add-title" hidden {
                    h2 id="local-ai-add-title" data-modal-title hidden aria-hidden="true" { "Add a model" }
                    article class="local-ai-disclosure-card" aria-labelledby="local-ai-import-title" {
                        h3 id="local-ai-import-title" { "Import from this console" }
                        form class="settings-form" id="ai-import-form" enctype="multipart/form-data" {
                            label { span { "GGUF file" } input class="field" type="file" name="model" accept=".gguf"; }
                            div class="local-ai-actions ux-row-actions" {
                                button class="btn btn--primary" type="submit" { "Import model" }
                                button class="btn btn--secondary" type="button" data-ai-action="models-rescan" { "Rescan storage" }
                            }
                            progress id="ai-import-progress" max="100" value="0" hidden {}
                        }
                    }
                    article class="local-ai-disclosure-card" data-hf-installer="true" aria-labelledby="local-ai-hf-title" {
                        h3 id="local-ai-hf-title" { "Hugging Face GGUF" }
                        label { span { "Repository" } input class="field" name="repoId" placeholder="TheBloke/example-GGUF" autocomplete="off"; }
                        label { span { "File" } input class="field" name="filename" placeholder="example.Q4_K_M.gguf" autocomplete="off"; }
                        label { span { "Revision" } input class="field" name="revision" placeholder="main" autocomplete="off"; }
                        div class="local-ai-actions ux-row-actions" {
                            button class="btn btn--secondary" type="button" data-ai-action="hf-list-files" { "Fetch files" }
                            button class="btn btn--primary" type="button" data-ai-action="hf-download" { "Download" }
                        }
                        div id="hf-file-results" class="local-ai-hf-results" data-bind-each="localAiFiles" data-bind-replace="true" aria-live="polite" {
                            template {
                                div class="local-ai-hf-row" {
                                    span class="local-ai-hf-file" data-bind="filename" {}
                                    span class="local-ai-hf-size" data-bind="sizeLabel" {}
                                    button class="btn btn--secondary" type="button" data-ai-hf-select="true" data-bind-attr="data-filename:filename" { "Use file" }
                                }
                            }
                        }
                    }
                }

                section class="local-ai-modal-section" data-ai-modal-section="access" aria-labelledby="local-ai-access-dialog-title" hidden {
                    h2 id="local-ai-access-dialog-title" data-modal-title hidden aria-hidden="true" { "Configure access" }
                    article class="local-ai-disclosure-card" aria-label="Listener and mode. Configured settings and listener observations are separate; remote reachability and API health are not tested." {
                        div class="local-ai-card-head" {
                            h3 { "Listener and mode" }
                            span class="system-status" data-bind="localAiPane.access.state" data-bind-class="localAiPane.access.stateClass" data-state="unknown" { (listener_state) }
                        }
                        dl class="local-ai-facts" {
                            div class="local-ai-fact" { dt { "Configured mode" } dd data-bind="localAiPane.access.mode" { "Read from Local AI settings" } }
                            div class="local-ai-fact" { dt { "LAN setting" } dd data-bind="localAiPane.access.lan" { "Read from Local AI settings" } }
                            div class="local-ai-fact" { dt { "Saved port" } dd data-bind="localAiPane.access.port" { (port_label) } }
                        }
                        p class="local-ai-note" data-bind="localAiPane.access.lanDetail" { "Remote reachability is not tested." }
                    }
                    form class="settings-form local-ai-port-form" id="ai-lan-form" data-active-port=(port.map(|value| value.to_string()).unwrap_or_default()) data-bind-attr="data-active-port:ai.inference.port" {
                        label {
                            span { "LAN/API port" }
                            input class="field" name="port" type="number" inputmode="numeric" min="1024" max="65535" value=(port.map(|value| value.to_string()).unwrap_or_default()) data-bind-value="ai.inference.port" data-bind-edit="draft" aria-describedby="ai-port-help";
                        }
                        p id="ai-port-help" class="local-ai-note" { "Saving changes the configured port; enabling LAN is a separate action." }
                        div class="local-ai-actions ux-row-actions" {
                            button class="btn btn--primary" type="submit" data-ai-port-save="true" { "Save port" }
                            button class="btn btn--secondary" type="button" data-ai-port-revert="true" { "Revert" }
                            button class="btn btn--secondary" type="button" data-ai-action="lan-enable" title="Expose Local AI on the trusted LAN" { "Enable LAN" }
                            button class="btn btn--secondary" type="button" data-ai-action="lan-disable" { "Disable LAN" }
                        }
                    }
                }

                section class="local-ai-modal-section" data-ai-modal-section="customize" aria-labelledby="local-ai-customize-title" hidden {
                    h2 id="local-ai-customize-title" data-modal-title hidden aria-hidden="true" { "Customize Local AI" }
                    details class="local-ai-disclosure" {
                        summary title="Saved values remain unchanged until you save the form." { "Model runtime settings" }
                        form class="settings-form local-ai-settings-grid" id="ai-settings-form" {
                            label { span { "Context size" } input class="field" name="contextSize" type="number" min="512" max="262144" value="" data-bind-value="ai.settings.contextSize" data-bind-edit="draft"; }
                            label { span { "Accelerator layers" } input class="field" name="gpuLayers" type="number" min="-1" max="999" value="" data-bind-value="ai.settings.gpuLayers" data-bind-edit="draft"; }
                            label { span { "Threads" } input class="field" name="threads" type="number" min="0" max="256" value="" data-bind-value="ai.settings.threads" data-bind-edit="draft"; }
                            label { span { "Batch size" } input class="field" name="batch" type="number" min="1" max="8192" value="" data-bind-value="ai.settings.batch" data-bind-edit="draft"; }
                            label class="local-ai-choice" { input type="checkbox" name="startApiOnBoot" data-bind-checked="ai.settings.startApiOnBoot" data-bind-edit="draft"; span { "Start API on boot" } }
                            label class="local-ai-choice" { input type="checkbox" name="autoLoadLastModel" data-bind-checked="ai.settings.autoLoadLastModel" data-bind-edit="draft"; span { "Auto-load last model" } }
                            div class="local-ai-actions ux-row-actions" { button class="btn btn--primary" type="submit" { "Save settings" } }
                        }
                    }
                    details class="local-ai-disclosure" {
                        summary { "Startup and selection" }
                        dl class="local-ai-facts" {
                            div class="local-ai-fact" { dt { "Preferred model" } dd data-bind="ai.settings.preferredModelId" { "None reported" } }
                            div class="local-ai-fact" { dt { "API startup" } dd data-bind="ai.settings.startApiOnBoot" { "Read from saved settings" } }
                            div class="local-ai-fact" { dt { "Last-model startup" } dd data-bind="ai.settings.autoLoadLastModel" { "Read from saved settings" } }
                        }
                    }
                }

                section class="local-ai-modal-section" data-ai-modal-section="resources" aria-labelledby="local-ai-resources-title" aria-description="Hardware telemetry and process observations are readbacks, not inference health checks." hidden {
                    h2 id="local-ai-resources-title" data-modal-title hidden aria-hidden="true" { "Resources & engines" }
                    dl class="local-ai-facts local-ai-resource-facts" {
                        div class="local-ai-fact" { dt { "Accelerator memory" } dd data-bind="status.local_ai.gpu_memory" { (status.local_ai.gpu_memory.as_deref().unwrap_or("No telemetry reported")) } }
                        div class="local-ai-fact" {
                            dt { "Accelerator use / total" }
                            dd {
                                span data-bind="status.local_ai.gpu_memory_used_bytes" data-bind-format="bytes" { (status.local_ai.gpu_memory_used_bytes.map(human_bytes).unwrap_or_else(|| "-".to_string())) }
                                " / "
                                span data-bind="status.local_ai.gpu_memory_total_bytes" data-bind-format="bytes" { (status.local_ai.gpu_memory_total_bytes.map(human_bytes).unwrap_or_else(|| "-".to_string())) }
                            }
                        }
                        div class="local-ai-fact" { dt { "Model storage" } dd data-bind="status.storage.ai_models.size" { (status.storage.ai_models.size) } }
                        div class="local-ai-fact" { dt { "Storage capacity" } dd data-bind="status.storage.total_bytes" data-bind-format="bytes" { (human_bytes(status.storage.total_bytes)) } }
                        div class="local-ai-fact" { dt { "Free storage" } dd data-bind="status.storage.free" { (status.storage.free) } }
                        div class="local-ai-fact" { dt { "Runtime process signal" } dd data-bind="ai.runtime.serverState" { "Read from runtime state" } }
                    }
                    div class="local-ai-actions ux-row-actions" { (nav_button("Open Storage", "storage")) }
                    h3 { "Resident engines" }
                    div class="local-ai-engine-list" data-bind-each="status.local_ai.resident_engines" data-bind-replace="true" aria-label="Resident AI engines" {
                        template {
                            div class="local-ai-engine-row" { strong data-bind="functionLabel" {} span data-bind="modelFilename" { "Model details unavailable" } }
                        }
                        @for engine in &status.local_ai.resident_engines {
                            div class="local-ai-engine-row" { strong { (&engine.function_label) } span { (engine.model_filename.as_deref().unwrap_or("Model details unavailable")) } }
                        }
                    }
                    div class="local-ai-empty" data-bind-show="localAiPane.model.noResidentEngines" hidden[!status.local_ai.resident_engines.is_empty()] {
                        p { "No resident engines are currently reported." }
                    }
                }

                section class="local-ai-modal-section" data-ai-modal-section="lanes" aria-labelledby="local-ai-lanes-title" aria-description="Lane capacity is read from living state; refresh requests the existing pulse endpoint." hidden {
                    h2 id="local-ai-lanes-title" data-modal-title hidden aria-hidden="true" { "Model serving lanes" }
                    div class="local-ai-card-head" {
                        span class="local-ai-note" { "Current lanes" }
                        (action_button(ButtonVariant::Secondary, "Refresh lanes", "model-lanes-pulse", "/api/caduceus/v1/model-lanes/pulse"))
                    }
                    div class="local-ai-lane-list" data-bind-each="modelLanes" data-bind-replace="true" aria-label="Model lanes" {
                        template {
                            article class="local-ai-lane-row" {
                                strong data-bind="alias" {}
                                span { "Slots " strong data-bind="total_slots" {} }
                                span { "Context " strong data-bind="n_ctx_per_slot" {} }
                                span { "Busy " strong data-bind="busy_slots" {} }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn installed_model_card(model: &crate::LocalAiModelStatus, status: &ConsoleStatus) -> Markup {
    let loaded = status.local_ai.loaded_model_id.as_deref() == Some(model.id.as_str());
    html! {
        article class="local-ai-model-row" data-selected="false" {
            div class="local-ai-model-copy" {
                strong class="local-ai-model-name" { (model.name) @if model.is_recommended { " · Recommended" } }
                span class="local-ai-model-filename" { (model.filename) }
                div class="local-ai-model-meta" {
                    span { "Size " strong { (model.size) } }
                    span { "Quantization " strong { (model.quantization.as_deref().unwrap_or("Unknown")) } }
                    span { "Use " strong { (model.recommended_use.unwrap_or("Balanced")) } }
                }
            }
            div class="local-ai-actions ux-row-actions" {
                button class="btn btn--secondary" type="button" data-ai-action="model-select" data-model-id=(model.id) { "Select" }
                @if !loaded { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(model.id) { "Load" } }
                @if loaded { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } }
                @if !loaded { button class="btn btn--danger" type="button" data-ai-action="model-remove" data-model-id=(model.id) data-filename=(model.filename) { "Remove" } }
            }
        }
    }
}

fn human_bytes(bytes: u64) -> String {
    let gib = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{gib:.1} GB")
    } else {
        format!("{} MB", bytes / 1024 / 1024)
    }
}
