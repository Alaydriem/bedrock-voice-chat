use crate::config::Server;
use rocket::{
    http::{ContentType, Status},
    State,
};

pub struct AssetsHandler;

impl AssetsHandler {
    fn serve(assets_path: &str, filename: &str) -> Option<(ContentType, Vec<u8>)> {
        let path = std::path::Path::new(assets_path).join(filename);
        if !path.exists() {
            return None;
        }
        match std::fs::read(&path) {
            Ok(bytes) if !bytes.is_empty() => Some((ContentType::PNG, bytes)),
            _ => None,
        }
    }
}

#[get("/avatar.png")]
pub async fn get_avatar(
    config: &State<Server>,
) -> Result<(ContentType, Vec<u8>), Status> {
    AssetsHandler::serve(&config.assets_path, "avatar.png").ok_or(Status::NotFound)
}

#[get("/canvas.png")]
pub async fn get_canvas(
    config: &State<Server>,
) -> Result<(ContentType, Vec<u8>), Status> {
    AssetsHandler::serve(&config.assets_path, "canvas.png").ok_or(Status::NotFound)
}
