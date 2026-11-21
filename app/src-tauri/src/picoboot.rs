//! Writes the firmware through the RP2040 bootloader's own USB interface, PICOBOOT (RP2040 datasheet, section 2.8.5),
//! instead of copying the UF2 file to its drive: macOS warns that a disk was not ejected properly when that drive goes
//! away as the board restarts. Its drive is unmounted first, so nothing is left to warn about.
//!
//! The first flash sector, which holds the code the chip checks before starting the firmware, is written last: until
//! then a board cut off halfway starts again in its bootloader, as a blank one does, and is written at the next try.

use nusb::transfer::{Buffer, Bulk, ControlIn, ControlOut, ControlType, In, Out, Recipient};
use nusb::{DeviceId, DeviceInfo, Interface, MaybeFuture};
use std::time::Duration;

const VENDOR: u16 = 0x2e8a;
const BOOTLOADER: u16 = 0x0003;
const MAGIC: u32 = 0x431f_d10b;
const FLASH: u32 = 0x1000_0000;
const PAGE: usize = 256;
const SECTOR: usize = 4096;
/// The top of the RP2040's RAM, the stack a reboot is given
const RAM_END: u32 = 0x2004_2000;
const TIMEOUT: Duration = Duration::from_secs(5);
/// Erasing a whole firmware takes a few seconds
const ERASE_TIMEOUT: Duration = Duration::from_secs(30);

const EXCLUSIVE_ACCESS: u8 = 0x01;
const REBOOT: u8 = 0x02;
const FLASH_ERASE: u8 = 0x03;
const READ: u8 = 0x84;
const WRITE: u8 = 0x05;
const EXIT_XIP: u8 = 0x06;
const INTERFACE_RESET: u8 = 0x41;
const COMMAND_STATUS: u8 = 0x42;

/// The chip's USB bootloaders connected now.
pub fn bootloaders() -> Vec<DeviceId> {
    nusb::list_devices()
        .wait()
        .map(|devices| devices.filter(is_bootloader).map(|device| device.id()).collect())
        .unwrap_or_default()
}

/// The bootloader connected now that is not in `before`.
pub fn new_bootloader(before: &[DeviceId]) -> Option<DeviceInfo> {
    nusb::list_devices()
        .wait()
        .ok()?
        .find(|device| is_bootloader(device) && !before.contains(&device.id()))
}

fn is_bootloader(device: &DeviceInfo) -> bool {
    device.vendor_id() == VENDOR && device.product_id() == BOOTLOADER
}

/// The flash sectors a UF2 file fills, as (address, 4 KB of data), the first sector of the flash first; pages the file
/// leaves out are erased, as the bootloader's drive leaves them.
fn sectors(image: &[u8]) -> Result<Vec<(u32, Vec<u8>)>, String> {
    let mut sectors: Vec<(u32, Vec<u8>)> = Vec::new();
    for block in image.chunks(512) {
        let word = |at: usize| u32::from_le_bytes(block[at..at + 4].try_into().unwrap());
        if block.len() != 512 || word(0) != 0x0a32_4655 || word(4) != 0x9e5d_5157 || word(508) != 0x0ab1_6f30 {
            return Err("the firmware is not a UF2 file".to_string());
        }
        let (address, size) = (word(12), word(16) as usize);
        if size != PAGE || address < FLASH || !(address as usize).is_multiple_of(PAGE) {
            return Err(format!("the firmware writes outside the flash, at {address:#x}"));
        }
        let start = address - address % SECTOR as u32;
        let sector = match sectors.iter_mut().find(|(at, _)| *at == start) {
            Some((_, data)) => data,
            None => {
                sectors.push((start, vec![0xff; SECTOR]));
                &mut sectors.last_mut().unwrap().1
            }
        };
        let offset = (address - start) as usize;
        sector[offset..offset + PAGE].copy_from_slice(&block[32..32 + PAGE]);
    }
    sectors.sort_by_key(|(address, _)| *address);
    if sectors.first().is_none_or(|(address, _)| *address != FLASH) {
        return Err("the firmware does not start at the beginning of the flash".to_string());
    }
    Ok(sectors)
}

fn args(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

/// A bootloader's PICOBOOT interface, claimed by the app.
pub struct Picoboot {
    interface: Interface,
    number: u8,
    to_board: nusb::Endpoint<Bulk, Out>,
    from_board: nusb::Endpoint<Bulk, In>,
    token: u32,
}

impl Picoboot {
    pub fn open(device: &DeviceInfo) -> Result<Self, String> {
        let number = device
            .interfaces()
            .find(|interface| interface.class() == 0xff)
            .ok_or("no PICOBOOT interface")?
            .interface_number();
        let device = device.open().wait().map_err(|error| error.to_string())?;
        let interface = device
            .claim_interface(number)
            .wait()
            .map_err(|error| error.to_string())?;
        let descriptor = interface.descriptor().ok_or("no interface descriptor")?;
        let address = |inward: bool| {
            descriptor
                .endpoints()
                .map(|endpoint| endpoint.address())
                .find(|address| (address & 0x80 != 0) == inward)
                .ok_or("no bulk endpoint")
        };
        let to_board = interface
            .endpoint::<Bulk, Out>(address(false)?)
            .map_err(|error| error.to_string())?;
        let from_board = interface
            .endpoint::<Bulk, In>(address(true)?)
            .map_err(|error| error.to_string())?;
        let mut board = Self {
            interface,
            number,
            to_board,
            from_board,
            token: 1,
        };
        board.reset()?;
        Ok(board)
    }

    /// Writes `image`, a UF2 file, to the board's flash, checks it and restarts the board with it. A failure before
    /// anything is erased restarts the board with its old firmware; one while writing is tried once more.
    pub fn write_firmware(mut self, image: &[u8]) -> Result<(), String> {
        let prepared = sectors(image).and_then(|sectors| {
            self.command(EXCLUSIVE_ACCESS, &[1], 0, TIMEOUT)?;
            self.command(EXIT_XIP, &[], 0, TIMEOUT)?;
            Ok(sectors)
        });
        let sectors = match prepared {
            Ok(sectors) => sectors,
            Err(error) => {
                let _ = self.restart();
                return Err(error);
            }
        };
        if let Err(error) = self.flash(&sectors) {
            crate::errors::log(format!("writing the firmware again: {error}"));
            self.reset()?;
            self.flash(&sectors)?;
        }
        self.restart()
    }

    fn flash(&mut self, sectors: &[(u32, Vec<u8>)]) -> Result<(), String> {
        let (first, rest) = sectors.split_first().ok_or("the firmware is empty")?;
        // From here on the board can only start in its bootloader, until the end
        self.command(EXIT_XIP, &[], 0, TIMEOUT)?;
        self.erase(first.0)?;
        for (address, data) in rest {
            self.erase(*address)?;
            self.write(*address, data)?;
        }
        self.check(rest)?;
        self.write(first.0, &first.1)?;
        self.check(std::slice::from_ref(first))
    }

    fn restart(&mut self) -> Result<(), String> {
        self.command(REBOOT, &args(&[0, RAM_END, 100]), 0, TIMEOUT)
    }

    fn reset(&mut self) -> Result<(), String> {
        self.interface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Interface,
                    request: INTERFACE_RESET,
                    value: 0,
                    index: self.number.into(),
                    data: &[],
                },
                TIMEOUT,
            )
            .wait()
            .map_err(|error| format!("cannot reset the bootloader's interface: {error}"))
    }

    fn erase(&mut self, address: u32) -> Result<(), String> {
        self.command(FLASH_ERASE, &args(&[address, SECTOR as u32]), 0, ERASE_TIMEOUT)
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), String> {
        self.command(WRITE, &args(&[address, data.len() as u32]), data.len(), TIMEOUT)?;
        self.send(data.to_vec())?;
        self.acknowledged()
    }

    fn check(&mut self, sectors: &[(u32, Vec<u8>)]) -> Result<(), String> {
        for (address, data) in sectors {
            if self.read(*address, data.len())? != *data {
                return Err(format!(
                    "the flash read back at {address:#x} differs from the firmware written"
                ));
            }
        }
        Ok(())
    }

    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>, String> {
        self.command(READ, &args(&[address, size as u32]), size, TIMEOUT)?;
        let packet = self.from_board.max_packet_size();
        let completion = self
            .from_board
            .transfer_blocking(Buffer::new(size.div_ceil(packet) * packet), TIMEOUT);
        completion.status.map_err(|error| self.failed(error))?;
        // A read is acknowledged the other way round
        self.send(Vec::new())?;
        Ok(completion.buffer[..completion.actual_len.min(size)].to_vec())
    }

    /// Sends a command; one without data is acknowledged at once, the others once their data has gone through.
    fn command(&mut self, id: u8, arguments: &[u8], transfer: usize, timeout: Duration) -> Result<(), String> {
        let mut packet = Vec::with_capacity(32);
        packet.extend(MAGIC.to_le_bytes());
        packet.extend(self.token.to_le_bytes());
        packet.extend([id, arguments.len() as u8, 0, 0]);
        packet.extend((transfer as u32).to_le_bytes());
        packet.extend(arguments);
        packet.resize(32, 0);
        self.token += 1;
        let completion = self.to_board.transfer_blocking(packet.into(), TIMEOUT);
        completion.status.map_err(|error| self.failed(error))?;
        if transfer == 0 {
            let packet = self.from_board.max_packet_size();
            let completion = self.from_board.transfer_blocking(Buffer::new(packet), timeout);
            completion.status.map_err(|error| self.failed(error))?;
        }
        Ok(())
    }

    fn send(&mut self, data: Vec<u8>) -> Result<(), String> {
        let completion = self.to_board.transfer_blocking(data.into(), TIMEOUT);
        completion.status.map_err(|error| self.failed(error))
    }

    fn acknowledged(&mut self) -> Result<(), String> {
        let packet = self.from_board.max_packet_size();
        let completion = self.from_board.transfer_blocking(Buffer::new(packet), TIMEOUT);
        completion.status.map_err(|error| self.failed(error))
    }

    /// What the bootloader says went wrong, when it refused a command by stalling.
    fn failed(&self, error: impl std::fmt::Display) -> String {
        let status = self
            .interface
            .control_in(
                ControlIn {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Interface,
                    request: COMMAND_STATUS,
                    value: 0,
                    index: self.number.into(),
                    length: 16,
                },
                TIMEOUT,
            )
            .wait();
        match status {
            Ok(status) if status.len() >= 8 => {
                let code = u32::from_le_bytes(status[4..8].try_into().unwrap());
                format!("the board's bootloader refused a command (status {code}): {error}")
            }
            _ => format!("the board's bootloader stopped answering: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(address: u32, fill: u8) -> Vec<u8> {
        let mut block = vec![0; 512];
        for (at, word) in [
            (0, 0x0a32_4655),
            (4, 0x9e5d_5157),
            (12, address),
            (16, 256),
            (508, 0x0ab1_6f30),
        ] {
            block[at..at + 4].copy_from_slice(&u32::to_le_bytes(word));
        }
        block[32..32 + PAGE].fill(fill);
        block
    }

    #[test]
    fn groups_the_pages_into_sectors_with_the_gaps_erased() {
        let image = [block(FLASH + 0x1100, 2), block(FLASH, 1)].concat();
        let sectors = sectors(&image).unwrap();
        assert_eq!(
            sectors.iter().map(|(address, _)| *address).collect::<Vec<_>>(),
            [FLASH, FLASH + 0x1000]
        );
        assert!(sectors[0].1[..PAGE].iter().all(|byte| *byte == 1));
        assert!(sectors[0].1[PAGE..].iter().all(|byte| *byte == 0xff));
        assert!(sectors[1].1[0x100..0x200].iter().all(|byte| *byte == 2));
    }

    #[test]
    fn refuses_what_is_not_a_flash_image() {
        assert!(sectors(&[0; 512]).is_err());
        assert!(sectors(&block(0x2000_0000, 0)).is_err());
        assert!(sectors(&block(FLASH + 0x1000, 0)).is_err());
    }

    #[test]
    fn the_bundled_firmware_starts_at_the_flash() {
        let sectors = sectors(crate::firmware::IMAGE).unwrap();
        assert_eq!(sectors[0].0, FLASH);
    }
}
