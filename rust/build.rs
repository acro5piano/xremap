fn main() {
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rerun-if-changed=build.rs");
}
