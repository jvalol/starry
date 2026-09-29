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
use crate::lifting::{Confirm, Lifting};
use crate::tiles::{self, Motion};

/// How far the board reaches, corner to corner, which is what the camera has to
/// take in.
const BOARD: f32 = tiles::STEP * WIDTH as f32;

/// The tray the tiles sit in. Wider than the board so it frames them, and dark
/// enough that the gap reads as a hole rather than a missing square.
const TRAY_MARGIN: f32 = tiles::TILE * 0.35;
const TRAY_DEPTH: f32 = tiles::THICKNESS * 2.5;
const TRAY_COLOR: glam::Vec4 = vec4(0.06, 0.07, 0.11, 1.0);

/// How far the tile under the cursor rises, so it can be found on a painting
/// that is busy in every square. A tint alone is not enough against this one.
const CURSOR_RISE: f32 = tiles::THICKNESS * 1.6;

/// What a tile is tinted while lift mode is open: the cursor's at full colour,
/// a stranding one well down, the rest a little down so the cursor reads.
const CURSOR_TINT: glam::Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
const RESTING_TINT: glam::Vec4 = vec4(0.72, 0.72, 0.78, 1.0);
const STRANDING_TINT: glam::Vec4 = vec4(0.34, 0.32, 0.40, 1.0);
/// The one that has been warned about and is one press from happening. Spec
/// 0003 wants the second press to look different from the first, not only to
/// read differently.
const WARNED_TINT: glam::Vec4 = vec4(1.0, 0.62, 0.36, 1.0);

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

    lifting: Lifting,
    /// What the game last had to say: a warning, or that a lift is gone.
    note: String,

    help: RenderText,
    readout: RenderText,
    note_text: RenderText,
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
            lifting: Lifting::new(),
            note: String::new(),
            help: RenderText {
                position: vec2(20.0, 20.0),
                color: vec4(1.0, 1.0, 1.0, 0.85),
                text: String::from("arrows slide, L lifts, escape quits"),
                size: 20.0,
                ..Default::default()
            },
            note_text: RenderText {
                position: vec2(20.0, 76.0),
                color: vec4(1.0, 0.86, 0.5, 0.95),
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

    /// The arrows move the cursor while the mode is open, and the gap while it
    /// is not.
    fn aim(&mut self, direction: Direction) {
        if self.lifting.is_open() {
            self.lifting.move_cursor(&self.board, direction);
        } else {
            self.press(direction);
        }
    }

    fn toggle_lifting(&mut self) {
        if self.lifting.is_open() {
            self.lifting.close();
        } else if !self.lifting.open(&self.board) {
            self.note = String::from("no lifts left");
        }
    }

    fn confirm_lift(&mut self) {
        if self.motion.is_some() {
            return;
        }

        match self.lifting.confirm(&mut self.board) {
            Confirm::Nothing => {}
            Confirm::Warned(_) => {
                self.note = String::from("this one cannot be undone by sliding. again to take it");
            }
            Confirm::Lifted { tile, from, to } => {
                self.note.clear();
                self.motion = Some(Motion::lift(tile, from, to));
            }
        }
    }

    fn rewind(&mut self) {
        if self.motion.is_none() && self.lifting.rewind(&mut self.board) {
            self.note = String::from("put back. the lift is still spent");
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
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        // the engine does not clear these, the game does, and a game that
        // forgets grows a vertex buffer until wgpu refuses to allocate it
        geometry.reset();
        text_renderer.reset();

        if let Some(motion) = self.motion.as_mut() {
            motion.advance(dt);
            if motion.done() {
                self.motion = None;
            }
        }

        self.readout.text = if self.board.is_solved() {
            String::from("solved")
        } else {
            format!("par {}   lifts {}", self.board.par(), self.lifting.left())
        };

        if self.lifting.can_rewind(&self.board) {
            self.note = String::from("stuck, and no lifts left. R puts the board back");
        }

        self.note_text.text = self.note.clone();

        text_renderer.render_texts.push(self.help.clone());
        text_renderer.render_texts.push(self.readout.clone());
        if !self.note.is_empty() {
            text_renderer.render_texts.push(self.note_text.clone());
        }
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

            let (tint, rise) = if !self.lifting.is_open() {
                (CURSOR_TINT, 0.0)
            } else if self.lifting.warned() == Some(cell) {
                (WARNED_TINT, CURSOR_RISE)
            } else if self.lifting.cursor() == Some(cell) {
                (CURSOR_TINT, CURSOR_RISE)
            } else if Lifting::strands(&self.board, cell) {
                (STRANDING_TINT, 0.0)
            } else {
                (RESTING_TINT, 0.0)
            };

            scene.push_textured(
                self.tiles[tile as usize - 1],
                painting,
                &Transform::at(self.drawn_at(tile, cell) + Vec3::Y * rise),
                tint,
                TILE_SHININESS,
            );
        }
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        if input.state != KeyboardKeyState::Pressed || input.repeat {
            return;
        }

        match input.key {
            // inside the mode escape means "not this", and outside it means
            // what it always did. A key whose meaning is defined in one state
            // and forgotten in another is how pause swallowed escape in three
            // other games. See spec 0003.
            KeyboardKey::Escape => {
                if self.lifting.is_open() {
                    self.lifting.close();
                } else {
                    self.quitting = true;
                }
            }
            KeyboardKey::L => self.toggle_lifting(),
            KeyboardKey::Return => self.confirm_lift(),
            KeyboardKey::R => self.rewind(),
            KeyboardKey::Up | KeyboardKey::W => self.aim(Direction::Up),
            KeyboardKey::Down | KeyboardKey::S => self.aim(Direction::Down),
            KeyboardKey::Left | KeyboardKey::A => self.aim(Direction::Left),
            KeyboardKey::Right | KeyboardKey::D => self.aim(Direction::Right),
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

    fn key(key: KeyboardKey) -> KeyboardInput {
        KeyboardInput::new(key, KeyboardKeyState::Pressed, false)
    }

    #[test]
    fn escape_cancels_the_mode() {
        let mut game = StarryGame::new();
        game.process_keyboard(key(KeyboardKey::L));
        assert!(game.lifting.is_open());

        game.process_keyboard(key(KeyboardKey::Escape));

        assert!(!game.lifting.is_open(), "it closed the mode");
        assert!(!game.is_quitting(), "and did not quit the game");
    }

    #[test]
    fn escape_outside_the_mode_still_quits() {
        let mut game = StarryGame::new();
        assert!(!game.lifting.is_open());

        game.process_keyboard(key(KeyboardKey::Escape));

        assert!(game.is_quitting());
    }

    #[test]
    fn the_arrows_stop_sliding_while_the_mode_is_open() {
        let mut game = StarryGame::new();
        game.process_keyboard(key(KeyboardKey::L));
        let before = game.board;

        game.process_keyboard(key(KeyboardKey::Right));
        game.process_keyboard(key(KeyboardKey::Down));

        assert_eq!(game.board, before, "the board held still");
        assert!(game.motion.is_none(), "and nothing is sliding");
    }

    #[test]
    fn a_frame_does_not_leave_its_text_behind() {
        // nothing clears these but the game, and two lines a frame reached
        // wgpu's buffer limit after a few minutes of running
        let mut game = StarryGame::new();
        let mut geometry = Geometry::new();
        let mut text_renderer = TextRenderer::new();
        let sounds = SoundSystem::new();

        game.update(0.016, &mut geometry, &mut text_renderer, &sounds);
        let after_one = text_renderer.render_texts.len();

        for _ in 0..50 {
            game.update(0.016, &mut geometry, &mut text_renderer, &sounds);
        }

        assert_eq!(
            text_renderer.render_texts.len(),
            after_one,
            "fifty frames left fifty frames of text"
        );
    }

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
