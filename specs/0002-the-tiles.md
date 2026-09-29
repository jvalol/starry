# 0002 The tiles

**Status:** draft
**Date:** 2026-09-28

## Goal

The board from spec 0001 as something you can see: fifteen slabs in a tray,
each carrying its slice of the painting, lit so the thickness shows.

## Behavior

**A tile is a slab, and every tile is its own mesh.** The engine bakes uvs into
a mesh's vertices, so tiles cannot share one: fifteen slices means fifteen
meshes. Instancing, which spec 0010 offers for the same mesh many times over,
buys nothing here for the same reason.

**One texture, not fifteen.** Each mesh's top face maps the rectangle of the
painting belonging to its cell. Fifteen meshes against one texture is fifteen
draws and fifteen meshes against fifteen textures is also fifteen, so the draws
are not the argument. One image loaded once is.

**The slices are inset by half a texel.** Sampling is linear, per spec 0011, so
a fragment at the very edge of a slice draws part of its value from texels
across the boundary, which belong to the tile next door. On a tile with a hard
silhouette against the gap beside it, that shows as a fringe of the neighbour.
Pulling each rectangle in by half a texel costs half a row of the painting,
which nobody will ever see, and removes it.

This is spec 0024's hazard arriving in the first thing that could have hit it.
That spec left the inset to the game on the grounds that a rectangle quietly
becoming a different rectangle is worse to debug, and this is the game doing it
where it can be seen.

Giving each tile its own texture would be the other way out and is worse here.
The sampler wraps rather than clamps, so a fragment past the edge of a tile's
own image comes back with the opposite edge, which is a more obvious artifact
than a neighbour's pixel.

**The bevel carries the tile's own edge.** The side faces take the uv of the
top face's edge along their length, so the outermost row of the slice smears
down the side. A tile then reads as printed through rather than as a picture
laid on a block. Continuing the painting past the tile's edge would make a
solved board look more continuous and is wrong the rest of the time, since a
tile in the wrong place would wear its neighbour's colours on its sides.

**Thickness is a ratio, tuned by eye.** Near one to eight against the tile's
width is where to start. The test is whether the shadow a tile drops into the
gap beside it looks like a shadow under something solid, which is a thing to
judge in front of the running game.

**A tray under the board**, wider than the tiles and dark enough that the gap
reads as a hole rather than a missing square. The gap is where the tray shows.

**One directional light, above and to a side, casting.** Spec 0015's shadows
are what make thickness visible: a slab lit from straight on is a rectangle.
The tray takes the tiles' shadows and the gap beside a tile is where they land.

**Where a cell is in the world** is its row and column times the tile's width
plus the gap between tiles, centred on the origin. That mapping is arithmetic
and it is tested, because everything drawn depends on it agreeing with spec
0001's cell numbering, and an off-by-one there is a transposed board that still
plays correctly.

**The board moves instantly and the view catches up.** Spec 0001's slide takes
no time: the tile is in its new cell the moment the key is pressed. What slides
is the drawing, over a short span, from where the tile was to where it now is.
A lift does the same in three parts, up, across and down, with its shadow
drawing in underneath it as it rises.

Nothing the view does can change the board. That separation is what keeps the
rules testable without a window, and it is worth a test of its own rather than
a convention someone later breaks by reaching back the other way.

**The painting ships with the game**, the first image to do so: the other five
carry a sound or build what they draw. It is at
`res/textures/starry-night.jpg`, 1024 square, with what is known about its
licence in the file beside it the way `res/sounds/_readme_and_license.txt`
already does for the blip.

It is already square, cropped from a 5:4 painting at a cost of 200 pixels from
each side. Tiles are square and a painting is not, so something had to give, and
which side of the canvas is worth losing is a question about that painting
rather than a rule this spec can state.

1024 gives each tile 256 pixels, against the roughly 150 a tile will occupy on
screen. There is room to lose some to the half texel inset and to a mip level
and still have more than the screen asks for.

**The picture is chosen for its tiles, not only for itself.** The Mona Lisa was
the first choice and the wrong one: a quarter of its tiles would have been plain
sky and another quarter dark dress, and tiles that look like each other make a
sliding puzzle tedious rather than hard. Starry Night has distinctive paint in
all sixteen, and its brushwork runs in a direction, which tells a player not
just which tile they are holding but which way up it goes. Any picture that
replaces it has to clear the same bar, and the check is to put a four by four
grid over it and look at the sixteen squares on their own.

## Acceptance criteria

- There is one mesh per tile. — `tiles::tests::fifteen_tiles_are_fifteen_meshes`
- The slices in order cover the painting exactly once. — `tiles::tests::the_slices_tile_the_image`
- No two tiles claim the same slice. — `tiles::tests::no_two_tiles_share_a_slice`
- Every slice is inset, and by the same amount. — `tiles::tests::every_slice_is_inset`
- The inset is half a texel of the image it was cut from. — `tiles::tests::the_inset_is_half_a_texel`
- A tile's sides take their uv from its own edge. — `tiles::tests::the_bevel_comes_from_the_tile_s_own_edge`
- A tile's box is its width by its width by its thickness. — `tiles::tests::a_tile_is_as_thick_as_it_says`
- The painting is square, which square tiles need. — `tiles::tests::the_painting_is_square`
- The top face points up. — `tiles::tests::the_top_face_points_up`
- Cell to world agrees with spec 0001's numbering, corner for corner. — `tiles::tests::the_corners_land_where_the_board_says`
- Cells a row apart are a tile and a gap apart in the world. — `tiles::tests::neighbouring_cells_are_one_step_apart`
- The board is centred on the origin. — `tiles::tests::the_board_is_centred`
- An animation in flight leaves the board alone. — `tiles::tests::animating_never_touches_the_board`
- An animation ends where the board already says the tile is. — `tiles::tests::a_slide_ends_where_the_tile_already_is`

### Verified by hand

- Tiles look like objects rather than a picture cut into squares. The shadow in
  the gap beside a tile is the thing doing that work, so if they look flat, the
  light is the first place to go.
- The bevel shows the tile's own edge continuing down it rather than the
  neighbouring tile's colours.
- Solved, the fifteen slices and the tray make one picture, with no fringe of
  the wrong tile along any edge. That is the half texel inset, and it is the
  check that says whether it was enough.
- A lift rises out of the tray with its shadow drawing in underneath it.
- Whether the camera wants an orthographic projection. Perspective is all the
  engine has, per spec 0008, and a board this small seen from far enough back
  with a narrow field of view may be indistinguishable from flat. A little
  splay may even help the tiles read as solid. This is the check that settles
  whether blitzkit 0008 needs amending, and the answer belongs back in this
  spec either way.

## Out of scope

A chamfer on the tile's edge, which would catch the light better than a square
one and is the next thing to try if the bevel reads poorly. Letting a player
choose the picture, which is a different game. Clicking a tile, which wants
blitzkit spec 0025. A camera the
player can move: the board is the whole scene and there is nothing to look at
from another angle.
