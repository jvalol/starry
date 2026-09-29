# 0004 Hints

**Status:** implemented
**Date:** 2026-09-29

## Goal

A way out for a player who is stuck, given a step at a time so that taking the
first one does not give away the last.

## Behavior

**H steps up a level and never back down.** Four of them, each a superset of the
one before:

1. **How many.** The fewest slides that finish the board from here.
2. **Which tile.** The one tile that moves next, marked.
3. **The next five.** Marked in order, so the shape of the plan is visible
   without the whole of it.
4. **Watch.** The game plays the moves out, one slide at a time, until a key
   takes it back.

A level, once reached, stays for that puzzle. A player cannot un-see a hint and
the game does not pretend otherwise.

A new board would start at none, and there is no new board: spec 0003 turned
the restart into a rewind, so nothing in this game ever deals a second puzzle.
Said here rather than carried as code with no caller, and worth remembering if a
new board ever arrives.

**Hints count slides, never lifts.** A solver allowed to lift would answer every
board in one or two moves and the three lifts are a resource the player is
spending on purpose. The number at level one is the number of slides, and the
moves demonstrated at level four are slides.

**The number is the true minimum when it can be had, and an honest floor when
it cannot.** It is found by iterative deepening with Manhattan distance and
linear conflict, which is exact rather than an estimate.

The cost is real and it is worth writing down what was measured rather than
discovering it in front of a player. Sixteen boards from this game's own 140
move scramble needed a median of 1,295 search nodes and at most 24,808, which is
nothing. Uniformly random solvable boards are a different distribution: 45 to 57
moves optimal, up to 10.8 million nodes. The scramble does not produce those,
but a player sliding without a plan for long enough drifts toward them.

So the search has a node budget. Inside it the game says "23 slides". Outside
it the game says "at least 19" and means it, because Manhattan plus linear
conflict is a lower bound on every board and costs nothing to compute. A game
that freezes for a minute is worse than one that admits what it does not know.

**The plan is computed once and followed.** Level two onwards needs the moves,
not only the count, and recomputing after every slide would pay the search cost
over and over. The solution is kept, and a slide that matches its next move just
advances it. A slide that does not, and any lift at all, throws it away and asks
again.

**Watching is the game moving the board.** Each slide is the one spec 0002
draws, the same animation a player's own move gets, so watching looks like
playing rather than like the board rearranging itself. The moves stop for any
key. Escape stops the watching rather than quitting the game, which is the same
rule spec 0003 states for lift mode and the same mistake three other games made
in one morning.

**A hinted run says so.** The readout carries it once any hint has been taken.
Par is still the Manhattan floor it always was and the move count is still the
move count, and neither means the same thing once the game has been answering.
Marking it costs nothing and keeps the number honest.

## Acceptance criteria

- The solver finds the optimal length on a board whose answer is known. — `hints::tests::the_solver_finds_a_known_optimum`
- Following the solution finishes the board. — `hints::tests::the_solution_finishes_the_board`
- The solution is exactly as long as the count claims. — `hints::tests::the_solution_is_as_long_as_it_says`
- Every move in a solution is a legal slide. — `hints::tests::every_move_is_a_slide`
- A solved board needs no moves. — `hints::tests::a_solved_board_is_already_done`
- The floor never claims more than the true optimum. — `hints::tests::the_floor_is_never_above_the_answer`
- A budget too small gives a floor rather than a wrong number. — `hints::tests::a_small_budget_gives_a_floor`
- The floor it gives is a true lower bound. — `hints::tests::the_floor_it_gives_is_honest`
- The solver never lifts. — `hints::tests::the_solver_only_slides`
- H steps one level at a time. — `hints::tests::h_steps_up_one_level`
- H stops at the last level rather than wrapping. — `hints::tests::h_stops_at_the_last_level`
- Hints do not step back down within a puzzle. — `hints::tests::a_level_does_not_go_back_down`
- Following the plan advances it without searching again. — `hints::tests::following_the_plan_advances_it`
- A move off the plan throws it away. — `hints::tests::a_move_off_the_plan_forgets_it`
- A lift throws it away whatever it was. — `hints::tests::a_lift_forgets_the_plan`
- Watching stops on a key. — `starry_game::tests::watching_stops_on_a_key`
- Escape stops watching rather than quitting. — `starry_game::tests::escape_stops_watching`
- Taking a hint marks the run. — `starry_game::tests::a_hint_marks_the_run`
- Watching gets the board closer to solved. — `starry_game::tests::watching_plays_the_board_towards_solved`
- The marked tiles are ones that move, and no more than five. — `hints::tests::the_marked_tiles_are_the_ones_about_to_move`
- A count points at nothing. — `hints::tests::level_one_marks_nothing`

The floor ones are the load-bearing pair. A lower bound that is sometimes above
the true answer is worse than no number at all, because a player who is told
"at least 30" and finishes in 26 has been lied to by something they trusted.

### Verified by hand

- The count at level one is believable: solve a board by hand and it took at
  least that many.
- The marked tile at level two is one that can actually move, every time.
- Watching reads as the game playing rather than as the board teleporting.
- Stepping through all four levels on one board never goes backwards and never
  shows less than it did.

## Out of scope

An escape that the engine owns. This is the third spec in this game to state
what escape means in a mode, after 0003 and the three games that had to be
fixed for swallowing it, and a binding every game inherits is the shape the
answer probably takes. Noted here rather than specced, because nothing is
blocked on it.

Hints for lifting: which tile to lift, or whether to. Undoing a hint. Any level
between the five shown at three and the whole solution, since the whole solution
is level four and watching it is the same information. A faster solver, pattern
databases among them, unless the budget turns out to be hit in play rather than
in theory.
