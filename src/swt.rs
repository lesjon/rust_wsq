use std::slice;

pub mod filter;
pub mod signal;


pub struct FloatImage {
    pub data: Vec<f64>,
    pub width: usize,
    pub height: usize,
    pub min_value: f64,
    pub max_value: f64,
}

impl From<Vec<Vec<f64>>> for FloatImage {
    fn from(value: Vec<Vec<f64>>) -> Self {
        let width = value[0].len();
        let height = value.len();
        let data = value.iter().flatten().map(|f| *f).collect();
        FloatImage {
            data,
            width,
            height,
            min_value: 0.,
            max_value: 1.,
        }
    }
}

impl FloatImage {
    pub fn get_mean_and_rescale(&self) -> (f64, f64) {
        let mean = self.data.iter().sum::<f64>() / self.data.len() as f64;
        let rescale = f64::max(self.max_value - mean, mean - self.min_value) / 128.;
        (mean, rescale)
    }

    pub fn auto_normalize(&mut self) {
        let (mean, rescale) = self.get_mean_and_rescale();
        self.normalize(mean, rescale)
    }

    pub fn normalize(&mut self, mean: f64, rescale: f64) {
        for f in self.data.iter_mut() {
            *f = (*f - mean) / rescale;
        }
    }
    fn rows(&self) -> slice::Chunks<'_, f64> {
        self.data.chunks(self.width)
    }
    pub fn find_min_max(&self) -> (f64, f64) {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        self.data.iter().for_each(|f| {
            min = f64::min(min, *f);
            max = f64::max(max, *f)
        });
        (min, max)
    }

    pub fn find_and_set_min_max(&mut self) {
        let (min, max) = self.find_min_max();
        log::debug!("Setting min and max value of float image to {}, {}", min, max);
        self.min_value = min;
        self.max_value = max;
    }
}

pub struct TwoChannelSubbandCoder<F> {
    h_lowpass: filter::Filter<F>,
    h_highpass: filter::Filter<F>,
    f_lowpass: filter::Filter<F>,
    f_highpass: filter::Filter<F>,

}

impl<F> TwoChannelSubbandCoder<F>
    where F: Copy + std::ops::Mul<F, Output=F> + std::ops::Neg<Output=F> + std::iter::Sum + Default + std::ops::AddAssign + std::fmt::Debug {
    pub fn new(h_lowpass: filter::Filter<F>, h_highpass: filter::Filter<F>) -> TwoChannelSubbandCoder<F> {
        let f_lowpass = h_highpass.invert();
        let f_highpass = h_lowpass.invert();
        Self { h_lowpass, h_highpass, f_lowpass, f_highpass }
    }
}

impl TwoChannelSubbandCoder<f64> {
    fn analysis_1d(&self, signal: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let lowpassed = self.h_lowpass.apply(signal);
        let highpassed = self.h_highpass.apply(signal);
        let lowpassed = Self::downsample(&lowpassed);
        let highpassed = Self::downsample(&highpassed);
        (lowpassed, highpassed)
    }

    fn synthesis_1d(&self, a_0: &[f64], a_1: &[f64]) -> Vec<f64> {
        let a_0 = Self::upsample(a_0);
        let a_1 = Self::upsample(a_1);
        let x_hat_0 = self.f_lowpass.apply(&a_0);
        let x_hat_1 = self.f_highpass.apply(&a_1);
        x_hat_0.iter().zip(x_hat_1).map(|(x_0, x_1)| *x_0 + x_1).collect()
    }

    pub fn analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage), Box<dyn std::error::Error>> {
        let mut rows_lowpass = vec![];
        let mut rows_highpass = vec![];

        for row in image.rows() {
            let (lowpassed, highpassed) = self.analysis_1d(row);
            rows_lowpass.push(lowpassed);
            rows_highpass.push(highpassed);
        }
        let mut lowpass_image = FloatImage::from(rows_lowpass);
        lowpass_image.min_value = image.min_value;
        lowpass_image.max_value = image.max_value;
        let mut highpass_image = FloatImage::from(rows_highpass);
        highpass_image.min_value = image.min_value;
        highpass_image.max_value = image.max_value;
        Ok((lowpass_image, highpass_image))
    }

    pub fn synthesis(&self, image_lowpass: &FloatImage, image_highpass: &FloatImage) -> Result<FloatImage, Box<dyn std::error::Error>> {
        let mut data = vec![];

        for (a_0, a_1) in image_lowpass.rows().zip(image_highpass.rows()) {
            let reconstructed = self.synthesis_1d(a_0, a_1);
            data.push(reconstructed);
        }

        let mut result = FloatImage::from(data);
        result.max_value = (image_highpass.max_value + image_lowpass.max_value) / 2.;
        Ok(result)
    }

    fn downsample(signal: &[f64]) -> Vec<f64> {
        let mut downsampled = Vec::with_capacity(signal.len() / 2);
        signal.iter().step_by(2).for_each(|s| downsampled.push(*s));
        downsampled
    }

    fn upsample(signal: &[f64]) -> Vec<f64> {
        let mut result = vec![f64::default(); signal.len() * 2];
        for (i, f) in signal.iter().enumerate() {
            result[i * 2] = *f;
        }
        result
    }
}
