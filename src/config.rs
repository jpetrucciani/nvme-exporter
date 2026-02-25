use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use clap::ValueEnum;

use crate::nvme::error::NvmeError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_address: SocketAddr,
    pub devices: String,
    pub discovery_interval: Duration,
    pub stale_device_grace: Duration,
    pub collect_namespace: bool,
    pub collect_error_log: bool,
    pub collect_self_test: bool,
    pub log_level: String,
    pub log_format: LogFormat,
    pub ioctl_timeout: Duration,
}

impl Config {
    pub fn parse() -> Result<Self, NvmeError> {
        let args = CliArgs::parse();

        let listen_address = SocketAddr::from_str(&args.listen_address).map_err(|error| {
            NvmeError::Parse(format!(
                "invalid listen address '{}': {}",
                args.listen_address, error
            ))
        })?;
        if args.discovery_interval == 0 {
            return Err(NvmeError::Parse(
                "discovery interval must be greater than zero".to_string(),
            ));
        }
        if args.stale_device_grace == 0 {
            return Err(NvmeError::Parse(
                "stale-device-grace must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            listen_address,
            devices: args.devices,
            discovery_interval: Duration::from_secs(args.discovery_interval),
            stale_device_grace: Duration::from_secs(args.stale_device_grace),
            collect_namespace: args.collect_namespace,
            collect_error_log: args.collect_error_log,
            collect_self_test: args.collect_self_test,
            log_level: args.log_level,
            log_format: args.log_format,
            ioctl_timeout: Duration::from_millis(5000),
        })
    }
}

#[derive(Clone, Debug, Parser)]
#[command(name = "nvme-exporter")]
#[command(about = "Prometheus exporter for NVMe health metrics")]
struct CliArgs {
    #[arg(
        short = 'l',
        long = "listen-address",
        env = "NVME_EXPORTER_LISTEN_ADDRESS",
        default_value = "0.0.0.0:9998"
    )]
    listen_address: String,

    #[arg(
        short = 'd',
        long = "devices",
        env = "NVME_EXPORTER_DEVICES",
        default_value = "/dev/nvme*"
    )]
    devices: String,

    #[arg(
        long = "discovery-interval",
        env = "NVME_EXPORTER_DISCOVERY_INTERVAL",
        default_value_t = 30_u64
    )]
    discovery_interval: u64,

    #[arg(
        long = "collect-namespace",
        env = "NVME_EXPORTER_COLLECT_NAMESPACE",
        default_value_t = true,
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    collect_namespace: bool,

    #[arg(
        long = "collect-error-log",
        env = "NVME_EXPORTER_COLLECT_ERROR_LOG",
        default_value_t = true,
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    collect_error_log: bool,

    #[arg(
        long = "collect-self-test",
        env = "NVME_EXPORTER_COLLECT_SELF_TEST",
        default_value_t = true,
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    collect_self_test: bool,

    #[arg(
        long = "stale-device-grace",
        env = "NVME_EXPORTER_STALE_DEVICE_GRACE",
        default_value_t = 300_u64
    )]
    stale_device_grace: u64,

    #[arg(
        long = "log-level",
        env = "NVME_EXPORTER_LOG_LEVEL",
        default_value = "info"
    )]
    log_level: String,

    #[arg(
        long = "log-format",
        env = "NVME_EXPORTER_LOG_FORMAT",
        value_enum,
        default_value_t = LogFormat::Text
    )]
    log_format: LogFormat,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::config::CliArgs;

    #[test]
    fn defaults_enable_optional_collectors() {
        let args = CliArgs::parse_from(["nvme-exporter"]);
        assert!(args.collect_namespace);
        assert!(args.collect_error_log);
        assert!(args.collect_self_test);
    }

    #[test]
    fn bool_flags_can_be_disabled() {
        let args = CliArgs::parse_from([
            "nvme-exporter",
            "--collect-namespace=false",
            "--collect-error-log=false",
            "--collect-self-test=false",
        ]);
        assert!(!args.collect_namespace);
        assert!(!args.collect_error_log);
        assert!(!args.collect_self_test);
    }
}
