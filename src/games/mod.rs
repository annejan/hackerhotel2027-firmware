//! The games. Each implements `game::Play`; `game::Game` runs them.

mod breakout;
mod chase;
mod flappy;
mod pong;
mod snake;
mod startris;

pub use breakout::Breakout;
pub use chase::Chase;
pub use flappy::Flappy;
pub use pong::Pong;
pub use snake::Snake;
pub use startris::Startris;
