use std::os::fd::RawFd;

use crate::nvme::error::NvmeError;
use crate::nvme::types::IDENTIFY_BYTES;

const NVME_IOCTL_ADMIN_CMD: libc::c_ulong = 0xC048_4E41;
const OPCODE_IDENTIFY: u8 = 0x06;
const OPCODE_GET_LOG_PAGE: u8 = 0x02;
const NSID_ALL: u32 = 0xFFFF_FFFF;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvmePassthruCmd {
    pub opcode: u8,
    pub flags: u8,
    pub rsvd1: u16,
    pub nsid: u32,
    pub cdw2: u32,
    pub cdw3: u32,
    pub metadata: u64,
    pub addr: u64,
    pub metadata_len: u32,
    pub data_len: u32,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
    pub timeout_ms: u32,
    pub result: u32,
}

impl NvmePassthruCmd {
    fn empty() -> Self {
        Self {
            opcode: 0,
            flags: 0,
            rsvd1: 0,
            nsid: 0,
            cdw2: 0,
            cdw3: 0,
            metadata: 0,
            addr: 0,
            metadata_len: 0,
            data_len: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
            timeout_ms: 0,
            result: 0,
        }
    }
}

pub fn identify_controller(
    fd: RawFd,
    device_name: &str,
    timeout_ms: u32,
) -> Result<[u8; IDENTIFY_BYTES], NvmeError> {
    let mut buffer = [0_u8; IDENTIFY_BYTES];
    let data_len = u32::try_from(buffer.len()).map_err(|_| {
        NvmeError::InvalidData("identify controller buffer length exceeds u32".to_string())
    })?;
    let mut cmd = NvmePassthruCmd::empty();
    cmd.opcode = OPCODE_IDENTIFY;
    cmd.nsid = 0;
    cmd.addr = buffer.as_mut_ptr() as u64;
    cmd.data_len = data_len;
    cmd.cdw10 = 0x01;
    cmd.timeout_ms = timeout_ms;

    admin_cmd(fd, device_name, &mut cmd)?;
    Ok(buffer)
}

pub fn identify_namespace(
    fd: RawFd,
    device_name: &str,
    nsid: u32,
    timeout_ms: u32,
) -> Result<[u8; IDENTIFY_BYTES], NvmeError> {
    let mut buffer = [0_u8; IDENTIFY_BYTES];
    let data_len = u32::try_from(buffer.len()).map_err(|_| {
        NvmeError::InvalidData("identify namespace buffer length exceeds u32".to_string())
    })?;
    let mut cmd = NvmePassthruCmd::empty();
    cmd.opcode = OPCODE_IDENTIFY;
    cmd.nsid = nsid;
    cmd.addr = buffer.as_mut_ptr() as u64;
    cmd.data_len = data_len;
    cmd.cdw10 = 0x00;
    cmd.timeout_ms = timeout_ms;

    admin_cmd(fd, device_name, &mut cmd)?;
    Ok(buffer)
}

pub fn get_log_page(
    fd: RawFd,
    device_name: &str,
    nsid: u32,
    lid: u8,
    data_len: usize,
    timeout_ms: u32,
) -> Result<Vec<u8>, NvmeError> {
    if data_len == 0 || !data_len.is_multiple_of(4) {
        return Err(NvmeError::InvalidData(format!(
            "log page length {} must be non-zero and divisible by 4",
            data_len
        )));
    }

    let numd_words = (data_len / 4).saturating_sub(1);
    let numd_words = u32::try_from(numd_words)
        .map_err(|_| NvmeError::InvalidData("log page length is too large".to_string()))?;
    let data_len_u32 = u32::try_from(data_len)
        .map_err(|_| NvmeError::InvalidData("log page length is too large".to_string()))?;

    let mut buffer = vec![0_u8; data_len];
    let mut cmd = NvmePassthruCmd::empty();
    cmd.opcode = OPCODE_GET_LOG_PAGE;
    cmd.nsid = nsid;
    cmd.addr = buffer.as_mut_ptr() as u64;
    cmd.data_len = data_len_u32;
    cmd.cdw10 = (numd_words << 16) | u32::from(lid);
    cmd.timeout_ms = timeout_ms;

    admin_cmd(fd, device_name, &mut cmd)?;
    Ok(buffer)
}

pub fn get_controller_log_page(
    fd: RawFd,
    device_name: &str,
    lid: u8,
    data_len: usize,
    timeout_ms: u32,
) -> Result<Vec<u8>, NvmeError> {
    get_log_page(fd, device_name, NSID_ALL, lid, data_len, timeout_ms)
}

fn admin_cmd(fd: RawFd, device_name: &str, cmd: &mut NvmePassthruCmd) -> Result<(), NvmeError> {
    let ret = unsafe { libc::ioctl(fd, NVME_IOCTL_ADMIN_CMD as _, cmd as *mut NvmePassthruCmd) };

    if ret < 0 {
        let source = std::io::Error::last_os_error();
        if source.kind() == std::io::ErrorKind::PermissionDenied {
            return Err(NvmeError::PermissionDenied {
                device: device_name.to_string(),
            });
        }
        return Err(NvmeError::Ioctl {
            device: device_name.to_string(),
            source,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::nvme::ioctl::NvmePassthruCmd;

    #[test]
    fn passthrough_layout_matches_kernel() {
        assert_eq!(std::mem::size_of::<NvmePassthruCmd>(), 72);
    }
}
