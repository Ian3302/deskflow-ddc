fn main() {
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            // Refresh generated resources even when prepare:engine is cached;
            // otherwise a new EXE can still ship an older DDC implementation.
            println!("cargo:rerun-if-changed=../native/windows/monitor.ps1");
            std::fs::create_dir_all("resources/native").expect("create native resource directory");
            std::fs::copy(
                "../native/windows/monitor.ps1",
                "resources/native/monitor.ps1",
            )
            .expect("stage the current Windows DDC bridge");
        }
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            println!("cargo:rerun-if-changed=../native/mac-lifecycle.m");
            println!("cargo:rerun-if-changed=../native/mac-source-popover.m");
            cc::Build::new()
                .file("../native/mac-lifecycle.m")
                .file("../native/mac-source-popover.m")
                .flag("-fobjc-arc")
                .compile("deskflow-ddc-lifecycle");
            println!("cargo:rustc-link-lib=framework=AppKit");
        }
        tauri_build::build()
    }
}
