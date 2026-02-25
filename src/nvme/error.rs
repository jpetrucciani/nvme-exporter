use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum NvmeError {
    Io {
        context: String,
        source: std::io::Error,
    },
    Ioctl {
        device: String,
        source: std::io::Error,
    },
    PermissionDenied {
        device: String,
    },
    UnexpectedSize {
        expected: usize,
        actual: usize,
    },
    InvalidData(String),
    Parse(String),
    NoReadableDevices,
    Internal(String),
}

impl NvmeError {
    pub fn io_context(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub fn io_path(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            context: path.display().to_string(),
            source,
        }
    }
}

impl fmt::Display for NvmeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NvmeError::Io { context, source } => write!(f, "io error ({}): {}", context, source),
            NvmeError::Ioctl { device, source } => {
                write!(f, "ioctl failed on {}: {}", device, source)
            }
            NvmeError::PermissionDenied { device } => {
                write!(
                    f,
                    "permission denied: {} (need CAP_SYS_RAWIO or root)",
                    device
                )
            }
            NvmeError::UnexpectedSize { expected, actual } => {
                write!(
                    f,
                    "unexpected data size: expected {}, got {}",
                    expected, actual
                )
            }
            NvmeError::InvalidData(message) => write!(f, "invalid data: {}", message),
            NvmeError::Parse(message) => write!(f, "parse error: {}", message),
            NvmeError::NoReadableDevices => write!(
                f,
                "no readable NVMe controllers found, ensure CAP_SYS_RAWIO or run as root"
            ),
            NvmeError::Internal(message) => write!(f, "internal error: {}", message),
        }
    }
}

impl std::error::Error for NvmeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NvmeError::Io { source, .. } => Some(source),
            NvmeError::Ioctl { source, .. } => Some(source),
            _ => None,
        }
    }
}
