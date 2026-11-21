use std::fs;
use std::path::Path;

/// The Voicemod client key is built into the app and kept out of git: it comes from the environment
/// or from the repository's `.env` file. A build without it runs, but cannot reach Voicemod.
const KEY_VAR: &str = "VOICEMOD_CLIENT_KEY";
const ENV_FILE: &str = "../../.env";

/// The board's firmware, built into the app (src/firmware.rs): the one `drezimo-<version>.uf2` in this folder, which
/// `firmware/build.sh --bundle` writes with its `.bin` beside it. It must be the version the firmware's source declares.
const FIRMWARE_DIR: &str = "firmware";
const FIRMWARE_SOURCE: &str = "../../firmware/drezimo/drezimo.ino";

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
    bundle_firmware();

    tauri_build::build()
}

fn bundle_firmware() {
    println!("cargo:rerun-if-changed={FIRMWARE_DIR}");
    println!("cargo:rerun-if-changed={FIRMWARE_SOURCE}");
    let images: Vec<_> = fs::read_dir(FIRMWARE_DIR)
        .expect("the firmware folder")
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let version = name.strip_prefix("drezimo-")?.strip_suffix(".uf2")?.to_string();
            Some((version, entry.path()))
        })
        .collect();
    let [(version, image)] = images.as_slice() else {
        panic!(
            "{FIRMWARE_DIR}/ must hold one drezimo-<version>.uf2, not {}",
            images.len()
        );
    };
    if let Ok(source) = fs::read_to_string(FIRMWARE_SOURCE) {
        let declared = source.lines().find_map(|line| {
            line.strip_prefix("const char FIRMWARE_VERSION[] = \"")?
                .strip_suffix("\";")
        });
        assert_eq!(
            declared,
            Some(version.as_str()),
            "the bundled firmware is not the source's version: run firmware/build.sh --bundle"
        );
    }
    let image = fs::canonicalize(image).expect("the bundled firmware");
    println!("cargo:rustc-env=DREZIMO_FIRMWARE_VERSION={version}");
    println!("cargo:rustc-env=DREZIMO_FIRMWARE_IMAGE={}", image.display());
    let binary = image.with_extension("bin");
    assert!(
        binary.exists(),
        "{} is missing: run firmware/build.sh --bundle",
        binary.display()
    );
    println!("cargo:rustc-env=DREZIMO_FIRMWARE_BINARY={}", binary.display());
}

fn key_from_env_file(path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let value = contents
        .lines()
        .find_map(|line| line.strip_prefix(KEY_VAR)?.trim_start().strip_prefix('='))?;
    Some(value.trim().trim_matches('"').to_string()).filter(|key| !key.is_empty())
}
