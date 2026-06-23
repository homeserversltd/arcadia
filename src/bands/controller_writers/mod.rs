// Tuple authority + ordered emulator writer bands (infinite-infinite child spine).
include!("tuple.rs");
include!("retroarch.rs");
include!("dolphin.rs");
include!("duckstation.rs");
include!("pcsx2.rs");
include!("ppsspp.rs");

struct RamrodEmulatorReceipt {
    emulator: String,
    staged_path: PathBuf,
    deployed: bool,
}

struct RamrodReceipt {
    entries: Vec<RamrodEmulatorReceipt>,
}

fn emulator_profile_body(
    emulator: &str,
    device_name: &str,
    handler: &str,
    bindings: &[ControllerBindingStatus],
    tuning: &ControllerTuningStatus,
    installed: bool,
) -> String {
    let mut lines = vec![
        format!("# Arcadia controller profile for {emulator}"),
        format!("device={device_name}"),
        format!("handler={handler}"),
        format!(
            "mode={}",
            if installed {
                "assigned"
            } else {
                "staged-for-install"
            }
        ),
    ];
    match emulator {
        "RetroArch" => {
            lines.push("analog_dpad_mode = \"0\"".to_string());
            lines.extend(retroarch_tuning_lines(tuning));
            lines.extend(retroarch_lines_from_bindings(bindings));
        }
        "Dolphin" => lines.extend(dolphin_lines_from_bindings(device_name, bindings)),
        "DuckStation" => lines.extend(duckstation_lines_from_bindings(bindings)),
        "PCSX2" => lines.extend(pcsx2_lines_from_bindings(bindings)),
        "PPSSPP" => lines.extend(ppsspp_lines_from_bindings(bindings)),
        _ => lines.extend(
            bindings
                .iter()
                .map(|b| format!("{}={}", b.control, b.binding)),
        ),
    }
    format!("{}\n", lines.join("\n"))
}

fn write_emulator_profile(
    emulator: &str,
    device_name: &str,
    handler: &str,
    bindings: &[ControllerBindingStatus],
    tuning: &ControllerTuningStatus,
    installed: bool,
    root: &Path,
) -> std::io::Result<PathBuf> {
    let dir = root.join("emulators").join(safe_file_stem(emulator));
    fs::create_dir_all(&dir)?;
    let staged = dir.join("default-profile.txt");
    let body = emulator_profile_body(emulator, device_name, handler, bindings, tuning, installed);
    fs::write(&staged, &body)?;
    if installed {
        if emulator == "RetroArch" {
            let autoconfig = retroarch_autoconfig_from_bindings(device_name, bindings, tuning);
            let staged_autoconfig = dir.join(format!("{}.cfg", safe_file_stem(device_name)));
            fs::write(&staged_autoconfig, &autoconfig)?;
            for deploy_path in retroarch_deploy_paths(device_name) {
                if let Some(parent) = deploy_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&deploy_path, &autoconfig);
            }
        } else {
            fs::write(dir.join("applied.ini"), &body)?;
        }
    }
    Ok(staged)
}

fn ramrod_controller_profiles(
    device_name: &str,
    handler: &str,
    bindings: &[ControllerBindingStatus],
    tuning: &ControllerTuningStatus,
    emulators: &[EmulatorControllerStatus],
) -> std::io::Result<RamrodReceipt> {
    let root = controller_profile_root();
    let mut entries = Vec::with_capacity(KNOWN_EMULATORS.len());
    for emulator in KNOWN_EMULATORS {
        let installed = emulators
            .iter()
            .find(|entry| entry.emulator == *emulator)
            .map(|entry| entry.state != "not installed")
            .unwrap_or(false);
        let staged_path = write_emulator_profile(
            emulator,
            device_name,
            handler,
            bindings,
            tuning,
            installed,
            &root,
        )?;
        entries.push(RamrodEmulatorReceipt {
            emulator: (*emulator).to_string(),
            staged_path,
            deployed: installed,
        });
    }
    Ok(RamrodReceipt { entries })
}

fn ramrod_stdout(receipt: &RamrodReceipt) -> String {
    receipt
        .entries
        .iter()
        .map(|entry| {
            format!(
                "{}: {} ({})",
                entry.emulator,
                entry.staged_path.display(),
                if entry.deployed {
                    "deployed"
                } else {
                    "staged"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}