use std::error::Error;
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
        let data = value.into_iter().flatten().collect::<Vec<f64>>();
        FloatImage {
            data,
            width,
            height,
            min_value: 0.,
            max_value: 1.,
        }
    }
}

struct Columns<'a, F> {
    data: &'a [F],
    width: usize,
    col: usize,
}

impl<'a, F> Columns<'a, F> {
    fn new(data: &'a [F], width: usize) -> Self {
        Self {
            data,
            width,
            col: 0,
        }
    }
}

impl<T> Iterator for Columns<'_, T> where T: Copy {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut col = vec![];
        if self.col >= self.width {
            return None;
        }
        for i in (0..self.data.len()).step_by(self.width) {
            col.push(self.data[i + self.col]);
        }
        self.col += 1;
        Some(col)
    }
}

impl FloatImage {
    pub fn rotate(&mut self) {
        self.data = self.columns().flatten().collect();
        let tmp = self.width;
        self.width = self.height;
        self.height = tmp;
    }
    pub fn get_mean_and_rescale(&self) -> (f64, f64) {
        let mean = self.data.iter().sum::<f64>() / self.data.len() as f64;
        let rescale = f64::max(self.max_value - mean, mean - self.min_value) / 128.;
        (mean, rescale)
    }

    pub fn auto_normalize(&mut self) {
        let (mean, rescale) = self.get_mean_and_rescale();
        log::debug!("Normalizing image with M:{} and R:{}", mean, rescale);
        self.normalize(mean, rescale)
    }

    pub fn normalize(&mut self, mean: f64, rescale: f64) {
        for f in self.data.iter_mut() {
            *f = (*f - mean) / rescale;
        }
    }

    fn columns(&self) -> Columns<'_, f64> {
        Columns::new(&self.data, self.width)
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
    fn downsample(signal: &[f64]) -> Vec<f64> {
        let mut downsampled = Vec::with_capacity(signal.len() / 2);
        // signal.into_iter().step_by(2).collect()
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

pub trait Analysis {
    fn analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage, FloatImage, FloatImage), Box<dyn Error>>;
    fn row_analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage), Box<dyn std::error::Error>>;
    fn column_analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage), Box<dyn std::error::Error>>;
    fn analysis_1d(&self, signal: &[f64]) -> (Vec<f64>, Vec<f64>);
}

pub trait Synthesis {
    fn synthesis_1d(&self, a_0: &[f64], a_1: &[f64]) -> Vec<f64>;
    fn synthesis(&self, a: &(FloatImage, FloatImage, FloatImage, FloatImage)) -> Result<FloatImage, Box<dyn Error>>;
    fn row_synthesis(&self, image_lowpass: &FloatImage, image_highpass: &FloatImage) -> Result<FloatImage, Box<dyn Error>>;
    fn column_synthesis(&self, image_lowpass: &FloatImage, image_highpass: &FloatImage) -> Result<FloatImage, Box<dyn Error>>;
}

impl Analysis for TwoChannelSubbandCoder<f64> {
    fn analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage, FloatImage, FloatImage), Box<dyn Error>> {
        let a_row = self.row_analysis(image)?;
        let a_0 = self.column_analysis(&a_row.0)?;
        let a_1 = self.column_analysis(&a_row.1)?;
        Ok((a_0.0, a_0.1, a_1.0, a_1.1))
    }
    fn row_analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage), Box<dyn std::error::Error>> {
        let mut rows_lowpass = vec![];
        let mut rows_highpass = vec![];

        for row in image.rows() {
            let (lowpassed, highpassed) = self.analysis_1d(row);
            rows_lowpass.push(lowpassed);
            rows_highpass.push(highpassed);
        }
        let rows_lowpass_image = FloatImage::from(rows_lowpass);
        let rows_highpass_image = FloatImage::from(rows_highpass);
        Ok((rows_lowpass_image, rows_highpass_image))
    }
    fn column_analysis(&self, image: &FloatImage) -> Result<(FloatImage, FloatImage), Box<dyn std::error::Error>> {
        let mut cols_lowpass = vec![];
        let mut cols_highpass = vec![];

        for col in image.columns() {
            let (lowpassed, highpassed) = self.analysis_1d(&col);
            cols_lowpass.push(lowpassed);
            cols_highpass.push(highpassed);
        }
        let mut rows_lowpass_image = FloatImage::from(cols_lowpass);
        let mut rows_highpass_image = FloatImage::from(cols_highpass);
        rows_lowpass_image.rotate();
        rows_highpass_image.rotate();
        Ok((rows_lowpass_image, rows_highpass_image))
    }

    fn analysis_1d(&self, signal: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let lowpassed = self.h_lowpass.apply(signal);
        let highpassed = self.h_highpass.apply(signal);
        let lowpassed = Self::downsample(&lowpassed);
        let highpassed = Self::downsample(&highpassed);
        (lowpassed, highpassed)
    }
}

impl Synthesis for TwoChannelSubbandCoder<f64> {
    fn synthesis_1d(&self, a_0: &[f64], a_1: &[f64]) -> Vec<f64> {
        let a_0 = Self::upsample(a_0);
        let a_1 = Self::upsample(a_1);
        let x_hat_0 = self.f_lowpass.apply(&a_0);
        let x_hat_1 = self.f_highpass.apply(&a_1);
        x_hat_0.iter().zip(x_hat_1).map(|(x_0, x_1)| *x_0 + x_1).collect()
    }


    fn synthesis(&self, a: &(FloatImage, FloatImage, FloatImage, FloatImage)) -> Result<FloatImage, Box<dyn Error>> {
        let y_0 = self.row_synthesis(&a.0, &a.1)?;
        let y_1 = self.row_synthesis(&a.2, &a.3)?;
        let x_hat = self.column_synthesis(&y_0, &y_1)?;
        Ok(x_hat)
    }
    fn row_synthesis(&self, image_lowpass: &FloatImage, image_highpass: &FloatImage) -> Result<FloatImage, Box<dyn Error>> {
        let mut data = vec![];

        for (a_0, a_1) in image_lowpass.rows().zip(image_highpass.rows()) {
            let reconstructed = self.synthesis_1d(a_0, a_1);
            data.push(reconstructed);
        }

        let mut result = FloatImage::from(data);
        result.max_value = (image_highpass.max_value + image_lowpass.max_value) / 2.;
        Ok(result)
    }

    fn column_synthesis(&self, image_lowpass: &FloatImage, image_highpass: &FloatImage) -> Result<FloatImage, Box<dyn Error>> {
        let mut data = vec![];

        for (a_0, a_1) in image_lowpass.columns().zip(image_highpass.columns()) {
            let reconstructed = self.synthesis_1d(&a_0, &a_1);
            data.push(reconstructed);
        }

        let mut result = FloatImage::from(data);
        result.rotate();
        result.max_value = (image_highpass.max_value + image_lowpass.max_value) / 2.;
        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use crate::swt::FloatImage;

    #[test]
    fn test_columns() {
        let image = FloatImage::from(vec![vec![1., 2., 3., 4., 5.], vec![6., 7., 8., 9., 0.], vec![1., 2., 3., 4., 5.], vec![6., 7., 8., 9., 0.], vec![1., 2., 3., 4., 5.]]);
        let mut cols = image.columns();
        assert_eq!(Some(vec![1., 6., 1., 6., 1.]), cols.next());
        assert_eq!(Some(vec![2., 7., 2., 7., 2.]), cols.next());
        assert_eq!(Some(vec![3., 8., 3., 8., 3.]), cols.next());
        assert_eq!(Some(vec![4., 9., 4., 9., 4.]), cols.next());
        assert_eq!(Some(vec![5., 0., 5., 0., 5.]), cols.next());
    }
}