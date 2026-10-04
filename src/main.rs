//! A sliding tile puzzle of Van Gogh's Starry Night. See `specs/`.

mod board;
mod hints;
mod lifting;
mod starry_game;
mod tiles;

use blitzkit::start;
use starry_game::StarryGame;

/// Whether this run is only here to be photographed, for `refresh-screenshots`
/// in the project above.
pub fn staged() -> bool {
    std::env::args().any(|arg| arg == "--screenshot")
}

fn main() {
    start("starry", Box::new(StarryGame::new()));
}
