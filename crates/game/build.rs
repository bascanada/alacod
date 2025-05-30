fn main() {
    let version = std::env::var("APP_VERSION").unwrap_or("v0.0.0".into());

    println!("cargo:rustc-env=APP_VERSION={}", version);
}
