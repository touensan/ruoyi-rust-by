fn main() {
    // Some musl-gcc specs inject an ELF interpreter even for a fully static PIE.
    // Our musl release must run without a host musl loader installed.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("musl") {
        println!("cargo:rustc-link-arg=-Wl,--no-dynamic-linker");
    }
}
