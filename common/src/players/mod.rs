mod generic;
mod hytale;
mod player_enum;
#[cfg(test)]
mod tests;

pub mod minecraft;
pub use generic::GenericPlayer;
pub use hytale::HytalePlayer;
pub use minecraft::MinecraftPlayer;
pub use player_enum::PlayerEnum;
