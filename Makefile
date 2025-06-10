all: build vdisk run

build:
	cd ./bootloader && cargo build

vdisk: ./target/x86_64-unknown-uefi/debug/bootloader.efi
	mkdir -p ./vmtest/EFI/Boot
	cp ./target/x86_64-unknown-uefi/debug/bootloader.efi ./vmtest/EFI/Boot/Bootx64.efi

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
	rm -rf vmtest
