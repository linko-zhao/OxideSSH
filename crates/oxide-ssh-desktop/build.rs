//! Embeds the application icon into the Windows executable so Explorer,
//! the taskbar, and shortcuts can display it. cargo-packager's `icons`
//! setting only feeds the installer templates; it does not touch the exe.

fn main() {
    let manifest_dir =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    copy_themes(&manifest_dir);

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = manifest_dir
        .join("../../assets/oxide-ssh.ico")
        .canonicalize()
        .expect("assets/oxide-ssh.ico");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let rc = out_dir.join("oxide-ssh.rc");
    // Forward slashes: accepted by rc.exe and llvm-rc, no escaping needed.
    std::fs::write(
        &rc,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write oxide-ssh.rc");

    println!("cargo:rerun-if-changed={}", icon.display());
    if let embed_resource::CompilationResult::Failed(reason) =
        embed_resource::compile(&rc, embed_resource::NONE)
    {
        panic!("failed to compile Windows icon resource: {reason}");
    }
}

fn copy_themes(manifest_dir: &std::path::Path) {
    let source = manifest_dir.join("../../themes");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let destination = out_dir
        .ancestors()
        .nth(3)
        .expect("profile output directory")
        .join("themes");
    std::fs::create_dir_all(&destination).expect("create themes output directory");
    for name in ["codex-light.json", "codex-dark.json"] {
        let file = source.join(name);
        println!("cargo:rerun-if-changed={}", file.display());
        std::fs::copy(&file, destination.join(name)).expect("copy bundled theme");
    }
}
