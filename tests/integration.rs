use nvme_exporter::metrics::encode_report;
use nvme_exporter::metrics::DeviceSnapshot;
use nvme_exporter::metrics::ErrorLogSnapshot;
use nvme_exporter::metrics::NamespaceSnapshot;
use nvme_exporter::metrics::ScrapeReport;
use nvme_exporter::metrics::SelfTestSnapshot;
use nvme_exporter::nvme::types::ErrorLogSummary;
use nvme_exporter::nvme::types::IdentifyController;
use nvme_exporter::nvme::types::IdentifyNamespace;
use nvme_exporter::nvme::types::SelfTestLogSummary;
use nvme_exporter::nvme::types::SmartLog;

#[test]
fn fixture_replay_report_emits_expected_metrics() {
    let identify = IdentifyController::parse(include_bytes!("fixture/id_ctrl.bin"))
        .expect("fixture id_ctrl should parse");
    let namespace = IdentifyNamespace::parse(include_bytes!("fixture/id_ns.bin"))
        .expect("fixture id_ns should parse");
    let smart =
        SmartLog::parse(include_bytes!("fixture/smart.bin")).expect("fixture smart should parse");
    let error = ErrorLogSummary::parse(include_bytes!("fixture/error.bin"))
        .expect("fixture error should parse");
    let self_test = SelfTestLogSummary::parse(include_bytes!("fixture/selftest.bin"))
        .expect("fixture selftest should parse");

    let report = ScrapeReport {
        duration_seconds: 0.42,
        success: true,
        discovered_device_count: 1,
        devices: vec![DeviceSnapshot {
            device: "nvme0".to_string(),
            model: identify.model.clone(),
            serial: identify.serial.clone(),
            firmware: identify.firmware_revision.clone(),
            accessible: true,
            smart: Some(smart),
            namespaces: vec![NamespaceSnapshot {
                namespace: "nvme0n1".to_string(),
                nsze: namespace.nsze,
                ncap: namespace.ncap,
                nuse: namespace.nuse,
            }],
            error_log: Some(ErrorLogSnapshot {
                non_zero_entries: error.non_zero_entries,
                max_error_count: error.max_error_count,
            }),
            self_test: Some(SelfTestSnapshot {
                current_operation: self_test.current_operation,
                current_completion_ratio: self_test.current_completion_ratio,
            }),
        }],
        collect_namespace: true,
        collect_error_log: true,
        collect_self_test: true,
    };

    let output = encode_report(&report).expect("fixture report should encode");

    let expected_info = format!(
        "nvme_info{{device=\"nvme0\",firmware=\"{}\",model=\"{}\",serial=\"{}\"}} 1",
        prometheus_escape(&identify.firmware_revision),
        prometheus_escape(&identify.model),
        prometheus_escape(&identify.serial)
    );
    assert!(output.contains(&expected_info));
    assert!(output.contains("nvme_device_accessible{device=\"nvme0\"} 1"));
    assert!(output.contains("nvme_namespace_size_sectors{device=\"nvme0\",namespace=\"nvme0n1\"}"));
    assert!(output.contains(&format!(
        "nvme_error_log_non_zero_entries{{device=\"nvme0\"}} {}",
        error.non_zero_entries
    )));
    assert!(output.contains(&format!(
        "nvme_self_test_current_operation{{device=\"nvme0\"}} {}",
        self_test.current_operation
    )));
    assert!(output.contains("nvme_exporter_scrape_success 1"));
}

#[test]
fn stale_device_snapshot_is_marked_inaccessible() {
    let report = ScrapeReport {
        duration_seconds: 0.11,
        success: false,
        discovered_device_count: 0,
        devices: vec![DeviceSnapshot {
            device: "nvme9".to_string(),
            model: "stale".to_string(),
            serial: "stale".to_string(),
            firmware: "stale".to_string(),
            accessible: false,
            smart: None,
            namespaces: Vec::new(),
            error_log: None,
            self_test: None,
        }],
        collect_namespace: true,
        collect_error_log: true,
        collect_self_test: true,
    };

    let output = encode_report(&report).expect("stale report should encode");

    assert!(output.contains("nvme_device_accessible{device=\"nvme9\"} 0"));
    assert!(output.contains("nvme_exporter_scrape_success 0"));
    assert!(!output.contains("nvme_temperature_celsius{device=\"nvme9\"}"));
}

fn prometheus_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}
