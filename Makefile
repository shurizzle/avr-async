.SILENT:

CHIPS := at90usb1286 atmega1280 atmega1284p atmega128rfa1 atmega164pa atmega168 atmega2560 atmega8 atmega8u2 atmega328p atmega328pb atmega32u4 atmega4809 atmega48p atmega64 atmega644 attiny13a attiny202 attiny2313 attiny2313a attiny84 attiny85 attiny88 attiny816 attiny841 attiny861 attiny167 attiny1614

RUSTUP_TOOLCHAIN ?= nightly

AVRDEV = avr-device
AVRDEVGIT = $(AVRDEV)/.git

all: avr-async/src/chip/mod.rs $(CHIPS)

$(foreach chip, $(CHIPS), $(eval $(chip): avr-async/src/chip/$(chip).rs))

avr-async/src/chip/mod.rs: $(foreach chip, $(CHIPS), avr-async/src/chip/$(chip).rs)
	@($(foreach chip, $(CHIPS), echo -e '#[cfg(feature = "$(chip)")]\n#[path = "$(chip).rs"]\nmod inner;\n';) echo -e '#[cfg(all(\n $(foreach chip,$(CHIPS),   not(feature = "$(chip)"),\n)))]\ncompile_error!("Select only one board");\n'; echo 'pub use inner::*;') > $@
	@RUSTUP_TOOLCHAIN="$(RUSTUP_TOOLCHAIN)" rustfmt $@

$(AVRDEVGIT):
	@echo -e "\tSYNC\t\t$(AVRDEV)"
	@git submodule sync $(AVRDEV)
	@git submodule update --init $(AVRDEV)

$(AVRDEV)/svd/%.svd.patched: $(AVRDEVGIT)
	@$(MAKE) -C $(AVRDEV) $(patsubst $(AVRDEV)/%,%,$@)

avr-async/src/chip/%.rs: $(AVRDEV)/svd/%.svd.patched
	@mkdir -p $(@D)
	@echo -e "\tFORM\t\t$*"
	@if ! which svd2async-runtime >/dev/null 2>/dev/null; then echo "Please install svd2async-runtime: cargo install --git https://github.com/shurizzle/svd2async-runtime.git" >&2; exit 1; fi
	@svd2async-runtime -v 1 $< > $@
	@RUSTUP_TOOLCHAIN="$(RUSTUP_TOOLCHAIN)" rustfmt $@

clean:
	@$(MAKE) -C $(AVRDEV) clean
	@rm -f $(foreach chip, $(CHIPS), avr-async/src/chip/$(chip).rs)
	@rm -f avr-async/src/chip/mod.rs
