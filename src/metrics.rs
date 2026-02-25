use prometheus::CounterVec;
use prometheus::Encoder;
use prometheus::Gauge;
use prometheus::GaugeVec;
use prometheus::Opts;
use prometheus::Registry;
use prometheus::TextEncoder;

use crate::nvme::error::NvmeError;
use crate::nvme::types::SmartLog;

#[derive(Clone, Debug)]
pub struct NamespaceSnapshot {
    pub namespace: String,
    pub nsze: u64,
    pub ncap: u64,
    pub nuse: u64,
}

#[derive(Clone, Debug)]
pub struct ErrorLogSnapshot {
    pub non_zero_entries: u64,
    pub max_error_count: u64,
}

#[derive(Clone, Debug)]
pub struct SelfTestSnapshot {
    pub current_operation: u8,
    pub current_completion_ratio: f64,
}

#[derive(Clone, Debug)]
pub struct DeviceSnapshot {
    pub device: String,
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub accessible: bool,
    pub smart: Option<SmartLog>,
    pub namespaces: Vec<NamespaceSnapshot>,
    pub error_log: Option<ErrorLogSnapshot>,
    pub self_test: Option<SelfTestSnapshot>,
}

#[derive(Clone, Debug)]
pub struct ScrapeReport {
    pub duration_seconds: f64,
    pub success: bool,
    pub discovered_device_count: usize,
    pub devices: Vec<DeviceSnapshot>,
    pub collect_namespace: bool,
    pub collect_error_log: bool,
    pub collect_self_test: bool,
}

pub fn encode_report(report: &ScrapeReport) -> Result<String, NvmeError> {
    let registry = Registry::new();

    let info = register_gauge_vec(
        &registry,
        "nvme_info",
        "NVMe device information",
        &["device", "model", "serial", "firmware"],
    )?;

    let critical_warning = register_gauge_vec(
        &registry,
        "nvme_critical_warning",
        "Raw critical warning bitfield",
        &["device"],
    )?;
    let critical_warning_available_spare = register_gauge_vec(
        &registry,
        "nvme_critical_warning_available_spare",
        "Critical warning bit 0",
        &["device"],
    )?;
    let critical_warning_temperature = register_gauge_vec(
        &registry,
        "nvme_critical_warning_temperature",
        "Critical warning bit 1",
        &["device"],
    )?;
    let critical_warning_reliability = register_gauge_vec(
        &registry,
        "nvme_critical_warning_reliability",
        "Critical warning bit 2",
        &["device"],
    )?;
    let critical_warning_read_only = register_gauge_vec(
        &registry,
        "nvme_critical_warning_read_only",
        "Critical warning bit 3",
        &["device"],
    )?;
    let critical_warning_volatile_backup = register_gauge_vec(
        &registry,
        "nvme_critical_warning_volatile_backup",
        "Critical warning bit 4",
        &["device"],
    )?;

    let temperature_celsius = register_gauge_vec(
        &registry,
        "nvme_temperature_celsius",
        "NVMe composite temperature in Celsius",
        &["device"],
    )?;
    let temperature_sensor_celsius = register_gauge_vec(
        &registry,
        "nvme_temperature_sensor_celsius",
        "NVMe temperature sensor readings in Celsius",
        &["device", "sensor"],
    )?;
    let available_spare_ratio = register_gauge_vec(
        &registry,
        "nvme_available_spare_ratio",
        "Available spare ratio",
        &["device"],
    )?;
    let available_spare_threshold_ratio = register_gauge_vec(
        &registry,
        "nvme_available_spare_threshold_ratio",
        "Available spare threshold ratio",
        &["device"],
    )?;
    let percentage_used_ratio = register_gauge_vec(
        &registry,
        "nvme_percentage_used_ratio",
        "Percentage used ratio, can be greater than 1.0",
        &["device"],
    )?;
    let healthy = register_gauge_vec(
        &registry,
        "nvme_healthy",
        "Derived health indicator",
        &["device"],
    )?;

    let data_units_read_total = register_counter_vec(
        &registry,
        "nvme_data_units_read_total",
        "Data units read",
        &["device"],
    )?;
    let data_units_written_total = register_counter_vec(
        &registry,
        "nvme_data_units_written_total",
        "Data units written",
        &["device"],
    )?;
    let host_read_commands_total = register_counter_vec(
        &registry,
        "nvme_host_read_commands_total",
        "Host read commands",
        &["device"],
    )?;
    let host_write_commands_total = register_counter_vec(
        &registry,
        "nvme_host_write_commands_total",
        "Host write commands",
        &["device"],
    )?;
    let controller_busy_time_seconds_total = register_counter_vec(
        &registry,
        "nvme_controller_busy_time_seconds_total",
        "Controller busy time in seconds",
        &["device"],
    )?;
    let power_cycles_total = register_counter_vec(
        &registry,
        "nvme_power_cycles_total",
        "Power cycle count",
        &["device"],
    )?;
    let power_on_hours_total = register_counter_vec(
        &registry,
        "nvme_power_on_hours_total",
        "Power on hours",
        &["device"],
    )?;
    let unsafe_shutdowns_total = register_counter_vec(
        &registry,
        "nvme_unsafe_shutdowns_total",
        "Unsafe shutdown count",
        &["device"],
    )?;
    let media_errors_total = register_counter_vec(
        &registry,
        "nvme_media_errors_total",
        "Media error count",
        &["device"],
    )?;
    let error_log_entries_total = register_counter_vec(
        &registry,
        "nvme_error_log_entries_total",
        "Error log entries",
        &["device"],
    )?;
    let warning_temperature_time_minutes_total = register_counter_vec(
        &registry,
        "nvme_warning_temperature_time_minutes_total",
        "Warning temperature time in minutes",
        &["device"],
    )?;
    let critical_temperature_time_minutes_total = register_counter_vec(
        &registry,
        "nvme_critical_temperature_time_minutes_total",
        "Critical temperature time in minutes",
        &["device"],
    )?;
    let thermal_mgmt_t1_transitions_total = register_counter_vec(
        &registry,
        "nvme_thermal_mgmt_t1_transitions_total",
        "Thermal management T1 transitions",
        &["device"],
    )?;
    let thermal_mgmt_t2_transitions_total = register_counter_vec(
        &registry,
        "nvme_thermal_mgmt_t2_transitions_total",
        "Thermal management T2 transitions",
        &["device"],
    )?;
    let thermal_mgmt_t1_time_seconds_total = register_counter_vec(
        &registry,
        "nvme_thermal_mgmt_t1_time_seconds_total",
        "Thermal management T1 total time in seconds",
        &["device"],
    )?;
    let thermal_mgmt_t2_time_seconds_total = register_counter_vec(
        &registry,
        "nvme_thermal_mgmt_t2_time_seconds_total",
        "Thermal management T2 total time in seconds",
        &["device"],
    )?;

    let namespace_size = register_gauge_vec(
        &registry,
        "nvme_namespace_size_sectors",
        "Namespace size in LBAs",
        &["device", "namespace"],
    )?;
    let namespace_capacity = register_gauge_vec(
        &registry,
        "nvme_namespace_capacity_sectors",
        "Namespace capacity in LBAs",
        &["device", "namespace"],
    )?;
    let namespace_utilization = register_gauge_vec(
        &registry,
        "nvme_namespace_utilization_sectors",
        "Namespace utilization in LBAs",
        &["device", "namespace"],
    )?;

    let device_accessible = register_gauge_vec(
        &registry,
        "nvme_device_accessible",
        "Whether the device is currently readable",
        &["device"],
    )?;
    let error_log_non_zero_entries = register_gauge_vec(
        &registry,
        "nvme_error_log_non_zero_entries",
        "Number of non-zero entries in log page 0x01",
        &["device"],
    )?;
    let error_log_max_error_count = register_gauge_vec(
        &registry,
        "nvme_error_log_max_error_count",
        "Largest error count found in log page 0x01",
        &["device"],
    )?;
    let self_test_current_operation = register_gauge_vec(
        &registry,
        "nvme_self_test_current_operation",
        "Current self-test operation from log page 0x06",
        &["device"],
    )?;
    let self_test_current_completion_ratio = register_gauge_vec(
        &registry,
        "nvme_self_test_current_completion_ratio",
        "Current self-test completion ratio from log page 0x06",
        &["device"],
    )?;

    let scrape_duration = register_gauge(
        &registry,
        "nvme_exporter_scrape_duration_seconds",
        "Time to collect all metrics",
    )?;
    let scrape_success = register_gauge(
        &registry,
        "nvme_exporter_scrape_success",
        "1 if scrape succeeded, 0 if errors occurred",
    )?;
    let device_count = register_gauge(
        &registry,
        "nvme_exporter_device_count",
        "Number of NVMe controllers discovered",
    )?;

    for device in &report.devices {
        info.with_label_values(&[
            &device.device,
            &device.model,
            &device.serial,
            &device.firmware,
        ])
        .set(1.0);

        device_accessible
            .with_label_values(&[&device.device])
            .set(bool_to_f64(device.accessible));

        if let Some(smart) = &device.smart {
            critical_warning
                .with_label_values(&[&device.device])
                .set(f64::from(smart.critical_warning));
            critical_warning_available_spare
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.critical_warning_available_spare()));
            critical_warning_temperature
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.critical_warning_temperature()));
            critical_warning_reliability
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.critical_warning_reliability()));
            critical_warning_read_only
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.critical_warning_read_only()));
            critical_warning_volatile_backup
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.critical_warning_volatile_backup()));

            if let Some(temp) = smart.temperature_celsius() {
                temperature_celsius
                    .with_label_values(&[&device.device])
                    .set(temp);
            }
            let mut sensor_index = 0_usize;
            while sensor_index < 8 {
                if let Some(temp) = smart.sensor_celsius(sensor_index) {
                    let sensor_label = (sensor_index + 1).to_string();
                    temperature_sensor_celsius
                        .with_label_values(&[&device.device, &sensor_label])
                        .set(temp);
                }
                sensor_index += 1;
            }

            available_spare_ratio
                .with_label_values(&[&device.device])
                .set(smart.available_spare_ratio());
            available_spare_threshold_ratio
                .with_label_values(&[&device.device])
                .set(smart.available_spare_threshold_ratio());
            percentage_used_ratio
                .with_label_values(&[&device.device])
                .set(smart.percent_used_ratio());
            healthy
                .with_label_values(&[&device.device])
                .set(bool_to_f64(smart.healthy()));

            data_units_read_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.data_units_read));
            data_units_written_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.data_units_written));
            host_read_commands_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.host_read_commands));
            host_write_commands_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.host_write_commands));
            controller_busy_time_seconds_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.ctrl_busy_time_minutes) * 60.0);
            power_cycles_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.power_cycles));
            power_on_hours_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.power_on_hours));
            unsafe_shutdowns_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.unsafe_shutdowns));
            media_errors_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.media_errors));
            error_log_entries_total
                .with_label_values(&[&device.device])
                .inc_by(u128_to_f64(smart.num_err_log_entries));
            warning_temperature_time_minutes_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.warning_temp_time_minutes));
            critical_temperature_time_minutes_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.critical_comp_time_minutes));
            thermal_mgmt_t1_transitions_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.thm_temp1_trans_count));
            thermal_mgmt_t2_transitions_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.thm_temp2_trans_count));
            thermal_mgmt_t1_time_seconds_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.thm_temp1_total_time_seconds));
            thermal_mgmt_t2_time_seconds_total
                .with_label_values(&[&device.device])
                .inc_by(f64::from(smart.thm_temp2_total_time_seconds));
        }

        if report.collect_namespace {
            for namespace in &device.namespaces {
                namespace_size
                    .with_label_values(&[&device.device, &namespace.namespace])
                    .set(namespace.nsze as f64);
                namespace_capacity
                    .with_label_values(&[&device.device, &namespace.namespace])
                    .set(namespace.ncap as f64);
                namespace_utilization
                    .with_label_values(&[&device.device, &namespace.namespace])
                    .set(namespace.nuse as f64);
            }
        }

        if report.collect_error_log {
            if let Some(error_log) = &device.error_log {
                error_log_non_zero_entries
                    .with_label_values(&[&device.device])
                    .set(error_log.non_zero_entries as f64);
                error_log_max_error_count
                    .with_label_values(&[&device.device])
                    .set(error_log.max_error_count as f64);
            }
        }

        if report.collect_self_test {
            if let Some(self_test) = &device.self_test {
                self_test_current_operation
                    .with_label_values(&[&device.device])
                    .set(f64::from(self_test.current_operation));
                self_test_current_completion_ratio
                    .with_label_values(&[&device.device])
                    .set(self_test.current_completion_ratio);
            }
        }
    }

    scrape_duration.set(report.duration_seconds);
    scrape_success.set(bool_to_f64(report.success));
    device_count.set(report.discovered_device_count as f64);

    let metric_families = registry.gather();
    let mut buffer = Vec::<u8>::new();
    TextEncoder::new()
        .encode(&metric_families, &mut buffer)
        .map_err(|error| NvmeError::Internal(format!("failed to encode metrics: {}", error)))?;

    String::from_utf8(buffer)
        .map_err(|error| NvmeError::Internal(format!("metrics are not valid utf8: {}", error)))
}

fn register_gauge(registry: &Registry, name: &str, help: &str) -> Result<Gauge, NvmeError> {
    let gauge = Gauge::with_opts(Opts::new(name, help))
        .map_err(|error| NvmeError::Internal(format!("failed to create {}: {}", name, error)))?;
    registry
        .register(Box::new(gauge.clone()))
        .map_err(|error| NvmeError::Internal(format!("failed to register {}: {}", name, error)))?;
    Ok(gauge)
}

fn register_gauge_vec(
    registry: &Registry,
    name: &str,
    help: &str,
    labels: &[&str],
) -> Result<GaugeVec, NvmeError> {
    let metric = GaugeVec::new(Opts::new(name, help), labels)
        .map_err(|error| NvmeError::Internal(format!("failed to create {}: {}", name, error)))?;
    registry
        .register(Box::new(metric.clone()))
        .map_err(|error| NvmeError::Internal(format!("failed to register {}: {}", name, error)))?;
    Ok(metric)
}

fn register_counter_vec(
    registry: &Registry,
    name: &str,
    help: &str,
    labels: &[&str],
) -> Result<CounterVec, NvmeError> {
    let metric = CounterVec::new(Opts::new(name, help), labels)
        .map_err(|error| NvmeError::Internal(format!("failed to create {}: {}", name, error)))?;
    registry
        .register(Box::new(metric.clone()))
        .map_err(|error| NvmeError::Internal(format!("failed to register {}: {}", name, error)))?;
    Ok(metric)
}

fn bool_to_f64(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn u128_to_f64(value: u128) -> f64 {
    value as f64
}
