//! zjy_probe — test the `ssd1322-zjy128x64` crate AS-IS on the Fates panel.
//!
//! Purpose: see what the crate's hardcoded remap (0x16, the norns-*shield*
//! value) produces on our panel, which we validated with 0x04 (the *Fates*
//! value) in fates-probe. If the image is mirrored / doubled / shifted, that
//! confirms we'd need the Fates remap and thus a fork or a custom DrawTarget.
//!
//! Transport: rppal (NOT linux-embedded-hal), because rppal is what the crate's
//! own example uses and is therefore known to compile against it. This keeps the
//! test focused on the *rendering* question, not on transport plumbing.
//! rppal is a throwaway test dependency here, not an architecture commitment.
//!
//! Pins: DC=GPIO17, RESET=GPIO4 — our validated Fates wiring (the crate's
//! example used 22/27, which are that author's wiring, not ours).
//!
//! NOTE: not compiled here. Expect to adjust imports/versions on the Pi.

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use rppal::{
    gpio::Gpio,
    hal::Delay,
    spi::{Bus, Mode, SlaveSelect, Spi},
};
use ssd1322_zjy128x64::SSD1322;

// Our validated Fates wiring.
const DC_PIN: u8 = 17;
const RESET_PIN: u8 = 4;

fn main() -> Result<(), Box<dyn Error>> {
    use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
    // + l'import de SimpleHalSpiDevice, chemin à confirmer sur docs.rs :
    use rppal::spi::SimpleHalSpiDevice;  // <-- HYPOTHÈSE, à vérifier

    let spi_bus = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;
    let spi = SimpleHalSpiDevice::new(spi_bus);

    let gpio = Gpio::new()?;
    let dc = gpio.get(DC_PIN)?.into_output();
    let res = gpio.get(RESET_PIN)?.into_output();

    let mut display = SSD1322::new(spi, dc, res);
    let mut delay = Delay::new();
    display.init(&mut delay)?;

    // Draw something with clear left/right and top/bottom asymmetry, so any
    // mirroring or column doubling is obvious at a glance.
    display.clear(Gray4::BLACK)?;

    // A frame around the whole panel: if the column window is wrong, the right
    // edge will be cut or wrapped.
    Rectangle::new(Point::new(0, 0), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(Gray4::new(8), 1))
        .draw(&mut display)?;

    // Text near the LEFT edge: if mirrored, it lands on the right.
    let style = MonoTextStyle::new(&FONT_10X20, Gray4::WHITE);
    Text::with_baseline("ttySeq", Point::new(2, 2), style, Baseline::Top)
        .draw(&mut display)?;

    // A small filled square in the TOP-LEFT corner: unambiguous orientation mark.
    Rectangle::new(Point::new(0, 0), Size::new(8, 8))
        .into_styled(PrimitiveStyle::with_fill(Gray4::WHITE))
        .draw(&mut display)?;

    display.flush()?;

    println!("Displayed test pattern with the crate's default remap (0x16).");
    println!("CHECK:");
    println!("  - Is 'ttySeq' on the LEFT and readable (not mirrored)?");
    println!("  - Is the white 8x8 square in the TOP-LEFT corner?");
    println!("  - Is the border a single clean frame (no doubling / wrap)?");
    println!("If any of these are wrong, the Fates remap (0x04) is needed.");

    sleep(Duration::from_secs(30));
    Ok(())
}
