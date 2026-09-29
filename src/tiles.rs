//! Slabs, the slices of the painting they carry, and where a cell sits in the
//! world. See `specs/0002-the-tiles.md`.
//!
//! The board in `board.rs` knows nothing about any of this, and nothing here
//! reaches back into it. A tile's slice follows its number, not the cell it is
//! sitting in, which is what makes a misplaced tile look misplaced.

use blitzkit::mesh::{MeshData, Vertex};
use glam::{vec2, vec3, Vec2, Vec3};

use crate::board::WIDTH;

/// A tile's width, and the unit everything else here is measured against.
pub const TILE: f32 = 1.0;

/// How thick a tile is. Near one to eight against its width, per spec 0002,
/// and the thing to move first if the shadow in the gap looks wrong.
pub const THICKNESS: f32 = TILE / 8.0;

/// The space between one tile and the next, which is where their shadows land.
pub const BETWEEN: f32 = 0.06;

/// Centre to centre.
pub const STEP: f32 = TILE + BETWEEN;

/// The painting, in texels along a side. It is square, which square tiles need.
pub const IMAGE: f32 = 1024.0;

/// Half a texel, in uv. Sampling is linear, so without this a fragment at the
/// edge of a slice takes part of its value from the tile next door. See spec
/// 0002 and blitzkit spec 0024.
pub const INSET: f32 = 0.5 / IMAGE;

/// The painting itself. The licence it arrives under is beside it.
pub const PAINTING: &[u8] = include_bytes!("../res/textures/starry-night.jpg");

/// The part of the painting a tile carries, in uv.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slice {
    pub min: Vec2,
    pub max: Vec2,
}

/// Where a tile's slice sits before the inset is taken off it. The fifteen of
/// these tile the painting exactly, which is the thing worth testing; the inset
/// is then the same on every side of every one.
pub fn slice_before_inset(tile: u8) -> Slice {
    let home = tile as usize - 1;
    let (row, column) = (home / WIDTH, home % WIDTH);
    let step = 1.0 / WIDTH as f32;

    Slice {
        min: vec2(column as f32 * step, row as f32 * step),
        max: vec2((column + 1) as f32 * step, (row + 1) as f32 * step),
    }
}

/// What a tile actually samples: its slice, pulled in by half a texel.
pub fn slice(tile: u8) -> Slice {
    let whole = slice_before_inset(tile);
    Slice {
        min: whole.min + Vec2::splat(INSET),
        max: whole.max - Vec2::splat(INSET),
    }
}

/// Where a cell is in the world, at the height a resting tile sits.
///
/// Row 0 is the top of the painting and lands at negative z, away from a camera
/// looking from positive z, so the picture reads the way it hangs.
pub fn cell_position(cell: usize) -> Vec3 {
    let (row, column) = (cell / WIDTH, cell % WIDTH);
    let offset = (WIDTH as f32 - 1.0) * 0.5;

    vec3(
        (column as f32 - offset) * STEP,
        0.0,
        (row as f32 - offset) * STEP,
    )
}

/// The slab a tile is drawn as, carrying its own slice.
///
/// The top face maps the slice across it. Each side takes the uv of the top
/// edge it runs along and holds it all the way down, so the outermost row of
/// the slice smears down the bevel and a tile reads as printed through.
pub fn tile_mesh(tile: u8) -> MeshData {
    let Slice { min, max } = slice(tile);
    let half = TILE * 0.5;
    let top = THICKNESS * 0.5;
    let bottom = -top;

    // the four corners of the top face, and the uv each one carries
    let corners = [
        (vec3(-half, top, -half), vec2(min.x, min.y)),
        (vec3(half, top, -half), vec2(max.x, min.y)),
        (vec3(half, top, half), vec2(max.x, max.y)),
        (vec3(-half, top, half), vec2(min.x, max.y)),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    let mut face = |quad: [(Vec3, Vec2); 4], normal: Vec3| {
        let first = vertices.len() as u32;
        for (position, uv) in quad {
            vertices.push(Vertex::new(
                position.to_array(),
                normal.to_array(),
                uv.to_array(),
            ));
        }
        indices.extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
    };

    // the top, which is the picture. Wound 3 2 1 0 so the cross product of its
    // first two edges comes out along +y, which is what stops it being culled
    // from above.
    face([corners[3], corners[2], corners[1], corners[0]], Vec3::Y);

    // the four sides. Each runs along one edge of the top face and drops to the
    // bottom carrying that edge's uv with it.
    for side in 0..4 {
        let (a, uv_a) = corners[side];
        let (b, uv_b) = corners[(side + 1) % 4];
        let outward = ((a + b) * 0.5).with_y(0.0).normalize();

        face(
            [
                (a, uv_a),
                (b, uv_b),
                (b.with_y(bottom), uv_b),
                (a.with_y(bottom), uv_a),
            ],
            outward,
        );
    }

    // the underside, which is never seen and still has to be there
    let middle = (min + max) * 0.5;
    face(
        [
            (corners[0].0.with_y(bottom), middle),
            (corners[1].0.with_y(bottom), middle),
            (corners[2].0.with_y(bottom), middle),
            (corners[3].0.with_y(bottom), middle),
        ],
        Vec3::NEG_Y,
    );

    MeshData::new(vertices, indices)
}

/// A tile on its way somewhere. The board has already moved it; this is the
/// drawing catching up. See spec 0002.
#[derive(Debug, Clone, Copy)]
pub struct Motion {
    pub tile: u8,
    from: Vec3,
    to: Vec3,
    /// How high it rises on the way. Zero for a slide, which stays down.
    lift: f32,
    elapsed: f32,
    seconds: f32,
}

impl Motion {
    pub const SLIDE_SECONDS: f32 = 0.11;
    #[allow(dead_code)]
    pub const LIFT_SECONDS: f32 = 0.38;
    /// How far a lifted tile rises, which is what shows its thickness.
    #[allow(dead_code)]
    pub const LIFT_HEIGHT: f32 = TILE * 0.55;

    pub fn slide(tile: u8, from: usize, to: usize) -> Self {
        Self {
            tile,
            from: cell_position(from),
            to: cell_position(to),
            lift: 0.0,
            elapsed: 0.0,
            seconds: Self::SLIDE_SECONDS,
        }
    }

    #[allow(dead_code)]
    pub fn lift(tile: u8, from: usize, to: usize) -> Self {
        Self {
            tile,
            from: cell_position(from),
            to: cell_position(to),
            lift: Self::LIFT_HEIGHT,
            elapsed: 0.0,
            seconds: Self::LIFT_SECONDS,
        }
    }

    pub fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.seconds);
    }

    pub fn done(&self) -> bool {
        self.elapsed >= self.seconds
    }

    /// Eased, so a tile leaves and arrives gently rather than starting at full
    /// speed, which is what makes a slide read as weight.
    fn t(&self) -> f32 {
        let t = (self.elapsed / self.seconds).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    pub fn position(&self) -> Vec3 {
        let t = self.t();
        // up and back down over the move, so a lift arches
        let height = self.lift * (t * std::f32::consts::PI).sin();

        self.from.lerp(self.to, t) + Vec3::Y * height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::CELLS;

    fn tiles() -> impl Iterator<Item = u8> {
        1..=(CELLS as u8 - 1)
    }

    #[test]
    fn fifteen_tiles_are_fifteen_meshes() {
        let meshes: Vec<MeshData> = tiles().map(tile_mesh).collect();

        assert_eq!(meshes.len(), 15);
        for mesh in &meshes {
            assert_eq!(mesh.triangle_count(), 12, "six faces of two triangles");
        }
    }

    #[test]
    fn the_painting_is_square() {
        // square tiles need one, and the crop that made it is recorded beside
        // the file
        let painting =
            blitzkit::texture::TextureData::from_bytes(PAINTING).expect("the painting decodes");

        assert_eq!(painting.width(), painting.height());
        assert_eq!(painting.width() as f32, IMAGE, "IMAGE follows the file");
    }

    #[test]
    fn the_slices_tile_the_image() {
        // every cell's rectangle, the gap's included, covers the painting once
        let mut covered = 0.0;
        for cell in 0..CELLS {
            let whole = slice_before_inset(cell as u8 + 1);
            covered += (whole.max.x - whole.min.x) * (whole.max.y - whole.min.y);
        }

        assert!((covered - 1.0).abs() < 1e-5, "covered {}", covered);
    }

    #[test]
    fn no_two_tiles_share_a_slice() {
        for a in tiles() {
            for b in tiles() {
                if a == b {
                    continue;
                }
                let (one, other) = (slice(a), slice(b));
                let apart = one.max.x <= other.min.x
                    || other.max.x <= one.min.x
                    || one.max.y <= other.min.y
                    || other.max.y <= one.min.y;

                assert!(apart, "{} and {} overlap", a, b);
            }
        }
    }

    #[test]
    fn every_slice_is_inset() {
        for tile in tiles() {
            let (whole, used) = (slice_before_inset(tile), slice(tile));

            assert!(used.min.x > whole.min.x, "tile {}", tile);
            assert!(used.min.y > whole.min.y, "tile {}", tile);
            assert!(used.max.x < whole.max.x, "tile {}", tile);
            assert!(used.max.y < whole.max.y, "tile {}", tile);
        }
    }

    #[test]
    fn the_inset_is_half_a_texel() {
        let whole = slice_before_inset(1);
        let used = slice(1);

        assert!((used.min.x - whole.min.x - 0.5 / IMAGE).abs() < 1e-9);
        assert!((whole.max.y - used.max.y - 0.5 / IMAGE).abs() < 1e-9);
    }

    #[test]
    fn the_bevel_comes_from_the_tile_s_own_edge() {
        // every uv on a tile lies inside that tile's own slice, so no side ever
        // shows the neighbour's paint
        for tile in tiles() {
            let used = slice(tile);
            for vertex in tile_mesh(tile).vertices.iter() {
                let [u, v] = vertex.uv;

                assert!(
                    u >= used.min.x - 1e-6 && u <= used.max.x + 1e-6,
                    "tile {} sampled u {}",
                    tile,
                    u
                );
                assert!(
                    v >= used.min.y - 1e-6 && v <= used.max.y + 1e-6,
                    "tile {} sampled v {}",
                    tile,
                    v
                );
            }
        }
    }

    #[test]
    fn a_tile_is_as_thick_as_it_says() {
        let bounds = tile_mesh(1).bounds();
        let size = bounds.size();

        assert!((size.x - TILE).abs() < 1e-6, "width {}", size.x);
        assert!((size.z - TILE).abs() < 1e-6, "depth {}", size.z);
        assert!((size.y - THICKNESS).abs() < 1e-6, "thickness {}", size.y);
    }

    #[test]
    fn the_top_face_points_up() {
        let mesh = tile_mesh(1);
        let top = THICKNESS * 0.5;
        let facing_up = mesh
            .vertices
            .iter()
            .filter(|v| (v.position[1] - top).abs() < 1e-6 && v.normal[1] > 0.5)
            .count();

        assert_eq!(facing_up, 4, "the four corners of the picture");
    }

    #[test]
    fn a_tile_s_faces_wind_with_their_normals() {
        // winding decides which side gets culled, and getting it backwards
        // shows as a tile you can see straight through
        let mesh = tile_mesh(1);
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let (pa, pb, pc) = (
                Vec3::from(a.position),
                Vec3::from(b.position),
                Vec3::from(c.position),
            );
            let wound = (pb - pa).cross(pc - pa).normalize();

            assert!(
                wound.dot(Vec3::from(a.normal)) > 0.9,
                "a face winds against its own normal"
            );
        }
    }

    #[test]
    fn the_board_is_centred() {
        let corners: Vec<Vec3> = [0, WIDTH - 1, CELLS - WIDTH, CELLS - 1]
            .iter()
            .map(|cell| cell_position(*cell))
            .collect();
        let middle: Vec3 = corners.iter().sum::<Vec3>() / 4.0;

        assert!(middle.length() < 1e-6, "middle at {:?}", middle);
    }

    #[test]
    fn neighbouring_cells_are_one_step_apart() {
        assert!((cell_position(1) - cell_position(0)).length() - STEP < 1e-6);
        assert!((cell_position(WIDTH) - cell_position(0)).length() - STEP < 1e-6);
    }

    #[test]
    fn the_corners_land_where_the_board_says() {
        // cell 0 is the top left of the painting, so it is left in x and away
        // in z, and cell 15 is the opposite corner
        let first = cell_position(0);
        let last = cell_position(CELLS - 1);

        assert!(first.x < 0.0 && first.z < 0.0, "cell 0 at {:?}", first);
        assert!(last.x > 0.0 && last.z > 0.0, "cell 15 at {:?}", last);
        assert!((first + last).length() < 1e-6, "opposite corners");
    }

    #[test]
    fn a_slide_ends_where_the_tile_already_is() {
        let mut motion = Motion::slide(7, 5, 9);
        motion.advance(Motion::SLIDE_SECONDS * 2.0);

        assert!(motion.done());
        assert!((motion.position() - cell_position(9)).length() < 1e-6);
    }

    #[test]
    fn a_lift_arches_and_comes_back_down() {
        let mut motion = Motion::lift(7, 0, 15);
        motion.advance(Motion::LIFT_SECONDS * 0.5);
        assert!(
            motion.position().y > Motion::LIFT_HEIGHT * 0.5,
            "up in the middle"
        );

        motion.advance(Motion::LIFT_SECONDS);
        assert!(motion.done());
        assert!((motion.position() - cell_position(15)).length() < 1e-6);
    }

    #[test]
    fn animating_never_touches_the_board() {
        // the rules are the board's and the drawing only ever reads them
        let mut board = crate::board::Board::solved();
        let before = board;
        board.slide(crate::board::Direction::Up);
        let after = board;

        let mut motion = Motion::slide(15, CELLS - 1, CELLS - 1 - WIDTH);
        for _ in 0..30 {
            motion.advance(0.01);
        }

        assert_ne!(before, after, "the board moved when it was told to");
        assert_eq!(board, after, "and not again while the drawing caught up");
    }
}
