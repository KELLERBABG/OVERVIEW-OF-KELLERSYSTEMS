/// A contiguous sub-band with its average interference power.
#[derive(Debug, Clone, Copy)]
pub struct SubBand {
    pub start_bin: usize,
    pub end_bin: usize,
    pub avg_power: f64,
}

/// Ranks contiguous clean sub-bands by lowest interference temperature.
pub struct WhitespaceFinder {
    /// Number of equal-width sub-bands to partition the spectrum into.
    pub sub_band_count: usize,
}

impl WhitespaceFinder {
    pub fn new(sub_band_count: usize) -> Self {
        Self { sub_band_count }
    }

    /// Partitions `psd` into `sub_band_count` equal bins and returns them
    /// sorted by ascending average power (cleanest first).
    pub fn rank(&self, psd: &[f64]) -> Vec<SubBand> {
        let n = psd.len();
        if n == 0 || self.sub_band_count == 0 {
            return Vec::new();
        }
        let bins_per_sub = n / self.sub_band_count;
        if bins_per_sub == 0 {
            return Vec::new();
        }

        let mut sub_bands: Vec<SubBand> = Vec::with_capacity(self.sub_band_count);
        for sb in 0..self.sub_band_count {
            let start = sb * bins_per_sub;
            let end = if sb == self.sub_band_count - 1 {
                n
            } else {
                (sb + 1) * bins_per_sub
            };
            let slice = &psd[start..end];
            let avg = slice.iter().sum::<f64>() / slice.len() as f64;
            sub_bands.push(SubBand {
                start_bin: start,
                end_bin: end - 1,
                avg_power: avg,
            });
        }

        sub_bands.sort_by(|a, b| a.avg_power.partial_cmp(&b.avg_power).unwrap());
        sub_bands
    }
}

impl Default for WhitespaceFinder {
    fn default() -> Self {
        Self::new(8)
    }
}

/// Legacy helper: returns contiguous regions where power is below `noise_floor`.
pub fn find_white_space(power_spectrum: &[f64], noise_floor: f64) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &power) in power_spectrum.iter().enumerate() {
        if power <= noise_floor {
            if start.is_none() {
                start = Some(i);
            }
        } else {
            if let Some(s) = start.take() {
                if i - s > 1 {
                    regions.push((s, i - 1));
                }
            }
        }
    }
    if let Some(s) = start {
        regions.push((s, power_spectrum.len() - 1));
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_cleanest_first() {
        // 4 sub-bands of 8 bins each; sub-band 2 is cleanest
        let mut psd = vec![1.0f64; 32];
        for i in 16..24 {
            psd[i] = 0.001;
        }
        let finder = WhitespaceFinder::new(4);
        let ranked = finder.rank(&psd);
        assert_eq!(ranked[0].start_bin, 16);
        assert!(ranked[0].avg_power < ranked[3].avg_power);
    }

    #[test]
    fn empty_spectrum_returns_empty() {
        let finder = WhitespaceFinder::new(4);
        assert!(finder.rank(&[]).is_empty());
    }
}
