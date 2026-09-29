//! Aiming a lift with the arrows, the warning before one that strands the
//! player, and the one way back out of a dead end. See `specs/0003-lifting.md`.
//!
//! The board is still the only thing that knows the rules. This holds where the
//! cursor is, how many lifts are left, and the one board it can put back.

use crate::board::{lift_is_safe, Board, Direction, CELLS, GAP, WIDTH};

/// How many lifts a puzzle comes with. Spec 0001 fixes it at three.
pub const LIFTS: u8 = 3;

/// What pressing enter did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    /// Nothing to confirm: the mode is shut.
    Nothing,
    /// This one strands the player, and they have been told so.
    Warned(usize),
    /// Done. The caller animates it, from the cell it left to the one it took.
    Lifted { tile: u8, from: usize, to: usize },
}

#[derive(Debug, Clone)]
pub struct Lifting {
    left: u8,
    /// Where the cursor is, and whether the mode is open at all.
    cursor: Option<usize>,
    /// The cell a warning is standing against. Moving the cursor forgets it.
    warned: Option<usize>,
    /// The board as it was before the last lift. One move, and no further.
    before: Option<Board>,
}

impl Lifting {
    pub fn new() -> Self {
        Self {
            left: LIFTS,
            cursor: None,
            warned: None,
            before: None,
        }
    }

    pub fn left(&self) -> u8 {
        self.left
    }

    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    pub fn warned(&self) -> Option<usize> {
        self.warned
    }

    pub fn is_open(&self) -> bool {
        self.cursor.is_some()
    }

    /// Whether a cell would be a stranding lift from here. Spec 0001's rule and
    /// nothing else: an even number of cells between the gap and the tile.
    pub fn strands(board: &Board, cell: usize) -> bool {
        cell != board.gap() && !lift_is_safe(board.gap(), cell)
    }

    /// Opens the mode, unless there is nothing left to spend.
    pub fn open(&mut self, board: &Board) -> bool {
        if self.left == 0 {
            return false;
        }

        // reading order, every time. A cursor that reappears where it was last
        // is worse than one that always starts in the corner.
        self.cursor = (0..CELLS).find(|cell| board.cell(*cell) != GAP);
        self.warned = None;
        self.cursor.is_some()
    }

    pub fn close(&mut self) {
        self.cursor = None;
        self.warned = None;
    }

    /// Moves the cursor one cell, stepping over the gap rather than onto it.
    /// Does not wrap, which is how the arrows behave when they are sliding.
    pub fn move_cursor(&mut self, board: &Board, direction: Direction) -> bool {
        let Some(from) = self.cursor else {
            return false;
        };

        let (dr, dc) = match direction {
            Direction::Up => (-1i32, 0i32),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1),
        };

        // one cell, then one further if that landed on the gap
        let mut row = (from / WIDTH) as i32;
        let mut column = (from % WIDTH) as i32;
        for _ in 0..2 {
            row += dr;
            column += dc;

            if !(0..WIDTH as i32).contains(&row) || !(0..WIDTH as i32).contains(&column) {
                return false;
            }

            let cell = row as usize * WIDTH + column as usize;
            if board.cell(cell) != GAP {
                self.cursor = Some(cell);
                self.warned = None;
                return true;
            }
        }

        false
    }

    /// Enter. A stranding lift is refused once and taken the second time.
    pub fn confirm(&mut self, board: &mut Board) -> Confirm {
        let Some(cell) = self.cursor else {
            return Confirm::Nothing;
        };

        if Self::strands(board, cell) && self.warned != Some(cell) {
            self.warned = Some(cell);
            return Confirm::Warned(cell);
        }

        let tile = board.cell(cell);
        let to = board.gap();

        let before = *board;
        if !board.lift(cell) {
            return Confirm::Nothing;
        }

        self.before = Some(before);
        self.left -= 1;
        self.close();

        Confirm::Lifted {
            tile,
            from: cell,
            to,
        }
    }

    /// Whether there is a way back, which there is only from a dead end: a
    /// board no sliding will finish and no lift left to repair it.
    pub fn can_rewind(&self, board: &Board) -> bool {
        self.before.is_some() && self.left == 0 && !board.is_solvable()
    }

    /// Puts the board back to before the lift. The lift stays spent, so the
    /// warning keeps its teeth. One move, and the way back goes with it.
    pub fn rewind(&mut self, board: &mut Board) -> bool {
        if !self.can_rewind(board) {
            return false;
        }

        *board = self.before.take().expect("can_rewind checked it");
        self.close();
        true
    }
}

impl Default for Lifting {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn board(seed: u64) -> Board {
        Board::scrambled(&mut StdRng::seed_from_u64(seed), 60)
    }

    /// A cell an odd number away from the gap, which is a lift that keeps the
    /// board finishable.
    fn safe_cell(board: &Board) -> usize {
        (0..CELLS)
            .find(|cell| *cell != board.gap() && lift_is_safe(board.gap(), *cell))
            .expect("some cell is an odd distance away")
    }

    fn stranding_cell(board: &Board) -> usize {
        (0..CELLS)
            .find(|cell| Lifting::strands(board, *cell))
            .expect("some cell is an even distance away")
    }

    #[test]
    fn l_opens_the_mode() {
        let mut lifting = Lifting::new();

        assert!(lifting.open(&board(1)));
        assert!(lifting.is_open());
        assert!(lifting.cursor().is_some());
    }

    #[test]
    fn no_lifts_left_means_no_mode() {
        let mut board = board(2);
        let mut lifting = Lifting::new();
        for _ in 0..LIFTS {
            lifting.open(&board);
            let safe = safe_cell(&board);
            lifting.cursor = Some(safe);
            lifting.confirm(&mut board);
        }
        assert_eq!(lifting.left(), 0);

        assert!(!lifting.open(&board));
        assert!(!lifting.is_open());
    }

    #[test]
    fn l_closes_it_again() {
        let board = board(3);
        let mut lifting = Lifting::new();
        lifting.open(&board);

        lifting.close();

        assert!(!lifting.is_open());
        assert_eq!(lifting.left(), LIFTS, "leaving spends nothing");
    }

    #[test]
    fn the_cursor_starts_in_reading_order() {
        let mut board = Board::solved();
        // put the gap first, so the answer is not trivially cell zero
        while board.gap() != 0 {
            board.slide(Direction::Up);
            board.slide(Direction::Left);
        }

        let mut lifting = Lifting::new();
        lifting.open(&board);

        assert_eq!(lifting.cursor(), Some(1), "the first cell that is not gap");
    }

    #[test]
    fn the_cursor_steps_over_the_gap() {
        let mut board = Board::solved();
        // gap at 15; put the cursor at 13 and walk it right, over 15's row
        let mut lifting = Lifting::new();
        lifting.open(&board);
        lifting.cursor = Some(CELLS - 3);

        assert!(lifting.move_cursor(&board, Direction::Right));
        assert_eq!(lifting.cursor(), Some(CELLS - 2));

        // the next step right would land on the gap at 15, and there is nothing
        // beyond it on this row
        assert!(!lifting.move_cursor(&board, Direction::Right));
        assert_eq!(lifting.cursor(), Some(CELLS - 2), "so it did not move");

        board.slide(Direction::Left);
        assert_ne!(board.gap(), CELLS - 1, "the gap moved off the corner");
    }

    #[test]
    fn the_cursor_does_not_wrap() {
        let board = Board::solved();
        let mut lifting = Lifting::new();
        lifting.open(&board);
        lifting.cursor = Some(0);

        assert!(!lifting.move_cursor(&board, Direction::Up));
        assert!(!lifting.move_cursor(&board, Direction::Left));
        assert_eq!(lifting.cursor(), Some(0));
    }

    #[test]
    fn the_arrows_move_the_cursor_not_the_board() {
        let board = board(4);
        let before = board;
        let mut lifting = Lifting::new();
        lifting.open(&board);
        let started = lifting.cursor();

        lifting.move_cursor(&board, Direction::Right);
        lifting.move_cursor(&board, Direction::Down);

        assert_eq!(board, before, "the board did not move");
        assert_ne!(lifting.cursor(), started, "and the cursor did");
    }

    #[test]
    fn enter_lifts_and_spends_one() {
        let mut board = board(5);
        let mut lifting = Lifting::new();
        lifting.open(&board);
        let safe = safe_cell(&board);
        lifting.cursor = Some(safe);
        let tile = board.cell(safe);
        let gap = board.gap();

        let result = lifting.confirm(&mut board);

        assert_eq!(
            result,
            Confirm::Lifted {
                tile,
                from: safe,
                to: gap
            }
        );
        assert_eq!(board.cell(gap), tile, "it went to the gap");
        assert_eq!(lifting.left(), LIFTS - 1);
        assert!(!lifting.is_open(), "and the mode shut behind it");
    }

    #[test]
    fn a_stranding_lift_warns_first() {
        let mut board = board(6);
        let before = board;
        let mut lifting = Lifting::new();
        lifting.open(&board);
        let bad = stranding_cell(&board);
        lifting.cursor = Some(bad);

        assert_eq!(lifting.confirm(&mut board), Confirm::Warned(bad));
        assert_eq!(board, before, "nothing moved");
        assert_eq!(lifting.left(), LIFTS, "and nothing was spent");
    }

    #[test]
    fn a_warned_lift_goes_through_on_the_second_press() {
        let mut board = board(7);
        let mut lifting = Lifting::new();
        lifting.open(&board);
        let bad = stranding_cell(&board);
        lifting.cursor = Some(bad);

        lifting.confirm(&mut board);
        let result = lifting.confirm(&mut board);

        assert!(matches!(result, Confirm::Lifted { .. }));
        assert_eq!(lifting.left(), LIFTS - 1);
        assert!(!board.is_solvable(), "which is what they were warned about");
    }

    #[test]
    fn moving_the_cursor_forgets_the_warning() {
        let mut board = board(8);
        let mut lifting = Lifting::new();
        lifting.open(&board);
        let bad = stranding_cell(&board);
        lifting.cursor = Some(bad);
        lifting.confirm(&mut board);
        assert_eq!(lifting.warned(), Some(bad));

        lifting.move_cursor(&board, Direction::Right);

        assert_eq!(lifting.warned(), None);
    }

    #[test]
    fn the_marked_tiles_are_the_stranding_ones() {
        // the marking is spec 0001's rule and nothing else
        for seed in 0..40 {
            let board = board(seed);
            for cell in 0..CELLS {
                if cell == board.gap() {
                    continue;
                }
                let mut after = board;
                after.lift(cell);

                assert_eq!(
                    Lifting::strands(&board, cell),
                    !after.is_solvable(),
                    "seed {} cell {}",
                    seed,
                    cell
                );
            }
        }
    }

    /// Spends every lift, the last one on a cell that strands the board.
    fn stuck_with_nothing_left(seed: u64) -> (Board, Lifting) {
        let mut board = board(seed);
        let mut lifting = Lifting::new();

        for spent in 0..LIFTS {
            lifting.open(&board);
            let cell = if spent == LIFTS - 1 {
                stranding_cell(&board)
            } else {
                safe_cell(&board)
            };
            lifting.cursor = Some(cell);
            lifting.confirm(&mut board);
            if spent == LIFTS - 1 {
                lifting.cursor = Some(cell);
                lifting.confirm(&mut board);
            }
        }

        (board, lifting)
    }

    #[test]
    fn a_rewind_is_offered_only_when_it_is_true() {
        let mut board = board(9);
        let mut lifting = Lifting::new();
        assert!(!lifting.can_rewind(&board), "nothing has been lifted");

        lifting.open(&board);
        lifting.cursor = Some(safe_cell(&board));
        lifting.confirm(&mut board);
        assert!(
            !lifting.can_rewind(&board),
            "a finishable board with lifts left is not a dead end"
        );

        let (stuck, lifting) = stuck_with_nothing_left(10);
        assert!(lifting.can_rewind(&stuck));
    }

    #[test]
    fn a_rewind_puts_the_board_back() {
        let (mut board, mut lifting) = stuck_with_nothing_left(11);
        let before = lifting.before.expect("a lift was taken");

        assert!(lifting.rewind(&mut board));

        assert_eq!(board, before);
        assert!(board.is_solvable(), "and it can be finished again");
    }

    #[test]
    fn a_rewind_does_not_return_the_lift() {
        let (mut board, mut lifting) = stuck_with_nothing_left(12);

        lifting.rewind(&mut board);

        assert_eq!(lifting.left(), 0, "the board comes back, the lift does not");
    }

    #[test]
    fn a_rewind_reaches_one_move() {
        let (mut board, mut lifting) = stuck_with_nothing_left(13);

        assert!(lifting.rewind(&mut board));
        assert!(!lifting.rewind(&mut board), "and no further back than that");
    }

    #[test]
    fn a_slide_cannot_be_rewound() {
        let mut board = board(14);
        let mut lifting = Lifting::new();

        for _ in 0..8 {
            board.slide(Direction::Up);
            board.slide(Direction::Left);
        }

        assert!(
            !lifting.can_rewind(&board),
            "sliding leaves nothing to undo"
        );
        assert!(!lifting.rewind(&mut board));
    }
}
