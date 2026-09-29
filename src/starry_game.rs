//! The board from spec 0001 drawn as spec 0002 describes it: fifteen slabs in
//! a tray, lit so their thickness shows.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene, TextureId};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::texture::TextureData;
use blitzkit::Game;
use glam::{vec2, vec3, vec4, Vec3};
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::board::{Board, Direction, CELLS, GAP, WIDTH};
use crate::tiles::{self, Motion};

/// How far the board reaches, corner to corner, which is what the camera has to
/// take in.
const BOARD: f32 = tiles::STEP * WIDTH as f32;

/// The tray the tiles sit in. Wider than the board so it frames them, and dark
/// enough that the gap reads as a hole rather than a missing square.
const TRAY_MARGIN: f32 = tiles::TILE * 0.35;
const TRAY_DEPTH: f32 = tiles::THICKNESS * 2.5;
const TRAY_COLOR: glam::Vec4 = vec4(0.06, 0.07, 0.11, 1.0);

/// Paint is close to chalk. Shininess is the exponent on the highlight, so a
/// high one keeps it small: at the engine's default of 32 a flat tile facing
/// the light washes out to white across its whole face.
const TILE_SHININESS: f32 = 128.0;
const TRAY_SHININESS: f32 = 96.0;

pub struct StarryGame {
    board: Board,
    quitting: bool,

    /// One mesh per tile, because each carries its own slice.
    tiles: Vec<MeshId>,
    painting: Option<TextureId>,
    tray: Option<MeshId>,

    /// The drawing catching up with the board. Never the other way round.
    motion: Option<Motion>,

    help: RenderText,
    readout: RenderText,
}

impl StarryGame {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();

        Self {
            board: Board::scrambled(&mut rng, 140),
            quitting: false,
            tiles: Vec::new(),
            painting: None,
            tray: None,
            motion: None,
            help: RenderText {
                position: vec2(20.0, 20.0),
                color: vec4(1.0, 1.0, 1.0, 0.85),
                text: String::from("arrows slide, escape quits"),
                size: 20.0,
                ..Default::default()
            },
            readout: RenderText {
                position: vec2(20.0, 48.0),
                color: vec4(1.0, 1.0, 1.0, 0.7),
                size: 20.0,
                ..Default::default()
            },
        }
    }

    /// Where a tile is drawn: its cell, unless it is the one still arriving.
    fn drawn_at(&self, tile: u8, cell: usize) -> Vec3 {
        match self.motion {
            Some(motion) if motion.tile == tile => motion.position(),
            _ => tiles::cell_position(cell),
        }
    }

    fn press(&mut self, direction: Direction) {
        if self.motion.is_some() {
            return;
        }

        let gap = self.board.gap();
        if !self.board.slide(direction) {
            return;
        }

        // the board has already moved it; this is the drawing being told to
        // catch up, from where the gap was to where the gap now is
        let moved = self.board.cell(gap);
        self.motion = Some(Motion::slide(moved, self.board.gap(), gap));
    }
}

impl Default for StarryGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for StarryGame {
    fn load(&mut self, renderer: &mut Renderer) {
        for tile in 1..CELLS as u8 {
            let id = renderer.add_mesh(&tiles::tile_mesh(tile));
            self.tiles.push(id);
        }

        let painting = TextureData::from_bytes(tiles::PAINTING)
            .expect("the painting ships with the game and is a jpeg");
        self.painting = Some(renderer.add_texture(&painting));

        self.tray = Some(renderer.add_mesh(&MeshData::cube()));

        // the shadow map covers the board and a little around it, rather than
        // the default box, so the shadows in the gaps are the sharp part
        let reach = BOARD * 0.5 + TRAY_MARGIN;
        renderer.set_scene_bounds(blitzkit::collision::Aabb::new(
            vec3(-reach, -TRAY_DEPTH * 2.0, -reach),
            vec3(reach, tiles::TILE, reach),
        ));
    }

    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
        _window_size: (f32, f32),
    ) {
    }

    fn update(
        &mut self,
        dt: f32,
        _geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        if let Some(motion) = self.motion.as_mut() {
            motion.advance(dt);
            if motion.done() {
                self.motion = None;
            }
        }

        self.readout.text = if self.board.is_solved() {
            String::from("solved")
        } else {
            format!("par {}", self.board.par())
        };

        text_renderer.render_texts.push(self.help.clone());
        text_renderer.render_texts.push(self.readout.clone());
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        // above and to one side, far enough back that the splay is slight. Spec
        // 0002's open question is whether this wants an orthographic projection
        // instead, and this is the arrangement that answers it.
        camera.position = vec3(0.0, BOARD * 2.05, BOARD * 1.15);
        camera.target = Vec3::ZERO;
        camera.fov_y = 26f32.to_radians();

        // across the board rather than straight down, so a tile's own shadow
        // falls into the gap beside it
        scene.light.direction = vec3(-0.45, -1.0, -0.35).normalize();
        scene.light.ambient = Vec3::splat(0.22);

        let (painting, tray) = match (self.painting, self.tray) {
            (Some(painting), Some(tray)) => (painting, tray),
            _ => return,
        };

        let reach = BOARD + TRAY_MARGIN * 2.0;
        scene.push_material(
            tray,
            &Transform::at(vec3(0.0, -TRAY_DEPTH * 0.5 - tiles::THICKNESS * 0.5, 0.0))
                .with_scale(vec3(reach, TRAY_DEPTH, reach)),
            TRAY_COLOR,
            TRAY_SHININESS,
        );

        for cell in 0..CELLS {
            let tile = self.board.cell(cell);
            if tile == GAP {
                continue;
            }

            scene.push_textured(
                self.tiles[tile as usize - 1],
                painting,
                &Transform::at(self.drawn_at(tile, cell)),
                vec4(1.0, 1.0, 1.0, 1.0),
                TILE_SHININESS,
            );
        }
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        if input.state != KeyboardKeyState::Pressed || input.repeat {
            return;
        }

        match input.key {
            KeyboardKey::Escape => self.quitting = true,
            KeyboardKey::Up | KeyboardKey::W => self.press(Direction::Up),
            KeyboardKey::Down | KeyboardKey::S => self.press(Direction::Down),
            KeyboardKey::Left | KeyboardKey::A => self.press(Direction::Left),
            KeyboardKey::Right | KeyboardKey::D => self.press(Direction::Right),
            _ => {}
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_moves_the_board_and_starts_the_drawing_catching_up() {
        let mut game = StarryGame::new();
        game.board = Board::solved();
        let before = game.board;

        game.press(Direction::Up);

        assert_ne!(game.board, before, "the board moved at once");
        assert!(game.motion.is_some(), "and the drawing has to catch up");
    }

    #[test]
    fn a_press_against_the_edge_does_nothing_at_all() {
        let mut game = StarryGame::new();
        game.board = Board::solved();
        let before = game.board;

        // the gap starts in the bottom right corner
        game.press(Direction::Down);

        assert_eq!(game.board, before);
        assert!(game.motion.is_none(), "and nothing is animating");
    }

    #[test]
    fn a_tile_is_drawn_at_its_cell_once_it_has_arrived() {
        let mut game = StarryGame::new();
        game.board = Board::solved();
        game.press(Direction::Up);

        let moving = game.motion.expect("a tile is on its way").tile;
        let cell = (0..CELLS)
            .find(|cell| game.board.cell(*cell) == moving)
            .expect("the board knows where it is");

        game.motion = None;
        let drawn = game.drawn_at(moving, cell);

        assert!((drawn - tiles::cell_position(cell)).length() < 1e-6);
    }
}
