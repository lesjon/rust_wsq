use std::slice::Chunks;

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
    fn rows(&self) -> Chunks<'_, f64> {
        self.data.chunks(self.width)
    }
}

pub struct TwoChannelSubbandCoder<F> {
    h_lowpass: filter::Filter<F>,
    h_highpass: filter::Filter<F>,
    f_lowpass: filter::Filter<F>,
    f_highpass: filter::Filter<F>,

}

impl<F> TwoChannelSubbandCoder<F>
    where F: Copy + std::ops::Mul<F, Output=F> + std::ops::Neg<Output=F> + std::iter::Sum + Default + std::ops::AddAssign {
    pub fn new(h_lowpass: filter::Filter<F>, h_highpass: filter::Filter<F>) -> TwoChannelSubbandCoder<F> {
        let f_lowpass = h_highpass.invert();
        let f_highpass = h_lowpass.invert();
        Self { h_lowpass, h_highpass, f_lowpass, f_highpass }
    }
}

impl TwoChannelSubbandCoder<f64> {
    pub fn analysis(&self, image: &FloatImage) -> (FloatImage, FloatImage) {
        let mut rows_lowpass = vec![];
        let mut rows_highpass = vec![];

        for row in image.rows() {
            rows_lowpass.push(self.h_lowpass.apply_lowpass(row.iter()));
            rows_highpass.push(self.h_highpass.apply_highpass(row.iter()));
        }
        let mut lowpass_image = FloatImage::from(rows_lowpass);
        lowpass_image.min_value = image.min_value;
        lowpass_image.max_value = image.max_value;
        let mut highpass_image = FloatImage::from(rows_highpass);
        highpass_image.min_value = image.min_value;
        highpass_image.max_value = image.max_value;
        (lowpass_image, highpass_image)
    }
}
