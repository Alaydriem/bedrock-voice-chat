use crate::config::Server;
use rocket::{http::ContentType, State};

static DEFAULT_AVATAR: &[u8] = include_bytes!("../../../assets/avatar.png");
static DEFAULT_CANVAS: &[u8] = include_bytes!("../../../assets/canvas.png");

pub struct AssetsHandler;

impl AssetsHandler {
    fn serve(assets_path: &str, filename: &str, default: &'static [u8]) -> (ContentType, Vec<u8>) {
        let path = std::path::Path::new(assets_path).join(filename);
        let bytes = if path.exists() {
            std::fs::read(&path).unwrap_or_else(|_| default.to_vec())
        } else {
            default.to_vec()
        };
        (ContentType::PNG, bytes)
    }
}

#[get("/avatar.png")]
pub async fn get_avatar(config: &State<Server>) -> (ContentType, Vec<u8>) {
    AssetsHandler::serve(&config.assets_path, "avatar.png", DEFAULT_AVATAR)
}

#[get("/canvas.png")]
pub async fn get_canvas(config: &State<Server>) -> (ContentType, Vec<u8>) {
    AssetsHandler::serve(&config.assets_path, "canvas.png", DEFAULT_CANVAS)
}
