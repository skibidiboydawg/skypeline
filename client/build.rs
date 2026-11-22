fn main() {
    println!("cargo:rerun-if-changed=cpp/audio_bridge.cpp");

    let mut build = cc::Build::new();
    build.cpp(true).file("cpp/audio_bridge.cpp");

    if build.get_compiler().is_like_msvc() {
        build.flag_if_supported("/EHsc");
    } else {
        build.flag_if_supported("-std=c++17");
    }

    build.compile("skypeline_audio_bridge");
}
