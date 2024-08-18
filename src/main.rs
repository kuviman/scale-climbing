#![allow(clippy::assigning_clones)]
use geng::prelude::*;

#[derive(geng::asset::Load)]
pub struct Assets {}

#[derive(Deserialize)]
pub struct Config {
    background: Rgba<f32>,
}

pub struct Game {
    geng: Geng,
    assets: Assets,
    config: Config,
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
            geng: geng.clone(),
            assets,
            config,
        }
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(self.config.background), Some(1.0), None);
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
