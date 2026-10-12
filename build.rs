fn main() {
    // our memory.x (the last flash page is kept for the name, see store.rs)
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={dir}");
    println!("cargo:rerun-if-changed=memory.x");
    // link.x comes from qingke-rt and includes memory.x
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
}
