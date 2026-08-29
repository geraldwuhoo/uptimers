use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIB: &str = "shoutrrr";

fn main() {
    let go_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("go");

    println!("cargo:rerun-if-env-changed=SHOUTRRR_LIB_DIR");
    for file in ["shoutrrr.go", "go.mod", "go.sum"] {
        println!("cargo:rerun-if-changed={}", go_dir.join(file).display());
    }

    // Container and CI builds produce the archive in a dedicated stage so the
    // Rust image does not need a Go toolchain; everywhere else we build it
    // ourselves so that a plain `cargo build` works on a fresh checkout.
    let lib_dir = match env::var_os("SHOUTRRR_LIB_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
            build_archive(&go_dir, &out_dir);
            out_dir
        }
    };

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static={LIB}");
}

/// Builds `libshoutrrr.a` from `go/shoutrrr.go` into `out_dir`.
///
/// Keep the flags here in sync with the `lib` stage of Dockerfile/Dockerfile.arm
/// and the `dependencies:libs` job in .gitlab-ci.yml, which build the same
/// archive ahead of time.
fn build_archive(go_dir: &Path, out_dir: &Path) {
    // cgo cross-compilation also needs a matching C toolchain, so a mismatch
    // here surfaces as a confusing link error much later. Fail loudly instead.
    let goos = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        other => panic!("no GOOS mapping for target OS `{other}`; set SHOUTRRR_LIB_DIR instead"),
    };
    let goarch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "arm",
        "x86" => "386",
        other => {
            panic!("no GOARCH mapping for target arch `{other}`; set SHOUTRRR_LIB_DIR instead")
        }
    };

    let status = Command::new("go")
        .current_dir(go_dir)
        .env("CGO_ENABLED", "1")
        .env("GOOS", goos)
        .env("GOARCH", goarch)
        .args([
            "build",
            "-buildmode=c-archive",
            "-trimpath",
            "-ldflags=-s -w",
        ])
        .arg("-o")
        .arg(out_dir.join(format!("lib{LIB}.a")))
        .arg("shoutrrr.go")
        .status()
        .expect(
            "failed to run `go build`: building uptimers requires a Go toolchain on PATH, \
             or set SHOUTRRR_LIB_DIR to a directory containing a prebuilt libshoutrrr.a",
        );

    assert!(status.success(), "`go build` failed to produce lib{LIB}.a");
}
