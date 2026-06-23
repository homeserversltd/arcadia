fn dolphin_value_for_binding(binding: &str) -> String {
    format!("SDL/0/{}", binding.replace(' ', "_"))
}

fn dolphin_lines_from_bindings(
    device_name: &str,
    bindings: &[ControllerBindingStatus],
) -> Vec<String> {
    bindings
        .iter()
        .map(|b| {
            format!(
                "Arcadia/{}/{} = {}",
                device_name,
                b.control,
                dolphin_value_for_binding(&b.binding)
            )
        })
        .collect()
}