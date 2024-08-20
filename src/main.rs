#![allow(clippy::assigning_clones)]
use geng::prelude::*;
use geng_egui::{egui, EguiGeng};

#[derive(geng::asset::Load)]
struct Shaders {
    invert: ugli::Program,
    insides: ugli::Program,
    background: ugli::Program,
    surface_dist: ugli::Program,
    level: ugli::Program,
    finish: ugli::Program,
    player: ugli::Program,
    selection: ugli::Program,
}

#[derive(geng::asset::Load)]
struct SfxAssets {
    hit: geng::Sound,
    #[load(options(looped = "true"))]
    scale_up: geng::Sound,
    #[load(options(looped = "true"))]
    scale_down: geng::Sound,
    level: geng::Sound,
    win: geng::Sound,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    #[load(path = "music.mp3", options(looped = "true"))]
    music: geng::Sound,
    sfx: SfxAssets,
    #[load(path = "font/Moby-Monospace.ttf")]
    font: geng::Font,
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
struct EditorConfig {
    snap_distance: f32,
    cursor_rotation_speed: f32,
    camera_speed: f32,
}

#[derive(Deserialize)]
struct CameraConfig {
    fov: f32,
    speed: f32,
}

#[derive(Deserialize)]
pub struct SfxConfig {
    level_volume: f32,
    win_volume: f32,
    master_volume: f32,
    music_volume: f32,
    scaling_max_volume: f32,
    hit_volume: f32,
    hit_max_volume_speed: f32,
}

#[derive(Deserialize)]
pub struct Config {
    sfx: SfxConfig,
    finish_radius: f32,
    tick_distance: f32,
    editor: EditorConfig,
    gravity: f32,
    bounciness: f32,
    friction: f32,
    r#static: StaticConfig,
    camera: CameraConfig,
    player: PlayerConfig,
    level_mesh: LevelMeshConfig,
    cursor: CursorConfig,
}

#[derive(ugli::Vertex)]
struct Vertex {
    a_pos: vec2<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
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
        if vec2::dot(a - b, p - b) <= 0.0 {
            return To {
                normal: (p - b).normalize_or_zero(),
                distance: (p - b).len(),
                closest_point: b,
            };
        }
        if vec2::dot(b - a, p - a) <= 0.0 {
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

#[derive(Serialize, Deserialize, Clone)]
struct Level {
    #[serde(default = "default_start")]
    start_pos: vec2<f32>,
    #[serde(default = "default_finish")]
    finish_pos: vec2<f32>,
    #[serde(default)]
    surfaces: Vec<Surface>,
}

fn default_finish() -> vec2<f32> {
    vec2(5.0, 0.0)
}

fn default_start() -> vec2<f32> {
    vec2::ZERO
}

struct Levels {
    list: Vec<String>,
    map: HashMap<String, Level>,
}

impl geng::asset::Load for Levels {
    type Options = ();
    fn load(
        _manager: &geng::asset::Manager,
        path: &std::path::Path,
        _options: &Self::Options,
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        async move {
            let path = &path;
            let list: Vec<String> = file::load_json(path.join("_list.json")).await.unwrap();
            let levels = future::join_all(list.into_iter().map(|level_name| async move {
                let level: Level = file::load_json(path.join(&level_name).with_extension("json"))
                    .await
                    .unwrap();
                (level_name, level)
            }))
            .await;
            Ok(Self {
                list: levels.iter().map(|(name, _level)| name.clone()).collect(),
                map: levels.into_iter().collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
}

struct LevelMesh {
    surfaces_dist: ugli::VertexBuffer<SurfaceVertex>,
    insides: ugli::VertexBuffer<Vertex>,
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
            insides: ugli::VertexBuffer::new_static(
                geng.ugli(),
                level
                    .surfaces
                    .iter()
                    .flat_map(|surface| {
                        let [a, b] = surface.ends;
                        if vec2::skew(a, b) < 0.0 {
                            [a, b, vec2::ZERO]
                        } else {
                            [b, a, vec2::ZERO]
                        }
                    })
                    .map(|p| Vertex { a_pos: p })
                    .collect(),
            ),
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
    scale_origin: vec2<f32>,
}

pub struct Game {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Assets,
    config: Config,
    camera: Camera2d,
    quad: ugli::VertexBuffer<Vertex>,
    level: Level,
    level_mesh: LevelMesh,
    time: f32,
    start_draw: Option<vec2<f32>>,
    temp_texture: ugli::Texture,
    temp_renderbuffer: ugli::Renderbuffer<ugli::DepthStencilValue>,
    player: Option<Player>,
    egui: Rc<RefCell<EguiGeng>>,
    editor_mode: bool,
    cli: CliArgs,
    unprocessed: f32,
    levels: Levels,
    current_level: usize,
    draw_insides: bool,
    finished: bool,
    scale_up_sfx: geng::SoundEffect,
    scale_down_sfx: geng::SoundEffect,
}

trait SoundExt {
    fn play_with_volume(&self, volume: f32) -> geng::SoundEffect;
}

impl SoundExt for geng::Sound {
    fn play_with_volume(&self, volume: f32) -> geng::SoundEffect {
        let mut eff = self.play();
        eff.set_volume(volume);
        eff
    }
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
        geng.audio()
            .master_volume()
            .set_value(config.sfx.master_volume);
        assets.music.play().set_volume(config.sfx.music_volume);

        geng.window().set_cursor_type(geng::CursorType::Custom {
            image: geng
                .asset_manager()
                .load(run_dir().join("assets").join("cursor.png"))
                .await
                .unwrap(),
            hotspot: config.cursor.hotspot,
        });
        let levels: Levels = geng
            .asset_manager()
            .load(run_dir().join("assets").join("levels"))
            .await
            .unwrap();
        let level = levels.map[&levels.list[0]].clone();
        let level_mesh = LevelMesh::new(geng, &config, &level);
        let mut result = Self {
            levels,
            framebuffer_size: vec2::splat(1.0),
            level,
            level_mesh,
            time: 0.0,
            geng: geng.clone(),
            camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::MinSide(config.camera.fov),
            },
            quad: ugli::VertexBuffer::new_static(
                geng.ugli(),
                [(0, 0), (1, 0), (1, 1), (0, 1)]
                    .into_iter()
                    .map(|(x, y)| Vertex {
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
                scale_origin: vec2::ZERO,
            }),
            editor_mode: false,
            egui: Rc::new(RefCell::new(EguiGeng::new(geng))),
            cli,
            unprocessed: 0.0,
            current_level: 0,
            draw_insides: true,
            finished: false,
            scale_up_sfx: assets.sfx.scale_up.play_with_volume(0.0),
            scale_down_sfx: assets.sfx.scale_down.play_with_volume(0.0),
            assets,
            config,
        };
        result.setup_level();
        result
    }

    fn prev_level(&mut self) {
        if self.current_level == 0 {
            return;
        }
        self.current_level -= 1;
        self.setup_level();
    }

    fn next_level(&mut self) {
        if self.current_level + 1 >= self.levels.list.len() {
            self.assets
                .sfx
                .win
                .play_with_volume(self.config.sfx.win_volume);
            self.finished = true;
            return;
        }
        self.assets
            .sfx
            .level
            .play_with_volume(self.config.sfx.level_volume);
        self.current_level += 1;
        self.setup_level();
    }

    fn setup_level(&mut self) {
        self.finished = false;
        self.level = self.levels.map[&self.levels.list[self.current_level]].clone();
        self.update_level();
        self.player = Some(Player {
            pos: self.level.start_pos,
            vel: vec2::ZERO,
            radius: self.config.player.radius,
            r#static: 0.0,
            scale_origin: vec2::ZERO,
        });
        self.camera.center = self.level.start_pos;
    }

    fn save_level(&mut self) {
        self.levels.map.insert(
            self.levels.list[self.current_level].clone(),
            self.level.clone(),
        );
        serde_json::to_writer_pretty(
            std::io::BufWriter::new(
                std::fs::File::create(
                    run_dir()
                        .join("assets")
                        .join("levels")
                        .join(&self.levels.list[self.current_level])
                        .with_extension("json"),
                )
                .unwrap(),
            ),
            &self.level,
        )
        .unwrap();
    }

    fn update_level(&mut self) {
        self.level_mesh = LevelMesh::new(&self.geng, &self.config, &self.level);
    }

    fn snapped(&self, screen_pos: vec2<f64>) -> vec2<f32> {
        let world_pos = self.screen_to_world(screen_pos);
        if let Some(p) = self
            .level
            .surfaces
            .iter()
            .flat_map(|surface| surface.ends)
            .filter(|&end| (end - world_pos).len() < self.config.editor.snap_distance)
            .min_by_key(|&end| r32((end - world_pos).len()))
        {
            return p;
        }
        world_pos
    }

    fn screen_to_world(&self, screen_pos: vec2<f64>) -> vec2<f32> {
        self.camera
            .screen_to_world(self.framebuffer_size, screen_pos.map(|x| x as f32))
    }

    fn ui(&mut self) {
        if !self.cli.enable_editor {
            return;
        }
        egui::Window::new("Editor").show(self.egui.clone().borrow().get_context(), |ui| {
            ui.checkbox(&mut self.draw_insides, "draw insides");
            ui.checkbox(&mut self.editor_mode, "Editor mode - F4");
            if ui.button("prev level - [").clicked() {
                self.prev_level();
            }
            if ui.button("next level - ]").clicked() {
                self.next_level();
            }
            ui.label("respawn at cursor - R");
            ui.label("new segment - Drag LMB");
            ui.label("remove segment - RMB");
            ui.label("set start - Z");
            ui.label("set finish - X");
            ui.label("level saves automatically");
        });
    }

    fn hovered_surface(&self, cursor: vec2<f32>) -> Option<usize> {
        self.level
            .surfaces
            .iter()
            .enumerate()
            .filter(|(_index, surface)| {
                surface.to(cursor).distance < self.config.editor.snap_distance
            })
            .min_by_key(|(_index, surface)| r32(surface.to(cursor).distance))
            .map(|(index, _)| index)
    }

    fn tick(&mut self, delta_time: f32) {
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
            let scaling_speed = (target_radius - player.radius) * self.config.player.scaling_speed;
            self.scale_up_sfx
                .set_volume((scaling_speed / self.config.sfx.scaling_max_volume).clamp(0.0, 1.0));
            self.scale_down_sfx
                .set_volume((-scaling_speed / self.config.sfx.scaling_max_volume).clamp(0.0, 1.0));
            if scaling_speed.abs() > self.config.r#static.max_vel {
                player.r#static = 0.0;
            }
            let old_radius = player.radius;
            let scale_origin = player.pos + (cursor_pos - player.pos).clamp_len(..=player.radius);
            player.scale_origin = scale_origin;
            let new_radius = (player.radius + scaling_speed * delta_time)
                .clamp(self.config.player.min_radius, self.config.player.max_radius);
            player.pos = scale_origin + (player.pos - scale_origin) * new_radius / old_radius;
            player.radius = new_radius;

            for surface in &self.level.surfaces {
                let to = surface.to(player.pos);
                if to.distance < player.radius {
                    let penetration = player.radius - to.distance;
                    player.pos += to.normal * penetration;
                    player.radius -= penetration;
                    let vel_at_collision_point =
                        player.vel + scaling_speed * (to.closest_point - scale_origin) / old_radius;
                    let normal_vel = vec2::dot(vel_at_collision_point, to.normal);
                    if normal_vel < 0.0 {
                        player.vel -= to.normal * normal_vel * (1.0 + self.config.bounciness);
                        let sfx_volume =
                            (-normal_vel / self.config.sfx.hit_max_volume_speed).clamp(0.0, 1.0);
                        if sfx_volume > 0.1 {
                            self.assets
                                .sfx
                                .hit
                                .play_with_volume(sfx_volume * self.config.sfx.hit_volume)
                                .set_speed(thread_rng().gen_range(0.8..1.2));
                        }
                    }
                    let along = to.normal.rotate_90();
                    let along_vel = vec2::dot(vel_at_collision_point, along);
                    player.vel -=
                        along * along_vel.clamp_abs(normal_vel.abs() * self.config.friction);
                }
            }

            if (player.pos - self.level.finish_pos).len()
                < player.radius + self.config.finish_radius
            {
                self.next_level();
            }
        }
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        if self.finished {
            self.scale_down_sfx.set_volume(0.0);
            self.scale_up_sfx.set_volume(0.0);
            return;
        }
        self.time += delta_time;
        if self.editor_mode {
            self.player = None;
            if self.geng.window().is_key_pressed(geng::Key::W) {
                self.camera.center.y += self.config.editor.camera_speed * delta_time;
            }
            if self.geng.window().is_key_pressed(geng::Key::A) {
                self.camera.center.x -= self.config.editor.camera_speed * delta_time;
            }
            if self.geng.window().is_key_pressed(geng::Key::S) {
                self.camera.center.y -= self.config.editor.camera_speed * delta_time;
            }
            if self.geng.window().is_key_pressed(geng::Key::D) {
                self.camera.center.x += self.config.editor.camera_speed * delta_time;
            }
        } else if self.player.is_none() {
            self.setup_level();
        }

        if let Some(player) = &self.player {
            self.camera.center += (player.pos - self.camera.center)
                * (self.config.camera.speed * delta_time).min(1.0);
        }
        if self.player.is_some() {
            self.unprocessed += delta_time;
            while self.unprocessed > 0.0 {
                let vel = self.player.as_ref().unwrap().vel.len();
                let dt = self
                    .unprocessed
                    .min(self.config.tick_distance / vel.max(1.0));
                self.tick(dt);
                self.unprocessed -= dt;
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
            ugli::clear(framebuffer, Some(Rgba::WHITE), None, Some(0));
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
            if self.draw_insides {
                ugli::draw(
                    framebuffer,
                    &self.assets.shaders.insides,
                    ugli::DrawMode::Triangles,
                    &self.level_mesh.insides,
                    &uniforms,
                    ugli::DrawParameters {
                        write_color: false,
                        write_depth: false,
                        stencil_mode: Some({
                            ugli::StencilMode::always(ugli::FaceStencilMode {
                                test: ugli::StencilTest {
                                    condition: ugli::Condition::Always,
                                    reference: 0,
                                    mask: 0,
                                },
                                op: ugli::StencilOp::always(ugli::StencilOpFunc::Invert),
                            })
                        }),
                        ..default()
                    },
                );
                ugli::draw(
                    framebuffer,
                    &self.assets.shaders.invert,
                    ugli::DrawMode::TriangleFan,
                    &self.quad,
                    (),
                    ugli::DrawParameters {
                        blend_mode: Some(ugli::BlendMode::combined(ugli::ChannelBlendMode {
                            src_factor: ugli::BlendFactor::OneMinusDstColor,
                            dst_factor: ugli::BlendFactor::Zero,
                            equation: ugli::BlendEquation::Add,
                        })),
                        stencil_mode: Some(ugli::StencilMode::always(ugli::FaceStencilMode {
                            test: ugli::StencilTest {
                                condition: ugli::Condition::Equal,
                                reference: 0,
                                mask: 0xff,
                            },
                            op: ugli::StencilOp::always(ugli::StencilOpFunc::Keep),
                        })),
                        ..default()
                    },
                );
            }
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
                        u_scale_origin: player.scale_origin,
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

        ugli::draw(
            framebuffer,
            &self.assets.shaders.finish,
            ugli::DrawMode::TriangleFan,
            &self.quad,
            (
                ugli::uniforms! {
                    u_pos: self.level.finish_pos,
                    u_radius: self.config.finish_radius,
                },
                &uniforms,
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::premultiplied_alpha()),
                ..default()
            },
        );

        if self.editor_mode {
            ugli::draw(
                framebuffer,
                &self.assets.shaders.player,
                ugli::DrawMode::TriangleFan,
                &self.quad,
                (
                    ugli::uniforms! {
                        u_static: 0.0,
                        u_pos: self.level.start_pos,
                        u_vel: vec2::<f32>::ZERO,
                        u_radius: self.config.player.radius,
                    },
                    &uniforms,
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::premultiplied_alpha()),
                    ..default()
                },
            );

            let cursor =
                self.screen_to_world(self.geng.window().cursor_position().unwrap_or(vec2::ZERO));
            let snapped_cursor =
                self.snapped(self.geng.window().cursor_position().unwrap_or(vec2::ZERO));

            if let Some(start) = self.start_draw {
                let v = snapped_cursor - start;
                let matrix = mat3::translate((snapped_cursor + start) / 2.0)
                    * mat3::from_orts(
                        v.normalize_or_zero()
                            * (v.len() / 2.0 + self.config.editor.snap_distance / 2.0),
                        v.normalize_or_zero().rotate_90() * self.config.editor.snap_distance / 2.0,
                    );
                ugli::draw(
                    framebuffer,
                    &self.assets.shaders.selection,
                    ugli::DrawMode::TriangleFan,
                    &self.quad,
                    (
                        ugli::uniforms! {
                            u_model_matrix: matrix,
                        },
                        &uniforms,
                    ),
                    ugli::DrawParameters {
                        blend_mode: None,
                        ..default()
                    },
                );
            } else {
                if let Some(surface) = self.hovered_surface(cursor) {
                    let surface = &self.level.surfaces[surface];
                    let v = surface.ends[1] - surface.ends[0];
                    let matrix = mat3::translate((surface.ends[0] + surface.ends[1]) / 2.0)
                        * mat3::from_orts(
                            v.normalize_or_zero()
                                * (v.len() / 2.0 + self.config.editor.snap_distance / 2.0),
                            v.normalize_or_zero().rotate_90() * self.config.editor.snap_distance
                                / 2.0,
                        );
                    ugli::draw(
                        framebuffer,
                        &self.assets.shaders.selection,
                        ugli::DrawMode::TriangleFan,
                        &self.quad,
                        (
                            ugli::uniforms! {
                                u_model_matrix: matrix,
                            },
                            &uniforms,
                        ),
                        ugli::DrawParameters {
                            blend_mode: Some(ugli::BlendMode {
                                rgb: ugli::ChannelBlendMode {
                                    src_factor: ugli::BlendFactor::OneMinusDstColor,
                                    dst_factor: ugli::BlendFactor::Zero,
                                    equation: ugli::BlendEquation::Add,
                                },
                                alpha: ugli::ChannelBlendMode {
                                    src_factor: ugli::BlendFactor::Zero,
                                    dst_factor: ugli::BlendFactor::One,
                                    equation: ugli::BlendEquation::Add,
                                },
                            }),
                            ..default()
                        },
                    );
                }
                if true {
                    let cursor_matrix = mat3::translate(snapped_cursor)
                        * mat3::rotate(Angle::from_degrees(
                            self.config.editor.cursor_rotation_speed * self.time,
                        ))
                        * mat3::scale_uniform(self.config.editor.snap_distance / 2.0);
                    ugli::draw(
                        framebuffer,
                        &self.assets.shaders.selection,
                        ugli::DrawMode::TriangleFan,
                        &self.quad,
                        (
                            ugli::uniforms! {
                                u_model_matrix: cursor_matrix,
                            },
                            &uniforms,
                        ),
                        ugli::DrawParameters {
                            blend_mode: Some(ugli::BlendMode {
                                rgb: ugli::ChannelBlendMode {
                                    src_factor: ugli::BlendFactor::OneMinusDstColor,
                                    dst_factor: ugli::BlendFactor::Zero,
                                    equation: ugli::BlendEquation::Add,
                                },
                                alpha: ugli::ChannelBlendMode {
                                    src_factor: ugli::BlendFactor::Zero,
                                    dst_factor: ugli::BlendFactor::One,
                                    equation: ugli::BlendEquation::Add,
                                },
                            }),
                            ..default()
                        },
                    );
                }
            }
        }

        self.assets.font.draw_with_outline(
            framebuffer,
            &Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(10.0),
            },
            &{
                let ms = (self.time * 1000.0) as i64;
                let seconds = ms / 1000;
                let minutes = seconds / 60;
                format!("{}:{:02}:{:03}", minutes, seconds % 60, ms % 1000)
            },
            vec2(geng::TextAlign::CENTER, geng::TextAlign::TOP),
            mat3::translate(vec2(0.0, 5.0)),
            Rgba::WHITE,
            0.05,
            Rgba::BLACK,
        );

        if self.finished {
            self.assets.font.draw_with_outline(
                framebuffer,
                &Camera2d {
                    center: vec2::ZERO,
                    rotation: Angle::ZERO,
                    fov: Camera2dFov::Vertical(10.0),
                },
                "You WIN!",
                vec2(geng::TextAlign::CENTER, geng::TextAlign::TOP),
                mat3::scale_uniform(1.5),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }

        self.egui.borrow_mut().begin_frame();
        self.ui();
        self.egui.borrow_mut().end_frame();
        self.egui.borrow_mut().draw(framebuffer);
    }
    fn handle_event(&mut self, event: geng::Event) {
        self.egui.borrow_mut().handle_event(event.clone());
        if self.egui.borrow().get_context().is_pointer_over_area() {
            return;
        }
        if matches!(event, geng::Event::KeyPress { key: geng::Key::R })
            && self.geng.window().is_key_pressed(geng::Key::ControlLeft)
        {
            self.time = 0.0;
            self.current_level = 0;
            self.setup_level();
            self.editor_mode = false;
            return;
        }
        if self.cli.enable_editor {
            match event {
                geng::Event::KeyPress { key } => match key {
                    geng::Key::F4 => self.editor_mode = !self.editor_mode,
                    geng::Key::R => {
                        if let Some(screen_pos) = self.geng.window().cursor_position() {
                            self.editor_mode = false;
                            self.player = Some(Player {
                                pos: self.screen_to_world(screen_pos),
                                vel: vec2::ZERO,
                                radius: self.config.player.radius,
                                r#static: 0.0,
                                scale_origin: vec2::ZERO,
                            });
                        }
                    }
                    geng::Key::BracketLeft => self.prev_level(),
                    geng::Key::BracketRight => self.next_level(),
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
                    self.start_draw = self
                        .geng
                        .window()
                        .cursor_position()
                        .map(|pos| self.snapped(pos));
                }
                geng::Event::MousePress {
                    button: geng::MouseButton::Right,
                } => {
                    let cursor = self.screen_to_world(
                        self.geng.window().cursor_position().unwrap_or(vec2::ZERO),
                    );
                    if let Some(index) = self.hovered_surface(cursor) {
                        self.level.surfaces.remove(index);
                        self.save_level();
                        self.update_level();
                    }
                }
                geng::Event::MouseRelease { .. } => {
                    if let Some(start) = self.start_draw.take() {
                        if let Some(end) = self
                            .geng
                            .window()
                            .cursor_position()
                            .map(|pos| self.snapped(pos))
                        {
                            self.level.surfaces.push(Surface { ends: [start, end] });
                            self.save_level();
                            self.update_level();
                        }
                    }
                }
                geng::Event::KeyPress { key } => match key {
                    geng::Key::Z => {
                        self.level.start_pos = self.screen_to_world(
                            self.geng.window().cursor_position().unwrap_or(vec2::ZERO),
                        );
                        self.save_level();
                    }
                    geng::Key::X => {
                        self.level.finish_pos = self.screen_to_world(
                            self.geng.window().cursor_position().unwrap_or(vec2::ZERO),
                        );
                        self.save_level();
                    }
                    _ => {}
                },
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
