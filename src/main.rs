#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_semihosting;
extern crate rtfm;
extern crate stm32g0xx_hal as hal;

mod r3tl;

use hal::exti::Event;
use hal::gpio::gpioa::{PA11, PA12};
use hal::gpio::gpioc::PC14;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use r3tl::Player;
use rtfm::app;

pub const RINGTONES: [&str; 5] = [
    "Simpsons:d=4,o=5,b=160:32p,c.6,e6,f#6,8a6,g.6,e6,c6,8a,8f#,8f#,8f#,2g",
    "Xfiles:d=4,o=5,b=140:e,b,a,b,d6,2b.,1p,e,b,a,b,e6,2b.,1p,g6,f#6,e6,d6,e6,2b.,1p,g6,f#6,e6,d6,f#6,2b.,1p,e,b,a,b,d6,2b.,1p,e,b,a,b,e6,2b.,1p",
    "MahnaMahna:d=16,o=6,b=180:c#,c.,b5,8a#.5,8f.,4g#,a#,g.,4d#,8p,c#,c.,b5,8a#.5,8f.,g#.,8a#.,4g,8p,c#,c.,b5,8a#.5,8f.,4g#,f,g.,8d#.,f,g.,8d#.,f,8g,8d#.,f,8g,d#,8c,a#5,8d#.,8d#.,4d#,8d#.",
    "Looney:d=4,o=5,b=180:32p,c6,8f6,8e6,8d6,8c6,a.,8c6,8f6,8e6,8d6,8d#6,e.6,8e6,8e6,8c6,8d6,8c6,8e6,8c6,8d6,8a,8c6,8g,8a#,8a,8f",
    "Muppets:d=4,o=5,b=160:c6,c6,a,b,8a,b,g,p,c6,c6,a,8b,8a,8p,g.,p,e,e,g,f,8e,f,8c6,8c,8d,e,8e,8e,8p,8e,g,2p,c6,c6,a,b,8a,b,g,p,c6,c6,a,8b,a,g.,p,e,e,g,f,8e,f,8c6,8c,8d,e,8e,d,8d,c",
];

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
        // SysSlock: 2MHz
        let cfg = rcc::Config::hsi(rcc::Prescaler::Div8);
        let mut rcc = ctx.device.RCC.freeze(cfg);
        rcc.enable_low_power_mode();

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);
        gpioa.pa0.listen(SignalEdge::Falling, &mut ctx.device.EXTI);

        init::LateResources {
            exti: ctx.device.EXTI,
            buzzer: gpioc.pc14.into_push_pull_output(),
            right_eye: gpioa.pa11.into_push_pull_output(),
            left_eye: gpioa.pa12.into_push_pull_output(),
            player: Player::new(
                ctx.device.TIM2.timer(&mut rcc),
                ctx.device.TIM3.timer(&mut rcc),
            ),
        }
    }

    #[task(resources = [player])]
    fn play_ringtone(mut ctx: play_ringtone::Context) {
        static mut COUNTER: usize = 0;
        let ringtone = RINGTONES[*COUNTER % RINGTONES.len()];
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
