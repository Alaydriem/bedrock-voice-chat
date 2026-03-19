use crate::config::Meridian;

pub struct MeridianService {
    config: Meridian,
    backend: String,
    tls_port: u32,
    quic_port: u32,
    hostname: String,
}

impl MeridianService {
    pub fn new(
        config: Meridian,
        backend: String,
        tls_port: u32,
        quic_port: u32,
        hostname: String,
    ) -> Self {
        Self {
            config,
            backend,
            tls_port,
            quic_port,
            hostname,
        }
    }

    pub async fn register(&self) -> Result<(), anyhow::Error> {
        let name = nanoid::nanoid!();
        let tcp_addr = format!("{}:{}", self.backend, self.tls_port);
        let udp_addr = format!("{}:{}", self.backend, self.quic_port);

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
