fn ppsspp_value_for_binding(binding: &str) -> String {
    format!("SDL.{}", binding.replace(' ', "_"))
}

fn ppsspp_lines_from_bindings(bindings: &[ControllerBindingStatus]) -> Vec<String> {
    bindings
        .iter()
        .map(|b| {
            format!(
                "{} = {}",
                b.control.replace(' ', "_"),
                ppsspp_value_for_binding(&b.binding)
            )
        })
        .collect()
}