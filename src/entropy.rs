//! Module for encoding and decoding using Huffman Tables
// 2.2 Structure of compressed data
//     Compressed image data is described by a uniform structure and a set of parameters. The various parts
//     of the compressed image data are identified by special two-byte codes called markers. Some markers are
//     followed by particular sequences of parameters such as table specifications and headers. Others are used
//     without parameters for functions such as marking the start-of-image and end-of-image. When a marker is
//     associated with a particular sequence of parameters, the marker and its parameters comprise a marker
//     segment.
//     The data created by the entropy encoder are also segmented, and one particular marker - the restart
//     marker - is used to isolate entropy-coded data segments. The encoder outputs the restart markers,
//     intermixed with the entropy-coded data, between certain subband boundaries. Restart markers can be
//     identified without having to decode the compressed data to find them. Because they can be independently
//     decoded, entropy-coded data segments provide for progressive transmission, and isolation of data
//     corruption.
// 2.3 Interchange format
//     In addition to certain required marker segments and the entropy-coded segments, the interchange
//     format shall include the marker segments for all filter coefficient, quantization, and entropy-coding tables
//     needed by the decoding process. This guarantees that a compressed image can cross the boundary
//     between identification systems, regardless of how each environment internally associates tables with
//     compressed image data.
// 2.4 Abbreviated format for compressed image data
//     The abbreviated format for compressed image data is identical to the interchange format, except that
//     it does not include all tables required for decoding (it may include some of them). This format is intended
//     for use within applications where alternative mechanisms are available for supplying some or all of the
//     table-specification data needed for decoding.
// 2.5 Abbreviated format for table-specification data
//     This format contains only table-specification data. It is a means by which the application may install
//     in the decoder the tables required to subsequently reconstruct one or more fingerprint images.
#![allow(dead_code)]
pub mod encoder {}

pub mod decoder {}

#[derive(Debug)]
struct HuffmanTable {}

#[derive(Debug)]
pub struct CompressedData {}

#[derive(Debug)]
pub struct CompressedImageData {
    pub data: CompressedData,
}

#[derive(Debug)]
pub struct InterchangeFormat {}

enum AbbreviatedFormat {
    Image {},
    TableSpecification {},
}

pub mod markers {
    // start of image
    const SOI: &[u8] = &[0xFFu8, 0xA0u8];
    // End of image
    const EOI: &[u8] = &[0xFFu8, 0xA1u8];
    // Start of frame
    const SOF: &[u8] = &[0xFFu8, 0xA2u8];
    // Start of block
    const SOB: &[u8] = &[0xFFu8, 0xA3u8];
    // Define transform table
    const DTT: &[u8] = &[0xFFu8, 0xA4u8];
    // Define quantization table
    const DQT: &[u8] = &[0xFFu8, 0xA5u8];
    // Define Huffman tables(s)
    const DHT: &[u8] = &[0xFFu8, 0xA6u8];
    // Define restart interval
    const DRI: &[u8] = &[0xFFu8,  0xA7u8];
    //  Restart with modulo 8 count “m”, here set to 0
    const RST_M: &[u8] = &[0xFFu8, 0xB0u8];
    // Comment
    const COM: &[u8] = &[0xFFu8, 0xA8u8];
}
