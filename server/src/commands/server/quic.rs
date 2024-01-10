use crate::config::ApplicationConfig;
use tokio::task::JoinHandle;

pub(crate) fn get_task(config: &ApplicationConfig) -> JoinHandle<()> {
    let app_config = config.to_owned();
    return tokio::task::spawn(async move {});
}
