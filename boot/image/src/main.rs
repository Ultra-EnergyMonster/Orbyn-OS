//! Reports the paths of the boot images created by this crate's build script.
//!
//! Build flow: `cargo build -p boot-image` (or `cargo run -p boot-image` from the repository
//! root) builds the kernel for `x86_64-unknown-none`, then generates `build/boot/bios.img` and
//! `build/boot/uefi.img`. Boot-testing them is `tools/test/qemu-smoke.sh bios|uefi`.

/// Path of the generated BIOS (SeaBIOS-compatible) disk image.
const BIOS_IMAGE_PATH: &str = env!("BIOS_IMAGE_PATH");

/// Path of the generated UEFI disk image.
const UEFI_IMAGE_PATH: &str = env!("UEFI_IMAGE_PATH");

fn main() {
    println!("boot images generated for the New OS kernel prototype:");
    println!("  BIOS (legacy): {BIOS_IMAGE_PATH}");
    println!("  UEFI:          {UEFI_IMAGE_PATH}");
    println!();
    println!("boot-test them with (requires QEMU):");
    println!("  tools/test/qemu-smoke.sh bios");
    println!("  tools/test/qemu-smoke.sh uefi");
}
