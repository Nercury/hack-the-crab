#![no_main]
#![no_std]

extern crate stm32g0 as device;

extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting;

// makes `panic!` print messages to the host stderr using semihosting
extern crate panic_semihosting;
//extern crate panic_halt;

use cortex_m_semihosting::hprintln;
use rt::entry;

extern crate rand;
use rand::Rng;
use rand::SeedableRng;
use rand::seq::SliceRandom;

// use `main` as the entry point of this application
// `main` is not allowed to return
#[entry]
fn main() -> ! {
    let peripherals = device::stm32g0x0::Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC;
    let mut port_a = peripherals.GPIOA;
    let mut port_c = peripherals.GPIOC;
    let mut tim2 = peripherals.TIM2;

    rcc.iopenr.modify(|_r, w|
        w
            .iopaen().bit(true)
            .iopcen().bit(true)
    );
    rcc.apbenr1.modify(|_r, w| w.tim2en().bit(true));

    port_a.pupdr.modify(|_r, w| unsafe {
        w
            .pupdr0().bits(0b10)
    });
    port_a.moder.modify(|_r, w| unsafe {
        w
            .moder0().bits(MODE_INPUT)
    });

    port_a.pupdr.modify(|_r, w| unsafe {
        w
            .pupdr11().bits(0)
            .pupdr12().bits(0)
    });
    port_a.ospeedr.modify(|_r, w| unsafe {
        w
            .ospeedr11().bits(OSPEED_LOW)
            .ospeedr12().bits(OSPEED_LOW)
    });
    port_a.otyper.modify(|_r, w|
        w
            .ot11().clear_bit()
            .ot12().clear_bit()
    );

    port_c.pupdr.modify(|_r, w| unsafe { w.pupdr14().bits(0) });
    port_c.ospeedr.modify(|_r, w| unsafe { w.ospeedr14().bits(OSPEED_LOW) });
    port_c.otyper.modify(|_r, w| w.ot14().clear_bit());

    let seed: [u8; 16] = [0; 16];
    let mut rng = rand::rngs::SmallRng::from_seed(seed);

    let notes = [A, B, C, A * 2, B * 2, C * 2, A * 3, B * 3, C * 3];
    let distr = rand::distributions::Uniform::new_inclusive(1, 100);
    let mut random_play_countdown = rng.gen_range(100, 1000);
    let mut playing = false;

    loop {
        if !playing {
            if port_a.idr.read().idr0().bit_is_set() {
                playing = true;
                sleep(200, &mut tim2);
            }
        } else {
            port_a.moder.modify(|_r, w| unsafe {
                w
                    .moder11().bits(MODE_OUTPUT)
                    .moder12().bits(MODE_OUTPUT)
            });
            port_c.moder.modify(|_r, w| unsafe {
                w
                    .moder14().bits(MODE_OUTPUT)
            });

            let left_eye = |v| {
                if v {
                    port_a.bsrr.write(|w|
                        w
                            .bs12().set_bit()
                    );
                } else {
                    port_a.brr.write(|w|
                        w
                            .br12().set_bit()
                    );
                }
            };

            let right_eye = |v| {
                if v {
                    port_a.bsrr.write(|w|
                        w
                            .bs11().set_bit()
                    );
                } else {
                    port_a.brr.write(|w|
                        w
                            .br11().set_bit()
                    );
                }
            };

            left_eye(true);
            right_eye(true);

            sleep(200, &mut tim2);

            let note = *notes.choose(&mut rng).unwrap();
            produce(250, note, &mut port_c, &mut tim2);

            sleep(200, &mut tim2);

            left_eye(false);
            right_eye(false);

            port_a.moder.modify(|_r, w| unsafe {
                w
                    .moder11().bits(MODE_INPUT)
                    .moder12().bits(MODE_INPUT)
            });
            port_c.moder.modify(|_r, w| unsafe {
                w
                    .moder14().bits(MODE_INPUT)
            });

            playing = false;
        }
    }
}

fn sleep(times: u32, tim2: &mut device::stm32g0x0::TIM2) {
    for i in 0..times {
        pause(tim2, A);
    }
}

fn produce(times: u32, note: u16, port_c: &mut device::stm32g0x0::GPIOC, tim2: &mut device::stm32g0x0::TIM2) {
    for i in 0..times {
        port_c.bsrr.write(|w|
            w
                .bs14().set_bit()
        );
        pause(tim2, note);
        port_c.brr.write(|w|
            w
                .br14().set_bit()
        );
        pause(tim2, note);
    }
}

const A: u16 = 0x8F7;
const B: u16 = (0x8F7 as f32 / 1.122462) as u16;
const C: u16 = (0x8F7 as f32 / 1.189207) as u16;

fn pause(tim2: &mut device::stm32g0x0::TIM2, time: u16) {
        tim2.egr.write(|w| w.ug().set_bit());
        tim2.cr1.write(|w| w.cen().set_bit());
        while tim2.cnt.read().cnt_l().bits() < time { cortex_m::asm::nop() }
        tim2.cr1.write(|w| w.cen().clear_bit());
}

const OSPEED_VERY_LOW: u8 = 0b00;
const OSPEED_LOW: u8 = 0b01;
const OSPEED_HIGH: u8 = 0b00;
const OSPEED_VERY_HIGH: u8 = 0b11;

const MODE_INPUT: u8 = 0b00;
const MODE_OUTPUT: u8 = 0b01;
const MODE_AF: u8 = 0b10;
const MODE_ANALOG: u8 = 0b11;