use stm32f4xx_hal::stm32;

// SDIO use DMA stream 3
const STREAM: usize = 3;

/// sdio dma init
pub fn init() {
    let rcc = unsafe {
        &*stm32::RCC::ptr()
    };

    let dma = unsafe {
        &*stm32::DMA2::ptr()
    };

    rcc.ahb1enr.modify(|_r, w| w.dma2en().set_bit());

    // get fifo register ptr
    let fifo_ptr = stm32::SDIO::ptr() as u32 + 0x80;

    // write to pa register
    dma.st[STREAM].par.write(|w| w.pa().bits(fifo_ptr));

    // use channel 4
    dma.st[STREAM].cr.write(|w| w.chsel().bits(4)
        .mburst().incr4()
        .pburst().incr4()
        .pl().very_high()
        .msize().bits32()
        .psize().bits32()
        .minc().set_bit()
        .pinc().clear_bit()
        .pfctrl().set_bit());

    // fifo enable, set full fifo
    dma.st[STREAM].fcr.write(|w| w.dmdis().set_bit().fth().full());
}

/// peripheral to memory mode
pub fn peripheral_to_memory(buf: &mut [u8]) {
    let ptr = unsafe {
        &*stm32::DMA2::ptr()
    };

    dma2_stream3_disable();

    // clean stream 3 interruption flag
    let clean_flag = 0b111101 << 22;
    ptr.lifcr.write(|w| unsafe { w.bits(clean_flag) });

    // set dir from peripheral to memory, set buffer ptr, set buffer length
    ptr.st[STREAM].cr.modify(|_r, w| w.dir().peripheral_to_memory());
    ptr.st[STREAM].m0ar.write(|w| w.m0a().bits(buf.as_ptr() as u32));
    ptr.st[STREAM].ndtr.write(|w| w.ndt().bits(buf.len() as u16));

    dma2_stream3_enable();
}

/// memory to peripheral mode
pub fn memory_to_peripheral(buf: &[u8]) {
    let ptr = unsafe {
        &*stm32::DMA2::ptr()
    };

    dma2_stream3_disable();

    // clean stream 3 interruption flag
    let clean_flag = 0b111101 << 22;
    ptr.lifcr.write(|w| unsafe { w.bits(clean_flag) });

    // set dir from memory to peripheral, set buffer ptr, set buffer length
    ptr.st[STREAM].cr.modify(|_r, w| w.dir().memory_to_peripheral());
    ptr.st[STREAM].m0ar.write(|w| w.m0a().bits(buf.as_ptr() as u32));
    ptr.st[STREAM].ndtr.write(|w| w.ndt().bits(buf.len() as u16));

    dma2_stream3_enable();
}

/// dma2 stream3 enable
fn dma2_stream3_enable() {
    let ptr = unsafe {
        &*stm32::DMA2::ptr()
    };

    ptr.st[STREAM].cr.modify(|_r, w| w.en().set_bit());
}

/// dma2 stream3 disable
fn dma2_stream3_disable() {
    let ptr = unsafe {
        &*stm32::DMA2::ptr()
    };

    ptr.st[STREAM].cr.modify(|_r, w| w.en().clear_bit());
}

/// get stream3 transfer complete flag
pub fn stream3_transfer_complete() -> bool {
    let ptr = unsafe {
        &*stm32::DMA2::ptr()
    };
    ptr.lisr.read().tcif3().bits()
}