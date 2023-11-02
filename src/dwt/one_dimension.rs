use std::fmt::Debug;
use std::ops::{AddAssign, Mul, Neg};

pub struct LinearTimeInvariantFilter<F> {
    coefficients: Vec<F>,
}

impl<F> LinearTimeInvariantFilter<F>
    where F: Mul<F, Output=F> + Copy + Neg<Output=F> + AddAssign + Default + Debug {
    pub fn new(coefficients: Vec<F>) -> Self {
        Self { coefficients }
    }
    fn apply(&self, input: &[F]) -> Vec<F> {
        let signal_len = input.len();
        let kernel_len = self.coefficients.len();
        log::debug!("signal_len={signal_len},kernel_len={kernel_len}");
        let mut output = vec![F::default(); signal_len + kernel_len - 1];
        log::debug!("output_len={:?}", output.len());
        for i in 0..signal_len {
            for j in 0..kernel_len {
                output[i + j] += input[i] * self.coefficients[j];
            }
        }
        output
    }
    fn invert_highpass(&self) -> LinearTimeInvariantFilter<F> {
        let mut inverted_coefficients = vec![];
        for (i, h) in self.coefficients.iter().enumerate() {
            let h = if i % 2 == 1 { *h } else { F::neg(*h) };
            inverted_coefficients.push(h);
        }
        LinearTimeInvariantFilter {
            coefficients: inverted_coefficients
        }
    }
    fn invert_lowpass(&self) -> LinearTimeInvariantFilter<F> {
        let mut inverted_coeffs = vec![];
        for (i, h) in self.coefficients.iter().enumerate() {
            let h = if i % 2 == 0 { *h } else { F::neg(*h) };
            inverted_coeffs.push(h);
        }
        LinearTimeInvariantFilter {
            coefficients: inverted_coeffs
        }
    }
}

pub struct TwoChannelSubbandCoder<F> {
    h_lowpass: LinearTimeInvariantFilter<F>,
    h_highpass: LinearTimeInvariantFilter<F>,
    f_lowpass: LinearTimeInvariantFilter<F>,
    f_highpass: LinearTimeInvariantFilter<F>,
}

impl<F> TwoChannelSubbandCoder<F>
    where F: Default + Copy + Mul<F, Output=F> + Neg<Output=F> + AddAssign + Debug {
    pub fn new(h_lowpass: LinearTimeInvariantFilter<F>, h_highpass: LinearTimeInvariantFilter<F>) -> Self {
        let f_lowpass = h_highpass.invert_highpass();
        let f_highpass = h_lowpass.invert_lowpass();
        Self {
            h_lowpass,
            h_highpass,
            f_lowpass,
            f_highpass,
        }
    }
    pub fn upsample(input: &[F]) -> Vec<F> {
        let mut result = vec![F::default(); input.len() * 2];
        for (index, f) in input.iter().enumerate() {
            result[2 * index] = *f;
        }
        result
    }
    pub fn downsample(input: &[F]) -> Vec<F> {
        input.iter().enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, f)| *f).collect()
    }
}

impl TwoChannelSubbandCoder<f64> {
    pub fn analysis(&self, input: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let y_0 = self.h_lowpass.apply(input);
        let y_1 = self.h_highpass.apply(input);
        let a_0 = Self::downsample(&y_0);
        let a_1 = Self::downsample(&y_1);
        (a_0, a_1)
    }
    pub fn synthesis(&self, a_0: &[f64], a_1: &[f64]) -> Vec<f64> {
        let y_0 = Self::upsample(&a_0);
        let y_1 = Self::upsample(&a_1);
        let x_0 = self.f_lowpass.apply(&y_0);
        let x_1 = self.f_highpass.apply(&y_1);
        let start_skip = self.f_lowpass.coefficients.len() - 1;
        let end_skip = self.f_highpass.coefficients.len();
        x_0[..x_0.len() - end_skip].iter().zip(x_1).skip(start_skip).map(|(x, y)| *x + y).collect()
    }
}

pub struct WholeSampleSymmetricExtension<'a, F> {
    inner: &'a [F],
    index: usize,
    period: isize,
    dir: isize,
}

impl<'a> From<&'a [f64]> for WholeSampleSymmetricExtension<'a, f64> {
    fn from(signal: &'a [f64]) -> Self {
        let period = 2 * signal.len() as isize - 2;
        Self {
            inner: signal,
            index: 0,
            period,
            dir: 1,
        }
    }
}

impl<'a, F> Iterator for WholeSampleSymmetricExtension<'a, F> {
    type Item = &'a F;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner.get(self.index);
        if self.index == 0 && self.dir == -1 {
            self.dir = -self.dir;
        } else if self.index == self.inner.len() - 1 && self.dir == 1 {
            self.dir = -self.dir;
        } else {
            self.index = (self.index as isize + self.dir) as usize;
        }
        result
    }
}

pub struct HalfSampleSymmetricExtension<'a, F> {
    inner: &'a [F],
    index: usize,
    period: isize,
    dir: isize,
}

impl<'a, F> HalfSampleSymmetricExtension<'a, F> {
    fn from(signal: &'a [F]) -> Self {
        Self {
            inner: signal,
            index: 0,
            period: signal.len() as isize * 2,
            dir: 1,
        }
    }
}

impl<'a, F> Iterator for HalfSampleSymmetricExtension<'a, F> {
    type Item = &'a F;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner.get(self.index);
        if self.index == 0 && self.dir == -1 {
            self.dir = -self.dir;
        } else if self.index == self.inner.len() - 1 && self.dir == 1 {
            self.dir = -self.dir;
        }
        self.index = (self.index as isize + self.dir) as usize;
        result
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 0.001;

    #[test]
    fn test_convolution() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![0.2, 0.4, 0.2];
        let expecteds = vec![0.2, 0.8, 1.6, 2.4, 3.2, 2.8, 1.0];
        let filter = LinearTimeInvariantFilter::new(kernel);
        let actuals = filter.apply(&input);
        assert_eq!(actuals.len(), expecteds.len());
        for (actual, expected) in actuals.iter().zip(expecteds) {
            assert!(f64::abs(actual - expected) < EPSILON)
        }
    }

    #[test]
    fn test_half_sample_extension() {
        let signal = &[1., 2., 3., 4.];
        let extension = HalfSampleSymmetricExtension::from(signal);
        let actual = extension.into_iter().take(13).map(|f| *f).collect::<Vec<f64>>();
        let expected = vec![1., 2., 3., 4., 3., 2., 1., 2., 3., 4., 3., 2., 1.];
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_whole_sample_extension() {
        let signal = &[1., 2., 3., 4.];
        let extension = WholeSampleSymmetricExtension::from(signal);
        let actual = extension.into_iter().take(16).map(|f| *f).collect::<Vec<f64>>();
        let expected = vec![1., 2., 3., 4., 4., 3., 2., 1., 1., 2., 3., 4., 4., 3., 2., 1.];
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_encoder_setup() {
        let h_lowpass = LinearTimeInvariantFilter::new(vec![0.0352, -0.0854, -0.1350, 0.4599, 0.8069, 0.3327]);
        let h_highpass = LinearTimeInvariantFilter::new(vec![-0.3327, 0.8069, -0.4599, -0.1350, 0.0854, 0.0352]);

        let encoder = TwoChannelSubbandCoder::new(h_lowpass, h_highpass);

        assert_eq!(vec![0.0352, 0.0854, -0.1350, -0.4599, 0.8069, -0.3327], encoder.f_highpass.coefficients);
        assert_eq!(vec![0.3327, 0.8069, 0.4599, -0.1350, -0.0854, 0.0352], encoder.f_lowpass.coefficients);
    }

    #[test]
    fn test_two_channel_subband_coder() {
        // Analysis filters (simplified example)
        let h_lowpass = LinearTimeInvariantFilter::new(vec![0.0352, -0.0854, -0.1350, 0.4599, 0.8069, 0.3327]);
        let h_highpass = LinearTimeInvariantFilter::new(vec![-0.3327, 0.8069, -0.4599, -0.1350, 0.0854, 0.0352]);
        let encoder = TwoChannelSubbandCoder::new(h_lowpass, h_highpass);

        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        // Encoding: Create subbands by filtering the original signal
        let (low_pass_subband, high_pass_subband) = encoder.analysis(&signal);

        // Decoding: Reconstruct the signal from subbands
        let reconstructed_signal = encoder.synthesis(&low_pass_subband, &high_pass_subband);
        assert_eq!(reconstructed_signal.len(), signal.len());
        for (x,y) in reconstructed_signal.iter().zip(signal){
            assert!(f64::abs(x-y) < EPSILON);
        }
    }

    #[test]
    fn test_inversions() {
        let h_0 = LinearTimeInvariantFilter::new(vec![0.1, 0.2, 0.3, 0.4]);
        let h_1 = LinearTimeInvariantFilter::new(vec![0.5, 0.6, 0.7, 0.8]);

        let f_0 = h_1.invert_highpass();
        let f_1 = h_0.invert_lowpass();

        let expected_f_0 = vec![-0.5, 0.6, -0.7, 0.8];
        let expected_f_1 = vec![0.1, -0.2, 0.3, -0.4];

        assert_eq!(f_0.coefficients, expected_f_0);
        assert_eq!(f_1.coefficients, expected_f_1);
    }

    #[test]
    fn test_downsample() {
        let signal = &[0., 1., 2., 3., 4.];
        let downsampled = TwoChannelSubbandCoder::downsample(signal);
        assert_eq!(downsampled, vec![0., 2., 4.]);
    }

    #[test]
    fn test_upsample() {
        let signal = &[0., 1., 2., 3., 4.];
        let upsampled = TwoChannelSubbandCoder::upsample(signal);
        assert_eq!(upsampled, vec![0., 0., 1., 0., 2., 0., 3., 0., 4., 0.]);
    }
}