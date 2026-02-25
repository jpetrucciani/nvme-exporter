use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

use tracing::warn;

use crate::config::Config;
use crate::metrics::DeviceSnapshot;
use crate::metrics::ErrorLogSnapshot;
use crate::metrics::NamespaceSnapshot;
use crate::metrics::ScrapeReport;
use crate::metrics::SelfTestSnapshot;
use crate::nvme::device::NvmeDevice;
use crate::nvme::discovery;
use crate::nvme::discovery::NvmeController;
use crate::nvme::error::NvmeError;

pub struct NvmeCollector {
    config: Config,
    state: Mutex<CollectorState>,
}

struct CollectorState {
    discovery_cache: Option<CachedDiscovery>,
    devices: HashMap<String, CachedDevice>,
}

#[derive(Clone)]
struct CachedDiscovery {
    controllers: Vec<NvmeController>,
    expires_at: Instant,
}

#[derive(Clone)]
struct CachedDevice {
    snapshot: DeviceSnapshot,
    last_seen: Instant,
}

impl NvmeCollector {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: Mutex::new(CollectorState {
                discovery_cache: None,
                devices: HashMap::new(),
            }),
        }
    }

    pub fn validate_startup_devices(&self) -> Result<(), NvmeError> {
        let now = Instant::now();
        let controllers = self.load_controllers(now)?;
        if controllers.is_empty() {
            return Err(NvmeError::NoReadableDevices);
        }

        let timeout_ms = u32::try_from(self.config.ioctl_timeout.as_millis())
            .map_err(|_| NvmeError::Parse("ioctl timeout exceeds u32".to_string()))?;
        let mut readable = 0_usize;
        for controller in &controllers {
            if let Ok(device) = NvmeDevice::open(&controller.dev_path) {
                if device.smart_log(timeout_ms).is_ok() {
                    readable += 1;
                }
            }
        }

        if readable == 0 {
            return Err(NvmeError::NoReadableDevices);
        }

        Ok(())
    }

    pub fn scrape(&self) -> Result<String, NvmeError> {
        let started_at = Instant::now();
        let now = Instant::now();
        let controllers = self.load_controllers(now)?;
        let previous_devices = self.load_previous_devices()?;
        let discovered_names: HashSet<String> = controllers
            .iter()
            .map(|controller| controller.name.clone())
            .collect();

        let mut collected_devices: HashMap<String, DeviceSnapshot> = HashMap::new();
        let mut scrape_success = true;

        for controller in &controllers {
            match self.collect_controller(controller) {
                Ok(snapshot) => {
                    collected_devices.insert(controller.name.clone(), snapshot);
                }
                Err(error) => {
                    scrape_success = false;
                    warn!(
                        controller = %controller.name,
                        device = %controller.dev_path.display(),
                        error = %error,
                        "failed to collect device metrics"
                    );
                    let fallback = previous_devices
                        .get(&controller.name)
                        .map(|cached| {
                            let mut snapshot = cached.snapshot.clone();
                            snapshot.accessible = false;
                            snapshot
                        })
                        .unwrap_or_else(|| self.minimal_snapshot(controller, false));
                    collected_devices.insert(controller.name.clone(), fallback);
                }
            }
        }

        let snapshots = self.merge_device_state(now, &discovered_names, collected_devices)?;

        let report = ScrapeReport {
            duration_seconds: started_at.elapsed().as_secs_f64(),
            success: scrape_success,
            discovered_device_count: controllers.len(),
            devices: snapshots,
            collect_namespace: self.config.collect_namespace,
            collect_error_log: self.config.collect_error_log,
            collect_self_test: self.config.collect_self_test,
        };

        crate::metrics::encode_report(&report)
    }

    fn collect_controller(&self, controller: &NvmeController) -> Result<DeviceSnapshot, NvmeError> {
        let device = NvmeDevice::open(&controller.dev_path)?;
        let timeout_ms = u32::try_from(self.config.ioctl_timeout.as_millis())
            .map_err(|_| NvmeError::Parse("ioctl timeout exceeds u32".to_string()))?;

        let identify = match device.identify_controller(timeout_ms) {
            Ok(value) => Some(value),
            Err(error) => {
                warn!(
                    controller = %controller.name,
                    error = %error,
                    "identify controller failed, continuing with discovery labels"
                );
                None
            }
        };

        let smart = device.smart_log(timeout_ms)?;
        let model = identify
            .as_ref()
            .map(|value| value.model.clone())
            .or_else(|| controller.model.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        let serial = identify
            .as_ref()
            .map(|value| value.serial.clone())
            .or_else(|| controller.serial.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        let firmware = identify
            .as_ref()
            .map(|value| value.firmware_revision.clone())
            .or_else(|| controller.firmware.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        let mut namespaces = Vec::<NamespaceSnapshot>::new();
        if self.config.collect_namespace {
            for namespace in &controller.namespaces {
                match device.identify_namespace(namespace.nsid, timeout_ms) {
                    Ok(identify_namespace) => namespaces.push(NamespaceSnapshot {
                        namespace: namespace.name.clone(),
                        nsze: identify_namespace.nsze,
                        ncap: identify_namespace.ncap,
                        nuse: identify_namespace.nuse,
                    }),
                    Err(error) => warn!(
                        controller = %controller.name,
                        namespace = %namespace.name,
                        error = %error,
                        "identify namespace failed"
                    ),
                }
            }
        }

        let error_log = if self.config.collect_error_log {
            match device.error_log(timeout_ms) {
                Ok(value) => Some(ErrorLogSnapshot {
                    non_zero_entries: value.non_zero_entries,
                    max_error_count: value.max_error_count,
                }),
                Err(error) => {
                    warn!(
                        controller = %controller.name,
                        error = %error,
                        "error log collection failed"
                    );
                    None
                }
            }
        } else {
            None
        };

        let self_test = if self.config.collect_self_test {
            match device.self_test_log(timeout_ms) {
                Ok(value) => Some(SelfTestSnapshot {
                    current_operation: value.current_operation,
                    current_completion_ratio: value.current_completion_ratio,
                }),
                Err(error) => {
                    warn!(
                        controller = %controller.name,
                        error = %error,
                        "self-test log collection failed"
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok(DeviceSnapshot {
            device: controller.name.clone(),
            model,
            serial,
            firmware,
            accessible: true,
            smart: Some(smart),
            namespaces,
            error_log,
            self_test,
        })
    }

    fn minimal_snapshot(&self, controller: &NvmeController, accessible: bool) -> DeviceSnapshot {
        DeviceSnapshot {
            device: controller.name.clone(),
            model: controller
                .model
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            serial: controller
                .serial
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            firmware: controller
                .firmware
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            accessible,
            smart: None,
            namespaces: Vec::new(),
            error_log: None,
            self_test: None,
        }
    }

    fn load_previous_devices(&self) -> Result<HashMap<String, CachedDevice>, NvmeError> {
        let state = self
            .state
            .lock()
            .map_err(|error| NvmeError::Internal(format!("collector mutex poisoned: {}", error)))?;
        Ok(state.devices.clone())
    }

    fn merge_device_state(
        &self,
        now: Instant,
        discovered_names: &HashSet<String>,
        collected_devices: HashMap<String, DeviceSnapshot>,
    ) -> Result<Vec<DeviceSnapshot>, NvmeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| NvmeError::Internal(format!("collector mutex poisoned: {}", error)))?;

        for (name, snapshot) in collected_devices {
            state.devices.insert(
                name,
                CachedDevice {
                    snapshot,
                    last_seen: now,
                },
            );
        }

        for (name, cached) in &mut state.devices {
            if !discovered_names.contains(name) {
                cached.snapshot.accessible = false;
            }
        }

        let grace = self.config.stale_device_grace;
        state.devices.retain(|name, cached| {
            if discovered_names.contains(name) {
                true
            } else {
                now.saturating_duration_since(cached.last_seen) <= grace
            }
        });

        let mut snapshots: Vec<DeviceSnapshot> = state
            .devices
            .values()
            .map(|cached| cached.snapshot.clone())
            .collect();
        snapshots.sort_by(|left, right| left.device.cmp(&right.device));
        Ok(snapshots)
    }

    fn load_controllers(&self, now: Instant) -> Result<Vec<NvmeController>, NvmeError> {
        {
            let state = self.state.lock().map_err(|error| {
                NvmeError::Internal(format!("collector mutex poisoned: {}", error))
            })?;
            if let Some(cache) = &state.discovery_cache {
                if now < cache.expires_at {
                    return Ok(cache.controllers.clone());
                }
            }
        }

        let controllers = discovery::discover_controllers(&self.config.devices)?;
        let expires_at = now + self.config.discovery_interval;
        let mut state = self
            .state
            .lock()
            .map_err(|error| NvmeError::Internal(format!("collector mutex poisoned: {}", error)))?;
        state.discovery_cache = Some(CachedDiscovery {
            controllers: controllers.clone(),
            expires_at,
        });

        Ok(controllers)
    }
}
