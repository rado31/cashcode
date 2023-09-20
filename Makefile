run:
	rm -rf target
	cargo build --target x86_64-pc-windows-gnu
	scp target/x86_64-pc-windows-gnu/debug/cashcode.exe richxcame@192.168.194.250:/C:/Users/richxcame/Desktop

