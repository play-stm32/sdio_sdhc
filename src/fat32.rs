use block_device::BlockDevice;
use crate::sdcard::{CmdError, Card};

/// impl BlockDevice for card
impl BlockDevice for Card {
    type Error = CmdError;

    fn read(&self, buf: &mut [u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.read_block(buf, address as u32)?
        } else {
            self.read_multi_blocks(buf, address as u32, number_of_blocks as u32)?
        }

        Ok(())
    }

    fn write(&self, buf: &[u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.write_block(buf, address as u32)?
        } else {
            self.write_multi_blocks(buf, address as u32, number_of_blocks as u32)?
        }

        Ok(())
    }
}
