fn duckstation_value_for_binding(binding: &str) -> String {
    format!("SDL-0/{}", binding.replace(' ', "_"))
}

fn duckstation_lines_from_bindings(bindings: &[ControllerBindingStatus]) -> Vec<String> {
    bindings
        .iter()
        .map(|b| {
            format!(
                "Pad1/{} = {}",
                b.control.replace(' ', ""),
                duckstation_value_for_binding(&b.binding)
            )
        })
        .collect()
}