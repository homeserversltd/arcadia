const APP_CSS: &str = include_str!("../../static/app.css");
const UX_CSS: &str = include_str!("../../static/ux/arcadia-ux.css");
const VIEWPORT_CSS: &str = include_str!("../../static/ux/arcadia-viewports.css");
const APP_JS: &str = include_str!("../../static/app.js");
const THEME_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/themes.css"));
const THEME_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/themes.js"));
const GUI_PIN_STATE_PATH: &str = "/var/lib/homeconsole/gui-pin-access.json";
const GUI_PIN_VERIFY_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-verify";
const GUI_PIN_ACCESS_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-access";
const GUI_PIN_CHANGE_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-change";
const GUI_PIN_RESET_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-reset-default";
const PROVIDER_KEYS_PATH: &str = "/etc/arch-game-sync/providers.env";
const PROVIDER_KEY_NAMES: [(&str, &str); 3] = [
    ("steamgriddb", "STEAMGRIDDB_API_KEY"),
    ("thegamesdb", "THEGAMESDB_API_KEY"),
    ("screenscraper", "SCREENSCRAPER_API_KEY"),
];
const HARMONIA_BIN: &str = "/usr/local/bin/harmonia";
const HOMECONSOLE_PROFILE: &str = "/etc/harmonia/profiles/homeconsole/index.json";
const HOMECONSOLE_SYNC_MODULE: &str = "/etc/harmonia/modules/homeconsole/sync/index.json";
const HARMONIA_HOMECONSOLE_LEDGER: &str = "/var/lib/harmonia/receipts/homeconsole-ledger.jsonl";
const ARCH_GAME_SYNC_BIN: &str = "/usr/local/bin/arch-game-sync";
const SYSTEMCTL_BIN: &str = "/usr/bin/systemctl";
const SYSTEMD_RUN_BIN: &str = "/usr/bin/systemd-run";
const GAMES_ROOT: &str = "/home/owner/Games";
const ARTWORK_ROOT: &str = "/home/owner/Games/artwork";
const TEMP_CLEAN_ROOTS: [&str; 2] = ["/tmp", "/var/tmp"];
const MODEL_SCAN_ROOTS: [&str; 3] = ["/home/owner", "/opt", "/var/lib"];
const MODEL_EXTENSIONS: [&str; 3] = ["gguf", "safetensors", "onnx"];
const GAME_SYSTEMS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];
const STEAM_USERDATA_ROOTS: [&str; 2] = [
    "/home/steam/.local/share/Steam/userdata",
    "/home/owner/.steam/steam/userdata",
];

fn game_system_storage_path(system: &str) -> PathBuf {
    let subpath = match system {
        "gba" | "genesis" | "snes" | "nes" | "ps1" | "n64" => ["roms", system],
        "ps2" | "sega-cd" | "psp" | "gamecube" | "wii" => ["isos", system],
        "dos" => ["pc", "dos"],
        _ => ["roms", system],
    };
    Path::new(GAMES_ROOT).join(subpath[0]).join(subpath[1])
}

const SYNC_MANIFEST_PATHS: [&str; 3] = [
    "/var/lib/homeconsole-sync/manifest.json",
    "/var/lib/arch-game-sync/manifest.json",
    "/var/lib/harmonia/state/homeconsole-sync-manifest.json",
];
const DEFAULT_LAN_INFERENCE_PORT: u16 = 7777;
const LOCAL_AI_STATE_PATH: &str = "/var/lib/arcadia/local-ai-state.json";
const LOCAL_AI_MODEL_ROOT: &str = "/var/lib/arcadia/models";
const LOCAL_AI_NGINX_CONF: &str = "/etc/nginx/conf.d/arcadia-local-ai.conf";
const LOCAL_AI_FIREWALL_RECEIPT: &str = "/var/lib/arcadia/local-ai-firewall.receipt";
const LOCAL_AI_TOKEN_PATH: &str = "/var/lib/arcadia/local-ai-token";
const LLAMA_SERVER_BIN: &str = "/usr/local/bin/llama-server";
const LLAMA_CPP_BIN: &str = "/usr/local/bin/llama-cli";
const SAMBA_SERVICE_NAMES: [&str; 2] = ["smb.service", "smbd.service"];
const NETWORK_MANAGER_BIN: &str = "/usr/bin/nmcli";
const SSHD_CONFIG_MANAGED_PATH: &str = "/etc/ssh/sshd_config.d/90-homeconsole-password-auth.conf";
const TRUST_MODE_PATH: &str = "/etc/arcadia/trust-mode.json";
const HOMECONSOLE_CA_ANCHOR_PATH: &str =
    "/etc/ca-certificates/trust-source/anchors/homeconsole-home-root-ca.crt";
const UPDATE_CA_TRUST_BIN: &str = "/usr/bin/update-ca-trust";
const HOME_ROOT_HTTPS_PROBE: &str = "https://home.arpa/";
