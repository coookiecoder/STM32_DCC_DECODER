#![no_std]
#![no_main]

#![allow(unused_imports)]

use panic_halt;

use stm32f4xx_hal as hal;

use hal::pac;

use hal::prelude::_stm32f4xx_hal_rcc_RccExt;
use hal::prelude::_stm32f4xx_hal_gpio_GpioExt;
use hal::prelude::_stm32f4xx_hal_timer_PwmExt;
use hal::prelude::_fugit_RateExtU32;

use hal::rcc::Config;

use hal::hal_02::digital::v2::InputPin;
use hal::hal_02::digital::v2::OutputPin;

use hal::spi::SpiSlave;
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

    let mut led_output = _gpio_c.pc13.into_push_pull_output();

    let dcc_input = _gpio_a.pa1.into_input();
    let mut pico_output = _gpio_a.pa2.into_push_pull_output();

    let (_pwm_mgr_t1, (c1, c2, c3, c4)) = _dp.TIM1.pwm_hz(20.kHz(), &mut rcc);
    let mut gate_s1 = c1.with(_gpio_a.pa8);
    let mut gate_s2 = c2.with(_gpio_a.pa9);
    let mut gate_s3 = c3.with(_gpio_a.pa10);
    let mut gate_s4 = c4.with(_gpio_a.pa11);

    gate_s1.set_duty(0);
    gate_s2.set_duty(0);
    gate_s3.set_duty(0);
    gate_s4.set_duty(0);

    let (_pwm_mgr_t2, (c1, _, _, _)) = _dp.TIM2.pwm_hz(20.kHz(), &mut rcc);
    let mut motor_output = c1.with(_gpio_a.pa15.into_push_pull_output());

    motor_output.set_duty(0);

    let (_pwm_mgr_t4, (c1, c2, c3, c4)) = _dp.TIM4.pwm_hz(20.kHz(), &mut rcc);
    let mut pwm_1 = c1.with(_gpio_b.pb6);
    let mut pwm_2 = c2.with(_gpio_b.pb7);
    let mut front_light = c3.with(_gpio_b.pb8);
    let mut rear_light = c4.with(_gpio_b.pb9);

    pwm_1.set_duty(0);
    pwm_2.set_duty(0);
    front_light.set_duty(0);
    rear_light.set_duty(0);

    let spi_clock = _gpio_a.pa5.into_alternate();
    let master_in_slave_out = _gpio_a.pa6.into_alternate();
    let master_out_slave_in = _gpio_a.pa7.into_alternate();
    let negative_slave_select = _gpio_a.pa4.into_alternate();

    let mode = Mode {polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition};

    let mut spi = SpiSlave::new(_dp.SPI1, (Some(spi_clock), Some(master_in_slave_out), Some(master_out_slave_in), Some(negative_slave_select)), mode, &mut rcc);

    let mut bits:usize = 0;
    let mut byte:usize = 0;

    let mut decoded_data: [u8; DECODED_DATA_SIZE] = [0u8; DECODED_DATA_SIZE];
    let mut data_from_pi_0: [u8; DECODED_DATA_SIZE] = [0u8; DECODED_DATA_SIZE];
    let mut preamble_size:usize = 0;

    led_output.set_low();
    set_pwm_state(&mut front_light, 20000);
    set_pwm_state(&mut rear_light, 20000);
    delay_ms(&_cp.DWT, 1000);
    led_output.set_high();
    set_pwm_state(&mut front_light, 0);
    set_pwm_state(&mut rear_light, 0);

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


                pico_output.set_high(); //tell pi 0 we are read to send data
                led_output.set_low();
                spi.transfer(&mut data_from_pi_0, &decoded_data).ok();
                pico_output.set_low(); //tell pi 0 we sent all the data
                led_output.set_high();

                set_pwm_state(&mut gate_s1, u16::from_be_bytes([data_from_pi_0[0], data_from_pi_0[1]]));
                set_pwm_state(&mut gate_s2, u16::from_be_bytes([data_from_pi_0[2], data_from_pi_0[3]]));
                set_pwm_state(&mut gate_s3, u16::from_be_bytes([data_from_pi_0[4], data_from_pi_0[5]]));
                set_pwm_state(&mut gate_s4, u16::from_be_bytes([data_from_pi_0[6], data_from_pi_0[7]]));

                set_pwm_state(&mut motor_output, u16::from_be_bytes([data_from_pi_0[8], data_from_pi_0[9]]));
                //byte 10 -- 11
                //byte 12 -- 13
                //byte 14 -- 15

                //byte 16 -- 17
                //byte 18 -- 19
                //byte 20 -- 21
                //byte 22 -- 23

                set_pwm_state(&mut pwm_1, u16::from_be_bytes([data_from_pi_0[24], data_from_pi_0[25]]));
                set_pwm_state(&mut pwm_2, u16::from_be_bytes([data_from_pi_0[26], data_from_pi_0[27]]));
                set_pwm_state(&mut front_light, u16::from_be_bytes([data_from_pi_0[28], data_from_pi_0[29]]));
                set_pwm_state(&mut rear_light, u16::from_be_bytes([data_from_pi_0[30], data_from_pi_0[31]]));

                decoded_data = [0u8; DECODED_DATA_SIZE];
                byte = 0;
                preamble_size = 0;
            }
        }

        preamble_size = 0;
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

fn delay_ms(dwt: &DWT, time_ms: u32) {
    let start_cycles = dwt.cyccnt.read();

    let until = time_ms.wrapping_mul(84_000);

    while dwt.cyccnt.read().wrapping_sub(start_cycles) < until {
        cortex_m::asm::nop();
    }
}

use stm32f4xx_hal::hal_02::PwmPin;

fn set_pwm_state(pwn_pin: &mut impl PwmPin<Duty = u16>, duty_cycle: u16) {
    if duty_cycle > 0 {
        pwn_pin.set_duty(duty_cycle);
        pwn_pin.enable();
    } else {
        pwn_pin.disable();
    }
}

/// Updates PWM pin states and light outputs from a 64-byte big-endian command payload.
///
/// Parses 16-bit big-endian unsigned integers from the payload slice `data_from_pi_0`
/// and updates the duty cycle or state of the corresponding hardware peripherals.
///
/// # Payload Mapping
///
/// | Byte Range | Target Peripheral | Description |
/// | :--- | :--- | :--- |
/// | `[0..1]` | `gate_s1` | Gate driver channel 1 PWM signal |
/// | `[2..3]` | `gate_s2` | Gate driver channel 2 PWM signal |
/// | `[4..5]` | `gate_s3` | Gate driver channel 3 PWM signal |
/// | `[6..7]` | `gate_s4` | Gate driver channel 4 PWM signal |
/// | `[8..9]` | `motor_output` | Main motor driver PWM duty cycle |
/// | `[10..11]` | *(Unused)* | Unused for now |
/// | `[12..13]` | *(Unused)* | Unused for now |
/// | `[14..15]` | *(Unused)* | Unused for now |
/// | `[16..17]` | *(Unused)* | Unused for now |
/// | `[18..19]` | *(Unused)* | Unused for now |
/// | `[20..21]` | *(Unused)* | Unused for now |
/// | `[22..24]` | *(Unused)* | Unused for now |
/// | `[24..25]` | `pwm_1` | Auxiliary PWM channel 1 |
/// | `[26..27]` | `pwm_2` | Auxiliary PWM channel 2 |
/// | `[28..29]` | `front_light` | Front light intensity PWM |
/// | `[30..31]` | `rear_light` | Rear light intensity PWM |
/// | `[32..33]` | *(Unused)* | Unused for now |
/// | `[34..35]` | *(Unused)* | Unused for now |
/// | `[36..37]` | *(Unused)* | Unused for now |
/// | `[38..39]` | *(Unused)* | Unused for now |
/// | `[40..41]` | *(Unused)* | Unused for now |
/// | `[42..43]` | *(Unused)* | Unused for now |
/// | `[44..45]` | *(Unused)* | Unused for now |
/// | `[46..47]` | *(Unused)* | Unused for now |
/// | `[48..49]` | *(Unused)* | Unused for now |
/// | `[50..51]` | *(Unused)* | Unused for now |
/// | `[52..53]` | *(Unused)* | Unused for now |
/// | `[54..55]` | *(Unused)* | Unused for now |
/// | `[56..57]` | *(Unused)* | Unused for now |
/// | `[58..59]` | *(Unused)* | Unused for now |
/// | `[60..61]` | *(Unused)* | Unused for now |
/// | `[62..63]` | *(Unused)* | Unused for now |
///
/// # Panics
///
/// Panics if `data_from_pi_0` contains fewer than 64 bytes (index out of bounds).
#[allow(dead_code)]
fn spi_help() -> () {

}