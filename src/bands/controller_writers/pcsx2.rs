fn pcsx2_value_for_binding(binding: &str) -> String {
    format!("SDL-0:{}", binding.replace(' ', "_"))
}

fn pcsx2_lines_from_bindings(bindings: &[ControllerBindingStatus]) -> Vec<String> {
    bindings
        .iter()
        .map(|b| {
            format!(
                "Pad1_{} = {}",
                b.control.replace(' ', ""),
                pcsx2_value_for_binding(&b.binding)
            )
        })
        .collect()
}