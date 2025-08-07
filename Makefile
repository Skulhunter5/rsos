BOOTLOADER_BIN=./target/x86_64-unknown-uefi/debug/bootloader.efi
KERNEL_BIN=./target/x86_64-rsos-kernel/debug/kernel
DISK_IMG=./vmtest/disk.img
OVMF=./vmtest/OVMF.fd

all: build vdisk run

build:
	# workspace global 'cargo build'
	# > not possible because crate-specific configuration-files are not evaluated correctly
	cd ./bootloader && cargo build
	cd ./kernel && cargo build

vdisk: setup $(BOOTLOADER_BIN) $(KERNEL_BIN)
	mkdir -p ./vmtest
	rm -f $(DISK_IMG)
	mkfs.vfat -n "RSOS" -C $(DISK_IMG) 65535
	mmd -i $(DISK_IMG) ::/EFI
	mmd -i $(DISK_IMG) ::/EFI/BOOT
	mcopy -i $(DISK_IMG) $(BOOTLOADER_BIN) ::/EFI/BOOT/BOOTX64.EFI
	mcopy -i $(DISK_IMG) $(KERNEL_BIN) ::/EFI/BOOT/

run: vdisk
	qemu-system-x86_64 \
		-machine q35 \
		-boot menu=off \
		-m 2G \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF) \
		-drive format=raw,file=$(DISK_IMG),media=disk \
		-serial stdio

setup:
	mkdir -p ./vmtest

clean:
	rm -rf ./target/
	rm -f ./Cargo.lock
	rm $(DISK_IMG)
