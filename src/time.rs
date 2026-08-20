/// Internal resolution: ticks per quarter note
pub const INTERNAL_PPQN: u32 = 960;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Tempo(pub f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frames(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalLength {
    Bars(u32),
    Beats(u32),
}

impl MusicalLength {
    /// Number of beats, where a beat is one unit of the signature's denominator
    /// In 7/8, a beat is an eigth note
    pub fn in_beats(&self, signature: TimeSignature) -> u64 {
        match *self {
            MusicalLength::Bars(n) => u64::from(n) * u64::from(signature.numerator),
            MusicalLength::Beats(n) => u64::from(n),
        }
    }

    pub fn in_ticks(&self, signature: TimeSignature) -> u64 {
        self.in_beats(signature) * u64::from(signature.ticks_per_beat())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatGrid {
    tempo: Tempo,
    sample_rate: SampleRate,
}

impl BeatGrid {
    pub fn new(tempo: Tempo, sample_rate: SampleRate) -> Self {
        Self { tempo, sample_rate }
    }

    /// Frame position of a tick, measured from the song origin.
    pub fn frame_at_ticks(&self, ticks: u64, ppqn: u32) -> Frames {
        let numerator = ticks as f64 * 60.0 * f64::from(self.sample_rate.0);
        let denominator = self.tempo.0 * f64::from(ppqn);
        Frames((numerator / denominator).round() as u64)
    }
}

impl TimeSignature {
    /// Ticks per beat, a beat being one unit of the denominator.
    pub fn ticks_per_beat(&self) -> u32 {
        INTERNAL_PPQN * 4 / u32::from(self.denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_quarter_at_120_bpm_lasts_half_a_second() {
        let grid = BeatGrid::new(Tempo(120.0), SampleRate(48_000));
        assert_eq!(
            grid.frame_at_ticks(u64::from(INTERNAL_PPQN), INTERNAL_PPQN),
            Frames(24_000)
        );
    }
}
