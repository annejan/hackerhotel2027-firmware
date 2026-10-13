//! The challenges; same as `c/src/challenge.c`: five games, one per arm,
//! five levels each, a level for every LED in the arm's line. All 25 and
//! the badge does its victory dance.
//!
//!   arm 0 (top)          MAZE          mazes solved: 1 2 3 4 5
//!   arm 1 (A's arm)      SIMON         sequence repeated: 4 6 8 10 12
//!   arm 2 (▶'s leg)      WHACK-A-MOLE  points: 20 40 60 80 100
//!   arm 3 (◀'s leg)      HIGHER LOWER  streak: 3 5 7 9 12
//!   arm 4 (B's arm)      SNAKE         food eaten: 5 10 15 20 30
//!
//! A level counts when a real game (not the demo) ends with that score.
//! The name tag shows the progress on the arms, so people can compare.
//! main.rs keeps it in flash (store.rs); it only ever goes up.

use crate::anim::FPS;
use crate::font::WIDTH as FONT_W;
use crate::frame::{draw_text, hsv, polar, sin8, Frame, Rgb, Rng};
use crate::game::{arm_line, arm_sides, Kind, ARM_LINE};
use crate::generated::{EDGE, GRID_W, RGB};

pub const CHALLENGES: usize = 5;
pub const LEVELS: u8 = 5;

struct Challenge {
    game: Kind,
    name: &'static str,
    level: [u16; LEVELS as usize], // the score each level needs
    colour: Rgb,
}

const CH: [Challenge; CHALLENGES] = [
    Challenge { game: Kind::Maze, name: "maze", level: [1, 2, 3, 4, 5], colour: Rgb(60, 120, 255) },
    Challenge { game: Kind::Simon, name: "simon", level: [4, 6, 8, 10, 12], colour: Rgb(255, 200, 0) },
    Challenge { game: Kind::Whack, name: "whack-a-mole", level: [20, 40, 60, 80, 100], colour: Rgb(255, 40, 0) },
    Challenge { game: Kind::Hilo, name: "higher lower", level: [3, 5, 7, 9, 12], colour: Rgb(170, 40, 255) },
    Challenge { game: Kind::Snake, name: "snake", level: [5, 10, 15, 20, 30], colour: Rgb(0, 255, 60) },
];

#[derive(Default)]
pub struct Challenges {
    level: [u8; CHALLENGES],
    changed: bool,
    victory_due: bool,
}

impl Challenges {
    /// Progress as kept in flash: 3 bits a challenge.
    pub fn load(&mut self, packed: u16) {
        for (c, l) in self.level.iter_mut().enumerate() {
            *l = ((packed >> (3 * c)) & 7).min(LEVELS as u16) as u8;
        }
    }

    pub fn packed(&self) -> u16 {
        self.level.iter().enumerate().map(|(c, &l)| (l as u16) << (3 * c)).sum()
    }

    /// The progress changed since the last call (for main.rs to save it).
    pub fn take_changed(&mut self) -> bool {
        core::mem::take(&mut self.changed)
    }

    pub fn level(&self, c: usize) -> u8 {
        self.level[c]
    }

    pub fn name(c: usize) -> &'static str {
        CH[c].name
    }

    /// 0..=25
    pub fn total(&self) -> u8 {
        self.level.iter().sum()
    }

    pub fn complete(&self) -> bool {
        self.total() == CHALLENGES as u8 * LEVELS
    }

    /// A real game of `kind` ended with `score`: the challenge (= arm) that
    /// went up a level.
    pub fn result(&mut self, kind: Kind, score: u16) -> Option<usize> {
        let c = CH.iter().position(|ch| ch.game == kind)?;
        let l = CH[c].level.iter().take_while(|&&need| score >= need).count() as u8;
        if l <= self.level[c] {
            return None;
        }
        self.level[c] = l;
        self.changed = true;
        if self.complete() {
            self.victory_due = true;
        }
        Some(c)
    }

    /// All 25 were just reached: the victory dance is due (once).
    pub fn victory_due(&self) -> bool {
        self.victory_due
    }

    pub fn victory_done(&mut self) {
        self.victory_due = false;
    }

    /// The progress on the arms; `fresh` blinks that arm's newest level.
    pub fn arms(&self, f: &mut Frame, t: u32, fresh: Option<usize>) {
        for (c, ch) in CH.iter().enumerate() {
            let l = self.level[c] as usize;
            for k in 0..ARM_LINE {
                let v = if k >= l {
                    14
                } else if fresh == Some(c) && k == l - 1 && t % 12 < 6 {
                    40
                } else {
                    255
                };
                arm_line(f, c as u8, k, ch.colour.scaled(v));
            }
            // the two beside the line: bright, and shimmering, when it is complete
            let v = if l == LEVELS as usize {
                (150 + sin8((t * 6 + c as u32 * 51) as u8) as u32 * 105 / 255) as u8
            } else {
                30
            };
            arm_sides(f, c as u8, ch.colour.scaled(v));
        }
    }
}

// ---- the victory dance ----

const TEXT: &str = "I'VE COMPLETED ALL THE CHALLENGES AND ALL I'VE GOT IS THIS LOUSY LED ANIMATION!";
const SPEED: u32 = 20; // columns a second
const RISE: u8 = 10; // frames up the arm
const BURST: u8 = 26; // frames of the burst

#[derive(Clone, Copy, Default)]
struct Rocket {
    arm: u8,
    hue: u8,
    t: u8, // 0: free
}

/// The text, dancing, with fireworks on the arms and the outline.
#[derive(Default)]
pub struct Victory {
    t: u32,
    rocket: [Rocket; 3],
}

fn tip_angle(arm: u8) -> u8 {
    RGB.iter().find(|l| l.arm == arm && l.rank == 6).map_or(0, |l| polar(l.r).1)
}

impl Victory {
    fn fireworks(&mut self, f: &mut Frame, rng: &mut Rng) {
        for &s in EDGE.iter() {
            // a faint twinkle everywhere
            f.set(s, if rng.byte() < 12 { 90 } else { 6 });
        }
        for r in self.rocket.iter_mut() {
            if r.t == 0 {
                if rng.byte() < 24 {
                    // a new one
                    r.arm = (rng.next() % 5) as u8;
                    r.hue = rng.byte();
                    r.t = 1;
                }
                continue;
            }
            if r.t <= RISE {
                // a spark climbs the arm, with a short trail
                let at = (r.t as usize - 1) * ARM_LINE / RISE as usize;
                arm_line(f, r.arm, at, Rgb(255, 230, 180));
                if at > 0 {
                    arm_line(f, r.arm, at - 1, Rgb(80, 50, 20));
                }
            } else {
                // and bursts: the arm and the outline by its tip, fading
                let fade = (255 - (r.t - RISE) as u32 * 255 / BURST as u32) as u8;
                let c = hsv(r.hue, 230, fade);
                for (i, l) in RGB.iter().enumerate() {
                    if l.arm == r.arm && rng.byte() < 200 {
                        f.rgb(i, c);
                    }
                }
                let tip = tip_angle(r.arm);
                for &s in EDGE.iter() {
                    let d = polar(s).1.wrapping_sub(tip) as i8;
                    if d > -14 && d < 14 && rng.byte() < 150 {
                        f.set(s, fade);
                    }
                }
            }
            r.t += 1;
            if r.t > RISE + BURST {
                r.t = 0;
            }
        }
    }

    /// One frame; false at the end of the text.
    pub fn render(&mut self, f: &mut Frame, rng: &mut Rng) -> bool {
        let t = self.t;
        self.t += 1;
        let x0 = GRID_W as i32 - (t * SPEED / FPS) as i32;
        // the letters dance: each bobs up and down, a little after the one before
        for (i, ch) in TEXT.bytes().enumerate() {
            let x = x0 + i as i32 * (FONT_W as i32 + 1);
            if x <= -(FONT_W as i32) || x >= GRID_W as i32 || ch == b' ' {
                continue;
            }
            let bob = (sin8((t * 12).wrapping_sub(i as u32 * 40) as u8) as i32 - 128) * 2 / 128;
            draw_text(f, core::str::from_utf8(&[ch]).unwrap_or(""), x, (4 + bob) as usize, 255);
        }
        self.fireworks(f, rng);
        x0 + TEXT.len() as i32 * (FONT_W as i32 + 1) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_count_up_only_and_25_is_victory() {
        let mut ch = Challenges::default();
        assert_eq!(ch.result(Kind::Snake, 12), Some(4));
        assert_eq!(ch.level(4), 2);
        assert_eq!(ch.result(Kind::Snake, 6), None, "a worse score never lowers it");
        assert_eq!(ch.result(Kind::Pong, 99), None, "not a challenge");
        for (kind, score) in [(Kind::Maze, 5), (Kind::Simon, 12), (Kind::Whack, 100), (Kind::Hilo, 12)] {
            ch.result(kind, score);
            assert!(!ch.victory_due());
        }
        ch.result(Kind::Snake, 30);
        assert!(ch.complete() && ch.victory_due());
        let mut back = Challenges::default();
        back.load(ch.packed());
        assert_eq!(back.total(), 25);
        assert!(ch.take_changed() && !ch.take_changed());
        // the dance ends after the text has gone by
        let (mut v, mut f, mut rng) = (Victory::default(), Frame::new(), Rng::new(1));
        let frames = (0..5000).take_while(|_| v.render(&mut f, &mut rng)).count();
        assert!(frames > 300 && frames < 1000, "{frames} frames");
    }
}
