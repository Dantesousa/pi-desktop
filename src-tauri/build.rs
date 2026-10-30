use std::process::Command;

fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    // Channel defaults to the cargo profile unless the release pipeline says otherwise.
    let channel = std::env::var("PI_CHANNEL").unwrap_or_else(|_| {
        if std::env::var("PROFILE").as_deref() == Ok("release") {
            "release".into()
        } else {
            "dev".into()
        }
    });
    println!("cargo:rustc-env=PI_CHANNEL={channel}");
    println!("cargo:rustc-env=PI_COMMIT={}", git(&["rev-parse", "--short", "HEAD"]));
    let tag = git(&["describe", "--tags", "--exact-match", "HEAD"]);
    println!("cargo:rustc-env=PI_TAG={tag}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PI_CHANNEL");
    tauri_build::build()
}
