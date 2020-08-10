/// RTTTL (RingTone Text Transfer Language) is the primary format used to distribute
/// ringtones for Nokia phones. An RTTTL file is a text file, containing the
/// ringtone name, a control section and a section containing a comma separated
/// sequence of ring tone commands. White space must be ignored by any reader
/// application.
use core::str::Split;

use crate::hal::prelude::*;
use crate::hal::stm32;
use crate::hal::timer::Timer;

pub type FrameTimer = Timer<stm32::TIM2>;
pub type SoundTimer = Timer<stm32::TIM3>;

pub struct Player {
    frame_timer: FrameTimer,
    sound_timer: SoundTimer,
    ringtone: Option<Ringtone>,
    frame: u32,
    tone: Tone,
}

impl Player {
    pub fn new(frame_timer: FrameTimer, sound_timer: SoundTimer) -> Player {
        Player {
            frame_timer,
            sound_timer,
            ringtone: None,
            tone: SILENCE,
            frame: 0,
        }
    }

    pub fn play(&mut self, r3tl: &'static str) {
        self.stop();

        let ringtone = Ringtone::parse(r3tl);
        self.frame_timer.start(ringtone.bpm.hz());
        self.frame_timer.listen();

        self.ringtone = Some(ringtone);
        self.frame = 0;
    }

    pub fn is_playing(&self) -> bool {
        self.ringtone.is_some()
    }

    pub fn stop(&mut self) {
        self.ringtone = None;
        self.frame_timer.unlisten();
        self.sound_timer.unlisten();
        self.frame_timer.pause();
        self.sound_timer.pause();
    }

    pub fn frame_tick(&mut self) {
        self.frame_timer.clear_irq();
        if self.frame == 0 {
            if let Some(ringtone) = &mut self.ringtone {
                if let Some(tone) = ringtone.next() {
                    self.sound_timer.start(tone.freq.hz());
                    self.tone = tone;
                } else {
                    return self.stop();
                };
            }
        }

        if self.tone.freq > 1 && self.frame <= self.tone.frames {
            self.sound_timer.listen();
        } else {
            self.sound_timer.unlisten();
        }
        self.frame = if self.frame == 64 { 0 } else { self.frame + 1 };
    }

    pub fn sound_tick(&mut self) {
        self.sound_timer.clear_irq()
    }
}

#[derive(Debug, Clone, Copy)]
struct Tone {
    pub freq: u32,
    pub frames: u32,
}

const SILENCE: Tone = Tone { freq: 10_000, frames: 0 };

#[derive(Debug)]
struct Ringtone {
    pub r3tl: &'static str,
    pub name: &'static str,
    pub duration: u32,
    pub octave: u32,
    pub bpm: u32,
    pub tones: Option<Split<'static, &'static str>>,
}

impl Ringtone {
    pub fn parse(r3tl: &'static str) -> Ringtone {
        let mut duration = 4;
        let mut octave = 6;
        let mut bpm = 65;

        let mut rtx = r3tl.split(':');
        let name = rtx.next().unwrap();
        let control = rtx.next().unwrap().split(',');
        for param in control {
            match param.split_at(2) {
                ("d=", val) => duration = val.parse().unwrap(),
                ("o=", val) => octave = val.parse().unwrap(),
                ("b=", val) => bpm = val.parse().unwrap(),
                _ => {}
            }
        }

        Ringtone {
            r3tl,
            name,
            duration,
            octave,
            bpm,
            tones: Some(rtx.next().unwrap().split(&",")),
        }
    }

    fn parse_tone(&self, tone: &'static str) -> Tone {
        const NOTES: [[u32; 13]; 4] = [
            [
                262, 277, 294, 311, 330, 349, 370, 392, 415, 440, 466, 494, 1,
            ],
            [
                523, 554, 587, 622, 659, 698, 740, 784, 831, 880, 932, 988, 1,
            ],
            [
                1046, 1109, 1175, 1245, 1319, 1397, 1480, 1568, 1661, 1760, 1865, 1976, 1,
            ],
            [
                2093, 2217, 2349, 2489, 2637, 2794, 2960, 3136, 3322, 3520, 3729, 3951, 1,
            ],
        ];

        let dur_len = tone.chars().take_while(|ch| ch.is_numeric()).count();
        let duration = if dur_len > 0 {
            tone[0..dur_len].parse().unwrap_or(self.duration)
        } else {
            self.duration
        };
        let tone = &tone[dur_len..];
        let note_len = tone
            .chars()
            .take_while(|ch| "#abcdefgp".contains(*ch))
            .count();
        let note_idx: usize = match &tone[..note_len] {
            "c" => 0,
            "c#" => 1,
            "d" => 2,
            "d#" => 3,
            "e" => 4,
            "f" => 5,
            "f#" => 6,
            "g" => 7,
            "g#" => 8,
            "a" => 9,
            "a#" => 10,
            "b" => 11,
            "p" => 12,
            _ => {
                return SILENCE;
            }
        };
        let mut dot = false;
        let mut octave = self.octave;
        for sym in tone[note_len..].chars() {
            match sym {
                '.' => dot = true,
                scale => octave = scale.to_digit(10).unwrap_or(self.octave),
            }
        }
        let mut frames = 120 / duration;
        if dot {
            frames += frames / 2
        }
        let octave_idx = octave as usize - 4;
        let freq = NOTES[octave_idx][note_idx];
        Tone { freq, frames }
    }
}

impl Iterator for Ringtone {
    type Item = Tone;

    fn next(&mut self) -> Option<Tone> {
        if let Some(tone) = &mut self.tones {
            return tone.next().map(|val| self.parse_tone(val));
        }
        None
    }
}
