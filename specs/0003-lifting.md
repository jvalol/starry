# 0003 Lifting

**Status:** implemented
**Date:** 2026-09-28

## Goal

The part of spec 0001 that makes this more than a fifteen puzzle: three lifts, a
way to aim them with the arrows, and a warning before one that cannot be undone.

## Behavior

Spec 0001 gave the board a lift and never said how a player points at a tile.
The arrows move the gap, so they cannot also move a choice, and the answer is a
mode rather than a second set of keys.

**L goes into lift mode and L comes back out.** While it is on, the arrows move
a cursor over the tiles instead of sliding anything, and nothing about the board
changes until a lift is confirmed. Leaving spends nothing.

**Escape cancels the mode rather than quitting the game.** This is the tessera
pause bug in a new costume: a key whose meaning is defined in one state and
forgotten in another. Escape in lift mode means "not this", and a second escape,
once the mode is off, means what it always did.

**The cursor starts in reading order**, on the first cell that is not the gap,
every time the mode is entered. It remembers nothing between one lift and the
next, because a cursor that reappears somewhere unexpected is worse than one
that always starts in the corner.

**The cursor never rests on the gap.** An arrow that would land it there carries
it one further in the same direction, and if that leaves the board it does not
move at all. The gap is the one cell a lift cannot take, so a cursor sitting on
it would be a state with nothing to do. It does not wrap around the edges,
which is how the arrows already behave when they are sliding.

**Enter lifts the tile under the cursor into the gap**, spends one of the three,
and turns the mode off. The tile takes the arc spec 0002 draws, up and across
and down, which is the only time a tile leaves the tray.

**A lift that strands the player is marked before it is chosen, and warned about
when it is.** Spec 0001 has the rule: a lift leaves the board finishable exactly
when the gap and the tile are an odd number of cells apart. The tiles that fail
that are dimmed for as long as lift mode is on, so the answer is visible before
the cursor ever reaches one. Pressing Enter on a dimmed tile does not lift it.
It says what will happen, and a second Enter does it anyway.

Two prompts rather than one, because the marking alone is easy to miss and the
warning alone arrives after the player has committed to the idea. Neither is
worth much without the other.

**Three lifts, and the mode will not open without one.** Pressing L with none
left says so rather than opening a mode whose only outcome is refusal.

**A board that cannot be finished offers to take the lift back.** Having spent
the last lift badly, a player has a board no amount of sliding will solve, which
spec 0001 says is theirs to have done. R puts the board back to the moment
before that lift. The offer is only made when the board is actually unfinishable
and there is no lift left to repair it, since that is the only moment it is the
truth rather than a suggestion to give up.

**The lift stays spent.** The board comes back; the lift does not. A player who
takes a warned lift and rewinds it is playing on from a finishable board with
one fewer lift than they had, which is the cost of having tried it. Handing the
lift back as well would leave the warning with nothing behind it, since any lift
could then be attempted and taken back, and a warning about a move you can
always undo is decoration.

This is one step out of a dead end rather than an undo. It reaches exactly one
move back, it is only there when there is no other way on, and nothing else in
the game can be taken back.

## Acceptance criteria

- L opens the mode when there are lifts left. — `lifting::tests::l_opens_the_mode`
- L with no lifts left does not open it. — `lifting::tests::no_lifts_left_means_no_mode`
- L again closes it and spends nothing. — `lifting::tests::l_closes_it_again`
- Escape closes the mode rather than quitting. — `starry_game::tests::escape_cancels_the_mode`
- Escape with the mode off still quits. — `starry_game::tests::escape_outside_the_mode_still_quits`
- The arrows move the cursor and not the board. — `lifting::tests::the_arrows_move_the_cursor_not_the_board`
- And the game stops sliding while the mode is open. — `starry_game::tests::the_arrows_stop_sliding_while_the_mode_is_open`
- The cursor starts on the first cell that is not the gap. — `lifting::tests::the_cursor_starts_in_reading_order`
- The cursor steps over the gap rather than onto it. — `lifting::tests::the_cursor_steps_over_the_gap`
- The cursor stays put rather than wrapping. — `lifting::tests::the_cursor_does_not_wrap`
- Enter on a safe tile lifts it and spends one. — `lifting::tests::enter_lifts_and_spends_one`
- Enter on a stranding tile warns and does not lift. — `lifting::tests::a_stranding_lift_warns_first`
- Enter again lifts it anyway. — `lifting::tests::a_warned_lift_goes_through_on_the_second_press`
- Moving the cursor takes the warning back. — `lifting::tests::moving_the_cursor_forgets_the_warning`
- Which tiles are marked is spec 0001's rule and nothing else. — `lifting::tests::the_marked_tiles_are_the_stranding_ones`
- The way back is offered only when the board is stuck and no lift is left. — `lifting::tests::a_rewind_is_offered_only_when_it_is_true`
- It puts the board back exactly as it was before the lift. — `lifting::tests::a_rewind_puts_the_board_back`
- It does not give the lift back. — `lifting::tests::a_rewind_does_not_return_the_lift`
- It reaches one lift back and no further. — `lifting::tests::a_rewind_reaches_one_move`
- Slides cannot be taken back. — `lifting::tests::a_slide_cannot_be_rewound`

The two escape ones are not padding. A key that means one thing in one mode and
is forgotten in another is exactly how pause ended up swallowing escape in
tessera, pong and snake on the same morning this game was started.

### Verified by hand

- The cursor is visible against the painting, which is busy everywhere. If it
  cannot be seen over the cypress it cannot be seen.
- The dimmed tiles read as unavailable rather than as shadowed, on a board whose
  tiles already have shadows on them.
- A lift arcs up out of the tray and sets back down, and the tile passing over
  the others is legible rather than confusing.
- Taking a warned lift, spending the last one badly, and being offered the way
  back feels like a consequence rather than a trap. The lift is gone and the
  puzzle is not, which is the balance this is trying to strike and the one thing
  here that only playing it can settle.

## Out of scope

Clicking a tile to lift it, which wants blitzkit spec 0025. Undo in general:
the rewind above is one move, only from a dead end, and no slide is ever taken
back. Any limit other than three, which spec 0001 fixes. A second warning for a
player who has already stranded the board and is about to do it again, since by
then the board is already unfinishable and another lift is the repair rather
than the damage.
