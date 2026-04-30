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

use hal::Listen;

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

#[entry]
fn main() -> ! {
    let _dp = pac::Peripherals::take().expect("cannot take peripherals");
    let _cp = pac::CorePeripherals::take().expect("cannot take core peripherals");

    let config = Config::hse(25.MHz()).sysclk(84.MHz()).require_pll48clk();
    let mut rcc = _dp.RCC.freeze(config);

    let _gpio_a = _dp.GPIOA.split(&mut rcc);
    let _gpio_b = _dp.GPIOB.split(&mut rcc);
    let _gpio_c = _dp.GPIOC.split(&mut rcc);
    let _gpio_d = _dp.GPIOD.split(&mut rcc);
    let _gpio_e = _dp.GPIOE.split(&mut rcc);

    let onboard_button = _gpio_a.pa0.into_pull_up_input();
    let mut onboard_led = _gpio_c.pc13.into_push_pull_output();

    loop {
        if onboard_button.is_low() {
            onboard_led.set_low();
        } else {
            onboard_led.set_high();
        }
    }
}
