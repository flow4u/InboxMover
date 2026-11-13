use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/inbox_mover.rc");
    println!("cargo:rerun-if-changed=assets/inbox_mover.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let assets_dir = manifest_dir.join("assets");
    let rc_file = assets_dir.join("inbox_mover.rc");

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        let res_file = out_dir.join("inbox_mover_icon.res");
        let status = Command::new("rc.exe")
            .arg("/nologo")
            .arg("/I")
            .arg(&assets_dir)
            .arg("/fo")
            .arg(&res_file)
            .arg(&rc_file)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg-bins={}", res_file.display());
            }
            _ => {
                println!("cargo:warning=rc.exe not found; skipping Windows PE icon embedding on MSVC");
            }
        }
    } else {
        let obj_file = out_dir.join("inbox_mover_icon.o");
        let candidates = ["x86_64-w64-mingw32-windres", "windres"];
        let mut compiled = false;
        for tool in candidates {
            let status = Command::new(tool)
                .arg("-I")
                .arg(&assets_dir)
                .arg("-i")
                .arg(&rc_file)
                .arg("-o")
                .arg(&obj_file)
                .status();
            if let Ok(s) = status {
                if s.success() {
                    println!("cargo:rustc-link-arg-bins={}", obj_file.display());
                    compiled = true;
                    break;
                }
            }
        }
        if !compiled {
            println!("cargo:warning=windres not found; skipping Windows PE icon embedding");
        }
    }
}
