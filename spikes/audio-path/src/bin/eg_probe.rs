//! eg_probe — SSD1322 driver with an `embedded-graphics` DrawTarget, written
//! from scratch on top of our own validated init + blit.
//!
//! Why this exists: the `ssd1322-zjy128x64` crate worked but hardcodes the
//! norns-*shield* remap (0x16), giving a 180-rotated image on our Fates panel,
//! and sends the whole framebuffer in one SPI write (needs bufsiz >= 8192).
//! Here we control all three: Fates remap (0x04), chunked blit sized to the
//! real bufsiz, and embedded-graphics for text/shapes.
//!
//! STILL A THROWAWAY BIN, but the `ssd1322` module below is deliberately
//! isolated so it can be lifted into a future `ttyseq-fates` crate. The engine
//! semantics stay out of it — it only knows how to draw pixels and blit.
//!
//! Hardware facts, all validated empirically on this Pi 4 / Fates:
//!   - /dev/spidev0.0, MODE_0, ~18.75 MHz
//!   - DC = GPIO17, RESET = GPIO4, gpiochip0
//!   - init dual-COM-line-mode 0x04 0x11 (Fates variant, NOT shield 0x16)
//!   - column window (28, 91), row (0, 63), 8192-byte frame, 1 byte/pixel,
//!     hardware uses the HIGH nibble of each byte
//!
//! Orientation note: we use the Fates remap 0x04 (vs shield 0x16). Per the
//! norns source, this register controls the screen flip, so 0x04 SHOULD give
//! the correct orientation with no per-pixel coordinate math. This is a strong
//! inference (norns' Fates branch + our first probe used 0x04), NOT yet proven
//! with oriented text on screen — the corner marker in main() is what confirms
//! it. If the image is still rotated, the remap bit-layout is the thing to
//! revisit (would need the Solomon Systech SSD1322 datasheet, reg 0xA0).
//!
//! NOT compiled here (no cargo in the authoring env). The embedded-graphics
//! and gpiod APIs were checked against their published sources, but expect
//! possible small adjustments on the Pi.

use std::fs;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::{Gray4, GrayColor},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

// ============================================================================
// module ssd1322 — extraction target for ttyseq-fates. No engine semantics.
// ============================================================================
mod ssd1322 {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU4;
    use gpiod::{Chip, Lines, Options, Output};
    use spidev::{SpiModeFlags, Spidev, SpidevOptions};

    pub const WIDTH: usize = 128;
    pub const HEIGHT: usize = 64;
    const BUF_LEN: usize = WIDTH * HEIGHT; // 8192, 1 byte/pixel (high nibble)

    const SPIDEV_PATH: &str = "/dev/spidev0.0";
    const GPIO_CHIP: &str = "gpiochip0";
    const DC_LINE: u32 = 17;
    const RESET_LINE: u32 = 4;
    const SPI_HZ: u32 = 1_200_000_000 / 64; // ~18.75 MHz (validated)

    // Fallback chunk if we can't read the real bufsiz. 4096 is universally safe.
    const CHUNK_FALLBACK: usize = 4096;

    // Commands (subset)
    const CMD_SET_COLUMN_ADDRESS: u8 = 0x15;
    const CMD_WRITE_RAM: u8 = 0x5C;
    const CMD_SET_ROW_ADDRESS: u8 = 0x75;
    const CMD_DUAL_COMM_LINE_MODE: u8 = 0xA0;
    const CMD_DISPLAY_START_LINE: u8 = 0xA1;
    const CMD_DISPLAY_OFFSET: u8 = 0xA2;
    const CMD_DISPLAY_MODE_NORMAL: u8 = 0xA6;
    const CMD_VDD_REGULATOR: u8 = 0xAB;
    const CMD_DISPLAY_OFF: u8 = 0xAE;
    const CMD_DISPLAY_ON: u8 = 0xAF;
    const CMD_PHASE_LENGTH: u8 = 0xB1;
    const CMD_OSCILLATOR_FREQUENCY: u8 = 0xB3;
    const CMD_DISPLAY_ENHANCEMENT_A: u8 = 0xB4;
    const CMD_DEFAULT_LINEAR_GRAY_SCALE: u8 = 0xB9;
    const CMD_PRECHARGE_VOLTAGE: u8 = 0xBB;
    const CMD_VCOMH_VOLTAGE: u8 = 0xBE;
    const CMD_CONTRAST_CURRENT: u8 = 0xC1;
    const CMD_MASTER_CURRENT: u8 = 0xC7;
    const CMD_MULTIPLEX_RATIO: u8 = 0xCA;

    // Column window for the 128px panel (validated): 28..=91.
    const COL_START: u8 = 28;
    const COL_END: u8 = 91;
    const ROW_START: u8 = 0;
    const ROW_END: u8 = 63;

    /// Reads the spidev per-transfer limit, falling back to a safe default.
    /// Not certain bufsiz is the *only* ceiling on BCM2711, but chunking to it
    /// is correct in all cases; the fallback covers a built-in (non-module)
    /// spidev where the sysfs file is absent.
    fn read_spi_chunk() -> usize {
        match fs::read_to_string("/sys/module/spidev/parameters/bufsiz") {
            Ok(s) => s.trim().parse::<usize>().unwrap_or(CHUNK_FALLBACK).max(1),
            Err(_) => CHUNK_FALLBACK,
        }
    }

    pub struct Ssd1322 {
        spi: Spidev,
        dc: Lines<Output>,
        reset: Lines<Output>,
        buf: [u8; BUF_LEN],
        chunk: usize,
    }

    impl Ssd1322 {
        pub fn open() -> std::io::Result<Self> {
            let mut spi = Spidev::open(SPIDEV_PATH)?;
            spi.configure(
                &SpidevOptions::new()
                    .bits_per_word(8)
                    .max_speed_hz(SPI_HZ)
                    .mode(SpiModeFlags::SPI_MODE_0)
                    .build(),
            )?;

            let chip = Chip::new(GPIO_CHIP)?;
            let dc = chip.request_lines(Options::output([DC_LINE]).consumer("ttyseq-dc"))?;
            let reset =
                chip.request_lines(Options::output([RESET_LINE]).consumer("ttyseq-rst"))?;

            let chunk = read_spi_chunk();

            Ok(Self { spi, dc, reset, buf: [0; BUF_LEN], chunk })
        }

        fn command(&mut self, cmd: u8, data: &[u8]) -> std::io::Result<()> {
            self.dc.set_values([false])?;
            self.spi.write_all(&[cmd])?;
            if !data.is_empty() {
                self.dc.set_values([true])?;
                self.spi.write_all(data)?;
            }
            Ok(())
        }

        fn hw_reset(&mut self) -> std::io::Result<()> {
            self.reset.set_values([false])?;
            sleep(Duration::from_millis(10));
            self.reset.set_values([true])?;
            sleep(Duration::from_millis(10));
            Ok(())
        }

        /// Init transcribed from norns ssd1322_init(), Fates branch (0x04).
        pub fn init(&mut self) -> std::io::Result<()> {
            self.hw_reset()?;
            self.command(CMD_DISPLAY_OFF, &[])?;
            self.command(CMD_DEFAULT_LINEAR_GRAY_SCALE, &[])?;
            self.command(CMD_OSCILLATOR_FREQUENCY, &[0x91])?;
            self.command(CMD_MULTIPLEX_RATIO, &[0x3F])?;
            self.command(CMD_DISPLAY_OFFSET, &[0x00])?;
            self.command(CMD_DISPLAY_START_LINE, &[0x00])?;
            self.command(CMD_VDD_REGULATOR, &[0x01])?;
            self.command(CMD_DISPLAY_ENHANCEMENT_A, &[0xA0, 0xFD])?;
            self.command(CMD_CONTRAST_CURRENT, &[0x7F])?;
            self.command(CMD_MASTER_CURRENT, &[0x0F])?;
            self.command(CMD_PHASE_LENGTH, &[0x02 | 0xF0])?;
            self.command(CMD_PRECHARGE_VOLTAGE, &[0x1F])?;
            self.command(CMD_VCOMH_VOLTAGE, &[0x04])?;
            self.command(CMD_DISPLAY_MODE_NORMAL, &[])?;
            // Fates orientation. Shield would be 0x16, 0x11.
            self.command(CMD_DUAL_COMM_LINE_MODE, &[0x04, 0x11])?;
            Ok(())
        }

        /// Push the internal buffer to the panel, chunked to the SPI limit.
        pub fn flush(&mut self) -> std::io::Result<()> {
            self.command(CMD_SET_COLUMN_ADDRESS, &[COL_START, COL_END])?;
            self.command(CMD_SET_ROW_ADDRESS, &[ROW_START, ROW_END])?;
            self.command(CMD_WRITE_RAM, &[])?;
            self.dc.set_values([true])?;
            // Borrow-split: copy chunk size out first to avoid borrow conflict.
            let chunk = self.chunk;
            // Can't iterate self.buf.chunks() while calling self.spi (both borrow
            // self). Take a raw slice via split.
            for start in (0..BUF_LEN).step_by(chunk) {
                let end = (start + chunk).min(BUF_LEN);
                self.spi.write_all(&self.buf[start..end])?;
            }
            Ok(())
        }

        pub fn display_on(&mut self) -> std::io::Result<()> {
            self.command(CMD_DISPLAY_ON, &[])
        }

        /// Set one logical pixel. Gray4 luma (0..=15) goes in the HIGH nibble;
        /// we duplicate it into the low nibble too, matching norns' behaviour.
        fn set_pixel(&mut self, x: i32, y: i32, color: Gray4) {
            if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                let luma = RawU4::from(color).into_inner(); // 0..=15
                let byte = (luma << 4) | luma;
                self.buf[y as usize * WIDTH + x as usize] = byte;
            }
        }
    }

    // ---- embedded-graphics glue ------------------------------------------

    impl OriginDimensions for Ssd1322 {
        fn size(&self) -> Size {
            Size::new(WIDTH as u32, HEIGHT as u32)
        }
    }

    impl DrawTarget for Ssd1322 {
        type Color = Gray4;
        type Error = std::convert::Infallible; // drawing to a RAM buffer can't fail

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(coord, color) in pixels {
                self.set_pixel(coord.x, coord.y, color);
            }
            Ok(())
        }

        fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
            let luma = RawU4::from(color).into_inner();
            self.buf.fill((luma << 4) | luma);
            Ok(())
        }
    }
}

// ============================================================================
// main — draws an orientation test pattern.
// ============================================================================
use ssd1322::Ssd1322;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("eg_probe: SSD1322 + embedded-graphics, Fates remap 0x04");

    let mut display = Ssd1322::open()?;
    display.init()?;

    // Clear to black, then draw asymmetric markers so orientation is obvious.
    display.clear(Gray4::BLACK)?;

    // Full-panel frame: if the column window is off, an edge is cut/wrapped.
    Rectangle::new(Point::new(0, 0), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(Gray4::new(6), 1))
        .draw(&mut display)?;

    // Solid 8x8 square in the TOP-LEFT: the unambiguous orientation mark.
    Rectangle::new(Point::new(0, 0), Size::new(8, 8))
        .into_styled(PrimitiveStyle::with_fill(Gray4::WHITE))
        .draw(&mut display)?;

    // Text near the top-left, reading left-to-right.
    let style = MonoTextStyle::new(&FONT_6X10, Gray4::WHITE);
    Text::with_baseline("ttySeq", Point::new(12, 2), style, Baseline::Top)
        .draw(&mut display)?;

    display.flush()?;
    display.display_on()?; // turn on only after first frame (avoid GDDRAM noise)

    println!("CHECK orientation:");
    println!("  - white 8x8 square in the TOP-LEFT corner?");
    println!("  - 'ttySeq' readable left-to-right, near the top?");
    println!("If rotated 180: the 0x04 remap assumption was wrong for this panel.");

    sleep(Duration::from_secs(30));
    Ok(())
}
