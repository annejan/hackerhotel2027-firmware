//! The challenges; same as `c/src/challenge.c`: five games, one per arm,
//! five levels each, a level for every LED in the arm's line. All 25 and
//! the badge does its victory dance: a dancing stick figure.
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
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};

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
//
// A stick figure dances on the grid, a new pose on every beat, dipping a
// row at the start of each; the arms flash disco colours on the beat and
// the outline chases. Between dances the text goes by a phrase at a time,
// with fireworks on the arms and the outline.

const PHRASES: [&str; 5] = ["I'VE COMPLETED", "ALL THE CHALLENGES", "AND ALL I'VE GOT", "IS THIS LOUSY", "LED ANIMATION!"];
const SPEED: u32 = 24; // text columns a second
const BEAT: u32 = 12; // frames a beat: 150 bpm
const DANCE_BEATS: u32 = 8;
const RISE: u8 = 10; // frames up the arm
const BURST: u8 = 26; // frames of the burst

/// A pose: (row, column) of the neck, hip, left elbow and hand, right
/// elbow and hand, left knee and foot, right knee and foot ("left" as you
/// look at it). The head is a ring two rows above the neck.
type Pose = [(i8, i8); 10];

macro_rules! stand {
    ($n:expr, $h:expr, $el:expr, $hl:expr, $er:expr, $hr:expr) => {
        [$n, $h, $el, $hl, $er, $hr, (11, 7), (14, 6), (11, 9), (14, 10)]
    };
}

const POSES: [Pose; 12] = [
    stand!((4, 8), (9, 8), (3, 6), (1, 4), (3, 10), (1, 12)), // 0 arms up (and Y)
    stand!((4, 8), (9, 8), (6, 6), (9, 5), (6, 10), (9, 11)), // 1 arms down
    // 2 point up right, left hand on the hip
    [(4, 8), (9, 8), (6, 5), (8, 7), (3, 11), (1, 13), (11, 7), (14, 5), (11, 9), (14, 10)],
    // 3 point up left, right hand on the hip
    [(4, 8), (9, 8), (3, 5), (1, 3), (6, 11), (8, 9), (11, 7), (14, 6), (11, 9), (14, 11)],
    stand!((4, 8), (9, 8), (5, 5), (4, 2), (5, 11), (4, 14)), // 4 arms out
    // 5 squat, arms forward
    [(5, 8), (10, 8), (6, 5), (5, 3), (6, 11), (5, 13), (11, 5), (14, 5), (11, 11), (14, 11)],
    // 6 jump: arms up, knees tucked
    [(3, 8), (8, 8), (2, 6), (0, 7), (2, 10), (0, 9), (10, 6), (12, 7), (10, 10), (12, 9)],
    stand!((4, 8), (9, 8), (3, 5), (1, 6), (3, 11), (1, 10)), // 7 M: hands on the head
    stand!((4, 8), (9, 8), (3, 6), (2, 4), (6, 7), (7, 4)),   // 8 C: both arms curved to the left
    stand!((4, 8), (9, 8), (2, 6), (0, 7), (2, 10), (0, 9)),  // 9 A: hands meet above the head
    // 10 kick right
    [(4, 8), (9, 8), (5, 5), (3, 3), (6, 10), (8, 11), (11, 8), (14, 7), (10, 10), (9, 13)],
    // 11 kick left
    [(4, 8), (9, 8), (6, 6), (8, 5), (5, 11), (3, 13), (10, 6), (9, 3), (11, 8), (14, 9)],
];
const JUMP: u8 = 6;

/// The moves, a pose a beat, round and round.
const STEPS: [u8; 24] = [0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 6, 0, 7, 8, 9, 10, 11, 10, 11, 5, 6, 4, 1];

#[derive(Clone, Copy, Default)]
struct Rocket {
    arm: u8,
    hue: u8,
    t: u8, // 0: free
}

/// The victory dance.
#[derive(Default)]
pub struct Victory {
    t: u32,    // frames into this part
    part: u32, // even: a dance, odd: a phrase; the last dance ends it
    beat: u32, // beats danced, over all the dances
    rocket: [Rocket; 3],
}

fn line(f: &mut Frame, (mut r0, mut c0): (i32, i32), (r1, c1): (i32, i32)) {
    let (dr, dc) = ((r1 - r0).abs(), (c1 - c0).abs());
    let (sr, sc) = (if r0 < r1 { 1 } else { -1 }, if c0 < c1 { 1 } else { -1 });
    let mut err = dc - dr;
    loop {
        grid(f, r0, c0, 255);
        if r0 == r1 && c0 == c1 {
            return;
        }
        let e2 = 2 * err;
        if e2 > -dr {
            err -= dr;
            c0 += sc;
        }
        if e2 < dc {
            err += dc;
            r0 += sr;
        }
    }
}

fn grid(f: &mut Frame, r: i32, c: i32, v: u8) {
    if (0..GRID_H as i32).contains(&r) && (0..GRID_W as i32).contains(&c) {
        f.grid(r as usize, c as usize, v);
    }
}

fn dancer(f: &mut Frame, p: &Pose, dy: i32) {
    let at = |k: usize| (p[k].0 as i32 + dy, p[k].1 as i32);
    line(f, at(0), at(1)); // body
    line(f, at(0), at(2)); // arms
    line(f, at(2), at(3));
    line(f, at(0), at(4));
    line(f, at(4), at(5));
    line(f, at(1), at(6)); // legs
    line(f, at(6), at(7));
    line(f, at(1), at(8));
    line(f, at(8), at(9));
    let (hr, hc) = (at(0).0 - 2, at(0).1); // the head: a ring
    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr != 0 || dc != 0 {
                grid(f, hr + dr, hc + dc, 255);
            }
        }
    }
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

    fn dance(&self, f: &mut Frame, t: u32) {
        let (in_beat, beat) = (t % BEAT, self.beat + t / BEAT);
        let pose = STEPS[beat as usize % STEPS.len()];
        // dip at the start of a beat; the jump goes up instead
        let dy = if pose == JUMP {
            if in_beat < BEAT / 2 { -1 } else { 0 }
        } else {
            (in_beat < 3) as i32
        };
        dancer(f, &POSES[pose as usize], dy);
        // disco: the arms flash a new colour every beat, the outline chases
        let flash = (255 - in_beat * 160 / BEAT) as u8;
        for (i, l) in RGB.iter().enumerate() {
            f.rgb(i, hsv((beat * 47 + l.arm as u32 * 20) as u8, 255, flash));
        }
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (j as u32 + beat) % 2 != 0 { 140 } else { 8 });
        }
    }

    /// One frame; false when it is over.
    pub fn render(&mut self, f: &mut Frame, rng: &mut Rng) -> bool {
        let t = self.t;
        self.t += 1;
        if self.part % 2 == 0 {
            // dancing
            self.dance(f, t);
            if t + 1 >= DANCE_BEATS * BEAT {
                self.beat += DANCE_BEATS;
                self.part += 1;
                self.t = 0;
            }
            return self.part <= 2 * PHRASES.len() as u32;
        }
        // a phrase goes by; its letters bob, a little after each other
        let s = PHRASES[self.part as usize / 2];
        let x0 = GRID_W as i32 - (t * SPEED / FPS) as i32;
        for (i, ch) in s.bytes().enumerate() {
            let x = x0 + i as i32 * (FONT_W as i32 + 1);
            if x <= -(FONT_W as i32) || x >= GRID_W as i32 || ch == b' ' {
                continue;
            }
            let bob = (sin8((t * 12).wrapping_sub(i as u32 * 40) as u8) as i32 - 128) * 2 / 128;
            draw_text(f, core::str::from_utf8(&[ch]).unwrap_or(""), x, (4 + bob) as usize, 255);
        }
        self.fireworks(f, rng);
        if x0 + s.len() as i32 * (FONT_W as i32 + 1) <= 0 {
            self.part += 1;
            self.t = 0;
        }
        true
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
        // six dances of 8 beats around the five phrases, then it ends
        let (mut v, mut f, mut rng) = (Victory::default(), Frame::new(), Rng::new(1));
        let frames = (0..5000).take_while(|_| v.render(&mut f, &mut rng)).count();
        assert!(frames > 6 * 8 * 12 && frames < 2000, "{frames} frames");
    }
}
