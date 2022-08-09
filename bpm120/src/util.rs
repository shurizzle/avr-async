pub fn reset_irqs(dp: &arduino_hal::Peripherals) {
    dp.EXINT.eimsk.reset(); // disable INTn
    dp.EXINT.pcmsk0.reset(); // disable PCINTn
    dp.TC0.timsk0.reset(); // disable TIMER0_* irqs
    dp.TC0.tccr0b.reset(); // disable TIMER0
    dp.TC1.timsk1.reset(); // disable TIMER1_* irqs
    dp.TC1.tccr1b.reset(); // disable TIMER1
    dp.TC3.timsk3.reset(); // disable TIMER3_* irqs
    dp.TC3.tccr3b.reset(); // disable TIMER3
    dp.TC4.timsk4.reset(); // disable TIMER4_* irqs
    dp.TC4.tccr4b.reset(); // disable TIMER4
    dp.USB_DEVICE.usbcon.reset(); // disable USB and interrupt
    dp.USB_DEVICE.udien.reset(); // disable USB interrupt
    dp.WDT.wdtcsr.reset(); // disable WDT
    dp.SPI.spcr.reset(); // disable SPI_STC
    dp.USART1.ucsr1b.reset(); // disable USART1_*
    dp.AC.acsr.reset(); // disable ANALOG_COMP
    dp.ADC.adcsra.reset(); // disable ADC
    dp.EEPROM.eecr.reset(); // disable EE_READY
    dp.TWI.twcr.reset(); // disable TWI
    dp.BOOT_LOAD.spmcsr.reset(); // disable SPM_READY
}
