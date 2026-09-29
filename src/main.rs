//! A sliding tile puzzle of Van Gogh's Starry Night. See `specs/`.

mod board;
mod lifting;
mod starry_game;
mod tiles;

use blitzkit::start;
use starry_game::StarryGame;

fn main() {
    start("starry", Box::new(StarryGame::new()));
}
