use anyhow::Result;
use bvc_relay::node::NodeIdentity;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tempfile::TempDir;

/// The state a beta.20 deployment actually has: a partially migrated database and every
/// secret on disk.
///
/// `BETA_20_MIGRATIONS` is the count of migrations that shipped in beta.20, so
/// `Migrator::up(conn, Some(n))` leaves the schema and the `seaql_migrations` rows exactly
/// where that release left them. Running the full migrator afterwards is the upgrade.
///
/// The column *types* are today's rather than beta.20's, because the migrations that created
/// them were later edited in place. On SQLite that is not a difference — integer affinity is
/// dynamic and 64-bit — and the same edit is covered for MySQL by
/// `m20260726_000001_signed_timestamps`.
pub struct Beta20Fixture {
    pub connection: DatabaseConnection,
    pub certs_dir: TempDir,
    ca_cert_pem: String,
    ca_key_pem: String,
    player_cert_pem: String,
    node_id: String,
    _db_dir: TempDir,
}

impl Beta20Fixture {
    /// m20231220_player, m20260119_player_game, m20260307_player_identity,
    /// m20260311_player_auth_code, m20260322_audio_file, m20260322_player_permission,
    /// m20260618_player_auth_code_ephemeral.
    const BETA_20_MIGRATIONS: u32 = 7;

    pub const ACCESS_TOKEN: &'static str = "beta20accesstoken0000000000000a";
    pub const ACME_ACCOUNT: &'static str = "{\"beta20\":true}";

    pub fn sans() -> Vec<String> {
        vec!["localhost".to_string(), "127.0.0.1".to_string()]
    }

    pub async fn create() -> Result<Self> {
        let db_dir = tempfile::tempdir()?;
        let path = db_dir.path().join("test.sqlite");
        let connection = Database::connect(format!("sqlite://{}?mode=rwc", path.display())).await?;
        Migrator::up(&connection, Some(Self::BETA_20_MIGRATIONS)).await?;

        let certs_dir = tempfile::tempdir()?;
        let certs_path = certs_dir.path().to_str().expect("utf-8 path").to_string();

        // The certificate authority, exactly as beta.20 wrote it.
        let (ca_cert_pem, ca_key_pem) =
            bvc_server_lib::runtime::ca_cert::CaCertManager::new(&certs_path)
                .ensure(&Self::sans())?;

        // A player certificate issued by that authority, before the upgrade.
        let cert_service = bvc_server_lib::services::CertificateService::new(&certs_path)?;
        let (player_cert, _player_key) =
            cert_service.sign_player_cert("Alaydriem", &common::Game::Minecraft)?;
        let player_cert_pem = player_cert.pem();

        std::fs::write(certs_dir.path().join("access_token"), Self::ACCESS_TOKEN)?;

        let node_id = NodeIdentity::load_or_create(&certs_path)?.node_id().to_string();

        let acme_dir = certs_dir.path().join("acme");
        std::fs::create_dir_all(&acme_dir)?;
        std::fs::write(acme_dir.join("account.json"), Self::ACME_ACCOUNT)?;

        Ok(Self {
            connection,
            certs_dir,
            ca_cert_pem,
            ca_key_pem,
            player_cert_pem,
            node_id,
            _db_dir: db_dir,
        })
    }

    /// The upgrade itself: every migration beta.21 adds.
    pub async fn migrate(&self) -> Result<()> {
        Migrator::up(&self.connection, None).await?;
        Ok(())
    }

    pub fn certs_path(&self) -> &str {
        self.certs_dir.path().to_str().expect("utf-8 path")
    }

    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    pub fn ca_key_pem(&self) -> &str {
        &self.ca_key_pem
    }

    pub fn player_cert_pem(&self) -> &str {
        &self.player_cert_pem
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}
