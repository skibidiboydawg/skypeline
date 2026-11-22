fn main() {
    println!("cargo:rerun-if-changed=cpp/audio_bridge.cpp");

    let mut build = cc::Build::new();
    build.cpp(true).file("cpp/audio_bridge.cpp");

    if build.get_compiler().is_like_msvc() {
        build.flag_if_supported("/EHsc");
    } else {
        build.flag_if_supported("-std=c++17");
    }

    // The Linux bridge uses std::thread for fire-and-forget chimes/notifications.
    // No ALSA symbols are used, so we deliberately do NOT link libasound here.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "linux" {
        build.flag_if_supported("-pthread");
        println!("cargo:rustc-link-arg=-pthread");
    }

    build.compile("skypeline_audio_bridge");
}
