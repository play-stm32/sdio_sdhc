# SDIO_SDHC
You can drive sdhc card in your stm32f407 board, other stm32f4xx boards have no test.
if you want to test other boards, you can edit library and feature:

```
stm32fxxx-hal = { version = "xxx", features = ["xxx"] }
```

## Using the crate

first you need to init some GPIO, like this:
```rust
pub fn gpio_init(
    rcc: &mut stm32::RCC,
    gpioc: &mut stm32::GPIOC,
    gpiod: &mut stm32::GPIOD,
) {
    // gpioc gpiod enable
    rcc.ahb1enr.modify(|_r, w| w.gpiocen().set_bit().gpioden().set_bit());

    gpioc.afrh.modify(|_r, w|
        w.afrh8().af12()
            .afrh9().af12()
            .afrh10().af12()
            .afrh11().af12()
            .afrh12().af12());
    gpiod.afrl.modify(|_r, w| w.afrl2().af12());

    gpioc.moder.modify(|_r, w|
        w.moder8().alternate()
            .moder9().alternate()
            .moder10().alternate()
            .moder11().alternate()
            .moder12().alternate());
    gpiod.moder.modify(|_r, w| w.moder2().alternate());

    gpioc.ospeedr.modify(|_r, w|
        w.ospeedr8().high_speed()
            .ospeedr9().high_speed()
            .ospeedr10().high_speed()
            .ospeedr11().high_speed()
            .ospeedr12().high_speed());
    gpiod.ospeedr.modify(|_r, w| w.ospeedr2().high_speed());

    gpioc.otyper.modify(|_r, w|
        w.ot8().push_pull()
            .ot9().push_pull()
            .ot10().push_pull()
            .ot11().push_pull()
            .ot12().push_pull());
    gpiod.otyper.modify(|_r, w| w.ot2().push_pull());

    gpioc.pupdr.modify(|_r, w|
        w.pupdr8().pull_up()
            .pupdr9().pull_up()
            .pupdr10().pull_up()
            .pupdr11().pull_up()
            .pupdr12().pull_up());
    gpiod.pupdr.modify(|_r, w| w.pupdr2().pull_up());
}
```

then you can drive your sdhc card, and do some tests
```rust
let card = Card::init().unwrap();
writeln!(USART1, "{:#?}", card).unwrap();
card.erase(0, card.capacity).unwrap();

let buf = [1; 512 * 2];
card.write_multi_blocks(&buf, 0, 2).unwrap();

let mut buf = [0; 512 * 2];
card.read_multi_blocks(&mut buf, 0, 2).unwrap();
writeln!(USART1, "{:?}", &buf[0..buf.len()]).unwrap();

let buf = [2; 512];
card.write_block(&buf, 512).unwrap();

let mut buf = [0; 512];
card.read_block(&mut buf, 512).unwrap();
writeln!(USART1, "{:?}", &buf[0..buf.len()]).unwrap();
```

will print in your ttl like this:
```
Card {
    capacity: 3963617280,
    block_size: 512,
    rca: 1,
}
[1, 1, 1, .......]
[2, 2, 2, .......]
```

## How to support fat32 filesystem

You can add feature like this. Visit [fat32](https://github.com/play-stm32/fat32) to check out usages for details
```
sdio_sdhc = { version = "0.2.0", features = ["filesystem"] }
```