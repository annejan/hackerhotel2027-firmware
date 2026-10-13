//! The games. Each implements `game::Play`; `game::Game` runs them.

mod breakout;
mod chase;
mod flappy;
mod hilo;
mod maze;
mod pong;
mod simon;
mod snake;
mod startris;
mod whack;

pub use breakout::Breakout;
pub use chase::Chase;
pub use flappy::Flappy;
pub use hilo::Hilo;
pub use maze::Maze;
pub use pong::Pong;
pub use simon::Simon;
pub use snake::Snake;
pub use startris::Startris;
pub use whack::Whack;
