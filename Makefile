BOOTLOADER_BIN=./target/x86_64-unknown-uefi/debug/bootloader.efi
KERNEL_BIN=./target/x86_64-rsos-kernel/debug/kernel

all: build vdisk run

build:
	# cargo build
	# > not possible because crate-specific configuration-files are not evaluated correctly
	cd ./bootloader && cargo build
	cd ./kernel && cargo build

vdisk: $(BOOTLOADER_BIN) $(KERNEL_BIN)
	mkdir -p ./vmtest/EFI/Boot
	cp $(BOOTLOADER_BIN) ./vmtest/EFI/Boot/Bootx64.efi
	cp $(KERNEL_BIN) ./vmtest/EFI/Boot/kernel

run: vdisk
	qemu-system-x86_64 \
		-machine q35 \
		-boot menu=off \
		-drive if=pflash,format=raw,readonly=on,file=./vmtest/ovmf/OVMF.fd \
		-drive format=raw,file=fat:rw:./vmtest \
		-serial stdio

clean:
	rm -rf target
	rm -rf Cargo.lock
	# rm -rf vmtest
