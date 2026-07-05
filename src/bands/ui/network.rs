fn network_view(status: &ConsoleStatus) -> Markup {
    let console_url = status.identity.web_origin.as_str();
    let ssh = &status.system.ssh;
    let trust = &status.system.trust;
    let lan_ai_port = status.local_ai.lan_inference_port.unwrap_or(7777);
    let lan_ip = status.network.ip_address.as_str();
    let lan_ai_ip_port = (lan_ip != "—").then(|| endpoint_ip_port(lan_ip, lan_ai_port));
    let lan_ai_url_port = lan_ai_ip_port
        .as_ref()
        .map(|_| endpoint_url_port(lan_ip, lan_ai_port, "http"));
    let lan_ai_detail = format!(":{lan_ai_port}");
    view_shell(
        "network",
        "",
        "",
        "",
        html! {
            div class="network-hub" {
                article class="network-section network-services-hub" aria-label="Services" {
                    div class="network-services-hub__head" {
                        div class="network-services-hub__identity" {
                            strong class="network-services-hub__title" { "Services" }
                            span class="network-services-hub__detail" data-bind="networkPane.connection.detail" {
                                (status.network.connection_session_detail)
                            }
                        }
                        span class=(format!("system-status system-status--{}", if status.network.online { "available" } else { "disabled" })) data-bind="networkPane.connection.headline" data-bind-class="networkPane.connection.state" {
                            (if status.network.online { "Online" } else { "Offline" })
                        }
                    }
                    div class="network-access-list" {
                        (network_access_row(
                            "Web Console",
                            console_url,
                            "Available",
                            html! { (bound_copy_button("Copy URL", console_url, "networkPane.addresses.consoleUrl", "networkPane.addresses.consoleUrl", true, "Console URL is available.")) },
                        ))
                        (network_access_row_bound(
                            "LAN IP",
                            &status.network.ip_address,
                            "networkPane.addresses.ip",
                            if status.network.console_reachable { "Reachable" } else { "Offline" },
                            "networkPane.reachability.console",
                            "networkPane.reachability.consoleState",
                            html! { (bound_copy_button("Copy IP", &status.network.ip_address, "networkPane.addresses.ip", "networkPane.addresses.ipCopyable", status.network.ip_address != "—", "No LAN IP is available yet.")) },
                        ))
                        (network_access_row_bound(
                            "LAN AI",
                            &lan_ai_detail,
                            "networkPane.addresses.aiIpPort",
                            if status.network.lan_ai_reachable { "Available" } else { "Disabled" },
                            "networkPane.services.lanAi",
                            "networkPane.services.lanAiState",
                            html! {
                                (bound_copy_button("Copy AI", lan_ai_ip_port.as_deref().unwrap_or("Unavailable"), "networkPane.addresses.aiIpPort", "networkPane.addresses.aiCopyable", lan_ai_ip_port.is_some() && status.network.lan_ai_reachable, "LAN AI is not reachable on the saved port."))
                                (bound_copy_button("Copy URL", lan_ai_url_port.as_deref().unwrap_or("Unavailable"), "networkPane.addresses.aiUrlPort", "networkPane.addresses.aiCopyable", lan_ai_url_port.is_some() && status.network.lan_ai_reachable, "LAN AI URL is not reachable yet."))
                                (nav_focus_button("Open Local AI", "local-ai", "local-ai-inference"))
                            },
                        ))
                        (network_access_row_bound(
                            "SSH",
                            &format!(":22 · {}", ssh.command),
                            "",
                            ssh_service_label(&ssh.service_state),
                            "networkPane.services.ssh",
                            "networkPane.services.sshState",
                            html! {
                                (copy_button("Copy Command", &ssh.command))
                                (nav_button("Open System", "system"))
                            },
                        ))
                        (network_access_row_bound(
                            "Home Root CA",
                            if trust.ca_installed { "Installed on this console" } else { "Import bundle from System" },
                            "",
                            if trust.ca_installed { "Installed" } else { "Needed" },
                            "networkPane.services.rootCa",
                            "networkPane.services.rootCaState",
                            html! {
                                button class="btn btn--secondary" type="button" disabled title="Root CA import stays in System for now." { "Coming soon" }
                                (nav_button("Open System", "system"))
                            },
                        ))
                    }
                }

                div class="network-workbench" {
                    article id="wifi-management" class="network-section network-card network-card--wifi" tabindex="-1" aria-label="Wi-Fi" data-bind-class="networkPane.wifi.state" {
                        div class="network-section-head" {
                            div class="network-card__titleblock" {
                                strong { "Wi-Fi" }
                                @if status.network.active_type == "wifi" {
                                    span class="network-wifi-ssid" { span data-bind="networkPane.wifi.ssid" { (status.network.ssid.as_deref().unwrap_or("Wi-Fi")) } }
                                } @else {
                                    span class="network-wifi-ssid network-wifi-ssid--idle" data-bind="networkPane.wifi.ssid" { (if status.network.wifi_adapter_available { "Ready for wireless setup" } else { "No Wi-Fi adapter" }) }
                                }
                            }
                            span class=(format!("system-status system-status--{}", if !status.network.wifi_adapter_available { "disabled" } else if status.network.active_type == "wifi" { "available" } else { "unknown" })) data-bind="networkPane.wifi.status" data-bind-class="networkPane.wifi.state" {
                                (wifi_status_label(status))
                            }
                        }
                        @if status.network.active_type == "wifi" {
                            div class="network-wifi-signal" aria-label="Wi-Fi signal strength" data-bind-show="networkPane.wifi.signalVisible" {
                                span class="network-wifi-signal__track" {
                                    span class="network-wifi-signal__fill" style=(format!("--wifi-signal: {}%", status.network.signal_percent.unwrap_or(0))) data-bind-style-var="--wifi-signal:networkPane.wifi.signalPercent" {}
                                }
                                em data-bind="networkPane.wifi.signalLabel" { (status.network.signal_percent.map(|v| format!("{v}% signal")).unwrap_or_else(|| "Signal unavailable".to_string())) }
                            }
                        }
                        div class="network-wifi-actions" {
                            button class="btn btn--primary network-wifi-choose" type="button" data-network-action="choose-wifi" disabled[!status.network.wifi_adapter_available] title=(if status.network.wifi_adapter_available { "Choose a Wi-Fi network." } else { "No Wi-Fi adapter is available on this console." }) { "Choose Network" }
                            div class="network-wifi-actions__secondary" {
                                button class="btn btn--secondary" type="button" data-open-hidden-wifi="true" disabled[!status.network.wifi_adapter_available] title=(if status.network.wifi_adapter_available { "Join a hidden Wi-Fi network." } else { "No Wi-Fi adapter is available on this console." }) { "Hidden" }
                                button class="btn btn--secondary" type="button" data-network-action="wifi-toggle" data-enabled=(if status.network.active_type == "wifi" { "false" } else { "true" }) disabled[!status.network.wifi_adapter_available] title=(if status.network.wifi_adapter_available { "Toggle the Wi-Fi radio." } else { "No Wi-Fi adapter is available on this console." }) {
                                    (if status.network.active_type == "wifi" { "Turn Off" } else { "Turn On" })
                                }
                            }
                        }
                        div id="wifi-message" class="message" hidden {}
                    }

                    article class="network-section network-card network-card--wired" aria-label="Wired LAN" {
                        div class="network-section-head" {
                            strong { "Wired LAN" }
                            span class=(format!("system-status system-status--{}", if status.network.ethernet_connected { "available" } else { "disabled" })) data-bind="networkPane.wired.badge" data-bind-class="networkPane.wired.state" {
                                (ethernet_head_badge(status))
                            }
                        }
                        div class="network-compact-grid network-compact-grid--small" {
                            (system_field_bound("Mode", if status.network.ethernet_dhcp { "DHCP" } else { "Manual" }, "networkPane.wired.mode"))
                            (system_field_bound("Nameservers", &status.network.resolv_nameservers, "networkPane.wired.nameservers"))
                            (system_field_bound("Search", &status.network.resolv_search, "networkPane.wired.search"))
                            (system_field_bound("Gateway", status.network.gateway.as_deref().unwrap_or("Unknown"), "networkPane.wired.gateway"))
                        }
                        @if !status.network.ethernet_available {
                            div class="empty-state" { strong { "No Ethernet adapter" } }
                        }
                        div class="network-speed-panel" {
                            button class="btn btn--secondary" type="button" data-network-action="speed-test" disabled[!status.network.ethernet_available] title=(if status.network.ethernet_available { "Run a network speed test." } else { "No Ethernet adapter is available on this console." }) { "Run speed test" }
                            div id="speed-test-results" class="network-speed-result" data-bind="networkPane.diagnostics.internet" hidden {}
                        }
                        div class="inline-actions inline-actions--compact network-card__actions" {
                            button class="btn btn--secondary" type="button" data-open-wired-details="true" { "Details" }
                            button class="btn btn--secondary" type="button" data-network-action="renew-dhcp" disabled[!status.network.ethernet_available] title=(if status.network.ethernet_available { "Renew the Ethernet DHCP lease." } else { "No Ethernet adapter is available on this console." }) { "Renew DHCP" }
                            button class="btn btn--secondary" type="button" data-open-ip-settings="true" disabled[!status.network.ethernet_available] title=(if status.network.ethernet_available { "Open IP settings." } else { "No Ethernet adapter is available on this console." }) { "IP Settings" }
                        }
                    }
                }

                details class="network-section network-diagnostics-foot sync-desktop-detail" {
                    summary { "Diagnostics" }
                    div class="inline-actions inline-actions--compact" data-diagnostics-actions="true" {
                        button class="btn btn--secondary" type="button" data-diagnostic="gateway" { "Test Gateway" }
                        button class="btn btn--secondary" type="button" data-diagnostic="dns" { "Test DNS" }
                        button class="btn btn--secondary" type="button" data-diagnostic="internet" { "Test Internet" }
                        button class="btn btn--secondary" type="button" data-diagnostic="lan-ai" { "Test LAN AI" }
                    }
                    div id="diagnostics-results" class="diagnostics-results" {
                        div class="network-diagnostic-grid" {
                            span data-bind="networkPane.diagnostics.gateway" { "Ready" }
                            span data-bind="networkPane.diagnostics.dns" { "Ready" }
                            span data-bind="networkPane.diagnostics.internet" { "Ready" }
                            span data-bind="networkPane.diagnostics.lanAi" { "Ready" }
                        }
                    }
                }
            }
        },
    )
}

fn endpoint_ip_port(ip: &str, port: u16) -> String {
    format!("{ip}:{port}")
}

fn endpoint_url_port(ip: &str, port: u16, scheme: &str) -> String {
    format!("{scheme}://{ip}:{port}")
}

fn ssh_service_label(state: &str) -> &'static str {
    match state {
        "running" | "available" | "active" => "Available",
        _ => "Disabled",
    }
}

fn ethernet_head_badge(status: &ConsoleStatus) -> String {
    if !status.network.ethernet_connected {
        return "No cable".to_string();
    }
    status
        .network
        .ethernet_speed_mbps
        .map(|v| format!("{v} Mbps link"))
        .unwrap_or_else(|| "Link up".to_string())
}

fn wifi_status_label(status: &ConsoleStatus) -> &'static str {
    if !status.network.wifi_adapter_available {
        "Unavailable"
    } else if status.network.active_type == "wifi" {
        "On"
    } else {
        "Available"
    }
}

fn network_access_row(name: &str, detail: &str, state: &str, actions: Markup) -> Markup {
    let state_key = state.to_ascii_lowercase();
    html! {
        div class="network-access-row" {
            div class="network-access-row__copy" {
                strong { (name) }
                em { (detail) }
            }
            span class=(format!("system-status system-status--{}", if state_key == "available" || state_key == "reachable" || state_key == "installed" { "available" } else if state_key == "needed" { "unknown" } else { "disabled" })) {
                (state)
            }
            div class="network-access-row__actions" { (actions) }
        }
    }
}

fn network_service_row(name: &str, state: &str, route: &str, action: Markup) -> Markup {
    html! { div class="network-service-row" { span { strong { (name) } em { (route) } } b class=(format!("system-status system-status--{}", state.to_lowercase())) { (state) } span class="system-row-actions" { (action) } } }
}

fn settings_action_row(
    title: &str,
    description: &str,
    icon: Option<&str>,
    status: Option<(&str, &str)>,
    action: Markup,
    destructive: bool,
) -> Markup {
    html! {
        article class=(if destructive { "settings-action-row settings-action-row--destructive" } else { "settings-action-row" }) {
            @if let Some(icon_text) = icon {
                span class="settings-action-row__icon" aria-hidden="true" { (icon_text) }
            }
            div class="settings-action-row__copy" {
                div class="settings-action-row__titleline" {
                    h3 { (title) }
                    @if let Some((label, class_name)) = status {
                        span class=(format!("system-status {class_name}")) { (label) }
                    }
                }
                p { (description) }
            }
            div class="settings-action-row__action" { (action) }
        }
    }
}

fn network_access_row_bound(name: &str, detail: &str, detail_bind: &str, state: &str, state_bind: &str, state_class_bind: &str, actions: Markup) -> Markup {
    let state_key = state.to_ascii_lowercase();
    let status_class = if state_key == "available" || state_key == "reachable" || state_key == "installed" { "available" } else if state_key == "needed" { "unknown" } else { "disabled" };
    html! {
        div class="network-access-row" {
            div class="network-access-row__copy" {
                strong { (name) }
                @if detail_bind.is_empty() { em { (detail) } } @else { em data-bind=(detail_bind) { (detail) } }
            }
            span class=(format!("system-status system-status--{}", status_class)) data-bind=(state_bind) data-bind-class=(state_class_bind) {
                (state)
            }
            div class="network-access-row__actions" { (actions) }
        }
    }
}

fn bound_copy_button(label: &str, value: &str, value_bind: &str, enabled_bind: &str, enabled: bool, disabled_title: &str) -> Markup {
    html! {
        button class="btn btn--secondary" type="button" data-copy-value=(value) data-bind-copy-value=(value_bind) data-bind-enabled=(enabled_bind) data-bind-class=(enabled_bind) disabled[!enabled] title=(if enabled { "Copy this value." } else { disabled_title }) { (label) }
    }
}

fn system_field_bound(label: &str, value: &str, bind: &str) -> Markup {
    html! {
        div class="system-field" {
            em { (label) }
            strong data-bind=(bind) { (value) }
        }
    }
}
