// $ cargo rb ferris
#![no_std]
#![no_main]

use crate::spi::Spi;
use defmt::info;
use display_interface_spi::SPIInterface;
use edrv_st7735::blocking::ST7735;
use edrv_st7735::Display160x80Type2;
use embassy_executor::Spawner;
use embassy_stm32::{spi, Config};
use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::time::mhz;
use embassy_time::{block_for, Delay, Duration, Timer};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_graphics::image::{ImageRaw, ImageRawLE};
use embedded_hal::delay::DelayNs;
use embedded_hal_bus::spi::{AtomicDevice, ExclusiveDevice};
use embedded_hal_bus::util::AtomicCell;

use tinybmp::Bmp;
use {defmt_rtt as _, panic_probe as _};



pub struct EmbassyDelay;

impl DelayNs for EmbassyDelay {
    fn delay_ns(&mut self, ns: u32) {
        let duration = Duration::from_nanos(ns as u64);
        block_for(duration);
    }

    fn delay_us(&mut self, us: u32) {
        let duration = Duration::from_micros(us as u64);
        block_for(duration);
    }

    fn delay_ms(&mut self, ms: u32) {
        let duration = Duration::from_millis(ms as u64);
        block_for(duration);
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL50,
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV8), // used by SPI3. 100Mhz.
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P; // 400 Mhz
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 200 Mhz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.voltage_scale = VoltageScale::Scale1;
    }
    let p = embassy_stm32::init(Default::default());

    let mut delay = EmbassyDelay;
    // cs: chip select pin
    let cs = Output::new(p.PE11.degrade(), Level::Low, Speed::High);
    // rst:  display reset pin, managed at driver level
    let rst = Output::new(p.PE10.degrade(), Level::High, Speed::High);
    // dc: data/command selection pin, managed at driver level
    let dc = Output::new(p.PE13.degrade(), Level::Low, Speed::High);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(1);

    
    let spi = spi::Spi::new_blocking_txonly(p.SPI4, p.PE12, p.PE14, spi_config);
    
    let spi_bus = AtomicCell::new(spi);
    let spi_dev = AtomicDevice::new(&spi_bus, cs, Delay).unwrap();

    //let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    
    let mut display: ST7735<Display160x80Type2, _, _> = ST7735::new(spi_dev, dc);

    info!("draw ferris");
    // draw ferris
    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(34, 8));
    image.draw(&mut display).unwrap();

    let raw_image: Bmp<Rgb565> = Bmp::from_slice(include_bytes!("ferris.bmp")).unwrap();
    let image = Image::new(&raw_image, Point::new(34, 24));
    image.draw(&mut display).unwrap();
    

    // LED is set to max, but can be modulated with pwm to change backlight brightness
    let mut backlight = Output::new(p.PE3, Level::High, Speed::High);
    loop {
        backlight.set_high();
        Timer::after(Duration::from_millis(700)).await;
        backlight.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}
