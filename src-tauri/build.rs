fn main() {
    println!("cargo:rerun-if-changed=native/MacBridge.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        assert_eq!(std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(), Ok("aarch64"), "Only Apple Silicon is supported");
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Missing Cargo output directory"));
        let cache = output.join("swift-module-cache");
        std::fs::create_dir_all(&cache).expect("Cannot create local Swift module cache");
        let status = std::process::Command::new("xcrun")
            .args(["swiftc", "-O", "-D", "BRIDGE_LIBRARY", "-parse-as-library", "-emit-library", "-static", "-module-name", "SwitcherMacBridge", "-module-cache-path"]).arg(cache)
            .args(["-target", "arm64-apple-macos13.0", "-framework", "Cocoa", "-framework", "ApplicationServices", "native/MacBridge.swift", "-o"]).arg(output.join("libmacos_bridge.a"))
            .status().expect("Install Xcode Command Line Tools to compile the macOS bridge");
        assert!(status.success(), "macOS bridge compilation failed");
        let compiler = std::process::Command::new("xcrun").args(["--find", "swiftc"]).output().expect("swiftc unavailable");
        let compiler = std::path::PathBuf::from(String::from_utf8(compiler.stdout).unwrap().trim());
        let libraries = compiler.parent().unwrap().parent().unwrap().join("lib/swift/macosx");
        println!("cargo:rustc-link-search=native={}", output.display());
        println!("cargo:rustc-link-search=native={}", libraries.display());
        println!("cargo:rustc-link-search=native=/usr/lib/swift");
        println!("cargo:rustc-link-lib=static=macos_bridge");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    tauri_build::build()
}
