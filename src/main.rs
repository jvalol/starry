//! A sliding tile puzzle of the Mona Lisa. See `specs/0001-the-board.md`.
//!
//! There is no window yet. What this prints is the rulebook working: a dealt
//! board, what it would cost at best, and which lifts from here would leave it
//! finishable. It is the thing to run while the rules are still moving.

mod board;

use board::{lift_is_safe, Board, Direction, CELLS, WIDTH};

fn show(board: &Board) {
    for row in 0..WIDTH {
        let line: Vec<String> = (0..WIDTH)
            .map(|column| match board.cell(row * WIDTH + column) {
                0 => "  .".to_string(),
                tile => format!("{:3}", tile),
            })
            .collect();
        println!("  {}", line.join(""));
    }
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut board = Board::scrambled(&mut rng, 120);

    println!("dealt:");
    show(&board);
    println!(
        "\n  par {} slides, finishable {}",
        board.par(),
        board.is_solvable()
    );

    board.slide(Direction::Up);
    board.slide(Direction::Left);
    println!("\nafter two slides, finishable {}", board.is_solvable());

    let gap = board.gap();
    let safe = (0..CELLS)
        .filter(|cell| *cell != gap && lift_is_safe(gap, *cell))
        .count();
    println!(
        "\nthe gap is at {}, and {} of the {} tiles could be lifted to it safely",
        gap,
        safe,
        CELLS - 1
    );

    if let Some(stranding) = (0..CELLS).find(|cell| *cell != gap && !lift_is_safe(gap, *cell)) {
        board.lift(stranding);
        println!(
            "lifting the one at {} instead leaves it finishable {}, which is the warning's whole job",
            stranding,
            board.is_solvable()
        );
    }
}
