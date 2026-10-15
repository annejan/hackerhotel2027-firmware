//! What the badge shows, and the controls.
//!
//! Anywhere:
//!   hold A            open the menu (also from a game)
//!   hold ◀ + ▶, 3 s   reboot into the USB boot ROM (main.rs)
//! Name, hotel and animations:
//!   ◀ / ▶             previous / next item
//!   A                 brightness
//! Menu:
//!   ◀ / ▶             previous / next item, A select, hold A close
//! EDIT NAME: see editor.rs; the new name comes out of `saved_name()` for
//! main.rs to keep in flash.
//! Games: demo mode until you press ◀, B or ▶; then the game owns the
//! buttons (see games/).
//! Infrared: B on the name tag sends the name (HELLO). A badge that gets a
//! HELLO greets the sender ("HI" and the name) and answers with its own
//! name (REPLY), which greets back; so one press swaps names. The packets
//! go out through `ir_take()` (main.rs hands them to ir.rs).
//! Hold A + B for 3 s: the bring-up test (selftest.rs).
//! Challenges (challenge.rs): the name tag's arms show the progress; at
//! 25 / 25 the victory dance plays, and VICTORY in the menu plays it again.
//! A freshly flashed badge starts with the factory test (factory.rs), and
//! the boot after it passed with the sponsor reel (sponsors.rs); main.rs
//! keeps track of that in flash through `stage()`.

use crate::anim::{self, Fire, Life, FPS};
use crate::challenge::{Challenges, Victory};
use crate::editor::{Done, Editor};
use crate::factory::{Factory, Status};
use crate::font::text_width;
use crate::frame::{draw_label, draw_number, draw_text, hsv, Frame, Rgb, Rng};
use crate::game::{Game, Kind};
use crate::generated::{EDGE, GRID_W, RGB};
use crate::irproto;
use crate::log;
use crate::nametag::{self, Name};
use crate::selftest::{Hw, Selftest};
use crate::sponsors::Reel;
use crate::Event;

/// How far the badge has got since it was flashed (kept in flash by
/// store.rs): the factory test runs until it passes, the sponsor reel
/// plays on the boot after that.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    New,
    Tested,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Name,
    Hotel,
    Ripple,
    Galaxy,
    Heartbeat,
    Life,
    Fire,
    Twinkle,
    Game(Kind),
    Victory,
    EditName,
    Light,
}

const ITEMS: [(Item, &str); 22] = [
    (Item::Name, "NAME"),
    (Item::Hotel, "HACKER HOTEL"),
    (Item::Ripple, "RIPPLE"),
    (Item::Galaxy, "GALAXY"),
    (Item::Heartbeat, "HEART"),
    (Item::Life, "LIFE"),
    (Item::Fire, "FIRE"),
    (Item::Twinkle, "TWINKLE"),
    (Item::Game(Kind::Snake), "SNAKE"),
    (Item::Game(Kind::Startris), "STARTRIS"),
    (Item::Game(Kind::Breakout), "BREAKOUT"),
    (Item::Game(Kind::Pong), "PONG"),
    (Item::Game(Kind::Flappy), "FLAPPY"),
    (Item::Game(Kind::Chase), "STAR CHASE"),
    (Item::Game(Kind::Whack), "WHACK-A-MOLE"),
    (Item::Game(Kind::Simon), "SIMON"),
    (Item::Game(Kind::Maze), "MAZE"),
    (Item::Game(Kind::Hilo), "HIGHER LOWER"),
    (Item::Game(Kind::Four), "4 IN A ROW"),
    (Item::Victory, "VICTORY"),
    (Item::EditName, "EDIT NAME"),
    (Item::Light, "LIGHT"),
];

/// Brightness steps, as a share of full PWM duty (applied after gamma):
/// night, 5%, 12.5% (the default), 31%, full. Bare LEDs at arm's length are
/// bright, so the steps stay low.
const LEVELS: [u8; 5] = [4, 12, 32, 80, 255];
const MENU_TIMEOUT: u32 = 20 * FPS;
const GREET_HI: u32 = FPS; // "HI" first, then the name
const GREET_MIN: u32 = 4 * FPS;

pub struct App {
    item: usize,
    menu: Option<usize>, // cursor while the menu is open
    idle: u32,
    level: usize,
    charging: bool,
    show_charge: bool,
    t: u32,
    rng: Rng,
    life: Life,
    fire: Fire,
    game: Game,
    name: Name,
    editor: Option<Editor>,
    saved: Option<Name>,
    // infrared
    ir_out: Option<irproto::Packet>, // a packet waiting for the IR driver
    beam: u32,                  // > 0: frames left of the "sending" sweep
    reply_in: u32,              // > 0: frames until we answer a HELLO
    greet_pending: bool,
    greeting: bool,
    greet_t: u32,
    greet_frames: u32,
    greet: Name, // the other badge's name
    met: u16,    // HELLOs and REPLYs since power on
    test: Option<Selftest>,
    factory: Option<Factory>,
    reel: Option<Reel>,
    challenges: Challenges,
    victory: Victory,
    dance: bool, // the victory dance, just earned; a tap ends it
    stage: Option<Stage>, // waits for main.rs to keep it in flash
    /// Hardware state for the bring-up test; main.rs keeps it current.
    pub hw: Hw,
}

impl App {
    /// `name`: the name kept in flash, or `Name::default()`.
    pub fn new(name: Name) -> Self {
        let mut rng = Rng::new(0x2027_5EA5);
        let life = Life::new(&mut rng);
        let game = Game::new(Kind::Snake, &mut rng);
        Self {
            item: 0,
            menu: None,
            idle: 0,
            level: 2,
            charging: false,
            show_charge: false,
            t: 0,
            rng,
            life,
            fire: Fire::new(),
            game,
            name,
            editor: None,
            saved: None,
            ir_out: None,
            beam: 0,
            reply_in: 0,
            greet_pending: false,
            greeting: false,
            greet_t: 0,
            greet_frames: 0,
            greet: Name::new(b""),
            met: 0,
            test: None,
            factory: None,
            reel: None,
            challenges: Challenges::default(),
            victory: Victory::default(),
            dance: false,
            stage: None,
            hw: Hw::default(),
        }
    }

    fn select(&mut self, i: usize) {
        self.item = i % ITEMS.len();
        self.t = 0;
        match ITEMS[self.item].0 {
            Item::Life => self.life = Life::new(&mut self.rng),
            Item::Fire => self.fire = Fire::new(),
            Item::Game(kind) => self.game = Game::new(kind, &mut self.rng),
            Item::EditName => self.editor = Some(Editor::new(&self.name)),
            Item::Victory => self.victory = Victory::default(),
            _ => {}
        }
    }

    /// A name the user just saved, once, for main.rs to keep in flash.
    pub fn saved_name(&mut self) -> Option<Name> {
        self.saved.take()
    }

    fn editor_done(&mut self, done: Done) {
        if let Done::Save(name) = done {
            self.name = name;
            self.saved = Some(name);
        }
        self.editor = None;
        self.select(0); // the name tag
    }

    fn step(i: usize, d: isize) -> usize {
        (i as isize + d).rem_euclid(ITEMS.len() as isize) as usize
    }

    /// Previous / next screen outside the menu; EDIT NAME and LIGHT are
    /// only menu entries.
    fn browse(&mut self, d: isize) {
        let mut i = Self::step(self.item, d);
        while matches!(ITEMS[i].0, Item::EditName | Item::Light) {
            i = Self::step(i, d);
        }
        self.select(i);
    }

    fn in_game(&self) -> bool {
        matches!(ITEMS[self.item].0, Item::Game(_))
    }

    pub fn handle(&mut self, ev: Event) {
        self.idle = 0;
        if ev == Event::Selftest && self.test.is_none() {
            return self.selftest();
        }
        if let Event::Charging(c) = ev {
            self.charging = c;
        }
        if let Some(factory) = self.factory.as_mut() {
            if !matches!(ev, Event::Charging(_)) {
                factory.handle(ev);
            }
            return;
        }
        if self.reel.is_some() {
            if let Event::Tap(_) = ev {
                self.reel_done();
            }
            return;
        }
        if self.dance {
            if let Event::Tap(_) = ev {
                self.dance = false;
                self.select(0);
            }
            return;
        }
        if let Some(test) = self.test.as_mut() {
            if !matches!(ev, Event::Charging(_)) {
                if !test.handle(ev, self.hw) {
                    self.test = None;
                    self.select(0);
                }
                return;
            }
        }
        if let Event::Charging(c) = ev {
            self.show_charge = c
                && !(self.in_game() && self.game.playing())
                && self.menu.is_none()
                && self.editor.is_none()
                && self.test.is_none();
            return;
        }
        if let Some(cursor) = self.menu {
            match ev {
                Event::Tap(0) => self.menu = Some(Self::step(cursor, -1)),
                Event::Tap(3) => self.menu = Some(Self::step(cursor, 1)),
                Event::Tap(2) if ITEMS[cursor].0 == Item::Light => {
                    self.level = (self.level + 1) % LEVELS.len();
                }
                Event::Tap(2) => {
                    self.menu = None;
                    self.select(cursor);
                }
                Event::Hold(2) => self.menu = None,
                _ => {}
            }
            return;
        }
        if let Some(editor) = self.editor.as_mut() {
            if let Some(done) = editor.handle(ev) {
                self.editor_done(done);
            }
            return;
        }
        if self.greeting || self.show_charge {
            if let Event::Tap(_) = ev {
                self.greeting = false;
                self.show_charge = false;
            }
            return;
        }
        if ITEMS[self.item].0 == Item::Name && ev == Event::Tap(1) {
            self.ir_hello();
            return;
        }
        if self.in_game() && self.game.handle(ev, &mut self.rng) {
            return;
        }
        match ev {
            Event::Hold(2) => self.menu = Some(self.item),
            Event::Tap(0) => self.browse(-1),
            Event::Tap(3) => self.browse(1),
            Event::Tap(2) => self.level = (self.level + 1) % LEVELS.len(),
            _ => {}
        }
    }

    pub fn brightness(&self) -> u8 {
        LEVELS[self.level]
    }

    // What a connected computer may read and set (USB protocol, console).

    pub fn items() -> usize {
        ITEMS.len()
    }

    pub fn label(i: usize) -> &'static str {
        ITEMS.get(i).map_or("", |&(_, label)| label)
    }

    pub fn item(&self) -> usize {
        self.item
    }

    /// Show item `i`; false for the menu-only entries (EDIT NAME, LIGHT).
    pub fn set_item(&mut self, i: usize) -> bool {
        if i >= ITEMS.len() || matches!(ITEMS[i].0, Item::EditName | Item::Light) {
            return false;
        }
        self.menu = None;
        self.editor = None;
        self.show_charge = false;
        self.greeting = false;
        self.select(i);
        true
    }

    pub fn levels() -> usize {
        LEVELS.len()
    }

    pub fn level(&self) -> usize {
        self.level
    }

    pub fn set_level(&mut self, level: usize) {
        if level < LEVELS.len() {
            self.level = level;
        }
    }

    pub fn name(&self) -> Name {
        self.name
    }

    /// Use this name from now on, and hand it to main.rs to keep in flash.
    pub fn set_name(&mut self, name: Name) {
        self.name = name;
        self.saved = Some(name);
        if self.editor.take().is_some() {
            self.select(0); // the editor held an old copy
        }
    }

    pub fn charging(&self) -> bool {
        self.charging
    }

    /// The bring-up test (selftest.rs), as A + B held for 3 s starts it.
    pub fn selftest(&mut self) {
        self.menu = None;
        self.editor = None;
        self.show_charge = false;
        self.greeting = false;
        self.factory = None;
        self.reel = None;
        self.test = Some(Selftest::new(self.hw));
    }

    /// The challenges' progress (for the console).
    pub fn challenges(&self) -> &Challenges {
        &self.challenges
    }

    /// Progress kept in flash, at power on.
    /// Mix `v` (the chip's unique ID) into the random generator.
    pub fn seed(&mut self, v: u32) {
        self.rng.seed(v);
    }

    pub fn load_progress(&mut self, packed: u16) {
        self.challenges.load(packed);
    }

    /// The challenges' progress when it changed, once, for main.rs to keep
    /// in flash.
    pub fn progress(&mut self) -> Option<u16> {
        self.challenges.take_changed().then(|| self.challenges.packed())
    }

    /// The stage the badge just reached, once, for main.rs to keep in flash.
    pub fn stage(&mut self) -> Option<Stage> {
        self.stage.take()
    }

    /// The factory test (factory.rs): main.rs starts it on a new badge,
    /// the console with `factory`.
    pub fn factory(&mut self) {
        self.menu = None;
        self.editor = None;
        self.show_charge = false;
        self.greeting = false;
        self.test = None;
        self.reel = None;
        self.factory = Some(Factory::new());
    }

    /// Mark the factory test as passed without running it (`factory skip`).
    pub fn factory_skip(&mut self) {
        log::line("factory test: skipped");
        self.stage = Some(Stage::Tested);
    }

    /// The sponsor reel (sponsors.rs).
    pub fn sponsors(&mut self) {
        self.menu = None;
        self.editor = None;
        self.show_charge = false;
        self.greeting = false;
        self.test = None;
        self.factory = None;
        log::line("sponsor reel");
        self.reel = Some(Reel::new());
    }

    fn reel_done(&mut self) {
        self.reel = None;
        self.stage = Some(Stage::Done);
        self.select(0);
    }

    /// Send our name over IR, as B on the name tag does; false while a
    /// packet is still waiting.
    pub fn ir_hello(&mut self) -> bool {
        if self.ir_out.is_some() {
            return false;
        }
        self.ir_out = Some(irproto::Packet::new(irproto::HELLO, self.name.bytes()));
        self.beam = FPS;
        log::line("ir: hello sent");
        true
    }

    /// A packet to send, for main.rs to hand to the IR driver.
    pub fn ir_take(&mut self) -> Option<irproto::Packet> {
        self.ir_out.take()
    }

    /// The bring-up test's IR check wants to hear our own packets.
    pub fn ir_loopback(&self) -> bool {
        self.test.as_ref().is_some_and(|t| t.loopback()) || self.factory.as_ref().is_some_and(|f| f.loopback())
    }

    /// A packet from another badge (or our own, in the IR test).
    pub fn ir_received(&mut self, ty: u8, data: &[u8]) {
        if ty == irproto::TEST {
            if let Some(test) = self.test.as_mut() {
                test.ir_heard();
            } else if let Some(factory) = self.factory.as_mut() {
                factory.ir_heard();
            }
            return;
        }
        if ty == irproto::FOUR {
            if self.in_game() && !self.busy_elsewhere() {
                self.game.ir_received(data, &mut self.rng);
            }
            return;
        }
        if ty != irproto::HELLO && ty != irproto::REPLY {
            return;
        }
        let name = Name::new(data);
        if name.bytes().is_empty() {
            return;
        }
        self.met = self.met.wrapping_add(1);
        log::text(if ty == irproto::HELLO { "ir: hello from " } else { "ir: reply from " });
        log::line(name.as_str());
        if ty == irproto::HELLO {
            self.reply_in = 3 + self.rng.next() % 6; // 0.1 to 0.3 s: let their receiver settle
        }
        self.greet = name;
        self.greet_pending = true;
    }

    /// The menu, EDIT NAME, a test, the reel or the dance has the screen.
    fn busy_elsewhere(&self) -> bool {
        self.menu.is_some()
            || self.editor.is_some()
            || self.test.is_some()
            || self.factory.is_some()
            || self.reel.is_some()
            || self.dance
    }

    fn ir_tick(&mut self) {
        if self.reply_in > 0 {
            self.reply_in -= 1;
            if self.reply_in == 0 {
                if self.ir_out.is_none() {
                    self.ir_out = Some(irproto::Packet::new(irproto::REPLY, self.name.bytes()));
                    log::line("ir: reply sent");
                } else {
                    self.reply_in = 1; // still sending something: next frame
                }
            }
        }
        if self.ir_out.is_none() {
            self.ir_out = self.game.ir_take();
        }
        let busy = self.busy_elsewhere() || (self.in_game() && self.game.playing());
        if self.greet_pending && !busy {
            self.greet_pending = false;
            self.greeting = true;
            self.show_charge = false;
            self.greet_t = 0;
            // long names scroll: stay until the name has gone by once
            let w = text_width(self.greet.as_str());
            self.greet_frames = GREET_HI + if w <= GRID_W { GREET_MIN } else { (GRID_W + w) as u32 * FPS / 10 };
        }
    }

    /// "HI", then the other badge's name where the name tag has its own;
    /// colours run round the arms.
    fn draw_greeting(&self, f: &mut Frame, t: u32) {
        let s = self.greet.as_str();
        let w = text_width(s);
        if t < GREET_HI {
            draw_text(f, "HI", (GRID_W - text_width("HI")) as i32 / 2, 4, 255);
        } else if w <= GRID_W {
            draw_text(f, s, (GRID_W - w) as i32 / 2, 4, 255);
        } else {
            // scrolls by once, at the name tag's speed
            draw_text(f, s, GRID_W as i32 - ((t - GREET_HI) * 10 / FPS) as i32, 4, 255);
        }
        for (i, led) in RGB.iter().enumerate() {
            let h = (led.arm as u32 * 51 + led.rank as u32 * 12).wrapping_sub(t * 6) as u8;
            f.rgb(i, hsv(h, 255, 200));
        }
        for (j, &s) in EDGE.iter().enumerate() {
            let k = (j as u32 * 7).wrapping_sub(t * 3) % 71;
            f.set(s, if k < 10 { 200 } else { 20 });
        }
    }

    /// Sending: a white wave runs out along the arms.
    fn draw_beam(&self, f: &mut Frame) {
        let front = ((FPS - self.beam) * 9 / FPS) as i32; // rank of the wave, 0..8
        for (i, led) in RGB.iter().enumerate() {
            let d = front - led.rank as i32;
            if (0..3).contains(&d) {
                f.rgb(i, Rgb(200, 220, 255).scaled(255 - d as u8 * 80));
            }
        }
    }

    /// `held`: buttons down right now, bit n = button n (◀ B A ▶).
    pub fn render(&mut self, f: &mut Frame, held: u8) {
        self.ir_tick();
        let t = self.t;
        self.t = self.t.wrapping_add(1);
        if let Some(test) = self.test.as_mut() {
            return test.render(f, held, self.hw, self.level, self.charging, &mut self.ir_out);
        }
        if let Some(factory) = self.factory.as_mut() {
            match factory.render(f, self.hw, self.charging, &mut self.ir_out) {
                Status::Passed => self.stage = Some(Stage::Tested),
                Status::Done => {
                    self.factory = None;
                    self.select(0);
                }
                Status::Busy => {}
            }
            return;
        }
        if let Some(reel) = self.reel.as_mut() {
            if reel.render(f) {
                return;
            }
            self.reel_done();
        }
        // all 25 just done: dance, once the game's score screen is over
        if self.challenges.victory_due()
            && !(self.in_game() && self.game.playing())
            && self.menu.is_none()
            && self.editor.is_none()
        {
            self.challenges.victory_done();
            log::line("all challenges done: victory dance");
            self.dance = true;
            self.victory = Victory::default();
        }
        if self.dance {
            if self.victory.render(f, &mut self.rng) {
                return;
            }
            self.dance = false;
            self.select(0);
        }
        if let Some(cursor) = self.menu {
            self.idle += 1;
            if self.idle > MENU_TIMEOUT {
                self.menu = None;
            }
            return self.draw_menu(f, cursor, t);
        }
        if let Some(editor) = self.editor.as_mut() {
            if let Some(done) = editor.render(f, held) {
                self.editor_done(done);
            }
            return;
        }
        if self.greeting {
            self.draw_greeting(f, self.greet_t);
            self.greet_t += 1;
            if self.greet_t >= self.greet_frames {
                self.greeting = false;
            }
            return;
        }
        if self.show_charge {
            return anim::charging(f, t);
        }
        match ITEMS[self.item].0 {
            Item::Name => {
                nametag::name(f, t, &mut self.rng, &self.name);
                self.challenges.arms(f, t, None); // the challenges' progress, to compare
                if self.beam > 0 {
                    self.draw_beam(f);
                    self.beam -= 1;
                }
            }
            Item::Hotel => nametag::hotel(f, t),
            Item::Ripple => anim::ripple(f, t),
            Item::Galaxy => anim::galaxy(f, t),
            Item::Heartbeat => anim::heartbeat(f, t),
            Item::Life => self.life.render(f, t, &mut self.rng),
            Item::Fire => self.fire.render(f, &mut self.rng),
            Item::Twinkle => anim::twinkle(f, t),
            Item::Game(_) => self.game.render(f, held, &mut self.rng, &mut self.challenges),
            Item::Victory => {
                if self.challenges.complete() {
                    if !self.victory.render(f, &mut self.rng) {
                        self.victory = Victory::default(); // over and over
                    }
                } else {
                    // not yet: how far, on the grid and the arms
                    draw_number(f, self.challenges.total() as u16, 5, 255);
                    self.challenges.arms(f, t, None);
                }
            }
            Item::EditName | Item::Light => self.select(0), // menu entries only
        }
    }

    /// Menu: the item's name on the grid, its place in the list on the
    /// outline, and its group in the arms' colour.
    fn draw_menu(&self, f: &mut Frame, cursor: usize, t: u32) {
        let (item, label) = ITEMS[cursor];
        draw_label(f, label, 4, t, FPS, 255);
        let n = EDGE.len();
        let (a, b) = (cursor * n / ITEMS.len(), (cursor + 1) * n / ITEMS.len());
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (a..b).contains(&j) { 220 } else { 12 });
        }
        let group = match item {
            Item::Name | Item::Hotel => Rgb(255, 150, 20),
            Item::Game(_) => Rgb(20, 255, 70),
            Item::EditName | Item::Light => Rgb(255, 255, 255),
            _ => Rgb(170, 40, 255),
        };
        for i in 0..RGB.len() {
            f.rgb(i, group.scaled(110));
        }
        if item == Item::Light {
            for (k, _) in LEVELS.iter().enumerate() {
                let v = if k == self.level { 255 } else if k < self.level { 90 } else { 15 };
                for c in 0..2 {
                    f.grid(13, 3 + k * 2 + c, v);
                }
            }
        }
    }
}
