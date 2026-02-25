use crate::nvme::error::NvmeError;

pub const SMART_LOG_BYTES: usize = 512;
pub const IDENTIFY_BYTES: usize = 4096;
pub const SELF_TEST_LOG_BYTES: usize = 564;
pub const ERROR_LOG_ENTRY_BYTES: usize = 64;
pub const ERROR_LOG_ENTRIES: usize = 16;
pub const ERROR_LOG_BYTES: usize = ERROR_LOG_ENTRY_BYTES * ERROR_LOG_ENTRIES;

#[derive(Clone, Copy, Debug)]
pub struct SmartLog {
    pub critical_warning: u8,
    pub temperature_kelvin: u16,
    pub avail_spare: u8,
    pub spare_thresh: u8,
    pub percent_used: u8,
    pub data_units_read: u128,
    pub data_units_written: u128,
    pub host_read_commands: u128,
    pub host_write_commands: u128,
    pub ctrl_busy_time_minutes: u128,
    pub power_cycles: u128,
    pub power_on_hours: u128,
    pub unsafe_shutdowns: u128,
    pub media_errors: u128,
    pub num_err_log_entries: u128,
    pub warning_temp_time_minutes: u32,
    pub critical_comp_time_minutes: u32,
    pub temp_sensor_kelvin: [u16; 8],
    pub thm_temp1_trans_count: u32,
    pub thm_temp2_trans_count: u32,
    pub thm_temp1_total_time_seconds: u32,
    pub thm_temp2_total_time_seconds: u32,
}

impl SmartLog {
    pub fn parse(bytes: &[u8]) -> Result<Self, NvmeError> {
        if bytes.len() != SMART_LOG_BYTES {
            return Err(NvmeError::UnexpectedSize {
                expected: SMART_LOG_BYTES,
                actual: bytes.len(),
            });
        }

        let mut temp_sensor_kelvin = [0_u16; 8];
        let mut sensor_index = 0_usize;
        while sensor_index < temp_sensor_kelvin.len() {
            temp_sensor_kelvin[sensor_index] = read_u16_le(bytes, 200 + (sensor_index * 2))?;
            sensor_index += 1;
        }

        Ok(Self {
            critical_warning: read_u8(bytes, 0)?,
            temperature_kelvin: read_u16_le(bytes, 1)?,
            avail_spare: read_u8(bytes, 3)?,
            spare_thresh: read_u8(bytes, 4)?,
            percent_used: read_u8(bytes, 5)?,
            data_units_read: read_u128_le(bytes, 32)?,
            data_units_written: read_u128_le(bytes, 48)?,
            host_read_commands: read_u128_le(bytes, 64)?,
            host_write_commands: read_u128_le(bytes, 80)?,
            ctrl_busy_time_minutes: read_u128_le(bytes, 96)?,
            power_cycles: read_u128_le(bytes, 112)?,
            power_on_hours: read_u128_le(bytes, 128)?,
            unsafe_shutdowns: read_u128_le(bytes, 144)?,
            media_errors: read_u128_le(bytes, 160)?,
            num_err_log_entries: read_u128_le(bytes, 176)?,
            warning_temp_time_minutes: read_u32_le(bytes, 192)?,
            critical_comp_time_minutes: read_u32_le(bytes, 196)?,
            temp_sensor_kelvin,
            thm_temp1_trans_count: read_u32_le(bytes, 216)?,
            thm_temp2_trans_count: read_u32_le(bytes, 220)?,
            thm_temp1_total_time_seconds: read_u32_le(bytes, 224)?,
            thm_temp2_total_time_seconds: read_u32_le(bytes, 228)?,
        })
    }

    pub fn temperature_celsius(&self) -> Option<f64> {
        kelvin_to_celsius(self.temperature_kelvin)
    }

    pub fn sensor_celsius(&self, sensor_index: usize) -> Option<f64> {
        self.temp_sensor_kelvin
            .get(sensor_index)
            .and_then(|value| kelvin_to_celsius(*value))
    }

    pub fn available_spare_ratio(&self) -> f64 {
        f64::from(self.avail_spare) / 100.0
    }

    pub fn available_spare_threshold_ratio(&self) -> f64 {
        f64::from(self.spare_thresh) / 100.0
    }

    pub fn percent_used_ratio(&self) -> f64 {
        f64::from(self.percent_used) / 100.0
    }

    pub fn critical_warning_available_spare(&self) -> bool {
        (self.critical_warning & (1 << 0)) != 0
    }

    pub fn critical_warning_temperature(&self) -> bool {
        (self.critical_warning & (1 << 1)) != 0
    }

    pub fn critical_warning_reliability(&self) -> bool {
        (self.critical_warning & (1 << 2)) != 0
    }

    pub fn critical_warning_read_only(&self) -> bool {
        (self.critical_warning & (1 << 3)) != 0
    }

    pub fn critical_warning_volatile_backup(&self) -> bool {
        (self.critical_warning & (1 << 4)) != 0
    }

    pub fn healthy(&self) -> bool {
        self.critical_warning == 0
            && u16::from(self.avail_spare) >= u16::from(self.spare_thresh)
            && self.media_errors == 0
    }
}

#[derive(Clone, Debug)]
pub struct IdentifyController {
    pub serial: String,
    pub model: String,
    pub firmware_revision: String,
}

impl IdentifyController {
    pub fn parse(bytes: &[u8]) -> Result<Self, NvmeError> {
        if bytes.len() != IDENTIFY_BYTES {
            return Err(NvmeError::UnexpectedSize {
                expected: IDENTIFY_BYTES,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            serial: trim_nvme_ascii(slice::<20>(bytes, 4)?),
            model: trim_nvme_ascii(slice::<40>(bytes, 24)?),
            firmware_revision: trim_nvme_ascii(slice::<8>(bytes, 64)?),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IdentifyNamespace {
    pub nsze: u64,
    pub ncap: u64,
    pub nuse: u64,
}

impl IdentifyNamespace {
    pub fn parse(bytes: &[u8]) -> Result<Self, NvmeError> {
        if bytes.len() != IDENTIFY_BYTES {
            return Err(NvmeError::UnexpectedSize {
                expected: IDENTIFY_BYTES,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            nsze: read_u64_le(bytes, 0)?,
            ncap: read_u64_le(bytes, 8)?,
            nuse: read_u64_le(bytes, 16)?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ErrorLogSummary {
    pub non_zero_entries: u64,
    pub max_error_count: u64,
}

impl ErrorLogSummary {
    pub fn parse(bytes: &[u8]) -> Result<Self, NvmeError> {
        if !bytes.len().is_multiple_of(ERROR_LOG_ENTRY_BYTES) {
            return Err(NvmeError::InvalidData(format!(
                "error log buffer size {} is not divisible by {}",
                bytes.len(),
                ERROR_LOG_ENTRY_BYTES
            )));
        }

        let mut non_zero_entries = 0_u64;
        let mut max_error_count = 0_u64;
        let mut offset = 0_usize;

        while offset < bytes.len() {
            let error_count = read_u64_le(bytes, offset)?;
            if error_count > 0 {
                non_zero_entries += 1;
            }
            if error_count > max_error_count {
                max_error_count = error_count;
            }
            offset += ERROR_LOG_ENTRY_BYTES;
        }

        Ok(Self {
            non_zero_entries,
            max_error_count,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SelfTestLogSummary {
    pub current_operation: u8,
    pub current_completion_ratio: f64,
}

impl SelfTestLogSummary {
    pub fn parse(bytes: &[u8]) -> Result<Self, NvmeError> {
        if bytes.len() != SELF_TEST_LOG_BYTES {
            return Err(NvmeError::UnexpectedSize {
                expected: SELF_TEST_LOG_BYTES,
                actual: bytes.len(),
            });
        }

        let current_operation = read_u8(bytes, 0)?;
        let current_completion = read_u8(bytes, 1)?;

        Ok(Self {
            current_operation,
            current_completion_ratio: f64::from(current_completion) / 100.0,
        })
    }
}

pub fn trim_nvme_ascii(bytes: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(bytes).into_owned();
    while value.ends_with('\0') {
        let _ = value.pop();
    }
    value.trim().to_string()
}

fn kelvin_to_celsius(value: u16) -> Option<f64> {
    if value == 0 {
        None
    } else {
        Some(f64::from(value) - 273.15)
    }
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, NvmeError> {
    bytes.get(offset).copied().ok_or_else(|| {
        NvmeError::Parse(format!(
            "requested byte {} from buffer of length {}",
            offset,
            bytes.len()
        ))
    })
}

fn slice<const N: usize>(bytes: &[u8], offset: usize) -> Result<&[u8], NvmeError> {
    let end = offset.saturating_add(N);
    bytes.get(offset..end).ok_or_else(|| {
        NvmeError::Parse(format!(
            "requested range {}..{} from buffer of length {}",
            offset,
            end,
            bytes.len()
        ))
    })
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, NvmeError> {
    let src = slice::<2>(bytes, offset)?;
    let mut value = [0_u8; 2];
    value.copy_from_slice(src);
    Ok(u16::from_le_bytes(value))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, NvmeError> {
    let src = slice::<4>(bytes, offset)?;
    let mut value = [0_u8; 4];
    value.copy_from_slice(src);
    Ok(u32::from_le_bytes(value))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Result<u64, NvmeError> {
    let src = slice::<8>(bytes, offset)?;
    let mut value = [0_u8; 8];
    value.copy_from_slice(src);
    Ok(u64::from_le_bytes(value))
}

fn read_u128_le(bytes: &[u8], offset: usize) -> Result<u128, NvmeError> {
    let src = slice::<16>(bytes, offset)?;
    let mut value = [0_u8; 16];
    value.copy_from_slice(src);
    Ok(u128::from_le_bytes(value))
}

#[cfg(test)]
mod tests {
    use crate::nvme::types::trim_nvme_ascii;
    use crate::nvme::types::ErrorLogSummary;
    use crate::nvme::types::SmartLog;
    use crate::nvme::types::ERROR_LOG_BYTES;
    use crate::nvme::types::SMART_LOG_BYTES;

    #[test]
    fn parses_u128_counter() {
        let mut bytes = [0_u8; SMART_LOG_BYTES];
        bytes[32..48].copy_from_slice(&u128::MAX.to_le_bytes());

        let parsed = SmartLog::parse(&bytes).expect("smart log should parse");
        assert_eq!(parsed.data_units_read, u128::MAX);
    }

    #[test]
    fn trims_ascii_padding() {
        let value = trim_nvme_ascii(b"Samsung SSD  \0\0\0");
        assert_eq!(value, "Samsung SSD");
    }

    #[test]
    fn temperature_conversion_handles_zero() {
        let mut bytes = [0_u8; SMART_LOG_BYTES];
        bytes[1..3].copy_from_slice(&0_u16.to_le_bytes());
        let parsed = SmartLog::parse(&bytes).expect("smart log should parse");
        assert_eq!(parsed.temperature_celsius(), None);
    }

    #[test]
    fn critical_warning_bits_parse() {
        let mut bytes = [0_u8; SMART_LOG_BYTES];
        bytes[0] = 0b0001_1111;
        let parsed = SmartLog::parse(&bytes).expect("smart log should parse");
        assert!(parsed.critical_warning_available_spare());
        assert!(parsed.critical_warning_temperature());
        assert!(parsed.critical_warning_reliability());
        assert!(parsed.critical_warning_read_only());
        assert!(parsed.critical_warning_volatile_backup());
    }

    #[test]
    fn error_log_summary_counts_non_zero_entries() {
        let mut bytes = [0_u8; ERROR_LOG_BYTES];
        bytes[0..8].copy_from_slice(&5_u64.to_le_bytes());
        bytes[64..72].copy_from_slice(&2_u64.to_le_bytes());
        let parsed = ErrorLogSummary::parse(&bytes).expect("error log should parse");
        assert_eq!(parsed.non_zero_entries, 2);
        assert_eq!(parsed.max_error_count, 5);
    }
}
