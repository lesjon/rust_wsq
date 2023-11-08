use crate::swt::signal;

pub enum Filter<F> {
    WSS(Vec<F>),
    HSS(Vec<F>),
    WSA(Vec<F>),
    HSA(Vec<F>),
}

impl<F> Filter<F>
    where F: Copy + std::ops::AddAssign + std::ops::Mul<F, Output=F> + std::ops::Neg<Output=F> + std::iter::Sum + Default + std::fmt::Debug {
    pub fn len(&self) -> usize {
        match self {
            Filter::WSS(coefficients) | Filter::WSA(coefficients) => coefficients.len() * 2 - 1,
            Filter::HSS(coefficients) | Filter::HSA(coefficients) => coefficients.len() * 2
        }
    }

    pub fn apply(&self, signal: &[F]) -> Vec<F> {
        let coefficients = self.coefficients();
        let signal_extension = match self {
            Filter::WSS(_) | Filter::WSA(_) => signal::SignalExtension::WholeSample(signal),
            Filter::HSS(_) | Filter::HSA(_) => signal::SignalExtension::HalfSample(signal),
        };
        let stacked = signal_extension.into_iter().map(|s| coefficients.map(|c| c * *s)
            .collect::<Vec<F>>()).collect::<Vec<_>>();
        let mut result = vec![];
        let max_size = (stacked.len() + stacked[0].len() - 1) / 2;
        let boundary = self.len() / 2 - 1;
        for i in boundary..max_size {
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

    fn coefficients(&self) -> FilterIter<'_, F> {
        match self {
            Filter::WSA(coefficients) => FilterExtension::WholeSampleHighpass(coefficients).into_iter(),
            Filter::HSA(coefficients) => FilterExtension::HalfSampleHighpass(coefficients).into_iter(),
            Filter::WSS(coefficients) => FilterExtension::WholeSampleLowpass(coefficients).into_iter(),
            Filter::HSS(coefficients) => FilterExtension::HalfSampleLowpass(coefficients).into_iter(),
        }
    }

    fn invert_even_negative(coefficients: &[F]) -> Vec<F> {
        coefficients.iter().enumerate()
            .map(|(i, f)| if i % 2 == 0 { F::neg(*f) } else { *f }).collect()
    }

    fn invert_odd_negative(coefficients: &[F]) -> Vec<F> {
        coefficients.iter().enumerate()
            .map(|(i, f)| if i % 2 == 1 { F::neg(*f) } else { *f }).collect()
    }

    pub fn invert(&self) -> Self {
        // f_0(n) = (-1)^n h_1(n-1)
        // f_1(n) = (-1)^(n-1) h_0(n-1)
        match self {
            Filter::WSS(coefficients) => Filter::WSA(Self::invert_even_negative(coefficients)),
            Filter::WSA(coefficients) => Filter::WSS(Self::invert_odd_negative(coefficients)),
            Filter::HSS(coefficients) => Filter::HSA(Self::invert_even_negative(coefficients)),
            Filter::HSA(coefficients) => Filter::HSS(Self::invert_odd_negative(coefficients)),
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
            Filter::WSS(coefficients) => Self::WholeSampleLowpass(coefficients),
            Filter::WSA(coefficients) => Self::WholeSampleHighpass(coefficients),
            Filter::HSA(coefficients) => Self::HalfSampleHighpass(coefficients),
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
pub struct FilterIter<'a, F> {
    extension: FilterExtension<'a, F>,
    index: usize,
    period: usize,
}

impl<'a, F> Iterator for FilterIter<'a, F>
    where F: Copy + std::ops::Neg<Output=F> + {
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        let (extension, highpass, whole_sample_offset) = match self.extension {
            FilterExtension::HalfSampleLowpass(extension) => (extension, false, 0),
            FilterExtension::WholeSampleLowpass(extension) => (extension, false, 1),
            FilterExtension::HalfSampleHighpass(extension) => (extension, true, 0),
            FilterExtension::WholeSampleHighpass(extension) => (extension, true, 1)
        };

        let last_mirrored = extension.len() - whole_sample_offset;

        let result = if self.index == self.period {
            None
        } else if self.index < last_mirrored {
            let coefficient = extension[extension.len() - self.index - 1];
            if highpass {
                Some(F::neg(coefficient))
            } else {
                Some(coefficient)
            }
        } else if self.index < self.period {
            Some(extension[self.index - last_mirrored])
        } else { None };
        self.index += 1;
        result
    }
}

impl<'a, F> IntoIterator for FilterExtension<'a, F>
    where F: Copy + std::ops::Neg<Output=F> {
    type Item = F;
    type IntoIter = FilterIter<'a, F>;

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

    const EPSILON: f64 = 0.001;

    fn assert_close_enough(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "len of {{ {:?} and {:?} }} not equal", actual, expected);
        for (a, e) in actual.iter().zip(expected) {
            assert!(f64::abs(a - e) < EPSILON, "value {} and {} of {{ {:?} and {:?} }} not equal", a, e, actual, expected);
        }
    }

    #[test]
    fn test_len() {
        let hss = Filter::HSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(6, hss.len());
        let wss = Filter::WSS(vec![1.0, 2.0, 3.0]);
        assert_eq!(5, wss.len());
    }

    #[test]
    fn test_wss_extension() {
        let wss = Filter::WSS(vec![3.0, 2.0, 1.0]);
        assert_eq!(5, wss.len());
        let extension = FilterExtension::from(&wss);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[1., 2., 3., 2., 1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_wsa_extension() {
        let filter = Filter::WSA(vec![3.0, 2.0, 1.0]);
        let extension = FilterExtension::from(&filter);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[-1., -2., 3., 2., 1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_hsa_extension() {
        let filter = Filter::HSA(vec![3.0, 2.0, 1.0]);
        let extension = FilterExtension::from(&filter);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = &[-1., -2., -3., 3., 2., 1.];
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn test_hss_extension() {
        let hss = Filter::HSS(vec![3.0, 2.0, 1.0]);
        assert_eq!(6, hss.len());
        let extension = FilterExtension::from(&hss);
        let actual = extension.into_iter().collect::<Vec<f64>>();
        let expected = vec![1., 2., 3., 3., 2., 1.];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_invert_hsa() {
        let hss = Filter::HSA(vec![1.0, 2.0, 3.0]);
        let inverted = hss.invert();
        let expected = vec![3.0, -2.0, 1.0, 1.0, -2.0, 3.0];
        let actual = inverted.coefficients().take(expected.len()).collect::<Vec<_>>();
        assert_close_enough(&actual, &expected)
    }

    #[test]
    fn test_invert_wsa() {
        // f_0(n) = (-1)^n h_1(n-1)
        let wss = Filter::WSA(vec![1.0, 2.0, 3.0]);
        let inverted = wss.invert();
        let expected = vec![3.0, -2.0, 1.0, -2.0, 3.0];
        let actual = inverted.coefficients().take(expected.len()).collect::<Vec<_>>();
        assert_close_enough(&actual, &expected)
    }

    #[test]
    fn test_invert_hss() {
        // f_1(n) = (-1)^(n-1) h_0(n-1)
        let hss = Filter::HSS(vec![3.0, 2.0, 1.0]);
        let inverted = hss.invert();
        let expected = vec![1., -2., 3., -3., 2., -1.];
        let actual = inverted.coefficients().take(expected.len()).collect::<Vec<_>>();
        assert_close_enough(&actual, &expected)
    }

    #[test]
    fn test_invert_wss() {
        // f_1(n) = (-1)^(n-1) h_0(n-1)
        let wss = Filter::WSS(vec![3.0, 2.0, 1.0]);
        assert_eq!(wss.coefficients().collect::<Vec<f64>>(), vec![1.,2.,3.,2.,1.]);
        let inverted = wss.invert();
        let expected = vec![1., -2., -3., 2., -1.];
        let actual = inverted.coefficients().take(expected.len()).collect::<Vec<_>>();
        assert_close_enough(&actual, &expected)
    }

    #[test]
    fn test_apply_hsa_to_constant() {
        let hsa = Filter::HSA(vec![0.0, 0.1, 0.8]);
        let signal = &[1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.];
        let actual = hsa.apply(signal);
        let expected = vec![-0.9, -0.9, -0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_close_enough(&actual, &expected)
    }
}
