#[cfg(test)]
mod tests {
    use super::*;

    include!("tests/provider_and_ux.rs");
    include!("tests/home_sync_storage.rs");
    include!("tests/network_system.rs");
    include!("tests/local_ai_and_css.rs");
}
