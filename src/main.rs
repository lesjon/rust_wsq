use std::{env, fs, io};
use netpbm;
use wsq::dwt;
use wsq::dwt::ImageSubbandCoder;

fn parse_args() -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "Not enough arguments")));
    }
    Ok((args.remove(1), "".to_string()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let (input_image, _) = parse_args()?;
    let contents = fs::read(input_image)?;
    let pgm_image = netpbm::parser::parse(&contents)?;
    let image_copy = pgm_image.clone();
    let mut display = netpbm::display::SdlDisplay::try_new()?;
    display.display_netpbm(&image_copy.data, image_copy.width, image_copy.height, image_copy.max_value);
    let image = dwt::FloatImage::from(&pgm_image);

    let h_lowpass = dwt::LinearTimeInvariantFilter::new(vec![0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]);
    let h_highpass = dwt::LinearTimeInvariantFilter::new(vec![0., 0., 0., 0.5, 1., -1., -0.5, 0., 0., 0.]);
    let encoder = dwt::TwoChannelSubbandCoder::new(h_lowpass, h_highpass);
    let image_encoder = ImageSubbandCoder::new(encoder);
    let (a_0, a_1) = image_encoder.naive_row_analysis(&image);
    let a_0_image = netpbm::parser::image_from_float_vec(&a_0.data, a_0.width, a_0.height, pgm_image.max_value)?;
    let a_1_image = netpbm::parser::image_from_float_vec(&a_1.data, a_1.width, a_1.height, pgm_image.max_value)?;
    display.display_netpbm(&a_0_image.data, a_0_image.width, a_0_image.height, image_copy.max_value);
    display.display_netpbm(&a_1_image.data, a_1_image.width, a_1_image.height, image_copy.max_value);

    let recon_img = image_encoder.naive_synthesis(&a_0, &a_1);
    let recon_pgm = netpbm::parser::image_from_float_vec(&recon_img.data, recon_img.width, recon_img.height, pgm_image.max_value)?;
    display.display_netpbm(&recon_pgm.data, recon_pgm.width, recon_pgm.height, pgm_image.max_value);
    display.wait_for_exit();
    Ok(())
}
