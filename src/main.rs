#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_semihosting;
extern crate rtfm;
extern crate stm32g0xx_hal as hal;

mod r3tl;
mod tones;

use hal::exti::Event;
use hal::gpio::gpioa::{PA11, PA12};
use hal::gpio::gpioc::PC14;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use r3tl::Player;
use tones::TONES;
use rtfm::app;

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: stm32::EXTI,
        player: Player,
        right_eye: PA11<Output<PushPull>>,
        left_eye: PA12<Output<PushPull>>,
        buzzer: PC14<Output<PushPull>>,
    }

    #[init(spawn = [play_ringtone])]
    fn init(mut ctx: init::Context) -> init::LateResources {
        // HSI prescaler: 8, sys_clk: 2MHz
        let cfg = rcc::Config::hsi(rcc::Prescaler::Div8);
        let mut rcc = ctx.device.RCC.freeze(cfg);
        rcc.enable_low_power_mode();

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);
        gpioa.pa0.listen(SignalEdge::Falling, &mut ctx.device.EXTI);

        let mut buzzer = gpioc.pc14.into_push_pull_output();
        let mut right_eye = gpioa.pa11.into_push_pull_output();
        let mut left_eye = gpioa.pa12.into_push_pull_output();
        buzzer.set_high().unwrap();
        right_eye.set_low().unwrap();
        left_eye.set_low().unwrap();

        init::LateResources {
            buzzer,
            right_eye,
            left_eye,
            exti: ctx.device.EXTI,
            player: Player::new(
                ctx.device.TIM2.timer(&mut rcc),
                ctx.device.TIM3.timer(&mut rcc),
            ),
        }
    }

    #[task(resources = [player])]
    fn play_ringtone(mut ctx: play_ringtone::Context) {
        static mut COUNTER: usize = 0;
        let ringtone = TONES[*COUNTER % TONES.len()];
        ctx.resources.player.lock(|player| {
            player.play(ringtone);
        });
        *COUNTER += 1;
    }

    #[task(binds = EXTI0_1, resources = [exti, player], spawn = [play_ringtone])]
    fn button_click(ctx: button_click::Context) {
        ctx.spawn.play_ringtone().unwrap();
        ctx.resources.exti.unpend(Event::GPIO0);
    }

    #[task(binds = TIM2, priority = 1, resources = [player, right_eye, left_eye, buzzer])]
    fn frame_tick(ctx: frame_tick::Context) {
        let mut player = ctx.resources.player;
        let mut left_eye = ctx.resources.left_eye;
        let mut right_eye = ctx.resources.right_eye;
        let mut buzzer = ctx.resources.buzzer;
        player.lock(|player| {
            player.frame_tick();
            if !player.is_playing() {
                left_eye.lock(|left_eye| left_eye.set_low().unwrap());
                right_eye.lock(|right_eye| right_eye.set_low().unwrap());
                buzzer.lock(|buzzer| buzzer.set_high().unwrap());
            }
        });
    }

    #[task(binds = TIM3, priority = 2, resources = [player, right_eye, left_eye, buzzer])]
    fn sound_tick(ctx: sound_tick::Context) {
        ctx.resources.left_eye.toggle().unwrap();
        ctx.resources.right_eye.toggle().unwrap();
        ctx.resources.buzzer.toggle().unwrap();
        ctx.resources.player.sound_tick();
    }

    extern "C" {
        fn I2C1();
    }
};
