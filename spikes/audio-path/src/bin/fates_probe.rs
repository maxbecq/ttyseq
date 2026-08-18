//! fates-probe — SSD1322 OLED "hello world" for the Monome Fates.
//!
//! Goal: reset → init → fill the whole panel with a solid gray → the screen
//! should light up uniformly. This validates the SPI protocol and GPIO wiring
//! before writing the real `ttyseq-fates` crate.
//!
//! Everything here is transcribed from the norns reference driver:
//!   monome/norns : matron/src/hardware/screen/ssd1322.{h,cc}
//!
//! TWO THINGS TO VALIDATE EMPIRICALLY (documented, not assumed):
//!   1. GPIO lines for DC and RESET. The norns .h uses BCM 5 (DC) / 6 (RESET),
//!      but the okyeron/fates overlay README uses DC=17 / RESET=4. These sources
//!      disagree. This probe defaults to the *Fates* values (17/4). If the screen
//!      stays dark, try 5/6 by editing DC_LINE / RESET_LINE below.
//!   2. Column window (28, 91). Taken verbatim from norns refresh(). Trusted but
//!      not independently derived — if the image is shifted, this is the suspect.
//!
//! Buffer format (from norns refresh): the driver sends 128*64 = 8192 bytes,
//! i.e. ONE BYTE PER PIXEL, and the hardware uses only the HIGH NIBBLE of each
//! byte (0x00 = black … 0xF0/0xFF = full white). So filling with 0xFF = all on.

use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use gpiod::{Chip, Options, Lines, Output};
use spidev::{SpiModeFlags, Spidev, SpidevOptions};

// ---- Hardware constants (from ssd1322.h) --------------------------------

const SPIDEV_PATH: &str = "/dev/spidev0.0";
const GPIO_CHIP: &str = "gpiochip0";

// Fates wiring (okyeron/fates overlay README). If dark, try DC=5, RESET=6.
const DC_LINE: u32 = 17;
const RESET_LINE: u32 = 4;

const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const BUF_LEN: usize = WIDTH * HEIGHT; // 8192 bytes, 1 byte/pixel (high nibble used)

// SPI: MODE_0, 8 bits, ~18.75 MHz, MSB-first (norns open_spi()).
const SPI_HZ: u32 = 1_200_000_000 / 64; // 18.75 MHz
const SPI_CHUNK: usize = 4096; // matches spidev.bufsize on this PI OS Lite

// SSD1322 commands (subset actually used here)
const CMD_SET_COLUMN_ADDRESS: u8 = 0x15;
const CMD_WRITE_RAM: u8 = 0x5C;
const CMD_SET_ROW_ADDRESS: u8 = 0x75;
const CMD_SET_DUAL_COMM_LINE_MODE: u8 = 0xA0;
const CMD_SET_DISPLAY_START_LINE: u8 = 0xA1;
const CMD_SET_DISPLAY_OFFSET: u8 = 0xA2;
const CMD_SET_DISPLAY_MODE_NORMAL: u8 = 0xA6;
const CMD_SET_VDD_REGULATOR: u8 = 0xAB;
const CMD_SET_DISPLAY_OFF: u8 = 0xAE;
const CMD_SET_DISPLAY_ON: u8 = 0xAF;
const CMD_SET_PHASE_LENGTH: u8 = 0xB1;
const CMD_SET_OSCILLATOR_FREQUENCY: u8 = 0xB3;
const CMD_SET_DISPLAY_ENHANCEMENT_A: u8 = 0xB4;
const CMD_SET_PRECHARGE_VOLTAGE: u8 = 0xBB;
const CMD_SET_VCOMH_VOLTAGE: u8 = 0xBE;
const CMD_SET_CONTRAST_CURRENT: u8 = 0xC1;
const CMD_MASTER_CURRENT_CONTROL: u8 = 0xC7;
const CMD_SET_MULTIPLEX_RATIO: u8 = 0xCA;
const CMD_SET_DEFAULT_LINEAR_GRAY_SCALE: u8 = 0xB9;

// Values copied from norns (originally fbtft-ssd1322.c)
const MUX_RATIO: u8 = 0x3F;
// PHASE_LENGTH: norns does (0x02 | 0xF0) = 0xF2
const PHASE_LENGTH: u8 = 0x02 | 0xF0;

// Column window for the 128px panel (norns refresh: 28..=91)
const COL_START: u8 = 28;
const COL_END: u8 = 91;
const ROW_START: u8 = 0;
const ROW_END: u8 = 63;

// -------------------------------------------------------------------------

/// Holds the open peripherals for the probe.
struct Ssd1322 {
    spi: Spidev,
    dc: Lines<Output>,
    reset: Lines<Output>,
}

impl Ssd1322 {
    fn open() -> std::io::Result<Self> {
        // --- SPI ---
        let mut spi = Spidev::open(SPIDEV_PATH)?;
        let opts = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(SPI_HZ)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&opts)?;

        // --- GPIO (character device, libgpiod v2 style via `gpiod` crate) ---
        let chip = Chip::new(GPIO_CHIP)?;
        let dc = chip.request_lines(
            Options::output([DC_LINE]).consumer("fates-probe-dc"),
        )?;
        let reset = chip.request_lines(
            Options::output([RESET_LINE]).consumer("fates-probe-rst"),
        )?;

        Ok(Self { spi, dc, reset })
    }

    /// Send a command byte (DC low), then optional data bytes (DC high).
    fn command(&mut self, cmd: u8, data: &[u8]) -> std::io::Result<()> {
        self.dc.set_values([false])?; // DC = 0 : command
        self.spi.write_all(&[cmd])?;
        if !data.is_empty() {
            self.dc.set_values([true])?; // DC = 1 : data
            self.spi.write_all(data)?;
        }
        Ok(())
    }

    /// Hardware reset pulse. Reset is active-low; hold low, then high for normal op.
    fn hw_reset(&mut self) -> std::io::Result<()> {
        self.reset.set_values([false])?;
        sleep(Duration::from_millis(10));
        self.reset.set_values([true])?; // "keep HIGH during normal operation"
        sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Init sequence transcribed verbatim from norns ssd1322_init().
    fn init(&mut self) -> std::io::Result<()> {
        self.hw_reset()?;

        self.command(CMD_SET_DISPLAY_OFF, &[])?;
        self.command(CMD_SET_DEFAULT_LINEAR_GRAY_SCALE, &[])?;
        self.command(CMD_SET_OSCILLATOR_FREQUENCY, &[0x91])?;
        self.command(CMD_SET_MULTIPLEX_RATIO, &[MUX_RATIO])?;
        self.command(CMD_SET_DISPLAY_OFFSET, &[0x00])?;
        self.command(CMD_SET_DISPLAY_START_LINE, &[0x00])?;
        self.command(CMD_SET_VDD_REGULATOR, &[0x01])?;
        self.command(CMD_SET_DISPLAY_ENHANCEMENT_A, &[0xA0, 0xFD])?;
        self.command(CMD_SET_CONTRAST_CURRENT, &[0x7F])?;
        self.command(CMD_MASTER_CURRENT_CONTROL, &[0x0F])?;
        self.command(CMD_SET_PHASE_LENGTH, &[PHASE_LENGTH])?;
        self.command(CMD_SET_PRECHARGE_VOLTAGE, &[0x1F])?;
        self.command(CMD_SET_VCOMH_VOLTAGE, &[0x04])?;
        self.command(CMD_SET_DISPLAY_MODE_NORMAL, &[])?;

        // Fates variant (NOT the shield's 0x16,0x11). This is the line that
        // differs between Fates and norns-shield — see ssd1322.cc.
        self.command(CMD_SET_DUAL_COMM_LINE_MODE, &[0x04, 0x11])?;

        Ok(())
    }

    /// Push a full 8192-byte frame to the panel and turn the display on.
    fn blit(&mut self, buf: &[u8; BUF_LEN]) -> std::io::Result<()> {
        self.command(CMD_SET_COLUMN_ADDRESS, &[COL_START, COL_END])?;
        self.command(CMD_SET_ROW_ADDRESS, &[ROW_START, ROW_END])?;
        self.command(CMD_WRITE_RAM, &[])?;

        // RAM write is data: DC high, then stream the pixel bytes.
        self.dc.set_values([true])?;
        for chunk in buf.chunks(SPI_CHUNK) {
            self.spi.write_all(chunk)?;
        }

        // Only turn on after first frame, to avoid showing GDDRAM noise.
        self.command(CMD_SET_DISPLAY_ON, &[])?;
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    println!("fates-probe: opening SSD1322 on {SPIDEV_PATH} (DC=GPIO{DC_LINE}, RST=GPIO{RESET_LINE})");

    let mut screen = Ssd1322::open()?;
    println!("  peripherals open, running init sequence...");
    screen.init()?;

    // Fill: every byte 0xFF -> high nibble 0xF -> full-brightness pixel.
    // If you see a uniformly lit panel, the protocol + wiring are correct.
    let frame = [0xFFu8; BUF_LEN];
    println!("  blitting full-white frame ({BUF_LEN} bytes)...");
    screen.blit(&frame)?;

    println!("done. The panel should now be uniformly lit.");
    println!("If it is DARK: try DC=5 / RESET=6 (edit DC_LINE / RESET_LINE).");
    println!("If it is SHIFTED: the column window (28,91) is the suspect.");

    // Keep the GPIO lines held (screen on) for a few seconds before exit,
    // otherwise dropping `screen` releases the lines.
    sleep(Duration::from_secs(10));
    Ok(())
}
