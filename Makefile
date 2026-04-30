compile:
	cargo build --target thumbv7em-none-eabihf --release
	cp target/thumbv7em-none-eabihf/release/STM32_DCC_DECODER ./target/STM32_DCC_DECODER.elf

add_target:
	rustup target add thumbv7em-none-eabihf
