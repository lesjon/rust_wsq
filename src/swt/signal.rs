pub enum SignalExtension<'a, F> {
    HalfSample(&'a [F]),
    WholeSample(&'a [F]),
}

pub struct SignalIter<'a, F> {
    extension: SignalExtension<'a, F>,
    index: usize,
    dir: isize,
    period_count: usize
}

impl<'a, F: Copy> Iterator for SignalIter<'a, F> {
    type Item = &'a F;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.extension {
            SignalExtension::HalfSample(extension) => {
                if self.index == 0 && self.dir == -1 {
                    self.period_count -= 1;
                    if self.period_count == 0 {
                        return None;
                    }
                    self.dir = 1;
                } else if self.index == extension.len() - 1 && self.dir == 1 {
                    self.dir = -1;
                } else {
                    self.index = (self.index as isize + self.dir) as usize;
                }
                extension.get(self.index)
            }
            SignalExtension::WholeSample(extension) => {
                if self.index == 0 && self.dir == -1 {
                    self.period_count -= 1;
                    if self.period_count == 0 {
                        return None;
                    }
                    self.dir = 1;
                } else if self.index == extension.len() - 1 && self.dir == 1 {
                    self.dir = -1;
                }
                self.index = (self.index as isize + self.dir) as usize;
                extension.get(self.index)
            }
        }
    }
}

impl<'a, F: Copy> IntoIterator for SignalExtension<'a, F> {
    type Item = &'a F;
    type IntoIter = SignalIter<'a, F>;

    fn into_iter(self) -> Self::IntoIter {
        let start_dir: isize = match self {
            SignalExtension::HalfSample(_) => -1,
            SignalExtension::WholeSample(_) => -1
        };
        let start_index: usize = match self {
            SignalExtension::HalfSample(_) => 0,
            SignalExtension::WholeSample(_) => 1
        };
        Self::IntoIter {
            extension: self,
            index: start_index,
            dir: start_dir,
            period_count: 2
        }
    }
}
