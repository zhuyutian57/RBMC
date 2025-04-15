
fn main() {
    // Set `TARGET` for future building
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
}
