use std::fs::File;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::path::PathBuf;

use crate::nvme::error::NvmeError;
use crate::nvme::ioctl;
use crate::nvme::types::ErrorLogSummary;
use crate::nvme::types::IdentifyController;
use crate::nvme::types::IdentifyNamespace;
use crate::nvme::types::SelfTestLogSummary;
use crate::nvme::types::SmartLog;
use crate::nvme::types::ERROR_LOG_BYTES;
use crate::nvme::types::SELF_TEST_LOG_BYTES;
use crate::nvme::types::SMART_LOG_BYTES;

const LID_ERROR_INFORMATION: u8 = 0x01;
const LID_SMART_HEALTH: u8 = 0x02;
const LID_SELF_TEST: u8 = 0x06;

pub struct NvmeDevice {
    path: PathBuf,
    file: File,
}

impl NvmeDevice {
    pub fn open(path: &Path) -> Result<Self, NvmeError> {
        let file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|source| NvmeError::io_path(path, source))?;

        Ok(Self {
            path: path.to_path_buf(),
            file,
        })
    }

    pub fn identify_controller(&self, timeout_ms: u32) -> Result<IdentifyController, NvmeError> {
        let bytes =
            ioctl::identify_controller(self.file.as_raw_fd(), &self.path_string(), timeout_ms)?;
        IdentifyController::parse(&bytes)
    }

    pub fn identify_namespace(
        &self,
        nsid: u32,
        timeout_ms: u32,
    ) -> Result<IdentifyNamespace, NvmeError> {
        let bytes = ioctl::identify_namespace(
            self.file.as_raw_fd(),
            &self.path_string(),
            nsid,
            timeout_ms,
        )?;
        IdentifyNamespace::parse(&bytes)
    }

    pub fn smart_log(&self, timeout_ms: u32) -> Result<SmartLog, NvmeError> {
        let bytes = ioctl::get_controller_log_page(
            self.file.as_raw_fd(),
            &self.path_string(),
            LID_SMART_HEALTH,
            SMART_LOG_BYTES,
            timeout_ms,
        )?;
        SmartLog::parse(&bytes)
    }

    pub fn error_log(&self, timeout_ms: u32) -> Result<ErrorLogSummary, NvmeError> {
        let bytes = ioctl::get_controller_log_page(
            self.file.as_raw_fd(),
            &self.path_string(),
            LID_ERROR_INFORMATION,
            ERROR_LOG_BYTES,
            timeout_ms,
        )?;
        ErrorLogSummary::parse(&bytes)
    }

    pub fn self_test_log(&self, timeout_ms: u32) -> Result<SelfTestLogSummary, NvmeError> {
        let bytes = ioctl::get_controller_log_page(
            self.file.as_raw_fd(),
            &self.path_string(),
            LID_SELF_TEST,
            SELF_TEST_LOG_BYTES,
            timeout_ms,
        )?;
        SelfTestLogSummary::parse(&bytes)
    }

    fn path_string(&self) -> String {
        self.path.display().to_string()
    }
}
