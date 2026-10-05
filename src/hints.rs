//! Four levels of help, from a number to a demonstration, and the search
//! behind them. See `specs/0004-hints.md`.
//!
//! The search is iterative deepening with Manhattan distance and linear
//! conflict, which is exact rather than an estimate. It is cheap on the boards
//! this game deals and expensive on boards it does not, so it has a budget and
//! an honest answer for when it runs out.

use crate::board::{Board, Direction, CELLS, GAP, WIDTH};

/// How many search nodes a hint may cost.
///
/// Sixteen boards gave a median of 1,295 and a worst case of 24,808, and 400,000
/// was read off that as an order of magnitude past anything play produces.
/// Sixteen was too few to see the tail. Over 200 seeds of this game's own
/// scramble, five beat 400,000: a player meets one about one game in forty, and
/// what they get is the hints quietly refusing to plan and the demonstration
/// switching itself off on its first frame.
///
/// All five solve at two million, taking between 150 and 550 milliseconds. That
/// is the cost of this number: one hitch of up to half a second when hints are
/// first turned on, on one board in forty, against those boards getting no
/// hints at all.
///
/// Uniformly random boards reach ten million, which is still the case this
/// refuses.
pub const BUDGET: usize = 2_000_000;

/// How many moves level three shows.
pub const FEW: usize = 5;

const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

/// What the search came back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The fewest slides that finish the board, and the slides themselves.
    Exact(Vec<Direction>),
    /// The search ran out of budget. A true lower bound, and said as one.
    Floor(usize),
}

/// How much help is on. Each level shows everything the one before it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Level {
    #[default]
    None,
    Count,
    NextTile,
    NextFew,
    Watch,
}

impl Level {
    /// The next one up, stopping at the top rather than wrapping. A hint
    /// cannot be un-seen, so there is no way back down within a puzzle.
    pub fn up(self) -> Level {
        match self {
            Level::None => Level::Count,
            Level::Count => Level::NextTile,
            Level::NextTile => Level::NextFew,
            Level::NextFew | Level::Watch => Level::Watch,
        }
    }

    pub fn shows_moves(self) -> bool {
        self >= Level::NextTile
    }
}

fn step(direction: Direction) -> (i32, i32) {
    match direction {
        Direction::Up => (-1, 0),
        Direction::Down => (1, 0),
        Direction::Left => (0, -1),
        Direction::Right => (0, 1),
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

fn cells_of(board: &Board) -> [u8; CELLS] {
    std::array::from_fn(|cell| board.cell(cell))
}

/// Where a tile belongs, as a row and column.
fn home(tile: u8) -> (usize, usize) {
    let index = tile as usize - 1;
    (index / WIDTH, index % WIDTH)
}

/// Manhattan distance plus linear conflict: a true lower bound on the slides
/// any board needs, and cheap enough to be free.
///
/// Two tiles in their home row, both needing to pass through each other to get
/// where they belong, cost two moves beyond their distances, because one has to
/// leave the row and come back.
pub fn floor_of(board: &Board) -> usize {
    let cells = cells_of(board);
    let mut total = 0usize;

    for (index, tile) in cells.iter().enumerate() {
        if *tile == GAP {
            continue;
        }
        let (row, column) = (index / WIDTH, index % WIDTH);
        let (hr, hc) = home(*tile);
        total += row.abs_diff(hr) + column.abs_diff(hc);
    }

    for line in 0..WIDTH {
        let row: Vec<u8> = (0..WIDTH)
            .map(|c| cells[line * WIDTH + c])
            .filter(|t| *t != GAP && home(*t).0 == line)
            .collect();
        total += conflicts(&row, |tile| home(tile).1);

        let column: Vec<u8> = (0..WIDTH)
            .map(|r| cells[r * WIDTH + line])
            .filter(|t| *t != GAP && home(*t).1 == line)
            .collect();
        total += conflicts(&column, |tile| home(tile).0);
    }

    total
}

fn conflicts(line: &[u8], along: impl Fn(u8) -> usize) -> usize {
    let mut extra = 0;
    for i in 0..line.len() {
        for j in i + 1..line.len() {
            if along(line[i]) > along(line[j]) {
                extra += 2;
            }
        }
    }
    extra
}

fn is_done(cells: &[u8; CELLS]) -> bool {
    cells.iter().enumerate().all(|(index, tile)| {
        *tile
            == if index == CELLS - 1 {
                GAP
            } else {
                index as u8 + 1
            }
    })
}

enum Found {
    Solved,
    Deeper(usize),
    Spent,
}

/// The fewest slides that finish this board, within a budget.
///
/// Lifts are never considered: they are a resource the player spends on
/// purpose, and a search allowed to use them would answer every board at once.
pub fn solve(board: &Board, budget: usize) -> Answer {
    let mut cells = cells_of(board);
    if is_done(&cells) {
        return Answer::Exact(Vec::new());
    }

    let mut bound = floor_of(board);
    let mut nodes = 0usize;
    let mut path: Vec<Direction> = Vec::new();

    loop {
        match search(
            &mut cells,
            board.gap(),
            0,
            bound,
            None,
            &mut path,
            &mut nodes,
            budget,
        ) {
            Found::Solved => return Answer::Exact(path),
            Found::Spent => return Answer::Floor(floor_of(board)),
            Found::Deeper(next) => {
                path.clear();
                bound = next;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    cells: &mut [u8; CELLS],
    gap: usize,
    g: usize,
    bound: usize,
    came_from: Option<Direction>,
    path: &mut Vec<Direction>,
    nodes: &mut usize,
    budget: usize,
) -> Found {
    let estimate = g + floor_from(cells);
    if estimate > bound {
        return Found::Deeper(estimate);
    }
    if is_done(cells) {
        return Found::Solved;
    }

    *nodes += 1;
    if *nodes > budget {
        return Found::Spent;
    }

    let mut shallowest = usize::MAX;
    let (row, column) = ((gap / WIDTH) as i32, (gap % WIDTH) as i32);

    for direction in DIRECTIONS {
        if came_from == Some(opposite(direction)) {
            continue;
        }
        let (dr, dc) = step(direction);
        let (nr, nc) = (row + dr, column + dc);
        if !(0..WIDTH as i32).contains(&nr) || !(0..WIDTH as i32).contains(&nc) {
            continue;
        }

        let moved = nr as usize * WIDTH + nc as usize;
        cells.swap(gap, moved);
        path.push(direction);

        match search(
            cells,
            moved,
            g + 1,
            bound,
            Some(direction),
            path,
            nodes,
            budget,
        ) {
            Found::Solved => return Found::Solved,
            Found::Spent => return Found::Spent,
            Found::Deeper(next) => shallowest = shallowest.min(next),
        }

        path.pop();
        cells.swap(gap, moved);
    }

    Found::Deeper(shallowest)
}

/// The same bound as `floor_of`, against a raw arrangement.
fn floor_from(cells: &[u8; CELLS]) -> usize {
    let mut total = 0usize;
    for (index, tile) in cells.iter().enumerate() {
        if *tile == GAP {
            continue;
        }
        let (row, column) = (index / WIDTH, index % WIDTH);
        let (hr, hc) = home(*tile);
        total += row.abs_diff(hr) + column.abs_diff(hc);
    }
    for line in 0..WIDTH {
        let row: Vec<u8> = (0..WIDTH)
            .map(|c| cells[line * WIDTH + c])
            .filter(|t| *t != GAP && home(*t).0 == line)
            .collect();
        total += conflicts(&row, |tile| home(tile).1);

        let column: Vec<u8> = (0..WIDTH)
            .map(|r| cells[r * WIDTH + line])
            .filter(|t| *t != GAP && home(*t).1 == line)
            .collect();
        total += conflicts(&column, |tile| home(tile).0);
    }
    total
}

/// How much is being shown, and the plan behind it.
#[derive(Debug, Clone, Default)]
pub struct Hints {
    level: Level,
    /// What is left of the solution, if one has been found and is still true.
    plan: Vec<Direction>,
    exact: bool,
    floor: usize,
    used: bool,
}

impl Hints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn used(&self) -> bool {
        self.used
    }

    /// What to say at level one: the count, and whether it is the answer or a
    /// floor under it.
    pub fn count(&self) -> (usize, bool) {
        if self.exact {
            (self.plan.len(), true)
        } else {
            (self.floor, false)
        }
    }

    /// H. Steps up, works out what it now needs to know, and never goes back.
    pub fn step_up(&mut self, board: &Board) {
        self.level = self.level.up();
        self.used = true;
        self.ensure(board);
    }

    /// Works out the plan if the level wants one and there is not one already.
    pub fn ensure(&mut self, board: &Board) {
        if self.level == Level::None {
            return;
        }
        if self.exact && !self.plan.is_empty() {
            return;
        }
        if self.exact && board.is_solved() {
            return;
        }

        match solve(board, BUDGET) {
            Answer::Exact(moves) => {
                self.plan = moves;
                self.exact = true;
                self.floor = self.plan.len();
            }
            Answer::Floor(n) => {
                self.plan.clear();
                self.exact = false;
                self.floor = n;
            }
        }
    }

    /// A slide happened. If it was the one the plan expected, the plan just
    /// moves on. If it was not, the plan is no longer about this board.
    pub fn slid(&mut self, direction: Direction) {
        if self.plan.first() == Some(&direction) {
            self.plan.remove(0);
        } else {
            self.forget();
        }
    }

    /// A lift rearranges the board in a way no plan survives.
    pub fn forget(&mut self) {
        self.plan.clear();
        self.exact = false;
        self.floor = 0;
    }

    /// The next move, when there is one to show.
    pub fn next_move(&self) -> Option<Direction> {
        if self.level.shows_moves() {
            self.plan.first().copied()
        } else {
            None
        }
    }

    /// The tiles that move next and the order they move in, at the cells they
    /// are sitting in now. A tile that moves twice keeps its first turn.
    pub fn marked(&self, board: &Board) -> Vec<(usize, usize)> {
        if !self.level.shows_moves() || self.plan.is_empty() {
            return Vec::new();
        }

        let showing = match self.level {
            Level::NextTile => 1,
            _ => FEW,
        };

        let mut playing = *board;
        let mut order: Vec<(u8, usize)> = Vec::new();

        for (turn, direction) in self.plan.iter().take(showing).enumerate() {
            let gap = playing.gap();
            let (dr, dc) = step(*direction);
            let (row, column) = ((gap / WIDTH) as i32 + dr, (gap % WIDTH) as i32 + dc);
            if !(0..WIDTH as i32).contains(&row) || !(0..WIDTH as i32).contains(&column) {
                break;
            }

            let tile = playing.cell(row as usize * WIDTH + column as usize);
            if !order.iter().any(|(already, _)| *already == tile) {
                order.push((tile, turn));
            }
            playing.slide(*direction);
        }

        order
            .into_iter()
            .filter_map(|(tile, turn)| {
                (0..CELLS)
                    .find(|cell| board.cell(*cell) == tile)
                    .map(|cell| (cell, turn))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn scrambled(seed: u64, moves: usize) -> Board {
        Board::scrambled(&mut StdRng::seed_from_u64(seed), moves)
    }

    /// A board whose floor equals the number of moves that made it is a board
    /// whose optimum is exactly that, since the floor can never be above it.
    fn known_optimum(seed: u64) -> Option<(Board, usize)> {
        for moves in 1..=12 {
            let board = scrambled(seed, moves);
            let floor = floor_of(&board);
            if floor == moves {
                return Some((board, moves));
            }
        }
        None
    }

    #[test]
    fn the_solver_finds_a_known_optimum() {
        let mut checked = 0;
        for seed in 0..60 {
            let Some((board, optimum)) = known_optimum(seed) else {
                continue;
            };
            checked += 1;

            let answer = solve(&board, BUDGET);

            let Answer::Exact(moves) = answer else {
                panic!("seed {} was not solved inside the budget", seed)
            };
            assert_eq!(moves.len(), optimum, "seed {}", seed);
        }
        assert!(checked > 5, "only {} boards had a known optimum", checked);
    }

    #[test]
    fn the_solution_finishes_the_board() {
        for seed in 0..25 {
            let mut board = scrambled(seed, 140);
            let Answer::Exact(moves) = solve(&board, BUDGET) else {
                panic!("seed {} was not solved inside the budget", seed);
            };

            for direction in moves {
                assert!(board.slide(direction), "seed {} made an illegal move", seed);
            }

            assert!(board.is_solved(), "seed {}", seed);
        }
    }

    #[test]
    fn the_solution_is_as_long_as_it_says() {
        let board = scrambled(3, 140);
        let mut hints = Hints::new();
        hints.step_up(&board);
        hints.step_up(&board);

        let (count, exact) = hints.count();

        assert!(exact);
        assert_eq!(count, hints.plan.len(), "the count is the plan's length");
    }

    #[test]
    fn every_move_is_a_slide() {
        // a move the board refuses is not a move, and the solver only has four
        let mut board = scrambled(9, 140);
        let Answer::Exact(moves) = solve(&board, BUDGET) else {
            panic!("not solved");
        };

        for direction in moves {
            assert!(board.slide(direction));
        }
    }

    #[test]
    fn a_solved_board_is_already_done() {
        assert_eq!(solve(&Board::solved(), BUDGET), Answer::Exact(Vec::new()));
    }

    #[test]
    fn the_floor_is_never_above_the_answer() {
        // the whole point of a floor. Being told "at least 30" and finishing in
        // 26 would be the game lying to a player who trusted it.
        for seed in 0..40 {
            let board = scrambled(seed, 140);
            let floor = floor_of(&board);
            let Answer::Exact(moves) = solve(&board, BUDGET) else {
                continue;
            };

            assert!(
                floor <= moves.len(),
                "seed {}: floor {} above optimum {}",
                seed,
                floor,
                moves.len()
            );
        }
    }

    #[test]
    fn a_small_budget_gives_a_floor() {
        let board = scrambled(5, 140);

        assert!(
            matches!(solve(&board, 1), Answer::Floor(_)),
            "a budget of one node cannot solve it"
        );
    }

    #[test]
    fn the_floor_it_gives_is_honest() {
        for seed in 0..25 {
            let board = scrambled(seed, 140);
            let Answer::Floor(given) = solve(&board, 1) else {
                panic!("a budget of one node should not solve anything");
            };
            let Answer::Exact(moves) = solve(&board, BUDGET) else {
                continue;
            };

            assert!(
                given <= moves.len(),
                "seed {}: said at least {}, answer is {}",
                seed,
                given,
                moves.len()
            );
        }
    }

    #[test]
    fn the_solver_only_slides() {
        // every move it returns is one the board accepts as a slide, and a
        // board it has finished is one sliding alone finished
        let mut board = scrambled(12, 140);
        let before = board;
        let Answer::Exact(moves) = solve(&board, BUDGET) else {
            panic!("not solved");
        };

        for direction in moves {
            board.slide(direction);
        }

        assert!(board.is_solved());
        assert!(
            before.is_solvable(),
            "and it was reachable by sliding all along"
        );
    }

    #[test]
    fn h_steps_up_one_level() {
        let board = scrambled(2, 140);
        let mut hints = Hints::new();
        assert_eq!(hints.level(), Level::None);

        hints.step_up(&board);
        assert_eq!(hints.level(), Level::Count);
        hints.step_up(&board);
        assert_eq!(hints.level(), Level::NextTile);
        hints.step_up(&board);
        assert_eq!(hints.level(), Level::NextFew);
        hints.step_up(&board);
        assert_eq!(hints.level(), Level::Watch);
    }

    #[test]
    fn h_stops_at_the_last_level() {
        let board = scrambled(2, 140);
        let mut hints = Hints::new();
        for _ in 0..8 {
            hints.step_up(&board);
        }

        assert_eq!(hints.level(), Level::Watch, "rather than wrapping round");
    }

    #[test]
    fn a_level_does_not_go_back_down() {
        let board = scrambled(2, 140);
        let mut hints = Hints::new();
        let mut seen = Level::None;

        for _ in 0..6 {
            hints.step_up(&board);
            assert!(hints.level() >= seen);
            seen = hints.level();
        }
    }

    #[test]
    fn following_the_plan_advances_it() {
        let board = scrambled(4, 140);
        let mut hints = Hints::new();
        hints.step_up(&board);
        hints.step_up(&board);
        let before = hints.plan.len();
        let next = hints.plan[0];

        hints.slid(next);

        assert_eq!(hints.plan.len(), before - 1, "one shorter, not recomputed");
    }

    #[test]
    fn a_move_off_the_plan_forgets_it() {
        let board = scrambled(6, 140);
        let mut hints = Hints::new();
        hints.step_up(&board);
        hints.step_up(&board);
        let next = hints.plan[0];
        let other = DIRECTIONS
            .iter()
            .find(|d| **d != next)
            .copied()
            .expect("there is another direction");

        hints.slid(other);

        assert!(hints.plan.is_empty());
    }

    #[test]
    fn a_lift_forgets_the_plan() {
        let board = scrambled(7, 140);
        let mut hints = Hints::new();
        hints.step_up(&board);
        hints.step_up(&board);
        assert!(!hints.plan.is_empty());

        hints.forget();

        assert!(hints.plan.is_empty());
    }

    #[test]
    fn the_marked_tiles_are_the_ones_about_to_move() {
        let board = scrambled(8, 140);
        let mut hints = Hints::new();
        for _ in 0..3 {
            hints.step_up(&board);
        }
        assert_eq!(hints.level(), Level::NextFew);

        let marked = hints.marked(&board);

        assert!(!marked.is_empty());
        assert!(marked.len() <= FEW);
        for (cell, _) in &marked {
            assert_ne!(board.cell(*cell), GAP, "the gap does not move, tiles do");
        }
    }

    #[test]
    fn level_one_marks_nothing() {
        let board = scrambled(8, 140);
        let mut hints = Hints::new();
        hints.step_up(&board);

        assert_eq!(hints.level(), Level::Count);
        assert!(hints.marked(&board).is_empty(), "a count is not a finger");
    }
}
