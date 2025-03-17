// $ cargo rb ferris
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::time::mhz;
use embassy_stm32::{spi, Config};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::image::{ImageRaw, ImageRawLE};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use tinybmp::Bmp;
use {defmt_rtt as _, panic_probe as _};

use st7735_embassy::{self, buffer_size, ST7735};

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

    // cs_pin: chip select pin
    let cs = Output::new(p.PE11.degrade(), Level::Low, Speed::High);
    // rst:  display reset pin, managed at driver level
    let rst = Output::new(p.PE10.degrade(), Level::High, Speed::High);
    // dc: data/command selection pin, managed at driver level
    let dc = Output::new(p.PE13.degrade(), Level::High, Speed::High);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(1);

    //let spi = spi::Spi::new_blocking(p.SPI4, p.PE12, p.PE14, p.PE5, spi_config);
    let spi = spi::Spi::new(
        p.SPI4, p.PE12, p.PE14, p.PE5, p.DMA1_CH3, p.DMA1_CH4, spi_config,
    );
    let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    let mut display = ST7735::<_, _, _, 80, 160, { buffer_size(80, 160) }>::new(
        spi_dev,
        dc,
        rst,
        Default::default(),
    );
    display.init(&mut Delay).await.unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    // draw ferris
    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(34, 8));
    image.draw(&mut display).unwrap();

    let raw_image: Bmp<Rgb565> = Bmp::from_slice(include_bytes!("ferris.bmp")).unwrap();
    let image = Image::new(&raw_image, Point::new(34, 24));
    image.draw(&mut display).unwrap();

    display.flush().await.unwrap();

    // LED is set to max, but can be modulated with pwm to change backlight brightness
    let mut backlight = Output::new(p.PE3, Level::High, Speed::High);
    loop {
        backlight.set_high();
        Timer::after(Duration::from_millis(700)).await;
        backlight.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}
