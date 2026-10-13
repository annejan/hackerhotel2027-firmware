//! Shared game shell. Every game starts in demo mode (it plays itself);
//! pressing ◀, B or ▶ starts a real game, and game over shows the score.
//! A stays free in demo mode, so holding it still opens the menu.
//!
//! A game implements `Play`: one `tick` per frame (returns false at game
//! over), `draw`, and its `score`. Input arrives as bitmasks of the buttons:
//! `pressed` since the last frame and `held` right now (bit n = button n: ◀ B A ▶).

use crate::anim::FPS;
use crate::frame::{draw_number, hsv, Frame, Rgb, Rng};
use crate::challenge::Challenges;
use crate::games::{Breakout, Chase, Flappy, Hilo, Maze, Pong, Simon, Snake, Startris, Whack};
use crate::generated::{EDGE, RGB};
use crate::Event;

pub const BTN_LEFT: u8 = 1 << 0;
pub const BTN_B: u8 = 1 << 1;
pub const BTN_A: u8 = 1 << 2;
pub const BTN_RIGHT: u8 = 1 << 3;

const CRASH_FRAMES: u8 = 20;
const SCORE_FRAMES: u8 = (2 * FPS) as u8;

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub pressed: u8,
    pub held: u8,
}

pub trait Play {
    fn new(rng: &mut Rng) -> Self;
    /// One frame. In demo mode the game drives itself and `inp` is empty.
    /// Returns false at game over.
    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool;
    fn draw(&self, f: &mut Frame, t: u32);
    fn score(&self) -> u16;
    /// The player won: skip the red crash and go straight to the score.
    fn won(&self) -> bool {
        false
    }
    /// The game draws on the outline itself (no demo-mode chase there).
    const USES_EDGE: bool = false;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Snake,
    Startris,
    Breakout,
    Pong,
    Flappy,
    Chase,
    // the challenges' own games (challenge.rs)
    Whack,
    Simon,
    Maze,
    Hilo,
}

/// The running game; only one exists at a time, so they share this slot.
enum Body {
    Snake(Snake),
    Startris(Startris),
    Breakout(Breakout),
    Pong(Pong),
    Flappy(Flappy),
    Chase(Chase),
    Whack(Whack),
    Simon(Simon),
    Maze(Maze),
    Hilo(Hilo),
}

macro_rules! each {
    ($body:expr, $g:ident => $e:expr) => {
        match $body {
            Body::Snake($g) => $e,
            Body::Startris($g) => $e,
            Body::Breakout($g) => $e,
            Body::Pong($g) => $e,
            Body::Flappy($g) => $e,
            Body::Chase($g) => $e,
            Body::Whack($g) => $e,
            Body::Simon($g) => $e,
            Body::Maze($g) => $e,
            Body::Hilo($g) => $e,
        }
    };
}

fn uses_edge<G: Play>(_: &G) -> bool {
    G::USES_EDGE
}

impl Body {
    fn new(kind: Kind, rng: &mut Rng) -> Self {
        match kind {
            Kind::Snake => Body::Snake(Snake::new(rng)),
            Kind::Startris => Body::Startris(Startris::new(rng)),
            Kind::Breakout => Body::Breakout(Breakout::new(rng)),
            Kind::Pong => Body::Pong(Pong::new(rng)),
            Kind::Flappy => Body::Flappy(Flappy::new(rng)),
            Kind::Chase => Body::Chase(Chase::new(rng)),
            Kind::Whack => Body::Whack(Whack::new(rng)),
            Kind::Simon => Body::Simon(Simon::new(rng)),
            Kind::Maze => Body::Maze(Maze::new(rng)),
            Kind::Hilo => Body::Hilo(Hilo::new(rng)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Run,
    Crash(u8),
    Score(u8),
}

pub struct Game {
    kind: Kind,
    body: Body,
    state: State,
    demo: bool,
    pressed: u8,
    t: u32,
    fresh: Option<usize>, // the challenge this game just moved up a level
}

impl Game {
    pub fn new(kind: Kind, rng: &mut Rng) -> Self {
        Self { kind, body: Body::new(kind, rng), state: State::Run, demo: true, pressed: 0, t: 0, fresh: None }
    }

    fn restart(&mut self, demo: bool, rng: &mut Rng) {
        *self = Self::new(self.kind, rng);
        self.demo = demo;
    }

    /// A real game is going on (not the demo).
    pub fn playing(&self) -> bool {
        !self.demo
    }

    /// Returns true when the event was used by the game.
    pub fn handle(&mut self, ev: Event, rng: &mut Rng) -> bool {
        match ev {
            Event::Press(p) if self.demo => {
                if p == 2 {
                    return false; // A: tap = brightness, hold = menu
                }
                self.restart(false, rng);
                true
            }
            Event::Press(p) => {
                self.pressed |= 1 << p;
                true
            }
            Event::Hold(2) => false, // menu, even from a game
            Event::Tap(_) | Event::Hold(_) => !self.demo,
            Event::Charging(_) | Event::Bootloader | Event::Selftest => false,
        }
    }

    /// `ch` hears about every real game's score.
    pub fn render(&mut self, f: &mut Frame, held: u8, rng: &mut Rng, ch: &mut Challenges) {
        self.t = self.t.wrapping_add(1);
        let inp = if self.demo {
            Input::default()
        } else {
            Input { pressed: core::mem::take(&mut self.pressed), held }
        };
        match self.state {
            State::Run => {
                let demo = self.demo;
                if !each!(&mut self.body, g => g.tick(demo, inp, rng)) {
                    let won = each!(&self.body, g => g.won());
                    if !demo {
                        self.fresh = ch.result(self.kind, each!(&self.body, g => g.score()));
                    }
                    self.state = if demo {
                        State::Score(SCORE_FRAMES / 2)
                    } else if won {
                        State::Score(SCORE_FRAMES)
                    } else {
                        State::Crash(CRASH_FRAMES)
                    };
                }
            }
            State::Crash(0) => self.state = State::Score(SCORE_FRAMES),
            State::Crash(n) => self.state = State::Crash(n - 1),
            State::Score(0) => return self.restart(true, rng),
            State::Score(n) => self.state = State::Score(n - 1),
        }

        let t = self.t;
        match self.state {
            State::Score(n) => {
                let score = each!(&self.body, g => g.score());
                draw_number(f, score, 5, if n % 10 < 7 { 255 } else { 70 });
                if self.fresh.is_some() {
                    // a challenge went up a level: its new LED blinks
                    ch.arms(f, t, self.fresh);
                } else {
                    for (i, l) in RGB.iter().enumerate() {
                        let hue = (l.arm * 51).wrapping_add(l.rank * 18).wrapping_sub((t * 6) as u8);
                        f.rgb(i, hsv(hue, 255, 180));
                    }
                }
            }
            State::Crash(n) => {
                if n % 6 < 3 {
                    each!(&self.body, g => g.draw(f, t));
                }
                for i in 0..RGB.len() {
                    f.rgb(i, Rgb(255, 10, 0).scaled(n * (255 / CRASH_FRAMES)));
                }
            }
            State::Run => {
                // demo mode: a slow dim chase round the outline says "press me"
                if self.demo && !each!(&self.body, g => uses_edge(g)) {
                    for (j, &s) in EDGE.iter().enumerate() {
                        if (j as u32 + t / 3) % 12 == 0 {
                            f.set(s, 60);
                        }
                    }
                }
                each!(&self.body, g => g.draw(f, t));
            }
        }
    }
}

/// Light arm `arm`'s first `n` LEDs (centre outward) in `c`.
pub fn arm_fill(f: &mut Frame, arm: u8, n: i32, c: Rgb) {
    for (i, l) in RGB.iter().enumerate() {
        if l.arm == arm && (l.rank as i32) < n {
            f.rgb(i, c);
        }
    }
}

/// The five LEDs of an arm that stand in a line, centre outward (ranks 1
/// and 2 sit beside rank 0, at the arm's root).
pub const ARM_LINE: usize = 5;
const ARM_LINE_RANK: [u8; ARM_LINE] = [0, 3, 4, 5, 6];
/// The arm a button sits under: ◀ 3, B 4, A 1, ▶ 2 (arm 0, the top, has none).
pub const BUTTON_ARM: [u8; 4] = [3, 4, 1, 2];

/// Light LED `k` (0 = centre) of arm `arm`'s line in `c`.
pub fn arm_line(f: &mut Frame, arm: u8, k: usize, c: Rgb) {
    for (i, l) in RGB.iter().enumerate() {
        if l.arm == arm && l.rank == ARM_LINE_RANK[k] {
            f.rgb(i, c);
        }
    }
}

/// Light the two LEDs beside the line, at the arm's root.
pub fn arm_sides(f: &mut Frame, arm: u8, c: Rgb) {
    for (i, l) in RGB.iter().enumerate() {
        if l.arm == arm && (l.rank == 1 || l.rank == 2) {
            f.rgb(i, c);
        }
    }
}

/// A score of up to 35 as 7 LEDs per arm, filling arm 0 first.
pub fn arms_score(f: &mut Frame, score: u16, c: Rgb) {
    for a in 0..5u8 {
        arm_fill(f, a, (score as i32 - a as i32 * 7).clamp(0, 7), c);
    }
}

/// Step towards `target` by one, at most once every `every` frames.
pub fn approach(pos: &mut i32, target: i32, t: u32, every: u32) -> i32 {
    if t % every != 0 || *pos == target {
        return 0;
    }
    let d = if target > *pos { 1 } else { -1 };
    *pos += d;
    d
}
