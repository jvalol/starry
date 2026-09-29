# 0001 The board

**Status:** draft
**Date:** 2026-09-28

## Goal

A sliding tile puzzle of the Mona Lisa, in tiles thick enough to have a shadow,
with something to decide on most turns.

## Behavior

**Four by four, fifteen tiles and a gap.** Each tile carries its own slice of
the painting on its top face. The slices tile the image exactly: no overlap, no
gap, and every pixel of the source on exactly one tile.

**A tile is a slab, not a square.** It has thickness, and the edge row of its
slice is extruded down the bevel, so a tile looks printed rather than cut out of
a poster. How thick is tuned by eye against the tile's width rather than fixed
here. Somewhere near one to eight is the starting point, and the test is whether
the shadow in the gap beside a tile looks like a shadow under something solid.

The thickness is the whole reason this is in 3D. Flat, it would be a picture of
a puzzle.

**Arrows move the gap.** Pressing a direction slides the neighbouring tile into
the gap, which is the same thing said from the tile's side. A direction with no
tile that way does nothing. A tile slides rather than jumps, because the slide
is where the thickness reads.

**A lift takes a tile out and puts it in the gap.** From anywhere on the board,
not just beside it. There are three in a puzzle and they do not come back. This
is the decision the puzzle otherwise lacks: a tile stranded on the wrong side
costs a dozen careful slides or one of your three lifts, and the answer changes
as the lifts run out.

**Every generated board is solvable.** Half of all arrangements are not
reachable by sliding, so the board is scrambled by making legal moves from
solved rather than by shuffling tiles into places. A generated board is also
never already solved.

**A lift can stand the player in a dead end, and is warned first.** A lift is a
slide over a distance. A slide moves the gap one cell and always leaves the
board solvable, and a lift leaves it solvable exactly when the gap and the
lifted tile are an odd number of cells apart, counted along the rows and columns
rather than straight across. An even distance strands the player.

Colour the board like a chessboard and the rule is that a lift has to change the
gap's colour. Which it does is not something a player will read off the board,
so the game says so before committing the move rather than after.

A player who takes it anyway has a board no amount of sliding will finish. With
lifts left, another lift repairs it, and that is their call to make. With none
left, the game offers a restart, because the alternative is a board that cannot
be finished and does not say so.

**The warning is two cell positions, not a search and not even a board.**
Whether a lift strands the player depends only on where the gap is and which
tile is lifted. It does not depend on the arrangement at all, which was checked
against every one of the 240 position pairs on a four wide board.

Solvability itself, which the generator needs, is also a parity check rather
than a search: for a four wide board an arrangement is reachable when its
inversion count plus the gap's row counted from the bottom is odd. The solved
board has no inversions and its gap on the first row from the bottom, which is
odd and therefore reachable, as it had better be.

**Par is the sum of every tile's distance from home**, counted along the rows
and columns rather than straight across. It is a true lower bound on the slides
needed, so it is an honest number to be measured against rather than an invented
one. A solved board is par zero.

A move is a move whether it slides or lifts. What separates them is that there
are only three lifts, which is the thing being spent.

## Acceptance criteria

- The slices cover the painting exactly once. — `board::tests::the_slices_tile_the_image`
- A generated board is solvable. — `board::tests::every_generated_board_is_solvable`
- A generated board is not already solved. — `board::tests::a_generated_board_is_not_solved`
- The solved board is solvable, which the parity rule had better agree with. — `board::tests::the_solved_board_is_reachable`
- Sliding never changes whether a board is solvable. — `board::tests::sliding_keeps_a_board_solvable`
- A direction with no tile that way does nothing. — `board::tests::a_move_off_the_edge_does_nothing`
- A lift an even number of cells away strands the player. — `board::tests::an_even_lift_breaks_it`
- A lift an odd number of cells away does not. — `board::tests::an_odd_lift_keeps_it`
- A slide is the one cell case of the same rule. — `board::tests::a_slide_is_a_lift_of_one`
- Two stranding lifts undo each other. — `board::tests::a_second_lift_repairs_the_first`
- The warning depends on the two cells and not on the arrangement. — `board::tests::the_warning_reads_only_the_positions`
- Three lifts, and the fourth is refused. — `board::tests::there_are_only_three_lifts`
- Par is the distance along rows and columns. — `board::tests::par_is_the_distance_home`
- A solved board is par zero. — `board::tests::a_solved_board_is_par_zero`
- Solved is the tiles in order with the gap last. — `board::tests::solved_is_in_order_with_the_gap_last`

The generated-board one runs over many seeds rather than one. A scramble that
produces an unsolvable board does it rarely and looks like a player's mistake
when it happens, which is the worst way for a bug to present.

The lift rule was wrong in this spec's first draft, which said row distance
decided it. Row distance predicts 112 of the 240 position pairs, which is worse
than a coin. It was caught by computing all 240 rather than by reasoning about
it, and the tests above are that computation kept.

### Verified by hand

- Tiles look like objects rather than a picture cut into squares. The shadow in
  the gap beside a tile is what does it.
- The bevel shows the tile's own edge continuing down it, not the neighbouring
  tile's pixels.
- A lift rises out of the tray with its shadow drawing in underneath it.
- Solved, the sixteen slices make one picture with no visible seams between them.

## Out of scope

Anything other than four by four. Choosing a picture, or any picture other than
the one. A timer. Saving a puzzle to finish later. Clicking a tile rather than
using the arrows, which wants blitzkit spec 0025 and can come after this works.
