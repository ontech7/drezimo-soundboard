use std::path::Path;

/// The Voicemod client key is built into the app and kept out of git: it comes from the environment
/// or from the repository's `.env` file. A build without it runs, but cannot reach Voicemod.
const KEY_VAR: &str = "VOICEMOD_CLIENT_KEY";
const ENV_FILE: &str = "../../.env";

fn main() {
    let env_file = Path::new(ENV_FILE);
    println!("cargo:rerun-if-env-changed={KEY_VAR}");
    // Cargo reruns a build script at every build while a file it watches is missing
    if env_file.exists() {
        println!("cargo:rerun-if-changed={ENV_FILE}");
    }
    let from_environment = std::env::var(KEY_VAR).ok().filter(|key| !key.is_empty());
    if let Some(key) = from_environment.or_else(|| key_from_env_file(env_file)) {
        println!("cargo:rustc-env={KEY_VAR}={key}");
    }

    tauri_build::build()
}

fn key_from_env_file(path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let value = contents
        .lines()
        .find_map(|line| line.strip_prefix(KEY_VAR)?.trim_start().strip_prefix('='))?;
    Some(value.trim().trim_matches('"').to_string()).filter(|key| !key.is_empty())
}
