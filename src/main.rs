#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_halt;
extern crate rand;
extern crate rtfm;
extern crate stm32g0xx_hal as hal;

use hal::delay::Delay;
use hal::exti::Event;
use hal::gpio::{gpioa::*, gpioc::*};
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::stm32;
use hal::time::MicroSecond;
use hal::timer::Timer;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rtfm::app;

mod sound;

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        delay: Delay,
        rng: SmallRng,
        exti: stm32::EXTI,
        buzzer: PC14<Output<PushPull>>,
        right_eye: PA11<Output<PushPull>>,
        left_eye: PA12<Output<PushPull>>,
        sound_timer: Timer<stm32::TIM17>,
    }

    #[init( spawn = [play_intro])]
    fn init(mut ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.constrain();

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        gpioa.pa0.listen(SignalEdge::Falling, &mut ctx.device.EXTI);
        ctx.spawn.play_intro().unwrap();
        init::LateResources {
            exti: ctx.device.EXTI,
            sound_timer: ctx.device.TIM17.timer(&mut rcc),
            rng: SmallRng::from_seed([0; 16]),
            delay: ctx.core.SYST.delay(&rcc.clocks),
            right_eye: gpioa.pa11.into_push_pull_output(),
            left_eye: gpioa.pa12.into_push_pull_output(),
            buzzer: gpioc.pc14.into_push_pull_output(),
        }
    }

    #[task(binds = EXTI0_1, resources = [exti, rng], spawn = [play_intro, play_tone])]
    fn button_click(mut ctx: button_click::Context) {
        static mut COUNTER: u32 = 0;
        if *COUNTER == 3 {
            ctx.spawn.play_intro().unwrap();
        } else {
            let freq = *sound::NOTES.choose(&mut ctx.resources.rng).unwrap();
            ctx.spawn.play_tone(freq, 300.ms()).unwrap();
        }
        *COUNTER += 1;
        ctx.resources.exti.unpend(Event::GPIO0);
    }

    #[task(priority = 1, spawn = [play_tone])]
    fn play_intro(ctx: play_intro::Context) {
        for (freq, duration) in sound::INTRO.iter() {
            ctx.spawn.play_tone(*freq, duration.ms()).unwrap();
        }
    }

    #[task(priority = 2, capacity = 64, resources = [left_eye, right_eye, delay, sound_timer])]
    fn play_tone(mut ctx: play_tone::Context, freq: u32, duration: MicroSecond) {
        ctx.resources.right_eye.set_high().unwrap();
        ctx.resources.left_eye.set_high().unwrap();

        if freq > 0 {
            ctx.resources.sound_timer.lock(|timer| {
                timer.start(freq.hz());
                timer.listen();
            });
        }
        ctx.resources.delay.delay(duration);
        ctx.resources.sound_timer.lock(|timer| {
            timer.unlisten();
        });

        ctx.resources.right_eye.set_low().unwrap();
        ctx.resources.left_eye.set_low().unwrap();
    }

    #[task(binds = TIM17, priority = 3, resources = [sound_timer, buzzer])]
    fn sound_tick(ctx: sound_tick::Context) {
        ctx.resources.buzzer.toggle().unwrap();
        ctx.resources.sound_timer.clear_irq();
    }

    extern "C" {
        fn I2C1();
        fn I2C2();
    }
};
