#![no_std]
#![no_main]

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

use cortex_m_rt::entry;

use stm32h7xx_hal::{
    delay::Delay, pac, prelude::*
};

#[entry]
fn main() -> ! {
    // Get handles to the hardware objects. These functions can only be called
    // once, so that the borrowchecker can ensure you don't reconfigure
    // something by accident.
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    let mut rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(96.MHz()).pclk1(48.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let mut gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
    let mut led = gpioe.pe3.into_push_pull_output();
    led.set_high();

    // let clocks = rcc.cfgr.sysclk(8.MHz()).freeze();

    loop {

    }

    // let mut rcc = dp.RCC.constrain();
    // let ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.SYSCFG);
    //
    // let mut gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
    // // This gives us an exclusive handle to the GPIOC peripheral. To get the
    // // handle to a single pin, we need to configure the pin first. Pin C13
    // // is usually connected to the Bluepills onboard LED.
    // let mut led = gpioe.pe3.into_push_pull_output();
    //
    // // Now we need a delay object. The delay is of course depending on the clock
    // // frequency of the microcontroller, so we need to fix the frequency
    // // first. The system frequency is set via the FLASH_ACR register, so we
    // // need to get a handle to the FLASH peripheral first:
    // let mut flash = dp.FLASH.constrain();
    // // Now we can set the controllers frequency to 8 MHz:
    // let clocks = rcc.CFGR.sysclk(8.MHz()).freeze(&mut flash.acr);
    // // The `clocks` handle ensures that the clocks are now configured and gives
    // // the `Delay::new` function access to the configured frequency. With
    // // this information it can later calculate how many cycles it has to
    // // wait. The function also consumes the System Timer peripheral, so that no
    // // other function can access it. Otherwise the timer could be reset during a
    // // delay.
    // let mut delay = Delay::new(cp.SYST, clocks);
    //
    // // Now, enjoy the lightshow!
    // loop {
    //     led.set_high();
    //     delay.delay_ms(1_000_u16);
    //     led.set_low();
    //     delay.delay_ms(1_000_u16);
    // }
}
