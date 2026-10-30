fn main(){
 println!("cargo:rerun-if-changed=native/MacBridge.swift");
 if std::env::var("CARGO_CFG_TARGET_OS").as_deref()==Ok("macos"){
  assert_eq!(std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(),Ok("aarch64"),"Only Apple Silicon is supported");
  let output="native/macos-bridge-aarch64-apple-darwin";
  let status=std::process::Command::new("xcrun").args(["swiftc","-O","-target","arm64-apple-macos13.0","-framework","Cocoa","-framework","ApplicationServices","native/MacBridge.swift","-o",output]).status().expect("Install Xcode Command Line Tools to compile the macOS bridge");
  assert!(status.success(),"macOS bridge compilation failed");
  let status=std::process::Command::new("codesign").args(["--force","--sign","-","--identifier","io.github.gpt-model-switcher.bridge",output]).status().expect("codesign unavailable");
  assert!(status.success(),"Ad-hoc signing failed");
 }
 tauri_build::build()
}
