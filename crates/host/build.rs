fn main() {
    println!("cargo:warning=heyyy");
    println!("cargo:rustc-check-cfg=cfg(no_std)");
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"no_std\"))");
    println!("cargo:rustc-check-cfg=cfg(wasmer_sys)");
    println!("cargo:rustc-check-cfg=cfg(wasmer_wamr)");

    if option_env!("WASMER_WAMR").is_some() {
        println!("cargo:rustc-cfg=wasmer_wamr");
    } else {
        println!("cargo:rustc-cfg=wasmer_sys");
    }
}
