use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use glob::Pattern;

use crate::nvme::error::NvmeError;

const SYS_CLASS_NVME: &str = "/sys/class/nvme";

#[derive(Clone, Debug)]
pub struct NvmeNamespace {
    pub name: String,
    pub nsid: u32,
}

#[derive(Clone, Debug)]
pub struct NvmeController {
    pub name: String,
    pub dev_path: PathBuf,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub namespaces: Vec<NvmeNamespace>,
}

pub fn discover_controllers(device_pattern: &str) -> Result<Vec<NvmeController>, NvmeError> {
    let pattern = Pattern::new(device_pattern)
        .map_err(|error| NvmeError::Parse(format!("invalid device pattern: {}", error)))?;

    let mut controllers = discover_from_sysfs(&pattern)?;
    if controllers.is_empty() {
        controllers = discover_from_devfs(&pattern)?;
    }

    controllers.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(controllers)
}

fn discover_from_sysfs(pattern: &Pattern) -> Result<Vec<NvmeController>, NvmeError> {
    let sysfs_dir = Path::new(SYS_CLASS_NVME);
    if !sysfs_dir.exists() {
        return Ok(Vec::new());
    }

    let entries =
        fs::read_dir(sysfs_dir).map_err(|source| NvmeError::io_path(sysfs_dir, source))?;
    let mut controllers = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|source| NvmeError::io_path(sysfs_dir, source))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_controller_name(&name) {
            continue;
        }

        let dev_path = PathBuf::from(format!("/dev/{}", name));
        if !pattern.matches_path(&dev_path) {
            continue;
        }

        let sys_path = entry.path();
        let model = read_attr(sys_path.join("model"));
        let serial = read_attr(sys_path.join("serial"));
        let firmware = read_attr(sys_path.join("firmware_rev"));
        let mut namespaces = discover_namespaces(&name, &sys_path);
        namespaces.sort_by(|left, right| left.name.cmp(&right.name));

        controllers.push(NvmeController {
            name,
            dev_path,
            model,
            serial,
            firmware,
            namespaces,
        });
    }

    Ok(controllers)
}

fn discover_from_devfs(pattern: &Pattern) -> Result<Vec<NvmeController>, NvmeError> {
    let mut controllers = BTreeMap::<String, NvmeController>::new();
    let paths =
        glob::glob("/dev/nvme[0-9]*").map_err(|error| NvmeError::Parse(format!("{}", error)))?;

    for path_result in paths {
        let path = match path_result {
            Ok(value) => value,
            Err(error) => {
                return Err(NvmeError::Parse(format!(
                    "failed to read /dev glob path: {}",
                    error
                )))
            }
        };

        if !pattern.matches_path(&path) {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };
        let name = file_name.to_string_lossy().to_string();
        if !is_controller_name(&name) {
            continue;
        }

        let controller = NvmeController {
            name: name.clone(),
            dev_path: path,
            model: None,
            serial: None,
            firmware: None,
            namespaces: Vec::new(),
        };

        controllers.insert(name, controller);
    }

    Ok(controllers.into_values().collect())
}

fn discover_namespaces(controller_name: &str, controller_sys_path: &Path) -> Vec<NvmeNamespace> {
    let mut namespaces = Vec::new();
    let entries = match fs::read_dir(controller_sys_path) {
        Ok(value) => value,
        Err(_) => return namespaces,
    };

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let namespace_name = entry.file_name().to_string_lossy().to_string();
        let Some(nsid) = parse_namespace_name(controller_name, &namespace_name) else {
            continue;
        };

        namespaces.push(NvmeNamespace {
            name: namespace_name.clone(),
            nsid,
        });
    }

    namespaces
}

fn parse_namespace_name(controller_name: &str, namespace_name: &str) -> Option<u32> {
    let prefix = format!("{}n", controller_name);
    let suffix = namespace_name.strip_prefix(&prefix)?;
    if suffix.is_empty() {
        return None;
    }

    let digits: String = suffix
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }

    digits.parse::<u32>().ok()
}

fn is_controller_name(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("nvme") else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
}

fn read_attr(path: PathBuf) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::nvme::discovery::is_controller_name;
    use crate::nvme::discovery::parse_namespace_name;

    #[test]
    fn parses_namespace_ids() {
        assert_eq!(parse_namespace_name("nvme0", "nvme0n1"), Some(1));
        assert_eq!(parse_namespace_name("nvme12", "nvme12n25"), Some(25));
    }

    #[test]
    fn rejects_invalid_namespace_names() {
        assert_eq!(parse_namespace_name("nvme0", "nvme1n1"), None);
        assert_eq!(parse_namespace_name("nvme0", "nvme0"), None);
        assert_eq!(parse_namespace_name("nvme0", "nvme0np1"), None);
    }

    #[test]
    fn matches_controller_names_only() {
        assert!(is_controller_name("nvme0"));
        assert!(is_controller_name("nvme24"));
        assert!(!is_controller_name("nvme0n1"));
        assert!(!is_controller_name("sda"));
    }
}
