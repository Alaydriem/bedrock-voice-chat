use crate::config::ApplicationConfigMeridian;

pub struct MeridianService {
    config: ApplicationConfigMeridian,
    public_addr: String,
    tls_port: u32,
    quic_port: u32,
    hostname: String,
}

impl MeridianService {
    pub fn new(
        config: ApplicationConfigMeridian,
        public_addr: String,
        tls_port: u32,
        quic_port: u32,
        hostname: String,
    ) -> Self {
        Self {
            config,
            public_addr,
            tls_port,
            quic_port,
            hostname,
        }
    }

    pub async fn register(&self) -> Result<(), anyhow::Error> {
        let name = nanoid::nanoid!();
        let tcp_addr = format!("{}:{}", self.public_addr, self.tls_port);
        let udp_addr = format!("{}:{}", self.public_addr, self.quic_port);

        tracing::info!(
            url = %self.config.url,
            name = %name,
            hostname = %self.hostname,
            tcp_addr = %tcp_addr,
            udp_addr = %udp_addr,
            instance_id = self.config.instance_id,
            "Registering with Meridian"
        );

        let client = meridian::api::MeridianClient::builder(&self.config.url, &self.config.api_key)
            .build()?;

        client
            .register(name, &self.hostname, tcp_addr, udp_addr, self.config.instance_id)
            .await?;

        tracing::info!("Successfully registered with Meridian");
        Ok(())
    }
}
