//! What the badge shows, and the controls.
//!
//! Anywhere:
//!   hold T3            open the menu (also from a game)
//!   hold T1 + T4, 3 s  reboot into the USB boot ROM (main.rs)
//! Name, hotel and animations:
//!   T1 / T4            previous / next item
//!   T3                 brightness
//! Menu:
//!   T1 / T4            previous / next item, T3 select, hold T3 close
//! EDIT NAME: see editor.rs; the new name comes out of `saved_name()` for
//! main.rs to keep in flash.
//! Games: demo mode until you touch T1, T2 or T4; then the game owns the
//! pads (see games/).

use crate::anim::{self, Fire, Life, FPS};
use crate::editor::{Done, Editor};
use crate::frame::{draw_label, Frame, Rgb, Rng};
use crate::game::{Game, Kind};
use crate::generated::{EDGE, RGB};
use crate::nametag::{self, Name};
use crate::Event;

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
    EditName,
    Light,
}

const ITEMS: [(Item, &str); 16] = [
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
    (Item::EditName, "EDIT NAME"),
    (Item::Light, "LIGHT"),
];

/// Brightness steps, as a share of full PWM duty (applied after gamma):
/// night, 5%, 12.5% (the default), 31%, full. Bare LEDs at arm's length are
/// bright, so the steps stay low.
const LEVELS: [u8; 5] = [4, 12, 32, 80, 255];
const MENU_TIMEOUT: u32 = 20 * FPS;

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
        if let Event::Charging(c) = ev {
            self.charging = c;
            self.show_charge = c && !(self.in_game() && self.game.playing()) && self.menu.is_none() && self.editor.is_none();
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
        if self.show_charge {
            if let Event::Tap(_) = ev {
                self.show_charge = false;
            }
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

    /// `held`: pads touched right now, bit n = pad Tn+1.
    pub fn render(&mut self, f: &mut Frame, held: u8) {
        let t = self.t;
        self.t = self.t.wrapping_add(1);
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
        if self.show_charge {
            return anim::charging(f, t);
        }
        match ITEMS[self.item].0 {
            Item::Name => nametag::name(f, t, &mut self.rng, &self.name),
            Item::Hotel => nametag::hotel(f, t),
            Item::Ripple => anim::ripple(f, t),
            Item::Galaxy => anim::galaxy(f, t),
            Item::Heartbeat => anim::heartbeat(f, t),
            Item::Life => self.life.render(f, t, &mut self.rng),
            Item::Fire => self.fire.render(f, &mut self.rng),
            Item::Twinkle => anim::twinkle(f, t),
            Item::Game(_) => self.game.render(f, held, &mut self.rng),
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
