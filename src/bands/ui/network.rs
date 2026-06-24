fn network_view(status: &ConsoleStatus) -> Markup {
    let console_url = status.identity.web_origin.as_str();
    view_shell(
        "network",
        "",
        "",
        "",
        html! {
            div class="network-hub" {
                article class="network-section network-summary-strip" data-network-current="true" {
                    div class="network-summary-strip__identity" {
                        strong class="network-summary-title" { (connection_summary_title(status)) }
                        span class="network-summary-strip__detail" {
                            (status.network.connection_type) " · " (status.network.ip_address)
                        }
                    }
                    span class=(format!("system-status system-status--{}", if status.network.online { "available" } else { "disabled" })) {
                        (if status.network.online { "Online" } else { "Offline" })
                    }
                    div class="network-compact-grid network-compact-grid--summary" {
                        (system_field("Gateway", status.network.gateway.as_deref().unwrap_or("Unknown")))
                        (system_field("Internet", internet_label(status.network.internet_reachable)))
                        (system_field("DNS", &status.network.dns_status))
                        (system_field("LAN", if status.network.console_reachable { "Reachable" } else { "Unavailable" }))
                        @if let Some(label) = timezone_label(status) {
                            (system_field("Time zone", &label))
                        }
                        (system_field("Hostname", &status.identity.hostname))
                    }
                    div class="inline-actions inline-actions--compact network-summary-strip__actions" {
                        (copy_button("Copy URL", console_url))
                        @if status.network.ip_address != "—" { (copy_button("Copy IP", &status.network.ip_address)) }
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
                            div class="inline-actions inline-actions--compact network-card__actions" {
                                button class="btn btn--primary" type="button" data-network-action="choose-wifi" { "Choose Network" }
                                button class="btn btn--secondary" type="button" data-network-action="scan-wifi" { "Scan" }
                                button class="btn btn--secondary" type="button" data-open-hidden-wifi="true" { "Hidden" }
                                button class="btn btn--secondary" type="button" data-network-action="wifi-toggle" data-enabled=(if status.network.active_type == "wifi" { "false" } else { "true" }) {
                                    (if status.network.active_type == "wifi" { "Turn Off" } else { "Turn On" })
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
                                (system_field("IP address", &status.network.ip_address))
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

                    article class="network-section network-card network-card--services" aria-label="Services" {
                        div class="network-section-head" { strong { "Services" } }
                        div class="network-service-list" {
                            (network_service_row("Web Console", "Available", console_url, html! { (copy_button("Copy URL", console_url)) }))
                            (network_service_row("LAN AI", if status.network.lan_ai_reachable { "Available" } else { "Disabled" }, if status.network.lan_ai_reachable { ":7777" } else { "" }, html! { (nav_focus_button("Open Local AI", "local-ai", "local-ai-inference")) }))
                            (network_service_row("SSH", "Disabled", "", html! { (nav_button("Open System", "system")) }))
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

fn timezone_label(status: &ConsoleStatus) -> Option<String> {
    let tz = status.network.timezone.as_deref()?;
    Some(match status.network.ntp_synchronized {
        Some(true) => format!("{tz} · NTP synced"),
        Some(false) => format!("{tz} · local clock"),
        None => tz.to_string(),
    })
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

fn connection_summary_title(status: &ConsoleStatus) -> &'static str {
    match status.network.active_type.as_str() {
        "ethernet" => "Connected by Ethernet",
        "wifi" => "Connected to Wi-Fi",
        "limited" => "Connected to LAN",
        "offline" => "Offline",
        _ => "Unknown",
    }
}

fn internet_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Reachable",
        Some(false) => "Unavailable",
        None => "Unknown",
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