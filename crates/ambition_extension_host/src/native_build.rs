//! Native code identity is read once, before simulation or preparation.

pub(super) fn running_build() -> Option<[u8; 32]> {
    // These hosts run code in a library or browser artifact. The process
    // executable does not identify that code.
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    { None }

    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        static BUILD: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
        Some(*BUILD.get_or_init(|| {
            // Linux keeps this handle on the running file even after a relink
            // replaces the path used to launch it.
            #[cfg(target_os = "linux")]
            let path = std::path::PathBuf::from("/proc/self/exe");
            #[cfg(not(target_os = "linux"))]
            let path = std::env::current_exe().expect("native module identity needs the running executable path");
            let file = std::fs::File::open(&path)
                .unwrap_or_else(|error| panic!("native module identity cannot read {}: {error}", path.display()));
            let mut digest = blake3::Hasher::new();
            digest.update_reader(file)
                .unwrap_or_else(|error| panic!("native module identity cannot read {}: {error}", path.display()));
            *digest.finalize().as_bytes()
        }))
    }
}
