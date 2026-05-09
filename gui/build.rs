fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let library_paths = std::collections::HashMap::from([(
        "components".to_string(),
        manifest_dir.join("ui/components"),
    )]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
