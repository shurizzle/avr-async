.SILENT:

CHIPS := at90usb1286 atmega1280 atmega1284p atmega128rfa1 atmega164pa atmega168 atmega2560 atmega8 atmega8u2 atmega328p atmega328pb atmega32u4 atmega4809 atmega48p atmega64 atmega644 attiny13a attiny202 attiny2313 attiny2313a attiny84 attiny85 attiny88 attiny816 attiny841 attiny861 attiny167 attiny1614

RUSTUP_TOOLCHAIN ?= nightly

AVRDEV = avr-device
AVRDEVGIT = $(AVRDEV)/.git

$(AVRDEVGIT):
	@echo -e "\tSYNC\t\t$(AVRDEV)"
	@git submodule sync $(AVRDEV)
	@git submodule update --init $(AVRDEV)

$(AVRDEV)/svd/%.svd.patched: $(AVRDEVGIT)
	@$(MAKE) -C $(AVRDEV) $(patsubst $(AVRDEV)/%,%,$@)

clean:
	@$(MAKE) -C $(AVRDEV) clean
