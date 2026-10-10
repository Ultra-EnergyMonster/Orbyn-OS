//! Build script for `boot-image`.
//!
//! Cargo builds the `kernel` crate for `x86_64-unknown-none` (the artifact dependency declared in
//! `Cargo.toml`) and passes the resulting ELF path in `CARGO_BIN_FILE_KERNEL_kernel`. This script
//! turns that ELF into bootable BIOS and UEFI disk images, using the same `bootloader` crate
//! version that the kernel was compiled against.
//!
//! Both images are plain files under `<repository root>/build/boot/`. No physical disk, block
//! device or host boot configuration is involved; `/build/` is gitignored.
//!
//! # When the images are regenerated
//!
//! Cargo only runs a build script when its own inputs change, so it cannot by itself notice that
//! the generated images were removed (`rm -rf build/`, documented as safe in
//! `docs/development/BUILDING.md`), left over from an older kernel, or overwritten by a test. This
//! script therefore records the content hash of the kernel ELF and of both images in `OUT_DIR` and
//! regenerates when any of the three differs from what it last wrote. That keeps generation
//! convergent: an up-to-date build writes nothing at all, while a missing or stale image is
//! rebuilt by the next `cargo build -p boot-image`.
//!
//! Environment read by this script is set by cargo itself, so the `expect` calls below describe
//! build-configuration errors, not untrusted input.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Output directory for generated images, relative to the repository root.
const OUTPUT_DIR: &str = "build/boot";

fn main() {
    let repo_root = repository_root();
    let output_dir = repo_root.join(OUTPUT_DIR);
    fs::create_dir_all(&output_dir)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", output_dir.display()));

    let kernel_path = std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").expect(
        "CARGO_BIN_FILE_KERNEL_kernel is not set: the `kernel` artifact dependency is missing \
         or artifact dependencies are not enabled ([unstable] bindeps = true in .cargo/config.toml)",
    );
    let kernel_path = PathBuf::from(kernel_path);

    let bios_image = output_dir.join("bios.img");
    let uefi_image = output_dir.join("uefi.img");

    // The stamp lives in `OUT_DIR`, which cargo owns and cleans, so it can never be confused with
    // a boot image or committed by accident.
    let stamp_path = PathBuf::from(
        std::env::var_os("OUT_DIR").expect("OUT_DIR is always set for cargo build scripts"),
    )
    .join("generated-images.stamp");

    let wanted = stamp_contents(&kernel_path, &bios_image, &uefi_image);
    let stored = fs::read_to_string(&stamp_path).ok();
    if stored.as_deref() != Some(wanted.as_str()) {
        bootloader::BiosBoot::new(&kernel_path)
            .create_disk_image(&bios_image)
            .unwrap_or_else(|err| panic!("failed to create {}: {err}", bios_image.display()));
        bootloader::UefiBoot::new(&kernel_path)
            .create_disk_image(&uefi_image)
            .unwrap_or_else(|err| panic!("failed to create {}: {err}", uefi_image.display()));
        let generated = stamp_contents(&kernel_path, &bios_image, &uefi_image);
        fs::write(&stamp_path, generated)
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", stamp_path.display()));
    }

    println!("cargo:rustc-env=BIOS_IMAGE_PATH={}", bios_image.display());
    println!("cargo:rustc-env=UEFI_IMAGE_PATH={}", uefi_image.display());
    println!("cargo:rerun-if-env-changed=CARGO_BIN_FILE_KERNEL_kernel");
    println!("cargo:rerun-if-changed=build.rs");
    // Watch the output directory so that a removed image (for example `rm -rf build/`) makes cargo
    // run this script again instead of reporting an up-to-date build with the images gone. Watching
    // the directory makes cargo look at the image files inside it too, which is harmless: the stamp
    // check above writes nothing while the kernel and both images are the ones it last produced.
    println!("cargo:rerun-if-changed={}", output_dir.display());
}

/// Describes the boot state this script last produced: the kernel ELF and both images.
///
/// Comparing this against the stamp file decides whether the images must be regenerated. Missing
/// images hash to `missing`, so a deleted image never matches a stored stamp.
fn stamp_contents(kernel_path: &Path, bios_image: &Path, uefi_image: &Path) -> String {
    format!(
        "kernel {}\nbios {}\nuefi {}\n",
        content_hash(kernel_path),
        content_hash(bios_image),
        content_hash(uefi_image)
    )
}

/// 64-bit FNV-1a hash of the file at `path`, or `missing` if the file does not exist.
///
/// This only has to notice whether a file changed, not to authenticate anything, so a small
/// non-cryptographic hash is enough and avoids a dependency in the build.
fn content_hash(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => {
            const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
            const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
            let mut hash = FNV_OFFSET_BASIS;
            for byte in bytes {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
            format!("{hash:016x}")
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => "missing".to_string(),
        Err(err) => panic!("failed to read {}: {err}", path.display()),
    }
}

/// Repository root, derived from this crate's location (`<root>/boot/image`).
fn repository_root() -> PathBuf {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is always set for cargo build scripts");
    let manifest_dir = Path::new(&manifest_dir);
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("boot/image must be two directories below the repository root")
        .to_path_buf()
}
