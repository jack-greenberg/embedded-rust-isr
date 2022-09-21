---
title: "Getting started with STM32H750VB in Rust"
date: 2022-09-21T16:18:56-04:00
draft: false
---

This is the story of my incredibly frustrating start with the STM32H750VB
microcontroller in Rust.

# Online shopping

A few months ago, I was browsing Adafruit and came across a cute looking dev
board called the [WeAct Studio STM32H750 Development Board]. It has

[WeAct Studio STM32H750 Development Board]: https://www.adafruit.com/product/5032

* an LCD screen,
* a camera,
* an SD card slot,
* a USB-C connector, and
* an STM32H750VB microcontroller.

This seemed like a great candidate to get started using embedded Rust,
especially because I was hoping to be able to use it as a platform to play
around with [Hubris].

[Hubris]: https://github.com/oxidecomputer/hubris

# Initial commit

While at the [Open Source Firmware Conference (OSFC)] in Sweden in September, I
started to mess around with the board[^1]. As with most embedded engineers, I
started with Blinky.

[^1]: This happened between the hours of 11pm and 1am as I was severly
  jet-lagged.

[Open Source Firmware Conference (OSFC)]: https://osfc.io

I began by cloning the [cortex-m-quickstart] repository on GitHub, a template
that can be used for ARM Cortex M microcontrollers. To begin with, I looked at
the configuration files to see what needed to change:

* `.cargo/config.toml`: The chip that I am using is a Cortex-M4F with an FPU, so
  the target (in `[build]`) should be `thumbv7em-none-eabihf`

* `Cargo.toml`: I added the `stm32h7xx_hal` crate that provides access to
  peripherals in a convenient way.

* `memory.x`: I set up the linker script to just include 128K of flash at
  `0x08000000` and 128K of RAM at `0x2000000`, which corresponds to the DTCM
  RAM. I tried using `0x30000000` for the SRAM1 region, but it didn't work[^2]
  :disappointed:

[^2]: Note to self: need to spend more time on this.

[cortex-m-quickstart]: https://github.com/rust-embedded/cortex-m-quickstart

In my `main.rs`, I started scaffolding the code for blinky:

```rust
#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;

use stm32h7xx_hal::{
    delay::Delay, pac, prelude::*
};

#[entry]
fn main() -> ! {
    // Init

    loop {

    }
}
```

We have our basic embedded application architecture with an [entrypoint], space
for initialization code, and a main loop.

[entrypoint]: https://twitter.com/jackgreenb/status/1572328501692305418?s=20&t=Z4xmnQe7G4iwrupgeeD2Dg

Let's start filling it in. In our initialization, we need access to the _device_
peripherals like the power configuration and GPIOs, as well as the _core_
peripherals (unique to Cortex-M devices) like the NVIC and SysTick timer:

```rust
// Device peripherals (i.e. GPIO)
let dp = pac::Peripherals::take().unwrap();

// Core peripherals (i.e. NVIC)
let cp = cortex_m::Peripherals::take().unwrap();
```

In Cortex-M-based devices (like ours), the **RCC** (reset and clock control)
peripheral controls which peripherals receive power and are enabled. We also
need to use the **PWR** (power) peripheral to make sure our device has the
correct power supply. We access these like so:

```rust
let pwrcfg: stm32h7xx_hal::pwr::PowerConfiguration = dp.PWR.constrain().freeze();
let rcc: stm32h7xx_hal::rcc::Rcc = dp.RCC.constrain();
```

When I saw this in the example code I was ~~pirating~~ learning from, I was
confused by `.constrain()` and `.freeze()`. It took a bit of digging, but I came
across [a great article by Jorge Aparicio] about the model that Rust uses to
interact with hardware peripherals (see the Aside for more details).

[a great article by Jorge Aparicio]: https://blog.japaric.io/brave-new-io/

> ### Aside: Hardware Peripherals in Rust
>
> In a nutshell, embedded Rust utilizes the ownership model for controlling
> access to peripherals.
>
> Starting with a concrete example, when we access `dp.RCC`, we are accessing a
> struct with a bunch of members that represent the parts of the PWR registers
> in our microcontroller. In C, we configure the PWR hardware block by writing
> to those registers. We do a similar thing in Rust, except it's abstracted.
> Calling `.constrain()` "consumes the original `RCC` which granted full access
> to every `RCC` register" and only allows us to modify aspects of the register
> defined in a struct called **`Parts`** (it's not crucial to know what is in
> `Parts`).
>
> When we call `.freeze()`, we consume the `Parts` struct, effectively
> preventing further modification of the peripherals. This ensures we don't have
> multiple places in our code trying to change the configuration of peripherals.
> Note that there are some peripherals that return a new object after calling
> `.freeze()` because there are parts that make sense to modify during the
> runtime of the device.

Ok back to blinky. We gain access to the CCDR (Core Clock Distribution and
Reset) struct using the RCC, PWR, and SYSCFG registers. (If this sounds like a
lot, don't worry--**it is**. I don't fully understand each and every register we
are writing to. Most of this code is copied and pasted and slightly tweaked
until I got things working.)

```rust
let ccdr: stm32h7xx_hal::rcc::Ccdr = rcc
    .sys_ck(96.MHz())
    .pclk1(48.MHz())
    .freeze(pwrcfg, &dp.SYSCFG);
```

The above code sets up the main clocks in the microcontroller at the given
frequencies, and then "freezes" the configuration so that it can't be modified.

This is the last bit of code we need for initializing the microcontroller
itself. Next, we will look at setting up GPIO specifically and blinking the LED.

## Blinking something

There are a number of GPIO "pin banks" in the STM32 microcontrollers. They are
in a group of 16 pins and each group is assigned a letter. So you might 16 pins
on Port A, labeled `PA0`, `PA1`, ..., `PA15`. To access individual pins, we have
to "split" the GPIO group:

```rust
let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
```

The LED on our board is connected to Port E, pin 3 (`PE3`), hence using `GPIOE`.
We can use the `gpioe` variable to access the individual pins like so:

```rust
let mut led = gpioe.pe3.into_push_pull_output();
```

We now have a variable that represents the pin our LED is attached to. We set it
to be _push/pull_ as opposed to _open drain_. The difference isn't important for
this post.

In order to blink the LED, we need to be able to delay for a certain amount of
time. Luckily, there is a helpful abstraction we can use: `delay::Delay`! This
is a struct with methods that allow a developer to insert arbitrary-time-length
delays into their code:

```rust
let mut delay = delay::Delay::new(cp.SYST, ccdr.clocks);
```

Now we have our LED variable and our delay variable, and that is all we need to
do for initialization! After all that, we implemenet a simple 2 line loop:

```rust
loop {
    led.toggle();
    delay.delay_ms(500_u16);
}
```

We toggle the LED and we delay for 500 milliseconds!

## Zooming back out

In all honesty, this was a pretty large effort. It took a while to understand
how the `.constrain()`, `.freeze()`, and `.split()` methods worked. On top of
that, some of the code examples I found online were outdated and didn't compile
straight out of the box.

> ### Aside: Embedded Rust Volatility
>
> One issue I have been having with my journey so far is how **volatile** the
> embedded Rust ecosystem is. I believe that the problem is definitely getting
> better, but there are small differences in different APIs and crates that make
> things difficult to just port over, and often the documentation associated
> with the crates isn't stellar. I can see this being one of the biggest
> hurdles to get over when starting off in embedded Rust.

The code to get blinky running is also substantially longer than the code would
be in C. However, as I would come to learn after trying a few more advanced
things that I'll cover in subsequent posts, the advantages of Rust are truly
enough to make me never want to write embedded C again.

The whole I/O system that Jorge Aparicio discusses in his blog post (linked
above and at the end) is quite nice. Rust's use of generic types and traits also
means that code is quite easy to port over. As I will explore in the next post,
abstractions make things very easy when you have complicated systems, such as an
LCD screen driven by an external chip using SPI _and_ GPIOs.

The next post will go into writing a simple program to display an image and text
on the little LCD screen of my dev board. It turns out to be far simpler than I
originally thought it might be :sweat_smile:

## Artifacts and Resources

* [Blinky with a ton of comments]: This is the same blinky I wrote about in this
  post, but with lots of comments on every line.
* [Brave new I/O]: A more thorough introduction to embedded Rust's approach to
  microcontroller code.
* [Learn modern embedded Rust]: A list of resources (very meta) for learning
  embedded Rust!
* [Rust embedded ecosystem and tools]: A list of tools to make embedded Rust
  easier and smoother
* [Demystifying Rust Embedded HAL Split and Constrain Methods]: An informative
  post about `.split()` and `.constrain()`.

[Blinky with a ton of comments]: https://github.com/jack-greenberg/embedded-rust-isr/blob/main/blinky/src/main.rs
[Brave new I/O]: https://blog.japaric.io/brave-new-io/
[Learn modern embedded Rust]: https://github.com/joaocarvalhoopen/How_to_learn_modern_Rust#embedded-rust
[Rust embedded ecosystem and tools]: https://www.anyleaf.org/blog/rust-embedded-ecosystem-and-tools
[WeAct Studio Dev Board]: https://www.adafruit.com/product/5032
[Demystifying Rust Embedded HAL Split and Constrain Methods]: https://dev.to/apollolabsbin/demystifying-rust-embedded-hal-split-and-constrain-methods-591e
