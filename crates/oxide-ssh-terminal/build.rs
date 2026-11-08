use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=LIBGHOSTTY_VT_LIB_DIR");
    if env::var_os("CARGO_FEATURE_GHOSTTY_VT").is_none() {
        return;
    }
    let path = env::var_os("LIBGHOSTTY_VT_LIB_DIR")
        .map(PathBuf::from)
        .expect("ghostty-vt feature requires LIBGHOSTTY_VT_LIB_DIR (build it with scripts/build-libghostty-vt.sh)");
    println!("cargo:rustc-link-search=native={}", path.display());
    let lib = if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        "ghostty-vt-static"
    } else {
        "ghostty-vt"
    };
    println!("cargo:rustc-link-lib=static={lib}");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=advapi32");
        println!("cargo:rustc-link-lib=dylib=userenv");
    }
}
