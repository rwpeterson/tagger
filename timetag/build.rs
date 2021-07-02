#[cfg(windows)]
fn main() {
    cxx_build::bridge("src/lib.rs")
        .cpp(true)
        .file("src/taghelper.cc")
        .flag_if_supported("/std:c++latest")
        //.object("lib/CTimeTagLib.lib")
        .compile("ctimetagextra");

    // Specifying here instead of .object() above works on Windows
    // when the CXX module is in lib.rs. y tho?
    println!("cargo:rustc-link-lib=CTimeTagLib");
    println!("cargo:rustc-link-search=lib/");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/taghelper.cc");
    println!("cargo:rerun-if-changed=include/taghelper.h");
}

#[cfg(not(windows))]
fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("src/taghelper.cc")
        .flag_if_supported("-std=c++20")
        .cpp_link_stdlib("stdc++")
        .object("../lib/libtimetag64.so")
        .compile("ctimetagextra");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/taghelper.cc");
    println!("cargo:rerun-if-changed=include/taghelper.h");
}
