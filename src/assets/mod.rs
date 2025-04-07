mod font;

pub use self::font::*;

use crate::{
    game::Controls,
    model::{Config, ConnectionKind, ResourceKind},
    prelude::Color,
};

use std::path::PathBuf;

use geng::prelude::*;
use geng_utils::gif::GifFrame;

#[derive(geng::asset::Load)]
pub struct LoadingAssets {
    #[load(path = "sprites/title.png", options(filter = "ugli::Filter::Nearest"))]
    pub title: ugli::Texture,
    #[load(path = "fonts/default.ttf")]
    pub font: Font,
    #[load(load_with = "load_gif(&manager, &base_path.join(\"sprites/loading_background.gif\"))")]
    pub background: Vec<GifFrame>,
}

fn load_gif(
    manager: &geng::asset::Manager,
    path: &std::path::Path,
) -> geng::asset::Future<Vec<GifFrame>> {
    let manager = manager.clone();
    let path = path.to_owned();
    async move {
        geng_utils::gif::load_gif(
            &manager,
            &path,
            geng_utils::gif::GifOptions {
                frame: geng::asset::TextureOptions {
                    filter: ugli::Filter::Nearest,
                    ..Default::default()
                },
            },
        )
        .await
    }
    .boxed_local()
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub palette: Palette,
    pub controls: Controls,
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub config: Config,
    pub fonts: Fonts,
}

impl Assets {
    pub async fn load(manager: &geng::asset::Manager) -> anyhow::Result<Self> {
        geng::asset::Load::load(manager, &run_dir().join("assets"), &()).await
    }
}

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: Rc<ugli::Program>,
    pub ellipse: Rc<ugli::Program>,
    pub masked: Rc<ugli::Program>,
    pub solid: Rc<ugli::Program>,
}

#[derive(geng::asset::Load)]
pub struct Fonts {
    pub default: Rc<Font>,
    #[load(path = "DeadRevolverGame.ttf")]
    pub revolver_game: Rc<Font>,
    #[load(path = "DeadRevolverDisplay.ttf")]
    pub revolver_display: Rc<Font>,
    #[load(path = "DeadRevolverArcadeOutlined.ttf")]
    pub revolver_arcade: Rc<Font>,
}

#[derive(geng::asset::Load)]
pub struct Sprites {
    pub coin: PixelTexture,
    pub fuel_small_node: PixelTexture,
    pub fuel_normal_node: PixelTexture,
    pub fill_thinner: PixelTexture,
    pub border_thinner: PixelTexture,
    pub border_ui: PixelTexture,
    pub border_game: PixelTexture,
    pub border_shop: PixelTexture,
    pub connect_dot: PixelTexture,

    pub power_node: PixelTexture,
    pub power_button_normal: PixelTexture,
    pub power_button_pressed: PixelTexture,

    pub shop_0_node: PixelTexture,
    pub shop_0_button_normal: PixelTexture,
    pub shop_0_button_pressed: PixelTexture,
    pub shop_1_button_normal: PixelTexture,
    pub shop_1_button_pressed: PixelTexture,
    pub shop_2_button_normal: PixelTexture,
    pub shop_2_button_pressed: PixelTexture,

    pub close_button_normal: PixelTexture,
    pub close_button_pressed: PixelTexture,
}

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "toml")]
pub struct Palette {
    pub default: Color,
    pub background: Color,
    pub ui_view: Color,
    pub game_view: Color,

    pub depth_text: Color,
    pub gold_text: Color,

    pub drill: Color,
    pub vision_circle: Color,

    pub fuel_back: Color,
    pub fuel_front: Color,

    pub rock: Color,
    pub resources: HashMap<ResourceKind, Color>,

    pub nodes: PaletteNodes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaletteNodes {
    pub connections: HashMap<ConnectionKind, Color>,
}

#[derive(Clone)]
pub struct PixelTexture {
    pub path: PathBuf,
    pub texture: Rc<ugli::Texture>,
}

impl Deref for PixelTexture {
    type Target = ugli::Texture;

    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}

impl Debug for PixelTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PixelTexture")
            .field("path", &self.path)
            .field("texture", &"<texture data>")
            .finish()
    }
}

impl geng::asset::Load for PixelTexture {
    type Options = <ugli::Texture as geng::asset::Load>::Options;

    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        options: &Self::Options,
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        let texture = ugli::Texture::load(manager, &path, options);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self {
                path,
                texture: Rc::new(texture),
            })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}
