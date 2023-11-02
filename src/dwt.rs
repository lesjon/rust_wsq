//! Discrete Wavelet Transformation
mod one_dimension;

use std::slice::Chunks;
use netpbm;
pub use crate::dwt::one_dimension::TwoChannelSubbandCoder;
pub use crate::dwt::one_dimension::LinearTimeInvariantFilter;

#[derive(Debug)]
pub struct TransformTable {}

pub struct FloatImage<F> {
    pub data: Vec<F>,
    pub width: usize,
    pub height: usize,
}

impl FloatImage<f64> {
    fn new(width: usize, height: usize) -> FloatImage<f64> {
        FloatImage {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    fn rows(&self) -> Chunks<'_, f64> {
        self.data.chunks(self.width)
    }

    pub fn from(source: &netpbm::Image<u16>) -> FloatImage<f64> {
        log::debug!("Creating FloatImage from netpbm::Image with size({},{})", source.width, source.height);
        let mut result = FloatImage::new(source.width, source.height);
        for (x, y) in (0..source.width).flat_map(|col| (0..source.height).map(move |row| (col, row))) {
            match source.data.get(y * source.width + x) {
                Some(point) => result.data[y * source.width + x] = *point as f64,
                None => {
                    log::error!("Source image size was insufficient for given width and height ({},{})", x, y);
                    panic!("Could not create FloatImage from netpbm::Image")
                }
            }
        }
        result
    }

    pub fn normalize(&mut self, midpoint: f64, rescale: f64) {
        for (x, y) in (0..self.width).flat_map(|col| (0..self.height).map(move |row| (row, col))) {
            self.data[y * self.width + x] = (self.data[y * self.width + x] - midpoint) / rescale;
        }
    }
}

pub struct ImageSubbandCoder {
    encoder: TwoChannelSubbandCoder<f64>,

}

impl ImageSubbandCoder {
    pub fn new(encoder: TwoChannelSubbandCoder<f64>) -> Self {
        ImageSubbandCoder {
            encoder
        }
    }

    pub fn naive_synthesis(&self, a_0: &FloatImage<f64>, a_1: &FloatImage<f64>) -> FloatImage<f64> {
        let mut data = vec![];
        for (a_0_row, a_1_row) in a_0.rows().zip(a_1.rows()) {
            data.extend_from_slice(a_0_row);
            data.extend_from_slice(a_1_row);
        }
        let mut result = FloatImage::new(a_0.width + a_1.width, a_0.height);
        result.data = data;
        result
    }

    pub fn naive_row_analysis(&self, image: &FloatImage<f64>) -> (FloatImage<f64>, FloatImage<f64>) {
        let mut a_0_rows = vec![];
        let mut a_1_rows = vec![];
        for row in image.rows() {
            let (a_0, a_1) = self.encoder.analysis(row);
            a_0_rows.push(a_0);
            a_1_rows.push(a_1);
        }
        let mut a_0 = FloatImage::new(a_0_rows[0].len(), image.height);
        a_0.data = a_0_rows.iter().flatten().map(|f| *f).collect();
        let mut a_1 = FloatImage::new(a_0_rows[0].len(), image.height);
        a_1.data = a_1_rows.iter().flatten().map(|f| *f).collect();
        (a_0, a_1)
    }
}