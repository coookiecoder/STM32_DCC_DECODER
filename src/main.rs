#![no_std]
#![no_main]

#![allow(unused_imports)]

use panic_halt;

use stm32f4xx_hal as hal;

use hal::pac;

use hal::prelude::_stm32f4xx_hal_rcc_RccExt;
use hal::prelude::_stm32f4xx_hal_gpio_GpioExt;
use hal::prelude::_fugit_RateExtU32;

use hal::rcc::Config;

use hal::hal_02::digital::v2::InputPin;
use hal::hal_02::digital::v2::OutputPin;

use hal::spi::Spi;
use hal::spi::Mode;
use hal::spi::Phase;
use hal::spi::Polarity;

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
    let _dp = pac::Peripherals::take().expect("cannot take peripherals");
    let mut _cp = pac::CorePeripherals::take().expect("cannot take core peripherals");

    let config = Config::hse(25.MHz()).sysclk(84.MHz()).require_pll48clk();
    let mut rcc = _dp.RCC.freeze(config);

    _cp.DCB.enable_trace();
    _cp.DWT.enable_cycle_counter();

    let _gpio_a = _dp.GPIOA.split(&mut rcc);
    let _gpio_b = _dp.GPIOB.split(&mut rcc);
    let _gpio_c = _dp.GPIOC.split(&mut rcc);
    let _gpio_d = _dp.GPIOD.split(&mut rcc);
    let _gpio_e = _dp.GPIOE.split(&mut rcc);

    let dcc_input = _gpio_a.pa1.into_pull_up_input();
    let mut pico_output = _gpio_a.pa2.into_push_pull_output();

    let spi_clock = _gpio_a.pa5.into_alternate();
    let master_in_slave_out = _gpio_a.pa6.into_alternate();
    let master_out_slave_in = _gpio_a.pa7.into_alternate();

    let mode = Mode {polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition};

    let mut spi = Spi::new(_dp.SPI1, (Some(spi_clock), Some(master_in_slave_out), Some(master_out_slave_in)), mode, 0.Hz(), &mut rcc);

    let mut bits:usize = 0;
    let mut byte:usize = 0;

    let mut decoded_data = [0u8; DECODED_DATA_SIZE];
    let mut preamble_size:usize = 0;

    loop {
        while get_dcc_data(&dcc_input, &_cp.DWT) {
            preamble_size += 1;
        }

        while preamble_size > 9 && preamble_size < 24 {
            if get_dcc_data(&dcc_input, &_cp.DWT) {
                decoded_data[byte] |= 1 << (7 - bits);
            } else {
                decoded_data[byte] &= !(1 << (7 - bits));
            }

            bits += 1;

            if bits >= 8 {
                if get_dcc_data(&dcc_input, &_cp.DWT) == false {
                    byte += 1;
                } else {
                    byte = DECODED_DATA_SIZE;
                }
                bits = 0;
            }

            if byte >= DECODED_DATA_SIZE {
                send_dcc_data(&decoded_data, &mut pico_output, &mut spi);

                decoded_data = [0u8; DECODED_DATA_SIZE];
                byte = 0;
                preamble_size = 0;
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
    let delta_us = delta_cycles / 84;

    return delta_us < 100;
}

fn send_dcc_data<P: OutputPin, SPI: stm32f4xx_hal::spi::Instance>(data: &[u8; DECODED_DATA_SIZE], pin: &mut P, spi: &mut Spi<SPI>) -> () {
    let mut size:usize = DECODED_DATA_SIZE;

    while data[size - 1] == 0 {
        size -= 1;
    }

    pin.set_high().ok();
    spi.write(&data[0..size]).ok();
    pin.set_low().ok();
}