fn main() {
    // memory.x comes from ch32-metapac (`memory-x` feature); link.x from qingke-rt
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
}
