use crate::sdio_dma;
use stm32f4xx_hal::stm32;

/// define response type
#[derive(Clone, Copy)]
pub enum ResponseType {
    NoResponse,
    ShortResponse,
    LongResponse,
}

/// define cmd error
#[derive(Clone, Copy, Debug)]
pub enum CmdError {
    REXOVERR,
    TXUNDERR,
    DTIMEOUT,
    CTIMEOUT,
    DCRCFAIL,
    CCRCFAIL,
    NoFindCard
}

/// define read or write operation
enum Operation {
    Read,
    Write,
}

/// init sdio
fn sdio_init() {
    let rcc = unsafe {
        &*stm32::RCC::ptr()
    };

    let sdio = unsafe {
        &*stm32::SDIO::ptr()
    };

    // sdio enable
    rcc.apb2enr.modify(|_r, w| w.sdioen().set_bit());

    // set init clock 400Khz, 48MHz / 400KHz - 2 = 118, clock enable
    sdio.clkcr.write(|w| unsafe { w.clkdiv().bits(118).clken().set_bit() });

    // 0b11 = 3, power on
    sdio.power.write(|w| unsafe { w.pwrctrl().bits(3) });
}

#[derive(Debug, Copy, Clone)]
pub struct Card {
    pub capacity: u32,
    block_size: u32,
    rca: u16,
}

impl Card {
    /// init sdcard (sdhc)
    pub fn init() -> Result<Card, CmdError> {
        sdio_init();
        sdio_dma::init();

        // send cmd0 into IDLE STAGE mode , no argument, no response
        cmd_send(0, 0, ResponseType::NoResponse);

        // send cmd8 to check card type, argument 0x000001AA, short response
        cmd_send(8, 0x000001AA, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        // wait card has power to ready
        for i in 0..0xFF {
            // send acmd 41 to check sdcard ready state, argument 0x80100000 | 0x40000000, short response
            acmd_send(41, 0, 0x80100000 | 0x40000000, ResponseType::ShortResponse)?;
            let resp = read_response(ResponseType::ShortResponse)?;

            // check voltage
            if resp.1[0] >> 31 == 1 {
                break;
            }

            if i == 0xFF - 1 {
                return Err(CmdError::NoFindCard);
            }
        }

        // send cmd 2 to get CID, no argument, long response
        cmd_send(2, 0, ResponseType::LongResponse);
        let _drop = read_response(ResponseType::LongResponse)?;

        // send cmd 3 to get RCA, no argument, short response
        cmd_send(3, 0, ResponseType::ShortResponse);
        let rca = read_response(ResponseType::ShortResponse)?;
        let rca = (rca.1[0] >> 16) as u16;

        // send cmd 9 to get CSD, argument RCA, long response
        cmd_send(9, (rca as u32) << 16, ResponseType::LongResponse);
        let csd = read_response(ResponseType::LongResponse)?;

        // get card_capacity
        let temp1 = (csd.1[1] & 0xFFFF0000) >> 16;
        let temp2 = (csd.1[2] & 0x3F) << 16;
        let card_capacity = ((temp2 | temp1) + 1) * 512 * 1024;

        select_card((rca as u32) << 16)?;
        enable_wide_bus((rca as u32) << 16)?;
        switch_work_clock();

        Ok(Card {
            capacity: card_capacity,
            block_size: 512,
            rca,
        })
    }

    /// read block
    pub fn read_block(&self, buf: &mut [u8], address: u32) -> Result<(), CmdError> {
        wait_card_programming(self.rca)?;
        set_block_size(self.block_size)?;

        sdio_dma::peripheral_to_memory(buf);
        data_control(self.block_size, self.block_size, Operation::Read);

        // send cmd 17 to read block data, argument block address, short response
        cmd_send(17, address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        while !sdio_dma::stream3_transfer_complete() {}
        loop {
            if get_datacount() == 0 {
                clean_all_state();
                break;
            }
        }

        Ok(())
    }

    /// read multi blocks
    pub fn read_multi_blocks(&self, buf: &mut [u8], address: u32, number_of_blocks: u32) -> Result<(), CmdError> {
        wait_card_programming(self.rca)?;
        set_block_size(self.block_size)?;

        sdio_dma::peripheral_to_memory(buf);
        data_control(self.block_size * number_of_blocks, self.block_size, Operation::Read);

        // send cmd 18 to read multi blocks data, argument block address, short response
        cmd_send(18, address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        while !sdio_dma::stream3_transfer_complete() {}
        loop {
            if get_datacount() == 0 {
                stop_read_or_write()?;
                clean_all_state();
                break;
            }
        }

        Ok(())
    }

    /// write block
    pub fn write_block(&self, buf: &[u8], address: u32) -> Result<(), CmdError> {
        wait_card_programming(self.rca)?;
        set_block_size(self.block_size)?;

        sdio_dma::memory_to_peripheral(buf);

        // send cmd 24 to write block, argument block address, short response
        cmd_send(24, address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;
        data_control(self.block_size, self.block_size, Operation::Write);

        while !sdio_dma::stream3_transfer_complete() {}
        loop {
            if get_datacount() == 0 {
                clean_all_state();
                break;
            }
        }

        Ok(())
    }

    /// write multi blocks
    pub fn write_multi_blocks(&self, buf: &[u8], address: u32, number_of_blocks: u32) -> Result<(), CmdError> {
        wait_card_programming(self.rca)?;
        set_block_size(self.block_size)?;

        sdio_dma::memory_to_peripheral(buf);

        // send acmd 23 to erase block data, argument blocks number, short response
        acmd_send(23, (self.rca as u32) << 16, number_of_blocks, ResponseType::ShortResponse)?;
        let _drop = read_response(ResponseType::ShortResponse)?;

        // send cmd 25 to write multi blocks, argument block address, short response
        cmd_send(25, address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        data_control(self.block_size * number_of_blocks, self.block_size, Operation::Write);

        while !sdio_dma::stream3_transfer_complete() {}
        loop {
            if get_datacount() == 0 {
                stop_read_or_write()?;
                clean_all_state();
                break;
            }
        }

        Ok(())
    }

    pub fn erase(&self, start_address: u32, end_address: u32) -> Result<(), CmdError> {
        wait_card_programming(self.rca)?;
        set_block_size(self.block_size)?;

        // send cmd 32 to set start block address, argument block address, short response
        cmd_send(32, start_address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        // send cmd 33 to set end block address, argument block address, short response
        cmd_send(33, end_address / self.block_size, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        // send cmd 38 to start erase, argument block address, short response
        cmd_send(38, 0, ResponseType::ShortResponse);
        let _drop = read_response(ResponseType::ShortResponse)?;

        wait_card_programming(self.rca)?;
        clean_all_state();

        Ok(())
    }
}

/// send cmd 12 to stop read or write process, no argument, short response
fn stop_read_or_write() -> Result<(), CmdError> {
    cmd_send(12, 0, ResponseType::ShortResponse);
    let _drop = read_response(ResponseType::ShortResponse)?;
    Ok(())
}

/// send cmd 16 to set card block size, argument block size, short response
fn set_block_size(block_size: u32) -> Result<(), CmdError> {
    cmd_send(16, block_size, ResponseType::ShortResponse);
    let _drop = read_response(ResponseType::ShortResponse)?;
    Ok(())
}

/// send cmd 13 to wait card could program, argument RCA, short response
fn wait_card_programming(rca: u16) -> Result<(), CmdError> {
    loop {
        cmd_send(13, (rca as u32) << 16, ResponseType::ShortResponse);
        let state = read_response(ResponseType::ShortResponse)?;
        if state.1[0] & 0x00000100 != 0 {
            break;
        }
    }
    Ok(())
}

/// send cmd
pub fn cmd_send(cmd: u8, arg: u32, response_type: ResponseType) {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    // wait last cmd has been sent
    while cmdact_state() {}

    ptr.arg.write(|w| unsafe { w.cmdarg().bits(arg) });

    let mut bits = cmd as u32;

    // set response type
    match response_type {
        ResponseType::NoResponse => {
            bits = 0b00 << 6 | bits;
        }
        ResponseType::ShortResponse => {
            bits = 0b01 << 6 | bits;
        }
        ResponseType::LongResponse => {
            bits = 0b11 << 6 | bits;
        }
    };

    // enable cmd
    bits = 1 << 10 | bits;
    bits = 1 << 12 | bits;

    ptr.cmd.write(|w| unsafe { w.bits(bits) });
}

/// read response
pub fn read_response(response_type: ResponseType) -> Result<(ResponseType, [u32; 4]), CmdError> {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    // check response state
    check_state()?;

    // get response
    let response = match response_type {
        ResponseType::NoResponse => {
            [0, 0, 0, 0]
        }
        ResponseType::ShortResponse => {
            let resp1 = ptr.resp1.read().cardstatus1().bits();
            [resp1, 0, 0, 0]
        }
        ResponseType::LongResponse => {
            let resp1 = ptr.resp1.read().cardstatus1().bits();
            let resp2 = ptr.resp2.read().cardstatus2().bits();
            let resp3 = ptr.resp3.read().cardstatus3().bits();
            let resp4 = ptr.resp4.read().cardstatus4().bits();
            [resp4, resp3, resp2, resp1]
        }
    };

    // clean all response state
    clean_all_state();
    Ok((response_type, response))
}

/// app cmd send
pub fn acmd_send(acmd: u8, cmd_arg: u32, acmd_arg: u32, response_type: ResponseType) -> Result<(), CmdError> {
    // send cmd 55 into app cmd mode, short response
    cmd_send(55, cmd_arg, ResponseType::ShortResponse);
    let _drop = read_response(ResponseType::ShortResponse)?;

    cmd_send(acmd, acmd_arg, response_type);
    Ok(())
}

/// switch to work clock (24Mhz)
fn switch_work_clock() {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    ptr.clkcr.modify(|_r, w| w.clken().clear_bit());

    // 24Mhz = 48Mhz / (2 + 0)
    ptr.clkcr.modify(|_r, w| unsafe {
        w.clkdiv().bits(0).clken().set_bit()
    })
}

/// send cmd 7 to select card, argument RCA, short response
fn select_card(rca: u32) -> Result<(), CmdError> {
    cmd_send(7, rca, ResponseType::ShortResponse);
    let _drop = read_response(ResponseType::ShortResponse)?;
    Ok(())
}

/// enable wide bus to work
fn enable_wide_bus(rca: u32) -> Result<(), CmdError> {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    ptr.clkcr.modify(|_r, w| w.clken().clear_bit());
    ptr.clkcr.modify(|_r, w| unsafe {
        w.widbus().bits(1).clken().set_bit()
    });

    acmd_send(6, rca, 2, ResponseType::ShortResponse)?;
    let _drop = read_response(ResponseType::ShortResponse)?;

    Ok(())
}

/// set data info
fn data_control(len: u32, block_size: u32, op: Operation) {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    let mut block_size = block_size;
    let mut block_bits = 0;

    while block_size != 1 {
        block_size >>= 1;
        block_bits += 1;
    }

    ptr.dtimer.write(|w| unsafe {
        w.datatime().bits(0xFFFFFFFF)
    });

    ptr.dlen.write(|w| unsafe {
        w.datalength().bits(len as u32)
    });

    // set block size (9 => 512), enale DMA mode, block transfer
    let mut bits = (block_bits as u32) << 4 | 1 << 3 | 0 << 2;

    // set transfer dir
    match op {
        Operation::Write => {}
        Operation::Read => {
            bits = bits | 1 << 1;
        }
    }

    // ready to transfer
    bits = bits | 1 << 0;

    ptr.dctrl.write(|w| unsafe { w.bits(bits) });
}

/// clean all response state
fn clean_all_state() {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    ptr.icr.write(|w| unsafe {
        w.bits(0x00C007FF)
    });
}

/// check response state
fn check_state() -> Result<(), CmdError> {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };

    // wait last cmd has been sent
    while cmdact_state() {};

    // delay
    for _ in 0..200 {
        cortex_m::asm::nop();
    }

    let status = ptr.sta.read().bits();

    // R3 response has no CCRC, if get reponse return
    if status & 0b1000000 >> 6 == 1 { return Ok(()); }

    if status & 0b000001 >> 0 == 1 { return Err(CmdError::CCRCFAIL); }
    if status & 0b000010 >> 1 == 1 { return Err(CmdError::DCRCFAIL); }
    if status & 0b000100 >> 2 == 1 { return Err(CmdError::CTIMEOUT); }
    if status & 0b001000 >> 3 == 1 { return Err(CmdError::DTIMEOUT); }
    if status & 0b010000 >> 4 == 1 { return Err(CmdError::TXUNDERR); }
    if status & 0b100000 >> 5 == 1 { return Err(CmdError::REXOVERR); }

    Ok(())
}

/// wait last cmd has been sent
pub fn cmdact_state() -> bool {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };
    ptr.sta.read().cmdact().bit()
}

/// get data count, if datacount == 0, data has been sent or received
fn get_datacount() -> u32 {
    let ptr = unsafe {
        &*stm32::SDIO::ptr()
    };
    ptr.dcount.read().datacount().bits()
}
