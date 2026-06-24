fn network_view(status: &ConsoleStatus) -> Markup {
    let console_url = status.identity.web_origin.as_str();
    let ssh = &status.system.ssh;
    let trust = &status.system.trust;
    let lan_ai_port = 7777u16;
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
                            span class="network-services-hub__detail" {
                                (status.network.connection_session_detail)
                            }
                        }
                        span class=(format!("system-status system-status--{}", if status.network.online { "available" } else { "disabled" })) {
                            (if status.network.online { "Online" } else { "Offline" })
                        }
                    }
                    div class="network-access-list" {
                        (network_access_row(
                            "Web Console",
                            console_url,
                            "Available",
                            html! { (copy_button("Copy URL", console_url)) },
                        ))
                        @if status.network.ip_address != "—" {
                            (network_access_row(
                                "LAN IP",
                                &status.network.ip_address,
                                "Reachable",
                                html! { (copy_button("Copy IP", &status.network.ip_address)) },
                            ))
                        }
                        (network_access_row(
                            "LAN AI",
                            &lan_ai_detail,
                            if status.network.lan_ai_reachable { "Available" } else { "Disabled" },
                            html! {
                                @if let (Some(ip_port), Some(url_port)) = (&lan_ai_ip_port, &lan_ai_url_port) {
                                    (copy_button("Copy IP", ip_port))
                                    (copy_button("Copy URL", url_port))
                                }
                                (nav_focus_button("Open Local AI", "local-ai", "local-ai-inference"))
                            },
                        ))
                        (network_access_row(
                            "SSH",
                            &format!(":22 · {}", ssh.command),
                            ssh_service_label(&ssh.service_state),
                            html! {
                                (copy_button("Copy Command", &ssh.command))
                                (nav_button("Open System", "system"))
                            },
                        ))
                        (network_access_row(
                            "Home Root CA",
                            if trust.ca_installed { "Installed on this console" } else { "Import bundle from System" },
                            if trust.ca_installed { "Installed" } else { "Needed" },
                            html! {
                                button class="btn btn--secondary" type="button" disabled title="Root CA import stays in System for now." { "Coming soon" }
                                (nav_button("Open System", "system"))
                            },
                        ))
                    }
                }

                div class="network-workbench" {
                    @if status.network.wifi_adapter_available {
                        article id="wifi-management" class="network-section network-card network-card--wifi" tabindex="-1" aria-label="Wi-Fi" {
                            div class="network-section-head" {
                                div class="network-card__titleblock" {
                                    strong { "Wi-Fi" }
                                    @if status.network.active_type == "wifi" {
                                        span class="network-wifi-ssid" { (status.network.ssid.as_deref().unwrap_or("Wi-Fi")) }
                                    } @else {
                                        span class="network-wifi-ssid network-wifi-ssid--idle" { "Ready for wireless setup" }
                                    }
                                }
                                span class=(format!("system-status system-status--{}", if status.network.active_type == "wifi" { "available" } else { "unknown" })) {
                                    (wifi_status_label(status))
                                }
                            }
                            @if status.network.active_type == "wifi" {
                                div class="network-wifi-signal" aria-label="Wi-Fi signal strength" {
                                    span class="network-wifi-signal__track" {
                                        span class="network-wifi-signal__fill" style=(format!("--wifi-signal: {}%", status.network.signal_percent.unwrap_or(0))) {}
                                    }
                                    em { (status.network.signal_percent.map(|v| format!("{v}% signal")).unwrap_or_else(|| "Signal unavailable".to_string())) }
                                }
                            }
                            div class="network-wifi-actions" {
                                button class="btn btn--primary network-wifi-choose" type="button" data-network-action="choose-wifi" { "Choose Network" }
                                div class="network-wifi-actions__secondary" {
                                    button class="btn btn--secondary" type="button" data-open-hidden-wifi="true" { "Hidden" }
                                    button class="btn btn--secondary" type="button" data-network-action="wifi-toggle" data-enabled=(if status.network.active_type == "wifi" { "false" } else { "true" }) {
                                        (if status.network.active_type == "wifi" { "Turn Off" } else { "Turn On" })
                                    }
                                }
                            }
                            div id="wifi-message" class="message" hidden {}
                        }
                    }

                    article class="network-section network-card network-card--wired" aria-label="Wired LAN" {
                        div class="network-section-head" {
                            strong { "Wired LAN" }
                            span class=(format!("system-status system-status--{}", if status.network.ethernet_connected { "available" } else { "disabled" })) {
                                (ethernet_head_badge(status))
                            }
                        }
                        @if status.network.ethernet_available {
                            div class="network-compact-grid network-compact-grid--small" {
                                (system_field("Mode", if status.network.ethernet_dhcp { "DHCP" } else { "Manual" }))
                                (system_field("Nameservers", &status.network.resolv_nameservers))
                                (system_field("Search", &status.network.resolv_search))
                                (system_field("Gateway", status.network.gateway.as_deref().unwrap_or("Unknown")))
                            }
                            div class="network-speed-panel" {
                                button class="btn btn--secondary" type="button" data-network-action="speed-test" { "Run speed test" }
                                div id="speed-test-results" class="network-speed-result" hidden {}
                            }
                            div class="inline-actions inline-actions--compact network-card__actions" {
                                button class="btn btn--secondary" type="button" data-open-wired-details="true" { "Details" }
                                button class="btn btn--secondary" type="button" data-network-action="renew-dhcp" { "Renew DHCP" }
                                button class="btn btn--secondary" type="button" data-open-ip-settings="true" { "IP Settings" }
                            }
                        } @else {
                            div class="empty-state" { strong { "No Ethernet adapter" } p { "Use Wi-Fi or check cabling." } }
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
                    div id="diagnostics-results" class="diagnostics-results" {}
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