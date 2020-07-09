use block_device::BlockDevice;
use crate::sdcard::{CmdError, Card};

/// impl BlockDevice for card
impl BlockDevice for Card {
    type Error = CmdError;

    fn read(&self, buf: &mut [u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.read_block(buf, address)?
        } else {
            self.read_multi_blocks(buf, address, number_of_blocks)?
        }

        Ok(())
    }

    fn write(&self, buf: &[u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.write_block(buf, address)?
        } else {
            self.write_multi_blocks(buf, address, number_of_blocks)?
        }

        Ok(())
    }
}
