//! The tiles, the gap, and what can be reached from where. See
//! `specs/0001-the-board.md`.
//!
//! None of this needs a window, which is why it exists before anything that
//! draws. The parity rules are the part most likely to be quietly wrong, and
//! one of them already was.

use rand::Rng;

/// Four by four, which spec 0001 fixes.
pub const WIDTH: usize = 4;
pub const CELLS: usize = WIDTH * WIDTH;

/// What sits in the gap. Tiles are 1 through 15.
pub const GAP: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Which way the gap travels, as a row and column step.
    fn step(self) -> (i32, i32) {
        match self {
            Direction::Up => (-1, 0),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    cells: [u8; CELLS],
}

/// How far apart two cells are along the rows and columns. A slide is a
/// distance of one, and spec 0001's rule for a lift is this being odd.
pub fn distance(a: usize, b: usize) -> usize {
    let (ar, ac) = (a / WIDTH, a % WIDTH);
    let (br, bc) = (b / WIDTH, b % WIDTH);
    ar.abs_diff(br) + ac.abs_diff(bc)
}

/// Whether moving the gap from `gap` to `tile` leaves the puzzle finishable.
///
/// It reads the two positions and nothing else: the arrangement does not come
/// into it, which was checked against all 240 pairs of cells. See spec 0001.
pub fn lift_is_safe(gap: usize, tile: usize) -> bool {
    !distance(gap, tile).is_multiple_of(2)
}

impl Board {
    /// The finished picture: tiles in order with the gap last.
    pub fn solved() -> Self {
        let mut cells = [GAP; CELLS];
        for (index, cell) in cells.iter_mut().enumerate().take(CELLS - 1) {
            *cell = index as u8 + 1;
        }
        Self { cells }
    }

    /// A board scrambled by making legal moves from solved, which is the only
    /// way to be sure it can be finished. Never hands back a solved board.
    pub fn scrambled(rng: &mut impl Rng, moves: usize) -> Self {
        let mut board = Self::solved();
        for _ in 0..moves.max(1) {
            let direction = match rng.gen_range(0..4) {
                0 => Direction::Up,
                1 => Direction::Down,
                2 => Direction::Left,
                _ => Direction::Right,
            };
            board.slide(direction);
        }

        if board.is_solved() {
            // vanishingly rare, and a solved board is not a puzzle
            board.slide(Direction::Up);
            board.slide(Direction::Left);
        }
        board
    }

    pub fn cell(&self, index: usize) -> u8 {
        self.cells[index]
    }

    pub fn gap(&self) -> usize {
        self.cells
            .iter()
            .position(|cell| *cell == GAP)
            .expect("a board always has its gap")
    }

    pub fn is_solved(&self) -> bool {
        *self == Self::solved()
    }

    /// Slides the tile on the given side of the gap into it. A direction with
    /// no tile that way does nothing, and says so.
    pub fn slide(&mut self, direction: Direction) -> bool {
        let gap = self.gap();
        let (dr, dc) = direction.step();
        let (row, column) = ((gap / WIDTH) as i32 + dr, (gap % WIDTH) as i32 + dc);

        if !(0..WIDTH as i32).contains(&row) || !(0..WIDTH as i32).contains(&column) {
            return false;
        }

        let from = row as usize * WIDTH + column as usize;
        self.cells.swap(gap, from);
        true
    }

    /// Takes the tile at `tile` out and puts it in the gap. Refuses only the
    /// gap itself: a lift that strands the player is allowed, because spec 0001
    /// warns them rather than stopping them.
    pub fn lift(&mut self, tile: usize) -> bool {
        let gap = self.gap();
        if tile == gap || tile >= CELLS {
            return false;
        }
        self.cells.swap(gap, tile);
        true
    }

    /// Whether this arrangement can be finished by sliding.
    ///
    /// For a four wide board that is its inversion count plus the gap's row
    /// counted from the bottom being odd. See spec 0001.
    pub fn is_solvable(&self) -> bool {
        let tiles: Vec<u8> = self.cells.iter().copied().filter(|c| *c != GAP).collect();
        let inversions = (0..tiles.len())
            .flat_map(|i| (i + 1..tiles.len()).map(move |j| (i, j)))
            .filter(|(i, j)| tiles[*i] > tiles[*j])
            .count();
        let row_from_bottom = WIDTH - self.gap() / WIDTH;

        !(inversions + row_from_bottom).is_multiple_of(2)
    }

    /// The fewest slides this board could possibly need: every tile's distance
    /// from where it belongs. A true lower bound, so it is honest as a par.
    pub fn par(&self) -> usize {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| **cell != GAP)
            .map(|(index, cell)| distance(index, *cell as usize - 1))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    #[test]
    fn solved_is_in_order_with_the_gap_last() {
        let board = Board::solved();

        assert_eq!(board.cell(0), 1);
        assert_eq!(board.cell(CELLS - 2), 15);
        assert_eq!(board.cell(CELLS - 1), GAP);
        assert!(board.is_solved());
    }

    #[test]
    fn the_solved_board_is_reachable() {
        // the parity rule had better agree that the finished picture is legal
        assert!(Board::solved().is_solvable());
    }

    #[test]
    fn a_solved_board_is_par_zero() {
        assert_eq!(Board::solved().par(), 0);
    }

    #[test]
    fn par_is_the_distance_home() {
        let mut board = Board::solved();
        board.slide(Direction::Up);

        // one tile moved one cell, so one slide is the least it can take
        assert_eq!(board.par(), 1);
    }

    #[test]
    fn every_generated_board_is_solvable() {
        // over many seeds: a scramble that fails rarely looks like the player's
        // mistake rather than a bug, which is the worst way for it to present
        for seed in 0..500 {
            let board = Board::scrambled(&mut rng(seed), 80);
            assert!(
                board.is_solvable(),
                "seed {} made an unfinishable board",
                seed
            );
        }
    }

    #[test]
    fn a_generated_board_is_not_solved() {
        for seed in 0..500 {
            assert!(!Board::scrambled(&mut rng(seed), 80).is_solved());
        }
    }

    #[test]
    fn sliding_keeps_a_board_solvable() {
        for seed in 0..200 {
            let mut board = Board::scrambled(&mut rng(seed), 40);
            for direction in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                board.slide(direction);
                assert!(board.is_solvable());
            }
        }
    }

    #[test]
    fn a_move_off_the_edge_does_nothing() {
        let mut board = Board::solved();
        // the gap starts in the bottom right corner
        let before = board;

        assert!(!board.slide(Direction::Down));
        assert!(!board.slide(Direction::Right));
        assert_eq!(board, before);
    }

    #[test]
    fn a_slide_is_a_lift_of_one() {
        // the lift rule is the slide rule generalised, which is why a slide
        // never strands anyone
        let mut board = Board::solved();
        board.slide(Direction::Up);
        let gap = board.gap();

        assert_eq!(distance(gap, CELLS - 1), 1);
        assert!(lift_is_safe(gap, CELLS - 1));
    }

    #[test]
    fn an_odd_lift_keeps_it() {
        for seed in 0..200 {
            let mut board = Board::scrambled(&mut rng(seed), 60);
            let gap = board.gap();
            let tile = (0..CELLS)
                .find(|cell| *cell != gap && !distance(gap, *cell).is_multiple_of(2))
                .expect("some cell is an odd distance away");

            assert!(lift_is_safe(gap, tile));
            board.lift(tile);
            assert!(board.is_solvable(), "seed {}", seed);
        }
    }

    #[test]
    fn an_even_lift_breaks_it() {
        for seed in 0..200 {
            let mut board = Board::scrambled(&mut rng(seed), 60);
            let gap = board.gap();
            let tile = (0..CELLS)
                .find(|cell| *cell != gap && distance(gap, *cell).is_multiple_of(2))
                .expect("some cell is an even distance away");

            assert!(!lift_is_safe(gap, tile));
            board.lift(tile);
            assert!(!board.is_solvable(), "seed {}", seed);
        }
    }

    #[test]
    fn a_second_lift_repairs_the_first() {
        // which is what makes stranding yourself the player's own business,
        // right up until the last lift is spent
        for seed in 0..200 {
            let mut board = Board::scrambled(&mut rng(seed), 60);
            let gap = board.gap();
            let tile = (0..CELLS)
                .find(|cell| *cell != gap && !lift_is_safe(gap, *cell))
                .expect("some cell is an even distance away");
            board.lift(tile);
            assert!(!board.is_solvable());

            let gap = board.gap();
            let second = (0..CELLS)
                .find(|cell| *cell != gap && !lift_is_safe(gap, *cell))
                .expect("some cell is an even distance away");
            board.lift(second);

            assert!(board.is_solvable(), "seed {}", seed);
        }
    }

    #[test]
    fn the_warning_reads_only_the_positions() {
        // no arrangement anywhere changes the answer, which is why the warning
        // needs two coordinates rather than a board. Arrangements here are any
        // permutation, reachable or not, because the claim is about the change
        // a lift makes rather than about where it starts.
        let mut rng = rng(99);

        for gap in 0..CELLS {
            for tile in 0..CELLS {
                if tile == gap {
                    continue;
                }

                for _ in 0..8 {
                    let mut tiles: Vec<u8> = (1..=15).collect();
                    for i in (1..tiles.len()).rev() {
                        tiles.swap(i, rng.gen_range(0..=i));
                    }

                    let mut cells = [GAP; CELLS];
                    let mut next = tiles.into_iter();
                    for (index, cell) in cells.iter_mut().enumerate() {
                        if index != gap {
                            *cell = next.next().expect("fifteen tiles for fifteen cells");
                        }
                    }

                    let before = Board { cells };
                    let mut after = before;
                    after.lift(tile);

                    assert_eq!(
                        before.is_solvable() == after.is_solvable(),
                        lift_is_safe(gap, tile),
                        "gap {} tile {}",
                        gap,
                        tile
                    );
                }
            }
        }
    }

    #[test]
    fn a_lift_onto_the_gap_does_nothing() {
        let mut board = Board::scrambled(&mut rng(1), 40);
        let before = board;

        assert!(!board.lift(board.gap()));
        assert_eq!(board, before);
    }
}
