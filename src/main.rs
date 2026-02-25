use std::sync::Arc;

use nvme_exporter::collector::NvmeCollector;
use nvme_exporter::config::Config;
use nvme_exporter::config::LogFormat;
use nvme_exporter::nvme::error::NvmeError;
use nvme_exporter::server;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("nvme-exporter error: {}", error);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), NvmeError> {
    let config = Config::parse()?;
    init_logging(&config)?;

    let collector = Arc::new(NvmeCollector::new(config.clone()));
    collector.validate_startup_devices()?;

    info!(
        listen_address = %config.listen_address,
        devices = %config.devices,
        "starting nvme-exporter"
    );

    server::run_server(&config, collector).await
}

fn init_logging(config: &Config) -> Result<(), NvmeError> {
    let env_filter = EnvFilter::try_new(config.log_level.clone()).map_err(|error| {
        NvmeError::Parse(format!(
            "invalid log level/filter '{}': {}",
            config.log_level, error
        ))
    })?;

    let init_result = match config.log_format {
        LogFormat::Text => tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .try_init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .try_init(),
    };

    init_result
        .map_err(|error| NvmeError::Internal(format!("failed to initialize logging: {}", error)))
}
