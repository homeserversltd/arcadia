fn retroarch_key_for_control(control: &str) -> &'static str {
    match control {
        "A" => "input_a_btn",
        "B" => "input_b_btn",
        "X" => "input_x_btn",
        "Y" => "input_y_btn",
        "L1" => "input_l_btn",
        "R1" => "input_r_btn",
        "L2" => "input_l2_axis",
        "R2" => "input_r2_axis",
        "L3" => "input_l3_btn",
        "R3" => "input_r3_btn",
        "Start" => "input_start_btn",
        "Select" => "input_select_btn",
        "D-pad Up" => "input_up_btn",
        "D-pad Down" => "input_down_btn",
        "D-pad Left" => "input_left_btn",
        "D-pad Right" => "input_right_btn",
        _ => "input_menu_toggle_btn",
    }
}

fn retroarch_axis_pair(control: &str) -> Option<(&'static str, &'static str)> {
    match control {
        "Left Stick X" => Some(("input_l_x_plus_axis", "input_l_x_minus_axis")),
        "Left Stick Y" => Some(("input_l_y_plus_axis", "input_l_y_minus_axis")),
        "Right Stick X" => Some(("input_r_x_plus_axis", "input_r_x_minus_axis")),
        "Right Stick Y" => Some(("input_r_y_plus_axis", "input_r_y_minus_axis")),
        _ => None,
    }
}

fn retroarch_axis_value(binding: &str, positive: bool) -> String {
    let num = binding_number(binding);
    if positive {
        format!("+{num}")
    } else {
        format!("-{num}")
    }
}

fn retroarch_value_for_binding(binding: &str) -> String {
    if binding.starts_with("button ") {
        binding_number(binding)
    } else if binding.contains("up") {
        "h0up".to_string()
    } else if binding.contains("down") {
        "h0down".to_string()
    } else if binding.contains("left") {
        "h0left".to_string()
    } else if binding.contains("right") {
        "h0right".to_string()
    } else if binding.starts_with("axis ") {
        retroarch_axis_value(binding, true)
    } else {
        binding.to_string()
    }
}

fn retroarch_lines_from_bindings(bindings: &[ControllerBindingStatus]) -> Vec<String> {
    let mut lines = Vec::new();
    for binding in bindings {
        if let Some((plus, minus)) = retroarch_axis_pair(&binding.control) {
            let plus_value = retroarch_axis_value(&binding.binding, true);
            let minus_value = retroarch_axis_value(&binding.binding, false);
            lines.push(format!("{plus} = \"{plus_value}\""));
            lines.push(format!("{minus} = \"{minus_value}\""));
        } else {
            let key = retroarch_key_for_control(&binding.control);
            let value = retroarch_value_for_binding(&binding.binding);
            lines.push(format!("{key} = \"{value}\""));
        }
    }
    lines
}

fn retroarch_autoconfig_from_bindings(name: &str, bindings: &[ControllerBindingStatus]) -> String {
    let mut lines = vec![
        format!("input_device = \"{name}\""),
        "input_driver = \"udev\"".to_string(),
        "analog_dpad_mode = \"0\"".to_string(),
    ];
    lines.extend(retroarch_lines_from_bindings(bindings));
    format!("{}\n", lines.join("\n"))
}

fn retroarch_deploy_paths(device_name: &str) -> Vec<PathBuf> {
    let file = format!("{}.cfg", safe_file_stem(device_name));
    [
        format!("/home/steam/.config/retroarch/autoconfig/udev/{file}"),
        format!("/home/owner/.config/retroarch/autoconfig/udev/{file}"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}