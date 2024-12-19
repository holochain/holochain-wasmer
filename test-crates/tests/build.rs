fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=*");

    for &m in [
        "test_wasm_core",
        "test_wasm_memory",
        "test_wasm_empty",
        "test_wasm_io",
    ]
    .iter()
    {
        let cargo_command = std::env::var_os("CARGO");
        let cargo_command = cargo_command.as_deref().unwrap_or_else(|| "cargo".as_ref());

        let status = std::process::Command::new(cargo_command)
            .arg("build")
            .arg("-p")
            .arg(m)
            .arg("--release")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .env("CARGO_TARGET_DIR", &out_dir)
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .status()
            .unwrap();

        assert!(status.success());
    }
}
