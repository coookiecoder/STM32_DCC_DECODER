#![no_std]
#![no_main]

#![allow(unused_imports)]

use panic_halt;

use stm32f3xx_hal as hal;

use hal::pac;

use hal::prelude::*;

use hal::spi::{Mode, Polarity, Phase, Spi};
use hal::spi::config::Config as SpiConfig;
use hal::hal::digital::v2::InputPin;
use hal::hal::digital::v2::OutputPin;

pub fn fake_exit() -> ! {
    loop {
        cortex_m::asm::nop();
    }
}

pub fn fake_debug_exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

use cortex_m_rt::entry;

const DECODED_DATA_SIZE: usize = 64;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().expect("cannot take peripherals");
    let mut cp = pac::CorePeripherals::take().expect("cannot take core peripherals");

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.sysclk(72.MHz()).freeze(&mut flash.acr);

    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

    let dcc_input = gpioa.pa1.into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
    let mut pico_output = gpioa.pa2.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);

    // Use into_af_push_pull with AF5 for SPI1
    let spi_clock = gpioa.pa5.into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
    let master_in_slave_out = gpioa.pa6.into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
    let master_out_slave_in = gpioa.pa7.into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
    let _negative_slave_select = gpioa.pa4.into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);

    let mode = Mode {polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition};
    let spi_config = SpiConfig::default().mode(mode);

    let mut spi = Spi::new(dp.SPI1, (spi_clock, master_in_slave_out, master_out_slave_in), spi_config, clocks, &mut rcc.apb2);

    // Force slave mode by clearing MSTR bit in CR1
    unsafe {
        (*pac::SPI1::ptr()).cr1.modify(|_, w| w.mstr().slave());
    }

    let mut decoded_data = [0u8; DECODED_DATA_SIZE];

    loop {
        let mut preamble_size: usize = 0;
        while get_dcc_data(&dcc_input, &cp.DWT) {
            preamble_size += 1;
        }

        if preamble_size >= 10 {
            let mut byte: usize = 0;
            let mut bits: usize = 0;
            decoded_data.fill(0);

            loop {
                if get_dcc_data(&dcc_input, &cp.DWT) {
                    decoded_data[byte] |= 1 << (7 - bits);
                } else {
                    decoded_data[byte] &= !(1 << (7 - bits));
                }

                bits += 1;

                if bits >= 8 {
                    byte += 1;
                    bits = 0;

                    // Packet End Bit (1) or Data Byte Separator (0)
                    if get_dcc_data(&dcc_input, &cp.DWT) {
                        // End of packet
                        break;
                    }
                }

                if byte >= DECODED_DATA_SIZE {
                    break;
                }
            }

            if byte > 0 {
                pico_output.set_high().ok(); // tell pico we are ready to send data
                spi.write(&decoded_data[0..byte]).ok();
                pico_output.set_low().ok(); // tell pico we sent all the data
            }
        }
    }
}

use cortex_m::peripheral::DWT;

fn get_dcc_data<P: InputPin>(pin: &P, dwt: &DWT) -> bool {
    while pin.is_low().unwrap_or(true) {
        cortex_m::asm::nop();
    }

    let start_cycles = dwt.cyccnt.read();

    while pin.is_high().unwrap_or(true) {
        cortex_m::asm::nop();
    }

    let end_cycles = dwt.cyccnt.read();
    let delta_cycles = end_cycles.wrapping_sub(start_cycles);
    let delta_us = delta_cycles / 72;

    return delta_us < 100;
}
