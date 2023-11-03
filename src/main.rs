use std::{env, fs, io};
use std::arch::x86_64::__m128;
use netpbm;
use netpbm::Image;
use wsq::swt::filter::Filter;
use wsq::swt::{FloatImage, TwoChannelSubbandCoder};

fn parse_args() -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "Not enough arguments")));
    }
    Ok((args.remove(1), "".to_string()))
}

fn pgm_from_float_image(float_image: &FloatImage) -> Image<u16> {
    let data = float_image.data.iter().map(|f| *f as u16).collect::<Vec<_>>();
    Image {
        data,
        width: float_image.width,
        height: float_image.height,
        max_value: float_image.max_value as u16,
    }
}

fn f_img_from_netpbm(netpbm_image: &Image<u16>) -> FloatImage {
    let data = netpbm_image.data.iter().map(|u| *u as f64).collect::<Vec<_>>();
    FloatImage {
        data,
        width: netpbm_image.width,
        height: netpbm_image.height,
        min_value: 0.0,
        max_value: netpbm_image.max_value as f64,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let (input_image, _) = parse_args()?;
    let contents = fs::read(input_image)?;
    let pgm_image = netpbm::parser::parse(&contents)?;
    let image_copy = pgm_image.clone();
    let mut display = netpbm::display::SdlDisplay::try_new()?;
    display.display_netpbm(&image_copy.data, image_copy.width, image_copy.height, image_copy.max_value);

    let lowpass = Filter::WSS(vec![0.1, 0.1, 0.1, 0.1, 0.1]);
    let highpass = Filter::WSS(vec![0.0, 0.0, 0.1, 0.5, 1.]);
    let f_img = f_img_from_netpbm(&pgm_image);

    let subband_coder = TwoChannelSubbandCoder::new(lowpass, highpass);
    let (a0, a1) = subband_coder.analysis(&f_img);
    let a0_pgm = pgm_from_float_image(&a0);
    let a1_pgm = pgm_from_float_image(&a1);
    display.display_netpbm(&a0_pgm.data, a0_pgm.width, a0_pgm.height, a0_pgm.max_value);
    display.display_netpbm(&a1_pgm.data, a1_pgm.width, a1_pgm.height, a1_pgm.max_value);

    display.wait_for_exit();
    Ok(())
}
