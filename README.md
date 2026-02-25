# nvme-exporter

[![uses nix](https://img.shields.io/badge/uses-nix-%237EBAE4)](https://nixos.org/)
![rust](https://img.shields.io/badge/Rust-1.95%2B-orange.svg)

`nvme-exporter` is a Prometheus exporter for NVMe health metrics using direct Linux ioctls.

## Features

- Direct `NVME_IOCTL_ADMIN_CMD` access, no `nvme-cli` runtime dependency
- On-scrape collection model for fresh SMART data
- Auto-discovery via `/sys/class/nvme` and `/dev/nvme*`
- Optional namespace, error log, and self-test collection
- Stale device retention with `nvme_device_accessible=0`

## Requirements

- Linux with NVMe character devices (`/dev/nvme*`)
- Permission to issue NVMe admin ioctls (`CAP_SYS_RAWIO` or root)

The exporter exits at startup if no readable NVMe controllers are found.

## Install

Download a release artifact for your platform, extract it, and place `nvme-exporter` on your PATH.

Example:

```bash
sudo install -m 0755 nvme-exporter /usr/local/bin/nvme-exporter
```

## Running

```bash
nvme-exporter \
  --listen-address 0.0.0.0:9998 \
  --devices "/dev/nvme*" \
  --discovery-interval 30 \
  --stale-device-grace 300
```

## Build From Source

```bash
cargo build --release
```

## Environment Variables

All runtime options are available as environment variables:

- `NVME_EXPORTER_LISTEN_ADDRESS`
- `NVME_EXPORTER_DEVICES`
- `NVME_EXPORTER_DISCOVERY_INTERVAL`
- `NVME_EXPORTER_COLLECT_NAMESPACE`
- `NVME_EXPORTER_COLLECT_ERROR_LOG`
- `NVME_EXPORTER_COLLECT_SELF_TEST`
- `NVME_EXPORTER_STALE_DEVICE_GRACE`
- `NVME_EXPORTER_LOG_LEVEL`
- `NVME_EXPORTER_LOG_FORMAT`

## Endpoints

- `GET /metrics`
- `GET /health`
- `GET /`

## Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: nvme
    scrape_interval: 60s
    static_configs:
      - targets:
          - localhost:9998
```

## Contrib

- `contrib/nvme-exporter.service`: baseline systemd unit
- `contrib/nvme-exporter.json`: Grafana dashboard
- `contrib/alerts.yml`: example Prometheus alerting rules
