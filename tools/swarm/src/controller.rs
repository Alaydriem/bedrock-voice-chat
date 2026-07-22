use std::sync::Arc;

use crate::config::SwarmConfig;
use crate::job::AgentJob;
use crate::lxd_client::LxdClient;
use crate::metrics_scrape::{MetricsScrape, MetricsSnapshot};
use crate::minter::CodeMinter;
use crate::report::AgentReport;

/// Orchestrates a whole run from the config file: obtain codes, launch an
/// ephemeral LXD container per unit of work on each target, push the binaries +
/// job, exec the agent, collect its report, and scrape the server's routing
/// metrics before and after.
pub struct SwarmController {
    config: SwarmConfig,
    codes: Vec<(String, String)>,
    ca_pem: Option<String>,
    client_bin: Arc<Vec<u8>>,
    swarm_bin: Arc<Vec<u8>>,
    cloud_init: Option<String>,
}

impl SwarmController {
    const CODE_MARGIN_SECS: u64 = 300;

    pub async fn prepare(
        config: SwarmConfig,
        codes_path: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let ca_pem = match &config.ca {
            Some(path) => Some(
                std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("reading ca {}: {}", path, e))?,
            ),
            None => None,
        };

        let client_bin = Arc::new(std::fs::read(&config.client_bin).map_err(|e| {
            anyhow::anyhow!("reading client_bin {}: {}", config.client_bin, e)
        })?);
        let swarm_bin = Arc::new(
            std::fs::read(&config.swarm_bin)
                .map_err(|e| anyhow::anyhow!("reading swarm_bin {}: {}", config.swarm_bin, e))?,
        );

        let cloud_init = match &config.lxd.cloud_init {
            Some(path) => Some(
                std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("reading cloud_init {}: {}", path, e))?,
            ),
            None => None,
        };

        let codes = match codes_path {
            Some(path) => Self::read_codes(&path)?,
            None => {
                let names: Vec<String> =
                    (0..config.total_bots()).map(|i| config.bot_name(i)).collect();
                eprintln!("[controller] minting {} codes...", names.len());
                let minter = CodeMinter::new(&config)?;
                minter
                    .mint(&names, config.duration_secs + Self::CODE_MARGIN_SECS)
                    .await?
            }
        };

        if codes.len() < config.total_bots() {
            return Err(anyhow::anyhow!(
                "have {} codes but config needs {} bots",
                codes.len(),
                config.total_bots()
            ));
        }

        Ok(Self {
            config,
            codes,
            ca_pem,
            client_bin,
            swarm_bin,
            cloud_init,
        })
    }

    fn read_codes(path: &str) -> Result<Vec<(String, String)>, anyhow::Error> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading codes {}: {}", path, e))?;
        let mut out = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (name, code) = line
                .split_once('\t')
                .ok_or_else(|| anyhow::anyhow!("codes line missing tab: {}", line))?;
            out.push((name.to_string(), code.to_string()));
        }
        Ok(out)
    }

    // LXD instance names must be a valid hostname: lowercase alnum + hyphen.
    fn sanitize(name: &str) -> String {
        let s: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        s.trim_matches('-').to_string()
    }

    fn job_for(&self, offset: usize, bots: usize) -> AgentJob {
        AgentJob {
            server: self.config.server.clone(),
            access_token: self.config.access_token.clone(),
            ca_pem: self.ca_pem.clone(),
            group_size: self.config.group_size,
            offset,
            duration_secs: self.config.duration_secs,
            codes: self.codes[offset..offset + bots].to_vec(),
        }
    }

    // Provision one container, run one agent in it, tear it down. `label` is the
    // human name for the report; `instance` is the LXD instance name.
    async fn run_container(
        lxd: Arc<LxdClient>,
        instance: String,
        label: String,
        cloud_init: Option<String>,
        client_bin: Arc<Vec<u8>>,
        swarm_bin: Arc<Vec<u8>>,
        job: AgentJob,
    ) -> Result<AgentReport, anyhow::Error> {
        eprintln!("[controller] launching {}", instance);
        lxd.launch(&instance, cloud_init.as_deref()).await?;

        let result = Self::drive_container(&lxd, &instance, &label, &client_bin, &swarm_bin, &job)
            .await;

        // Always tear the container down (ephemeral -> auto-deletes on stop).
        if let Err(e) = lxd.stop(&instance).await {
            eprintln!("[controller] warning: stopping {} failed: {}", instance, e);
        }
        result
    }

    async fn drive_container(
        lxd: &LxdClient,
        instance: &str,
        label: &str,
        client_bin: &[u8],
        swarm_bin: &[u8],
        job: &AgentJob,
    ) -> Result<AgentReport, anyhow::Error> {
        // Let cloud-init finish installing runtime deps before running the bot.
        let _ = lxd
            .exec(instance, vec!["cloud-init".into(), "status".into(), "--wait".into()])
            .await;

        lxd.push_file(instance, "/root/swarm", swarm_bin.to_vec(), 0o755)
            .await?;
        lxd.push_file(instance, "/root/bvc_client_e2e", client_bin.to_vec(), 0o755)
            .await?;
        let job_json = serde_json::to_vec(job)
            .map_err(|e| anyhow::anyhow!("serializing job for {}: {}", instance, e))?;
        lxd.push_file(instance, "/root/job.json", job_json, 0o644)
            .await?;

        // xvfb-run gives the Wry/GTK client a display even though its window is
        // cleared; the agent reads its job from stdin (redirected file).
        let cmd = vec![
            "xvfb-run".to_string(),
            "-a".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            "cd /root && ./swarm agent --bin ./bvc_client_e2e < job.json".to_string(),
        ];
        let out = lxd.exec(instance, cmd).await?;
        if out.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "agent in {} exited {}: {}",
                instance,
                out.exit_code,
                out.stderr.lines().last().unwrap_or("")
            ));
        }

        let report_line = out
            .stdout
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("agent in {} produced no report", instance))?;
        let mut report: AgentReport = serde_json::from_str(report_line.trim())
            .map_err(|e| anyhow::anyhow!("parsing report from {}: {}", instance, e))?;
        report.host = label.to_string();
        Ok(report)
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        let scrape = MetricsScrape::new(&self.config.server, self.ca_pem.as_deref())?;
        let before = scrape.snapshot().await.unwrap_or_default();
        eprintln!(
            "[controller] before: frames_routed={} drops={}",
            before.frames_routed, before.recipient_drops
        );

        let pid = std::process::id();
        let mut offset = 0usize;
        let mut handles = Vec::new();

        for target in &self.config.target {
            let lxd = Arc::new(LxdClient::new(&self.config.lxd, target)?);
            for c in 0..target.containers {
                let bots = target.bots_per_container;
                let job = self.job_for(offset, bots);
                offset += bots;

                let instance = format!("swarm-{}-c{}-{}", Self::sanitize(&target.name), offset, pid);
                let label = format!("{}#{}", target.name, c);
                let lxd = lxd.clone();
                let cloud_init = self.cloud_init.clone();
                let client_bin = self.client_bin.clone();
                let swarm_bin = self.swarm_bin.clone();

                handles.push(tokio::spawn(async move {
                    Self::run_container(
                        lxd, instance, label, cloud_init, client_bin, swarm_bin, job,
                    )
                    .await
                }));
            }
        }

        let mut reports = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(report)) => reports.push(report),
                Ok(Err(e)) => eprintln!("[controller] container error: {}", e),
                Err(e) => eprintln!("[controller] container task panicked: {}", e),
            }
        }

        let after = scrape.snapshot().await.unwrap_or_default();
        let delta = after.delta(&before);

        Self::print_summary(&reports, &delta);
        Ok(())
    }

    fn print_summary(reports: &[AgentReport], delta: &MetricsSnapshot) {
        let total_bots: usize = reports.iter().map(|r| r.bots.len()).sum();
        let total_connected: usize = reports.iter().map(|r| r.connected).sum();
        let total_received: u64 = reports.iter().map(|r| r.total_frames_received).sum();
        let total_sent: u64 = reports.iter().map(|r| r.total_frames_sent).sum();

        println!("==================== SWARM RESULTS ====================");
        for r in reports {
            println!(
                "container {:<24} {:>3}/{:<3} connected  sent={:>8}  recv={:>10}",
                r.host,
                r.connected,
                r.bots.len(),
                r.total_frames_sent,
                r.total_frames_received
            );
        }
        println!("-------------------------------------------------------");
        println!(
            "TOTAL {:>3}/{:<3} connected  sent={}  recv={}",
            total_connected, total_bots, total_sent, total_received
        );
        println!();
        println!("server-side routing over the run window:");
        println!("  frames routed:       {}", delta.frames_routed);
        println!("  recipient drops:     {}", delta.recipient_drops);
        println!("  mean route duration: {:.1} µs", delta.mean_route_us());
        println!("=======================================================");
    }
}
