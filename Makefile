all: build vdisk run

build:
	cargo build

vdisk: ./target/x86_64-unknown-uefi/debug/rsos.efi
	mkdir -p ./vmtest/EFI/Boot
	cp ./target/x86_64-unknown-uefi/debug/rsos.efi ./vmtest/EFI/Boot/Bootx64.efi

run: vdisk
	qemu-system-x86_64 \
		-machine q35 \
		-boot menu=off \
		-drive if=pflash,format=raw,readonly=on,file=./vmtest/ovmf/OVMF.fd \
		-drive format=raw,file=fat:rw:./vmtest \
		-serial stdio

clean:
	rm -rf vmtest
