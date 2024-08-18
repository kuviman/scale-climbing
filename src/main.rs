#![allow(clippy::assigning_clones)]
use geng::prelude::*;

#[derive(geng::asset::Load)]
struct Shaders {
    background: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    shaders: Shaders,
}

#[derive(Deserialize)]
pub struct Config {
    fov: f32,
}

#[derive(ugli::Vertex)]
struct Vertex {
    a_pos: vec2<f32>,
}

pub struct Game {
    geng: Geng,
    assets: Assets,
    config: Config,
    camera: Camera2d,
    quad: ugli::VertexBuffer<Vertex>,
    time: f32,
}

impl Game {
    pub async fn new(geng: &Geng) -> Self {
        let assets: Assets = geng
            .asset_manager()
            .load(run_dir().join("assets"))
            .await
            .unwrap();
        let config: Config = file::load_detect(run_dir().join("assets").join("config.toml"))
            .await
            .unwrap();
        Self {
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
                    .map(|(x, y)| Vertex {
                        a_pos: vec2(x, y).map(|x| x as f32),
                    })
                    .collect(),
            ),
            assets,
            config,
        }
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.time += delta_time;
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
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
    }
}

#[derive(clap::Parser)]
struct CliArgs {
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
            options
        },
        move |geng| async move {
            let state = Game::new(&geng).await;
            geng.run_state(state).await;
        },
    );
}
