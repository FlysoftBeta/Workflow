fn main() {
    // Cargo and build.sh use the same freestanding, relocation-free static PIE.
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-static-pie");
    println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    println!("cargo:rustc-link-arg=-Wl,-z,noexecstack");
    println!("cargo:rustc-link-arg=-Wl,--no-dynamic-linker");
}
