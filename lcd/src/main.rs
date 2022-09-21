#![no_std] // Doesn't use the Rust standard library
#![no_main] // A bit misleading: our program _does_ in fact have a main function, but we need to
            // include this because otherwise, the Rust compiler makes assumptions about the the
            // environment that our program executes in. We will still define a main function using
            // the #[entry] tag.

// Use panic_halt as a panicking behavior.
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// Use #[entry] to mark `main` as the entrypoint
use cortex_m_rt::entry;

// Import things from the HAL that we will use
use stm32h7xx_hal::{delay, pac, prelude::*, spi};

use st7735_lcd::{self, Orientation};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Text},
};

const LCD_WIDTH: u32 = 162;
const LCD_HEIGHT: u32 = 132;
const LCD_COLOR: bool = false; // BGR instead of RGB
const LCD_INVERTED: bool = true; // Color should be inverted to display correctly


#[entry]
fn main() -> ! {
    // Device peripherals give access to things like GPIO, SPI, etc
    let dp = pac::Peripherals::take().unwrap();

    // Core peripherals give access to things like the SysTick timer and other Cortex M
    // peripherals
    let cp = cortex_m::Peripherals::take().unwrap();

    // Access to the power configuration registers.
    //
    // .constrain() consumes the dp.PWR struct (part of the PAC), meaning you can now only access
    // registers that are exposed in the stm32h7xx_hal::Pwr struct (different than pac::PWR).
    //
    // .freeze() consumes the stm32h7xx_hal::Pwr struct, preventing further modification,
    // and returning a PowerConfiguration struct that contains the (immutable) values for the
    // power configuration of the chip
    //
    // Since it's frozen, we can now use it in the instantiation of other peripherals
    let pwrcfg: stm32h7xx_hal::pwr::PowerConfiguration = dp.PWR.constrain().freeze();

    // Reset and Clock Control
    //
    // Link: https://docs.rs/stm32h7xx-hal/0.12.2/stm32h7xx_hal/rcc/index.html (General)
    // Link: https://docs.rs/stm32h7xx-hal/0.12.2/stm32h7xx_hal/rcc/struct.Rcc.html (Struct)
    //
    // Gives us access to the RCC peripheral (this allows us to enable any other device peripherals
    // by enabling the clock to that region).
    //
    // The dp.RCC struct represents the set of RCC registers in the PAC.
    // .constrain() gives us access to the Rcc struct abstraction in the HAL
    let rcc: stm32h7xx_hal::rcc::Rcc = dp.RCC.constrain();

    // Core Clock Distribution and Reset
    //
    // Link: https://docs.rs/stm32h7xx-hal/0.12.2/stm32h7xx_hal/rcc/struct.Ccdr.html
    //
    // First we configure aspects of the RCC: the sys clock (sys_ck) and the APB clock pclk1. Once
    // we call .freeze(), we get an instance of the Ccdr struct, which allows us to still modify a
    // few other parts of the Rcc so that we can enable/reset peripherals. We will use ccdr to
    // enable/reset peripherals.
    let ccdr: stm32h7xx_hal::rcc::Ccdr = rcc
        .sys_ck(96.MHz())
        .pclk1(48.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);

    // Type note included because of length. GPIO Port E
    //
    // Gives us access to the individual pins within GPIO. From the docs:
    //
    // "[.split()] Takes the GPIO peripheral and splits it into Zero-Sized Types (ZSTs) representing
    // individual pins."
    //
    // We pass in ccdr.peripheral.GPIOE so that we can enable the clock to the peripheral and reset
    // it.
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    // Type not included because of length. It is essentially of type
    // stm32h7xx_hal::gpio::Pin<MODE=Output>
    let mut led = gpioe.pe3.into_push_pull_output();

    // Gives us access to a Delay object that allows us to create arbitrary time-based delays
    let mut delay = delay::Delay::new(cp.SYST, ccdr.clocks);

    let sck = gpioe.pe12.into_alternate::<5>();
    // miso isn't routed on the board, so we don't need to declare it. In our SPI setup, we can use
    // spi::NoMiso to indicate that we don't receive messages
    let mosi = gpioe.pe14.into_alternate::<5>();

    // Woohoo! Turns out we need to use CS !!!
    let _cs = gpioe.pe11.into_push_pull_output().set_low();
    let rst = gpioe.pe9.into_push_pull_output(); // This is reset. On our device, it is just tied
                                                  // high, so we don't control it
    let dc = gpioe.pe13.into_push_pull_output(); // Data/Command pin

    let spi: spi::Spi<_, _, u8> = dp.SPI4.spi(
        (sck, spi::NoMiso, mosi),
        spi::MODE_0,
        16.MHz(),
        ccdr.peripheral.SPI4,
        &ccdr.clocks,
    );

    let lcd_brightness = gpioe.pe10.into_alternate::<1>();
    
    let gpioj = dp.GPIOJ.split(ccdr.peripheral.GPIOJ);
    let pwm = dp.TIM1.pwm(gpioj.pj11.into_alternate::<1>(), 1.MHz(), ccdr.peripheral.TIM1, &ccdr.clocks);
    let mut pwm = pwm.into_complementary(lcd_brightness);
    
    let max_duty = pwm.get_max_duty();
    pwm.set_duty(max_duty);

    // Basically this just sets up our device driver for the ST7735 which controls the LCD screen
    //
    // https://www.waveshare.com/wiki/0.96inch_LCD_Module
    let mut lcd = st7735_lcd::ST7735::new(spi, dc, rst, LCD_COLOR, LCD_INVERTED, LCD_WIDTH, LCD_HEIGHT);

    // Initialize the LCD display
    lcd.init(&mut delay).unwrap();
    
    // Sideways plz
    lcd.set_orientation(&Orientation::LandscapeSwapped).unwrap();

    // Clear the screen to black
    lcd.clear(Rgb565::BLACK).unwrap();

    // Offset of something... not sure what
    //
    // "Global offset of the image"
    //
    // This is an area for more research...
    lcd.set_offset(0, 25);

    // Sets the backlight of the LCD to enabled so we can see it
    pwm.enable();

    // Let's get a font that is 6x10px and white
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    // Create a text at position (0, 0) and draw it using the previously defined style

    let text = "Hello, OSFC!\nDo new lines work?";
    for i in 0..(text.len() + 1) {
        Text::new(&text[0..i], Point::new(5, 20), style)
            .draw(&mut lcd)
            .unwrap();
        delay.delay_ms(100_u16);
    }
    // Text::new("H", Point::new(5, 20), style).draw(&mut lcd).unwrap();
    // delay.delay_ms(100_u16);
    // Text::new("He", Point::new(5, 20), style).draw(&mut lcd).unwrap();
    // delay.delay_ms(100_u16);
    // Text::new("Hel", Point::new(5, 20), style).draw(&mut lcd).unwrap();
    // delay.delay_ms(100_u16);
    // Text::new("Hell", Point::new(5, 20), style).draw(&mut lcd).unwrap();
    // delay.delay_ms(100_u16);
    // Text::new("Hello", Point::new(5, 20), style).draw(&mut lcd).unwrap();

    loop {
        // Toggle the LED (on -> off, or off -> on)
        led.toggle();

        // Delays for 500 milliseconds
        delay.delay_ms(500_u16);
    }
}
