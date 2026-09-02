fn main() {
    // `velo::app!` reads the app directory at macro-expansion time (files map
    // to routes / layouts / error.rs / loading.rs). Cargo only notices changes
    // to files already listed in dep-info (via the generated `include!`s), so
    // ADDING or REMOVING an app file wouldn't invalidate the build. Watch the
    // directory to make those changes re-expand the macro.
    println!("cargo:rerun-if-changed=src/app");
}