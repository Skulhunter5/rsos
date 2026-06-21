# RSOS – Rust x86-64 OS

## Build & Run

```sh
make build       # builds bootloader + kernel (each crate separately)
make vdisk       # creates FAT disk image with bootloader + kernel
make run         # builds + creates disk + boots in QEMU
make clean       # rm -rf target/ Cargo.lock vmtest disk image
```

Crate-level builds (workspace `cargo build` from root does NOT work — crate-specific `.cargo/config.toml` is ignored):

```sh
cd bootloader && cargo build    # target: x86_64-unknown-uefi
cd kernel && cargo build        # target: x86_64-rsos-kernel (custom JSON target)
```

Single commands for bootloader/kernel only:

```sh
cargo build -p bootloader   # uses workspace root config — only works if config is there
# Prefer: cd bootloader && cargo build
```

QEMU prerequisites: `mkfs.vfat`, `mmd`, `mcopy` (from `dosfstools` + `mtools`), `OVMF.fd` (UEFI firmware).

## Architecture

| Crate | Target | Role |
|-------|--------|------|
| `common/` | library (no_std) | Shared types: `PhysicalAddress`, `VirtualAddress`, `BootInfo`, `FixedBufferAllocator`, spinlock, paging primitives |
| `acpi/` | library (no_std) | ACPI table parsing (RSDP, RSDT, iterators) |
| `bootloader/` | `x86_64-unknown-uefi` (UEFI .efi) | UEFI app: PCI enumeration, AHCI driver, FAT filesystem, ELF loader, page table setup, exits boot services, jumps to kernel |
| `kernel/` | `x86_64-rsos-kernel` (custom, freestanding) | Physical memory mgr, VMM, GDT/IDT/interrupts, UART |

### Entrypoints

- **Bootloader**: `efi_main` (`extern "efiapi"`) in `bootloader/src/main.rs:129`
- **Kernel**: `_start` (`extern "sysv64"`) in `kernel/src/main.rs:122` → calls `kernel_main`
- Panic handler in each: infinite `hlt` loop, prints via UART

## Toolchain

- **Rust nightly** required (in root + per-crate `rust-toolchain.toml`)
- `edition = "2024"` across all crates
- Both bootloader and kernel use `build-std = ["core", "compiler_builtins", "alloc"]` with `compiler-builtins-mem`
- Kernel uses custom target spec: `kernel/x86_64-rsos-kernel.json` + custom linker script `kernel/link.ld`
- Kernel disables redzone, uses soft-float; features `-mmx,-sse,+soft-float`
- Bootloader target: `x86_64-unknown-uefi`

## Debug Output

All debug output goes through **UART serial** (`-serial stdio` in QEMU). Both bootloader and kernel have `print!`/`println!` macros wrapping UART. There is no VGA/text-mode output.

## Editor Setup

- `.vscode/settings.json`: rust-analyzer target = `x86_64-unknown-uefi`, `allTargets = false`
- `bootloader/rust-analyzer.toml` and `kernel/rust-analyzer.toml`: same config (target UEFI)
- Kernel's rust-analyzer uses UEFI target (not the kernel target) — this is intentional for editor support

## Key Conventions

- All crates are `#![no_std]` — no alloc crate by default (each one does `extern crate alloc`)
- Heavy use of nightly features: `allocator_api`, `ptr_metadata`, `abi_x86_interrupt`, `const_trait_impl`, etc.
- No test framework — verification is QEMU-based (`make run`)
- No CI pipelines found
- `Cargo.lock` and `vmtest/` are gitignored
- `vmtest/OVMF.fd` is tracked (UEFI firmware binary), `disk.img` and `NvVars` are not
