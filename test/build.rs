use std::path::Path;

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=*");

    let _path = std::env::var_os("PATH").unwrap();

    for (k, v) in std::env::vars_os() {
        let k = k.to_string_lossy().to_string();
        let v = v.to_string_lossy().to_string();
        println!("cargo:warning={k}={v}");
    }

    for &m in ["test_wasm", "wasm_memory", "wasm_empty", "wasm_io"].iter() {
        let cargo_toml = Path::new(m).join("Cargo.toml");

        let cargo_command = std::env::var_os("CARGO");
        let cargo_command = cargo_command.as_deref().unwrap_or_else(|| "cargo".as_ref());

        let status = std::process::Command::new(cargo_command)
            .arg("build")
            .arg("--manifest-path")
            .arg(cargo_toml)
            .arg("--release")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            // .env_clear()
            // .env("PATH", path.clone())
            .env("CARGO_TARGET_DIR", &out_dir)
            .env_remove("RUSTFLAGS")
            .status()
            .unwrap();

        assert!(status.success());
    }
}
