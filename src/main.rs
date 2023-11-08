use std::{env, fs, io};
use netpbm;
use wsq::swt;
use wsq::swt::{Analysis, Synthesis};
use wsq::swt::filter::Filter;

fn parse_args() -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "Not enough arguments")));
    }
    Ok((args.remove(1), "".to_string()))
}

fn pgm_from_float_image(float_image: &swt::FloatImage) -> netpbm::Image<u16> {
    fn map_float_to_u16(f: f64, min_f: f64, max_f: f64) -> u16 {
        ((f - min_f) * u16::MAX as f64 / max_f) as u16
    }
    let data = float_image.data.iter().map(|f| map_float_to_u16(*f, float_image.min_value, float_image.max_value))
        .collect::<Vec<_>>();
    netpbm::Image {
        data,
        width: float_image.width as u32,
        height: float_image.height as u32,
        max_value: u16::MAX,
    }
}

fn f_img_from_netpbm(netpbm_image: &netpbm::Image<u16>) -> swt::FloatImage {
    let data = netpbm_image.data.iter().map(|u| *u as f64).collect::<Vec<_>>();
    swt::FloatImage {
        data,
        width: netpbm_image.width as usize,
        height: netpbm_image.height as usize,
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
    display.display_netpbm(&image_copy, "Original")?;

    let lowpass = Filter::WSS(vec![0.85269867900940, 0.37740285561265, -0.11062440441842, -0.02384946501938, 0.037828455506995]);
    let highpass = Filter::WSA(vec![0.78848561640566, -0.41809227322221, -0.040689417609558, 0.064538882628938]);
    // let lowpass = Filter::WSS(vec![0.1, 0.1, 0.1, 0.1, 0.1]);
    // let highpass = Filter::WSA(vec![0., 0.5, 0., 0.]);
    let subband_coder = swt::TwoChannelSubbandCoder::new(lowpass, highpass);

    let mut f_img = f_img_from_netpbm(&pgm_image);
    f_img.auto_normalize();
    f_img.find_and_set_min_max();

    let mut a = subband_coder.analysis(&f_img)?;
    a.0.find_and_set_min_max();
    a.1.find_and_set_min_max();
    a.2.find_and_set_min_max();
    a.3.find_and_set_min_max();
    let a00_pgm = pgm_from_float_image(&a.0);
    let a01_pgm = pgm_from_float_image(&a.1);
    let a10_pgm = pgm_from_float_image(&a.2);
    let a11_pgm = pgm_from_float_image(&a.3);
    display.display_netpbm(&a00_pgm, "a00")?;
    display.display_netpbm(&a01_pgm, "a01")?;
    display.display_netpbm(&a10_pgm, "a10")?;
    display.display_netpbm(&a11_pgm, "a11")?;
    let mut reconstructed = subband_coder.synthesis(&a)?;
    reconstructed.find_and_set_min_max();
    let reconstructed = pgm_from_float_image(&reconstructed);
    display.display_netpbm(&reconstructed, "Reconstruction")?;

    display.wait_for_exit();
    Ok(())
}
