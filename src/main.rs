#![allow(clippy::assigning_clones)]
use geng::prelude::*;
use geng_egui::{egui, EguiGeng};

#[derive(geng::asset::Load)]
struct Shaders {
    background: ugli::Program,
    surface_dist: ugli::Program,
    level: ugli::Program,
    player: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    shaders: Shaders,
}

#[derive(Deserialize)]
struct LevelMeshConfig {
    max_distance: f32,
}

#[derive(Deserialize)]
pub struct CursorConfig {
    hotspot: vec2<u16>,
}

#[derive(Deserialize)]
pub struct PlayerConfig {
    radius: f32,
    min_radius: f32,
    max_radius: f32,
    scaling_speed: f32,
}

#[derive(Deserialize)]
struct StaticConfig {
    max_vel: f32,
    time_to_full: f32,
}

#[derive(Deserialize)]
pub struct Config {
    gravity: f32,
    bounciness: f32,
    friction: f32,
    r#static: StaticConfig,
    fov: f32,
    player: PlayerConfig,
    level_mesh: LevelMeshConfig,
    cursor: CursorConfig,
}

#[derive(ugli::Vertex)]
struct QuadVertex {
    a_pos: vec2<f32>,
}

#[derive(Serialize, Deserialize)]
struct Surface {
    ends: [vec2<f32>; 2],
}

struct To {
    normal: vec2<f32>,
    distance: f32,
    closest_point: vec2<f32>,
}

impl Surface {
    fn to(&self, p: vec2<f32>) -> To {
        let [a, b] = self.ends;
        if vec2::dot(a - b, p - b) < 0.0 {
            return To {
                normal: (p - b).normalize_or_zero(),
                distance: (p - b).len(),
                closest_point: b,
            };
        }
        if vec2::dot(b - a, p - a) < 0.0 {
            return To {
                normal: (p - a).normalize_or_zero(),
                distance: (p - a).len(),
                closest_point: a,
            };
        }
        let mut normal = (b - a).rotate_90().normalize();
        let mut distance = vec2::dot(normal, p - a);
        if distance < 0.0 {
            normal = -normal;
            distance = -distance;
        }
        To {
            normal,
            distance,
            closest_point: p - normal * distance,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Level {
    surfaces: Vec<Surface>,
}

struct LevelMesh {
    surfaces_dist: ugli::VertexBuffer<SurfaceVertex>,
}

#[derive(ugli::Vertex, Copy, Clone)]
struct SurfaceVertex {
    a_pos: vec2<f32>,
    a_dist: vec2<f32>,
}

impl LevelMesh {
    fn new(geng: &Geng, config: &Config, level: &Level) -> Self {
        let config = &config.level_mesh;
        Self {
            surfaces_dist: ugli::VertexBuffer::new_static(
                geng.ugli(),
                level
                    .surfaces
                    .iter()
                    .flat_map(|surface| {
                        let middle = [(0, -1), (1, -1), (1, 1), (0, 1)].map(|(x, y)| {
                            let x = x as f32;
                            let y = y as f32;
                            let [p0, p1] = surface.ends;
                            let normal = (p1 - p0).rotate_90().normalize_or_zero();
                            SurfaceVertex {
                                a_pos: p0 + (p1 - p0) * x + normal * y * config.max_distance,
                                a_dist: vec2(0.0, y * config.max_distance),
                            }
                        });
                        let mk_end = |end: vec2<f32>| {
                            [(-1, -1), (-1, 1), (1, 1), (1, -1)].map(move |(x, y)| {
                                let delta = vec2(x as f32, y as f32) * config.max_distance;
                                SurfaceVertex {
                                    a_pos: end + delta,
                                    a_dist: delta,
                                }
                            })
                        };
                        [middle, mk_end(surface.ends[0]), mk_end(surface.ends[1])]
                            .into_iter()
                            .flat_map(|quad| [quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]])
                    })
                    .collect(),
            ),
        }
    }
}

struct Player {
    pos: vec2<f32>,
    vel: vec2<f32>,
    radius: f32,
    r#static: f32,
}

pub struct Game {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Assets,
    config: Config,
    camera: Camera2d,
    quad: ugli::VertexBuffer<QuadVertex>,
    level: Level,
    level_mesh: LevelMesh,
    time: f32,
    start_draw: Option<vec2<f64>>,
    temp_texture: ugli::Texture,
    temp_renderbuffer: ugli::Renderbuffer<ugli::DepthStencilValue>,
    player: Option<Player>,
    egui: EguiGeng,
    editor_mode: bool,
    cli: CliArgs,
}

impl Game {
    async fn new(geng: &Geng, cli: CliArgs) -> Self {
        let assets: Assets = geng
            .asset_manager()
            .load(run_dir().join("assets"))
            .await
            .unwrap();
        let config: Config = file::load_detect(run_dir().join("assets").join("config.toml"))
            .await
            .unwrap();

        geng.window().set_cursor_type(geng::CursorType::Custom {
            image: geng
                .asset_manager()
                .load(run_dir().join("assets").join("cursor.png"))
                .await
                .unwrap(),
            hotspot: config.cursor.hotspot,
        });
        let level = file::load_json(run_dir().join("assets").join("level.json"))
            .await
            .unwrap();
        let level_mesh = LevelMesh::new(geng, &config, &level);
        Self {
            framebuffer_size: vec2::splat(1.0),
            level,
            level_mesh,
            time: 0.0,
            geng: geng.clone(),
            camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::MinSide(config.fov),
            },
            quad: ugli::VertexBuffer::new_static(
                geng.ugli(),
                [(0, 0), (1, 0), (1, 1), (0, 1)]
                    .into_iter()
                    .map(|(x, y)| QuadVertex {
                        a_pos: vec2(x, y).map(|x| x as f32),
                    })
                    .collect(),
            ),
            start_draw: None,
            temp_texture: ugli::Texture2d::new_with(geng.ugli(), vec2::splat(1), |_| Rgba::WHITE),
            temp_renderbuffer: ugli::Renderbuffer::new(geng.ugli(), vec2::splat(1)),
            player: Some(Player {
                pos: vec2::ZERO,
                vel: vec2::ZERO,
                radius: config.player.radius,
                r#static: 0.0,
            }),
            assets,
            config,
            editor_mode: false,
            egui: EguiGeng::new(geng),
            cli,
        }
    }

    fn update_level(&mut self) {
        self.level_mesh = LevelMesh::new(&self.geng, &self.config, &self.level);
    }

    fn screen_to_world(&self, screen: vec2<f64>) -> vec2<f32> {
        self.camera
            .screen_to_world(self.framebuffer_size, screen.map(|x| x as f32))
    }

    fn ui(&mut self) {
        if !self.cli.enable_editor {
            return;
        }
        egui::Window::new("Editor").show(self.egui.get_context(), |ui| {
            ui.checkbox(&mut self.editor_mode, "Editor mode");
        });
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.time += delta_time;
    }

    fn fixed_update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let cursor_pos =
            self.screen_to_world(self.geng.window().cursor_position().unwrap_or(vec2::ZERO));
        if let Some(player) = &mut self.player {
            player.r#static =
                (player.r#static + delta_time / self.config.r#static.time_to_full).min(1.0);
            if player.vel.len() > self.config.r#static.max_vel {
                player.r#static = 0.0;
            }
            player.vel.y -= self.config.gravity * delta_time * (1.0 - player.r#static);
            player.pos += player.vel * delta_time * (1.0 - player.r#static);

            let target_radius = if self
                .geng
                .window()
                .is_button_pressed(geng::MouseButton::Left)
            {
                self.config.player.max_radius
            } else if self
                .geng
                .window()
                .is_button_pressed(geng::MouseButton::Right)
            {
                self.config.player.min_radius
            } else {
                self.config.player.radius
            };
            let player_scaling_speed =
                (target_radius - player.radius) * self.config.player.scaling_speed;
            if player_scaling_speed.abs() > self.config.r#static.max_vel {
                player.r#static = 0.0;
            }
            let old_radius = player.radius;
            let scale_origin = player.pos + (cursor_pos - player.pos).clamp_len(..=player.radius);
            let new_radius = (player.radius + player_scaling_speed * delta_time)
                .clamp(self.config.player.min_radius, self.config.player.max_radius);
            player.pos = scale_origin + (player.pos - scale_origin) * new_radius / old_radius;
            player.radius = new_radius;

            for surface in &self.level.surfaces {
                let to = surface.to(player.pos);
                if to.distance < player.radius {
                    let penetration = player.radius - to.distance;
                    player.pos += to.normal * penetration;
                    let vel_at_collision_point = player.vel
                        + player_scaling_speed * (to.closest_point - scale_origin) / old_radius;
                    let normal_vel = vec2::dot(vel_at_collision_point, to.normal);
                    if normal_vel < 0.0 {
                        player.vel -= to.normal * normal_vel * (1.0 + self.config.bounciness);
                    }
                    let along = to.normal.rotate_90();
                    let along_vel = vec2::dot(vel_at_collision_point, along);
                    player.vel -=
                        along * along_vel.clamp_abs(normal_vel.abs() * self.config.friction);
                }
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        let uniforms = (
            ugli::uniforms! {
                u_time: self.time,
            },
            self.camera.uniforms(framebuffer.size().map(|x| x as f32)),
        );
        ugli::clear(framebuffer, None, Some(1.0), None);
        ugli::draw(
            framebuffer,
            &self.assets.shaders.background,
            ugli::DrawMode::TriangleFan,
            &self.quad,
            &uniforms,
            ugli::DrawParameters::default(),
        );

        if self.temp_texture.size() != framebuffer.size() {
            self.temp_texture =
                ugli::Texture::new_uninitialized(self.geng.ugli(), framebuffer.size());
            self.temp_renderbuffer = ugli::Renderbuffer::new(self.geng.ugli(), framebuffer.size());
        }

        // level
        {
            let mut framebuffer = ugli::Framebuffer::new(
                self.geng.ugli(),
                ugli::ColorAttachment::Texture(&mut self.temp_texture),
                ugli::DepthAttachment::RenderbufferWithStencil(&mut self.temp_renderbuffer),
            );
            let framebuffer = &mut framebuffer;
            ugli::clear(framebuffer, Some(Rgba::WHITE), None, None);
            ugli::draw(
                framebuffer,
                &self.assets.shaders.surface_dist,
                ugli::DrawMode::Triangles,
                &self.level_mesh.surfaces_dist,
                (
                    ugli::uniforms! {
                        u_max_distance: self.config.level_mesh.max_distance,
                    },
                    &uniforms,
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::combined(ugli::ChannelBlendMode {
                        src_factor: ugli::BlendFactor::One,
                        dst_factor: ugli::BlendFactor::One,
                        equation: ugli::BlendEquation::Min,
                    })),
                    ..default()
                },
            );
        }

        ugli::draw(
            framebuffer,
            &self.assets.shaders.level,
            ugli::DrawMode::TriangleFan,
            &self.quad,
            (
                ugli::uniforms! {
                    u_max_distance: self.config.level_mesh.max_distance,
                    u_surface_dist: &self.temp_texture,
                },
                &uniforms,
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::premultiplied_alpha()),
                ..default()
            },
        );

        if let Some(player) = &self.player {
            ugli::draw(
                framebuffer,
                &self.assets.shaders.player,
                ugli::DrawMode::TriangleFan,
                &self.quad,
                (
                    ugli::uniforms! {
                        u_static: player.r#static,
                        u_pos: player.pos,
                        u_vel: player.vel,
                        u_radius: player.radius,
                    },
                    &uniforms,
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::premultiplied_alpha()),
                    ..default()
                },
            );
        }

        self.egui.begin_frame();
        self.ui();
        self.egui.end_frame();
        self.egui.draw(framebuffer);
    }
    fn handle_event(&mut self, event: geng::Event) {
        self.egui.handle_event(event.clone());
        if self.cli.enable_editor {
            match event {
                geng::Event::KeyPress { key } => match key {
                    geng::Key::F4 => self.editor_mode = !self.editor_mode,
                    geng::Key::R => {
                        if let Some(screen_pos) = self.geng.window().cursor_position() {
                            self.player = Some(Player {
                                pos: self.screen_to_world(screen_pos),
                                vel: vec2::ZERO,
                                radius: self.config.player.radius,
                                r#static: 0.0,
                            });
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if self.editor_mode {
            match event {
                geng::Event::MousePress {
                    button: geng::MouseButton::Left,
                } => {
                    self.start_draw = self.geng.window().cursor_position();
                }
                geng::Event::MouseRelease { .. } => {
                    if let Some(start) = self.start_draw.take() {
                        if let Some(end) = self.geng.window().cursor_position() {
                            let ends = [start, end].map(|p| self.screen_to_world(p));
                            self.level.surfaces.push(Surface { ends });
                            self.update_level();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(clap::Parser)]
struct CliArgs {
    #[clap(long)]
    enable_editor: bool,
    #[clap(flatten)]
    geng: geng::CliArgs,
}

fn main() {
    let cli: CliArgs = cli::parse();
    Geng::run_with(
        &{
            let mut options = geng::ContextOptions::default();
            options.window.title = env!("CARGO_PKG_NAME").to_owned();
            options.with_cli(&cli.geng);
            options.fixed_delta_time = 1.0 / 200.0;
            options
        },
        move |geng| async move {
            let state = Game::new(&geng, cli).await;
            geng.run_state(state).await;
        },
    );
}
