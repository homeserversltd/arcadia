fn write_ssh_password_auth(enabled: bool) -> (StatusCode, Json<ConsoleActionResponse>) {
    let value = if enabled { "yes" } else { "no" };
    let content = format!("# Managed by Arcadia HomeConsole\nPasswordAuthentication {}\nKbdInteractiveAuthentication {}\n", value, value);
    let result = Path::new(SSHD_CONFIG_MANAGED_PATH)
        .parent()
        .map(fs::create_dir_all)
        .transpose()
        .and_then(|_| fs::write(SSHD_CONFIG_MANAGED_PATH, content))
        .and_then(|_| {
            Command::new(SYSTEMCTL_BIN)
                .args(["reload", "sshd.service"])
                .output()
                .map(|_| ())
        })
        .or_else(|_| {
            Command::new(SYSTEMCTL_BIN)
                .args(["restart", "sshd.service"])
                .output()
                .map(|_| ())
        });
    match result {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: if enabled {
                    "enable-ssh-password"
                } else {
                    "disable-ssh-password"
                },
                command: SSHD_CONFIG_MANAGED_PATH,
                exit_code: Some(0),
                message: if enabled {
                    "SSH password login enabled."
                } else {
                    "SSH password login disabled."
                }
                .to_string(),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            if enabled {
                "enable-ssh-password"
            } else {
                "disable-ssh-password"
            },
            SSHD_CONFIG_MANAGED_PATH,
            &format!("SSH password configuration failed: {err}"),
        ),
    }
}

fn install_authorized_key(raw: &str) -> (StatusCode, Json<ConsoleActionResponse>) {
    let Some(key) = normalize_public_key(raw) else {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "install-authorized-key",
            "authorized_keys",
            "Enter one valid SSH public key.",
        );
    };
    let user = ssh_username();
    let home = user_home(&user);
    let ssh_dir = Path::new(&home).join(".ssh");
    let auth = ssh_dir.join("authorized_keys");
    let mut existing = fs::read_to_string(&auth).unwrap_or_default();
    let already = existing
        .lines()
        .any(|line| normalize_public_key(line).as_deref() == Some(key.as_str()));
    if !already {
        if !existing.is_empty() && !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(&key);
        existing.push('\n');
    }
    let write = fs::create_dir_all(&ssh_dir)
        .and_then(|_| fs::write(&auth, existing))
        .and_then(|_| set_mode(&ssh_dir, 0o700))
        .and_then(|_| set_mode(&auth, 0o600))
        .and_then(|_| chown_path(&ssh_dir, &user))
        .and_then(|_| chown_path(&auth, &user));
    match write {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "install-authorized-key",
                command: "authorized_keys",
                exit_code: Some(0),
                message: if already {
                    "SSH public key already installed."
                } else {
                    "SSH public key installed."
                }
                .to_string(),
                stdout: format!("{}\nmode .ssh=0700 authorized_keys=0600", auth.display()),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "install-authorized-key",
            "authorized_keys",
            &format!("SSH public key could not be installed: {err}"),
        ),
    }
}

fn install_root_ca(raw: &str) -> (StatusCode, Json<ConsoleActionResponse>) {
    let Some(pem) = normalize_ca_bundle(raw) else {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "install-root-ca",
            HOMECONSOLE_CA_ANCHOR_PATH,
            "Paste a PEM/CRT CA certificate bundle.",
        );
    };
    let result = Path::new(HOMECONSOLE_CA_ANCHOR_PATH)
        .parent()
        .map(fs::create_dir_all)
        .transpose()
        .and_then(|_| fs::write(HOMECONSOLE_CA_ANCHOR_PATH, pem))
        .and_then(|_| set_mode(Path::new(HOMECONSOLE_CA_ANCHOR_PATH), 0o644))
        .and_then(|_| Command::new(UPDATE_CA_TRUST_BIN).output().map(|_| ()));
    match result {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "install-root-ca",
                command: UPDATE_CA_TRUST_BIN,
                exit_code: Some(0),
                message: "Home Root CA installed into appliance trust.".to_string(),
                stdout: ca_metadata_text(),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "install-root-ca",
            UPDATE_CA_TRUST_BIN,
            &format!("Home Root CA install failed: {err}"),
        ),
    }
}

fn set_trust_mode(mode: &str, confirm: Option<&str>) -> (StatusCode, Json<ConsoleActionResponse>) {
    if !matches!(mode, "http" | "https") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "set-trust-mode",
            TRUST_MODE_PATH,
            "Mode must be http or https.",
        );
    }
    if mode == "https" {
        if confirm != Some("ENABLE_HTTPS") {
            return console_action_error(
                StatusCode::BAD_REQUEST,
                "set-trust-mode",
                TRUST_MODE_PATH,
                "Confirm before switching to HTTPS mode.",
            );
        }
        if !trust_status().ca_installed {
            return console_action_error(
                StatusCode::BAD_REQUEST,
                "set-trust-mode",
                TRUST_MODE_PATH,
                "Install the Home Root CA before enabling HTTPS mode.",
            );
        }
        if !https_probe_ok() {
            return console_action_error(
                StatusCode::BAD_REQUEST,
                "set-trust-mode",
                TRUST_MODE_PATH,
                "HTTPS validation failed; HTTP mode remains active.",
            );
        }
    }
    if mode == "http" && confirm != Some("ENABLE_HTTP") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "set-trust-mode",
            TRUST_MODE_PATH,
            "Confirm before switching to HTTP mode.",
        );
    }
    let body = format!("{{\"mode\":\"{}\"}}\n", mode);
    let result = Path::new(TRUST_MODE_PATH)
        .parent()
        .map(fs::create_dir_all)
        .transpose()
        .and_then(|_| fs::write(TRUST_MODE_PATH, body))
        .and_then(|_| set_mode(Path::new(TRUST_MODE_PATH), 0o644));
    match result {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "set-trust-mode",
                command: TRUST_MODE_PATH,
                exit_code: Some(0),
                message: if mode == "https" {
                    "HTTPS with Home Root CA enabled."
                } else {
                    "HTTP mode enabled."
                }
                .to_string(),
                stdout: format!("active_mode={}", mode),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "set-trust-mode",
            TRUST_MODE_PATH,
            &format!("Mode could not be saved: {err}"),
        ),
    }
}

fn run_console_command(
    action: &'static str,
    command: &'static str,
    args: &[&str],
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if !helper_exists(command) {
        return console_action_error(
            StatusCode::NOT_IMPLEMENTED,
            action,
            command,
            "Required command is not installed on this console.",
        );
    }
    match Command::new(command).args(args).output() {
        Ok(output) => {
            let ok = output.status.success();
            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                },
                Json(ConsoleActionResponse {
                    ok,
                    action,
                    command,
                    exit_code: output.status.code(),
                    message: if ok { success_message } else { failure_message }.to_string(),
                    stdout: redacted_output(&output.stdout),
                    stderr: redacted_output(&output.stderr),
                }),
            )
        }
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            action,
            command,
            &format!("Command could not start: {err}"),
        ),
    }
}

fn console_action_error(
    status: StatusCode,
    action: &'static str,
    command: &'static str,
    message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    (
        status,
        Json(ConsoleActionResponse {
            ok: false,
            action,
            command,
            exit_code: None,
            message: message.to_string(),
            stdout: String::new(),
            stderr: String::new(),
        }),
    )
}

fn redacted_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .take(80)
        .map(|line| {
            if line.to_ascii_lowercase().contains("key=")
                || line.to_ascii_lowercase().contains("password")
                || line.to_ascii_lowercase().contains("token")
            {
                "[REDACTED]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
