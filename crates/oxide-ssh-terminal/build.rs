use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=LIBGHOSTTY_VT_LIB_DIR");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    let target = env::var("TARGET").expect("TARGET");
    let lib = if target_os == "windows" {
        "ghostty-vt-static"
    } else {
        "ghostty-vt"
    };
    let default_path =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
            .join("../../target/libghostty-vt")
            .join(target)
            .join("stage/usr/lib");
    let path = env::var_os("LIBGHOSTTY_VT_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or(default_path);
    let library = path.join(if target_os == "windows" {
        "ghostty-vt-static.lib"
    } else {
        "libghostty-vt.a"
    });
    if !library.is_file() {
        panic!(
            "missing libghostty-vt at {}; build it with scripts/build-libghostty-vt.sh or set LIBGHOSTTY_VT_LIB_DIR",
            library.display()
        );
    }
    println!("cargo:rerun-if-changed={}", library.display());
    println!("cargo:rustc-link-search=native={}", path.display());
    println!("cargo:rustc-link-lib=static={lib}");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=advapi32");
        println!("cargo:rustc-link-lib=dylib=userenv");
    }
}
