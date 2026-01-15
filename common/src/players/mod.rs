mod generic;
mod hytale;
mod minecraft;
mod player_enum;
#[cfg(test)]
mod tests;

pub use generic::GenericPlayer;
pub use hytale::HytalePlayer;
pub use minecraft::MinecraftPlayer;
pub use player_enum::PlayerEnum;
