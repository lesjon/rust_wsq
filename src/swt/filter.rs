use std::slice::Iter;

pub enum Filter<F> {
    WSS(Vec<F>),
    HSS(Vec<F>),
}

impl<F> Filter<F>
    where F: Copy + std::ops::AddAssign + std::ops::Mul<F, Output=F> + std::ops::Neg<Output=F> + std::iter::Sum + Default {
    pub fn len(&self) -> usize {
        match self {
            Filter::WSS(coefficients) => coefficients.len() * 2 - 1,
            Filter::HSS(coefficients) => coefficients.len() * 2
        }
    }

    pub fn apply_highpass(&self, signal: Iter<'_, F>) -> Vec<F> {
        Self::apply(signal, self.coefficients_highpass())
    }

    pub fn apply_lowpass(&self, signal: Iter<'_, F>) -> Vec<F> {
        Self::apply(signal, self.coefficients_lowpass())
    }

    fn apply(signal: Iter<'_, F>, coefficients: ExtensionIter<'_, F>) -> Vec<F> {
        let stacked = signal.map(|s| coefficients.map(|c| c * *s).collect::<Vec<F>>()).collect::<Vec<_>>();
        let mut result = vec![];
        let max_size = stacked.len() + stacked[0].len() - 1;
        for i in 0..max_size {
            let mut diag_sum = F::default();
            for j in 0..i + 1 {
                if let Some(column) = stacked.get(i - j) {
                    if let Some(val) = column.get(j) {
                        diag_sum += *val;
                    }
                }
            }
            result.push(diag_sum);
        }
        result
    }

    fn coefficients_highpass(&self) -> ExtensionIter<'_, F> {
        match self {
            Filter::WSS(coefficients) => FilterExtension::WholeSampleHighpass(coefficients).into_iter(),
            Filter::HSS(coefficients) => FilterExtension::HalfSampleHighpass(coefficients).into_iter()
        }
    }
    fn coefficients_lowpass(&self) -> ExtensionIter<'_, F> {
        match self {
            Filter::WSS(coefficients) => FilterExtension::WholeSampleLowpass(coefficients).into_iter(),
            Filter::HSS(coefficients) => FilterExtension::HalfSampleLowpass(coefficients).into_iter()
        }
    }

    pub(crate) fn invert(&self) -> Self {
        match self {
            Filter::WSS(coefficients) => {
                let new_coefficients = coefficients.iter().enumerate()
                    .map(|(i, f)| if i % 2 == 0 { F::neg(*f) } else { *f });
                Filter::WSS(new_coefficients.collect())
            }
            Filter::HSS(coefficients) => {
                let new_coeff = coefficients.iter().enumerate()
                    .map(|(i, f)| if i % 2 == 1 { F::neg(*f) } else { *f });
                Filter::HSS(new_coeff.collect())
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum FilterExtension<'a, F> {
    HalfSampleLowpass(&'a [F]),
    WholeSampleLowpass(&'a [F]),
    HalfSampleHighpass(&'a [F]),
    WholeSampleHighpass(&'a [F]),
}

impl<'a, F> From<&'a Filter<F>> for FilterExtension<'a, F> {
    fn from(value: &'a Filter<F>) -> Self {
        match value {
            Filter::HSS(coefficients) => Self::HalfSampleLowpass(coefficients),
            Filter::WSS(coefficients) => Self::WholeSampleLowpass(coefficients)
        }
    }
}

impl<'a, F> FilterExtension<'a, F> {
    pub fn period(&self) -> usize {
        match self {
            FilterExtension::WholeSampleLowpass(coefficients) => coefficients.len() * 2 - 1,
            FilterExtension::HalfSampleLowpass(coefficients) => coefficients.len() * 2,
            FilterExtension::WholeSampleHighpass(coefficients) => coefficients.len() * 2 - 1,
            FilterExtension::HalfSampleHighpass(coefficients) => coefficients.len() * 2
        }
    }
}

#[derive(Copy, Clone)]
pub struct ExtensionIter<'a, F> {
    extension: FilterExtension<'a, F>,
    index: usize,
    period: usize,
}

impl<'a, F> Iterator for ExtensionIter<'a, F>
    where F: Copy + std::ops::Neg<Output=F> + {
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        let (extension, highpass) = match self.extension {
            FilterExtension::HalfSampleLowpass(extension) => (extension, false),
            FilterExtension::WholeSampleLowpass(extension) => (extension, false),
            FilterExtension::HalfSampleHighpass(extension) => (extension, true),
            FilterExtension::WholeSampleHighpass(extension) => (extension, true)
        };
        let result = if self.index == self.period {
            None
        } else if self.index < extension.len() {
            Some(extension[self.index])
        } else {
            if let Some(c) = extension.get(self.period - 1 - self.index) {
                if highpass {
                    Some(F::neg(*c))
                } else {
                    Some(*c)
                }
            } else {
                None
            }
        };
        self.index += 1;
        result
    }
}

impl<'a, F> IntoIterator for FilterExtension<'a, F>
    where F: Copy + std::ops::Neg<Output=F> {
    type Item = F;
    type IntoIter = ExtensionIter<'a, F>;

    fn into_iter(self) -> Self::IntoIter {
        let period = self.period();
        Self::IntoIter {
            extension: self,
            index: 0,
            period,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Filter, FilterExtension};

    #[test]
    fn test_len() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(6, hss.len());
        let wss = Filter::WSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(5, wss.len());
    }

    #[test]
    fn test_wss_extension_lowpass() {
        let wss = Filter::WSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(5, wss.len());
        let extension = FilterExtension::from(&wss);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[1., 2., 3., 2., 1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_wss_extension_highpass() {
        let extension = FilterExtension::WholeSampleHighpass::<f64>(&[1.0, 2.0, 3.0]);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[1., 2., 3., -2., -1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_hss_extension_highpass() {
        let extension = FilterExtension::HalfSampleHighpass::<f64>(&[1.0, 2.0, 3.0]);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[1., 2., 3., -3., -2., -1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_hss_extension() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(6, hss.len());
        let extension = FilterExtension::from(&hss);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = vec![1., 2., 3., 3., 2., 1.];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_invert_hss() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        let inverted = hss.invert();
        let expected = vec![1., -2., 3., 3., -2., 1.];
        let actual = inverted.coefficients_lowpass().take(expected.len()).collect::<Vec<f64>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_invert_wss() {
        let wss = Filter::WSS(vec![1.0, 2.0, 3.0]);
        let inverted = wss.invert();
        let expected = vec![-1., 2., -3., 2., -1.];
        let actual = inverted.coefficients_lowpass().take(expected.len()).collect::<Vec<f64>>();
        assert_eq!(expected, actual);
    }


    #[test]
    fn test_apply_highpass() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        let signal = &[1., 2., 3., 4.];
        let actual = hss.apply_highpass(signal.iter());
        let expected = vec![1.0, 4.0, 10.0, 13.0, 9.0, -2.0, -20.0, -11.0, -4.0];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_apply_lowpass() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        let signal = &[1., 2., 3., 4.];
        let actual = hss.apply_lowpass(signal.iter());
        let expected = vec![1., 4., 10., 19., 25., 26., 20., 11., 4.];
        assert_eq!(actual, expected);
    }
}
