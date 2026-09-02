use super::{list, mint, revoke, rotate, show};

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// List every issued token, including the deprecated pre-identifier scalar
    List(list::Config),
    /// Issue a token. The value is printed once and cannot be recovered
    Mint(mint::Config),
    /// Retire a token by id. `legacy` removes the pre-identifier scalar
    Revoke(revoke::Config),
    /// Issue a replacement and retire the old token in one step
    Rotate(rotate::Config),
    /// Print the deprecated pre-identifier scalar
    Show(show::Config),
}
