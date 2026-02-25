# contrib

Integration artifacts for running and monitoring `nvme-exporter`.

## Contents

### `nvme-exporter.service`

Baseline `systemd` unit for running the exporter as a service.

- Runs `/usr/local/bin/nvme-exporter --listen-address 0.0.0.0:9998`
- Uses a dedicated `nvme-exporter` user/group
- Includes basic hardening and `CAP_SYS_RAWIO` for NVMe admin ioctls

Adjust user/group, binary path, and listen address for your environment.

### `alerts.yml`

Example Prometheus alert rules for the `nvme` job.

- `NvmeDeviceUnhealthy`
- `NvmeSpareRunningLow`
- `NvmeHighWear`
- `NvmeTemperatureWarning`
- `NvmeMediaErrors`
- `NvmeExporterDown`

Tune thresholds and severities for your fleet before enabling in production.

### `nvme-exporter.json`

Grafana dashboard (`NVMe Exporter Overview`) for Prometheus metrics from `nvme-exporter`.

- Template variables: `datasource`, `device`
- Panels include device count, health, accessibility, scrape status, temperature,
  spare percentage, wear, throughput, command rate, media errors, and power-on hours

Import this JSON in Grafana and select your Prometheus datasource.
