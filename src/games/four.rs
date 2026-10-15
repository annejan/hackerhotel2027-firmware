//! Four in a row; same as `c/src/games/four.c`: drop discs into seven
//! columns, six high; four of yours in a line (across, up or diagonal)
//! wins. ◀ / ▶ aim, B drops.
//!
//! Against another badge over infrared, or against this one. A new game
//! looks for another badge first: point the two badges at each other with
//! 4 IN A ROW open on both, and they pair up. Dropping a disc before that
//! plays against the badge instead.
//!
//! Your discs are bright, the other side's dim. The arms are green on your
//! turn and orange on theirs, and sweep blue while looking for a badge.
//!
//! Infrared: packet `irproto::FOUR`, data [game, move, column, flags].
//! `game` is a random number that keeps two pairs of players apart. A
//! badge looking for a partner sends move 0 with `FL_ANSWER` every second
//! or so; a badge with the game open answers and joins, and the one that
//! asked moves first (when both ask, the higher number asks). The one that
//! joined sets `FL_SECOND` in everything it sends. After that each move
//! goes out as soon as it is made, again every 0.8 s until the other badge
//! answers. An answer is our own last move, so whichever packet arrives,
//! both badges end up with the same board. While it is the other badge's
//! turn, ours asks every 3 s whether it is still there; 20 s without an
//! answer ends the game. Every game is logged on the UART with its moves.

use crate::anim::FPS;
use crate::frame::{sin8, Frame, Rgb, Rng};
use crate::game::{Input, Play, BTN_B, BTN_LEFT, BTN_RIGHT};
use crate::generated::{GRID_W, RGB};
use crate::irproto::{self, Packet};
use crate::log;

const W: usize = 7;
const H: usize = 6;
const X0: usize = 2; // column c is grid column X0 + 2c
const Y0: usize = 5; // row r (0 = top) is grid row Y0 + r
const CURSOR_Y: usize = 3;
const FL_ANSWER: u8 = 1; // please answer: we have not heard you got this
const FL_SECOND: u8 = 2; // the sender moves second
const WIN_BIT: u8 = 4; // cell is part of the winning line
const DEMO_EVERY: u16 = 24; // frames per demo move
const BADGE_THINK: u16 = 15; // frames before the badge answers your move
const RESEND: u16 = (FPS * 4 / 5) as u16; // resend an unanswered move this often, plus jitter
const PING: u16 = (3 * FPS) as u16; // their turn: ask whether they are still there
const LOST: u16 = (20 * FPS) as u16; // no answer for this long: the link is gone
const END_FRAMES: u8 = (3 * FPS) as u8; // show the result this long
const DIRS: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Seek,
    Local,
    Link,
}

pub struct Four {
    cell: [[u8; W]; H], // 0, or the player (1 moves first, 2) | WIN_BIT
    moves: [u8; W * H], // the columns, in order
    n: u8,
    last: (usize, usize), // the last disc
    cursor: u8,
    me: u8, // we are player me + 1
    mode: Mode,
    game: u8,      // the infrared game's number
    heard: u8,     // the other badge has our moves up to this one
    answer_in: u8, // frames until we answer the other badge, 0 = no
    winner: u8,    // 0, the player, or 3 for a draw
    end: u8,       // frames of the result shown
    resend_in: u16,
    quiet: u16, // frames without an answer
    since: u16, // frames since the last move
    demo: bool,
    out: Option<Packet>, // for the IR driver
}

impl Four {
    /// The row a disc dropped in column `c` lands in.
    fn row_free(&self, c: usize) -> Option<usize> {
        (0..H).rev().find(|&r| self.cell[r][c] == 0)
    }

    fn owner(&self, r: i32, c: i32) -> u8 {
        if (0..H as i32).contains(&r) && (0..W as i32).contains(&c) {
            self.cell[r as usize][c as usize] & 3
        } else {
            0
        }
    }

    /// Discs of player `p` through (r, c) in direction `d`, (r, c) counted
    /// as p's; `mark` = also mark them as the winning line.
    fn line(&mut self, r: usize, c: usize, (dr, dc): (i32, i32), p: u8, mark: bool) -> usize {
        let mut n = 1;
        for s in [-1, 1] {
            let mut k = 1;
            while self.owner(r as i32 + s * k * dr, c as i32 + s * k * dc) == p {
                n += 1;
                if mark {
                    self.cell[(r as i32 + s * k * dr) as usize][(c as i32 + s * k * dc) as usize] |= WIN_BIT;
                }
                k += 1;
            }
        }
        n
    }

    fn wins(&mut self, r: usize, c: usize, p: u8) -> bool {
        DIRS.iter().any(|&d| self.line(r, c, d, p, false) >= 4)
    }

    fn wins_in(&mut self, c: usize, p: u8) -> bool {
        self.row_free(c).is_some_and(|r| self.wins(r, c, p))
    }

    /// The badge's move for player `p`: win, block, keep away from the
    /// edges, and do not set up the other side's win on top.
    fn ai(&mut self, p: u8, rng: &mut Rng) -> usize {
        for q in [p, 3 - p] {
            if let Some(c) = (0..W).find(|&c| self.wins_in(c, q)) {
                return c;
            }
        }
        let (mut best, mut best_v) = (0, i32::MIN);
        for c in 0..W {
            let Some(r) = self.row_free(c) else { continue };
            let mut v = (3 - (c as i32 - 3).abs()) * 4 + (rng.next() % 4) as i32;
            self.cell[r][c] = p;
            if r > 0 && self.wins(r - 1, c, 3 - p) {
                v -= 50;
            }
            self.cell[r][c] = 0;
            if v > best_v {
                (best, best_v) = (c, v);
            }
        }
        best
    }

    fn result(&self) {
        if self.demo {
            return;
        }
        log::text(if self.winner == 3 {
            "four: draw"
        } else if self.winner == self.me + 1 {
            "four: you win"
        } else {
            "four: you lose"
        });
        log::text(", moves ");
        let mut m = [0u8; W * H];
        for (d, &c) in m.iter_mut().zip(&self.moves[..self.n as usize]) {
            *d = b'1' + c;
        }
        log::line(log::ascii(&m[..self.n as usize]));
    }

    /// The next move, in column `c` (it fits).
    fn drop(&mut self, c: usize) {
        let p = self.n % 2 + 1;
        let Some(r) = self.row_free(c) else { return };
        self.cell[r][c] = p;
        self.moves[self.n as usize] = c as u8;
        self.n += 1;
        self.last = (r, c);
        self.since = 0;
        let mut won = false;
        for d in DIRS {
            if self.line(r, c, d, p, false) >= 4 {
                self.line(r, c, d, p, true);
                won = true;
            }
        }
        if won {
            self.cell[r][c] |= WIN_BIT;
            self.winner = p;
        } else if self.n as usize == W * H {
            self.winner = 3;
        }
        if self.winner != 0 {
            self.result();
        }
    }

    fn my_turn(&self) -> bool {
        self.n % 2 == self.me
    }

    fn send(&mut self, answer: bool) {
        if self.out.is_some() {
            self.answer_in = 1; // still sending: try again next frame
            return;
        }
        let last = if self.n > 0 { self.moves[self.n as usize - 1] } else { 0 };
        let flags = if answer { FL_ANSWER } else { 0 } | if self.me != 0 { FL_SECOND } else { 0 };
        self.out = Some(Packet::new(irproto::FOUR, &[self.game, self.n, last, flags]));
    }

    /// A packet for the IR driver.
    pub fn ir_take(&mut self) -> Option<Packet> {
        self.out.take()
    }

    /// An infrared packet; `live` = a real game is running. Returns true
    /// for an invitation that should start one.
    pub fn ir(&mut self, live: bool, d: &[u8], rng: &mut Rng) -> bool {
        let &[game, n, c, flags] = d else { return false };
        let asks = flags & FL_ANSWER != 0;
        let second = flags & FL_SECOND != 0;
        let invite = n == 0 && asks && !second;
        if !live {
            return invite; // the shell starts a game
        }
        self.demo = false;
        if self.mode == Mode::Seek {
            if invite && game > self.game {
                // join theirs; they move first
                self.game = game;
                self.me = 1;
            } else if n == 0 && game == self.game && second {
                self.me = 0; // they joined ours
            } else {
                if invite && game == self.game {
                    // the same number: pick another
                    self.game = (1 + rng.next() % 255) as u8;
                }
                return false;
            }
            self.mode = Mode::Link;
            self.resend_in = PING;
            log::line(if self.me != 0 { "four: linked, they start" } else { "four: linked, you start" });
        } else if self.mode != Mode::Link || game != self.game || second == (self.me != 0) {
            return false; // not our game, or not the other player
        }
        self.quiet = 0;
        let c = c as usize;
        if n == self.n + 1 && self.winner == 0 && !self.my_turn() && c < W && self.row_free(c).is_some() {
            self.drop(c);
        }
        if n <= self.n && n > self.heard {
            self.heard = n;
        }
        if asks && self.answer_in == 0 {
            self.answer_in = (3 + rng.next() % 6) as u8; // let their receiver settle
        }
        false
    }
}

impl Play for Four {
    fn new(rng: &mut Rng) -> Self {
        Self {
            cell: [[0; W]; H],
            moves: [0; W * H],
            n: 0,
            last: (0, 0),
            cursor: 3,
            me: 0,
            mode: Mode::Seek,
            game: (1 + rng.next() % 255) as u8,
            heard: 0,
            answer_in: 0,
            winner: 0,
            end: 0,
            resend_in: 1,
            quiet: 0,
            since: 0,
            demo: true,
            out: None,
        }
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.demo = demo;
        self.since = self.since.saturating_add(1);
        if self.answer_in > 0 {
            self.answer_in -= 1;
            if self.answer_in == 0 {
                self.send(self.n > self.heard);
            }
        }
        if !demo && self.mode == Mode::Seek {
            self.resend_in -= 1;
            if self.resend_in == 0 {
                self.resend_in = (FPS + rng.next() % 20) as u16;
                self.send(true);
            }
        }
        // they have not got our last move, or it is their turn: keep asking
        if self.mode == Mode::Link && (self.n > self.heard || (!self.my_turn() && self.winner == 0)) {
            self.quiet += 1;
            if self.quiet > LOST {
                log::line("four: link lost");
                return false;
            }
            self.resend_in = self.resend_in.saturating_sub(1);
            if self.resend_in == 0 {
                self.resend_in = if self.n > self.heard { RESEND } else { PING } + (rng.next() % 12) as u16;
                self.send(true);
            }
        }
        if self.winner != 0 {
            self.end += 1;
            return self.end < END_FRAMES;
        }

        if demo {
            if self.since >= DEMO_EVERY {
                let c = self.ai(self.n % 2 + 1, rng);
                self.cursor = c as u8;
                self.drop(c);
            }
            return true;
        }
        if inp.pressed & BTN_LEFT != 0 && self.cursor > 0 {
            self.cursor -= 1;
        }
        if inp.pressed & BTN_RIGHT != 0 && (self.cursor as usize) < W - 1 {
            self.cursor += 1;
        }
        let cursor = self.cursor as usize;
        if self.mode == Mode::Local && self.n % 2 == 1 {
            // the badge's turn
            if self.since >= BADGE_THINK {
                let c = self.ai(2, rng);
                self.drop(c);
            }
        } else if inp.pressed & BTN_B != 0 && self.my_turn() && self.row_free(cursor).is_some() {
            if self.mode == Mode::Seek {
                self.mode = Mode::Local;
            }
            self.drop(cursor);
            if self.mode == Mode::Link {
                self.quiet = 0;
                self.resend_in = 1;
            }
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        for x in 1..GRID_W - 1 {
            f.grid(Y0 + H, x, 20); // the floor
        }
        let mine = if self.demo { 1 } else { self.me + 1 };
        for r in 0..H {
            for c in 0..W {
                let v = self.cell[r][c];
                if v == 0 {
                    continue;
                }
                let mut b = if v & 3 == mine { 255 } else { 60 };
                if v & WIN_BIT != 0 && t % 10 < 5 {
                    b = 0;
                } else if self.n > 0 && (r, c) == self.last && self.since < FPS as u16 && self.since % 8 < 4 {
                    b = 0; // the last disc blinks a little
                }
                f.grid(Y0 + r, X0 + 2 * c, b);
            }
        }
        if self.winner == 0 && !self.demo && self.my_turn() {
            f.grid(CURSOR_Y, X0 + 2 * self.cursor as usize, if t % 16 < 10 { 255 } else { 90 });
        }

        let green = Rgb(0, 255, 60);
        let orange = Rgb(255, 90, 0);
        let c = if self.winner != 0 {
            if self.winner == 3 {
                Rgb(200, 200, 200)
            } else if self.demo || self.winner == self.me + 1 {
                green
            } else {
                Rgb(255, 20, 0)
            }
        } else if self.mode == Mode::Seek && !self.demo {
            // looking for a badge: a blue wave runs out
            for (i, l) in RGB.iter().enumerate() {
                if (t / 3) % 9 == l.rank as u32 {
                    f.rgb(i, Rgb(0, 80, 255));
                }
            }
            return;
        } else if self.demo {
            if self.n % 2 == 1 { orange } else { green }
        } else if self.my_turn() {
            green
        } else {
            orange
        };
        let pulse = 120 + sin8((t * 4) as u8) / 2;
        for i in 0..RGB.len() {
            f.rgb(i, c.scaled(pulse));
        }
    }

    fn score(&self) -> u16 {
        // discs left in hand when you won
        if self.won() { (W * H / 2) as u16 - (self.n + 1 - self.me) as u16 / 2 } else { 0 }
    }

    fn won(&self) -> bool {
        self.winner != 0 && self.winner == self.me + 1 && self.mode != Mode::Seek
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_across_up_and_diagonal_win() {
        let mut rng = Rng::new(1);
        for moves in [&[0, 0, 1, 1, 2, 2, 3][..], &[0, 1, 0, 1, 0, 1, 0], &[0, 1, 1, 2, 2, 3, 2, 3, 3, 6, 3]] {
            let mut g = Four::new(&mut rng);
            for &c in moves {
                assert_eq!(g.winner, 0);
                g.drop(c);
            }
            assert_eq!(g.winner, 1, "{moves:?}");
            assert_eq!(g.cell.iter().flatten().filter(|&&v| v & WIN_BIT != 0).count(), 4);
        }
    }

    #[test]
    fn the_badge_takes_a_win_and_blocks_one() {
        let mut rng = Rng::new(1);
        let mut g = Four::new(&mut rng);
        for c in [0, 6, 1, 6, 2] {
            g.drop(c);
        }
        assert_eq!(g.ai(2, &mut rng), 3, "blocks 0 1 2");
        g.drop(5);
        assert_eq!(g.ai(1, &mut rng), 3, "takes 0 1 2 3");
    }

    #[test]
    fn a_full_board_without_four_is_a_draw() {
        let mut rng = Rng::new(1);
        let mut g = Four::new(&mut rng);
        // a full board with no four in a row, but for the last disc (top right, player 2)
        g.cell = [
            [1, 1, 2, 1, 2, 1, 0],
            [2, 2, 1, 2, 2, 1, 1],
            [2, 2, 2, 1, 1, 2, 1],
            [2, 1, 2, 1, 2, 1, 1],
            [1, 2, 2, 1, 1, 2, 2],
            [1, 2, 1, 2, 1, 1, 2],
        ];
        g.n = (W * H - 1) as u8;
        g.drop(6);
        assert_eq!(g.winner, 3);
    }
}
