# Arcadia Rust bands

Arcadia applies the infinite-infinite strut to the Rust service body.

`src/main.rs` is the thin process/router face. The band files here hold coherent transition surfaces and remain intentionally small enough to keep each responsibility inspectable.

This tranche uses crate-root `include!` bands to preserve existing privacy and behavior while removing the 7k-line monolith. Later tranches may promote these bands into explicit Rust modules once each boundary has typed public interfaces.
