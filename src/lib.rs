use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use image::{ImageBuffer, RgbImage};
use std::io::{Cursor, Read};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert_png_to_jpg(png_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    // wrapping obtained png data to an in-memory buffer
    let mut reader = Cursor::new(png_data);

    // collecting first 8 byte of PNG signature
    let mut signature = [0; 8];
    reader
        .read_exact(&mut signature)
        .map_err(|e| e.to_string())?;

    // PNG all start with same 8 byte of data so verifying them to check if its png or not
    if &signature != b"\x89PNG\r\n\x1a\n" {
        return Err("Not a valid PNG, currently only support PNG".into());
    }

    // metadata of the png to be split later
    let mut width = 0;
    let mut height = 0;
    let mut bit_depth = 0;
    let mut color_type = 0;
    let mut idat_data = Vec::new();

    loop {
        // u32 is 32bit unsigned integer so it is 4 byte in size. read_u32 read 4 byte of data i.e the chunk
        // that gives the length of chunks data field which is read in BigEndian format is the order in which the data is read
        // which is most significant bytes come first (MS -> 1.....2 <- LS)
        let length = match reader.read_u32::<BigEndian>() {
            Ok(len) => len,
            Err(_) => break,
        };

        // PNG are grouped in 4 byte chunk data sequentially so we have to extract them sequentially
        // after the length chunk we have a 4 byte chunk that tells if its IHDR, IDAT or IEND
        let mut chunk_type = [0; 4];
        reader
            .read_exact(&mut chunk_type)
            .map_err(|e| e.to_string())?;
        let chunk_type_str = std::str::from_utf8(&chunk_type).map_err(|e| e.to_string())?;

        let mut data = vec![0; length as usize];
        reader.read_exact(&mut data).map_err(|e| e.to_string())?;
        let _crc = reader.read_u32::<BigEndian>().map_err(|e| e.to_string())?;

        match chunk_type_str {
            // IHDR values are not supported in jpeg so we have to strip them
            // it is basically the data about height, width, bitdepth and color type of the image i.e. Metadata
            "IHDR" => {
                // error handling the data, bit depth and color type are 1 byte each not 4 which is
                // asssumed by bigendian
                // let mut ihdr = &data[..];
                // width = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
                // height = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
                // bit_depth = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
                // color_type = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;

                let mut ihdr = &data[..];
                width = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
                height = ihdr.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
                bit_depth = ihdr.read_u8().map_err(|e| e.to_string())?;
                color_type = ihdr.read_u8().map_err(|e| e.to_string())?;

                // there are further data in ihdr like compression filter_method and interlace,
                // previous version was only accounting for linear scanline and not interlaced png,
                // which still isnt being accounted.
                let _compression = ihdr.read_u8().map_err(|e| e.to_string())?;
                let _filter_method = ihdr.read_u8().map_err(|e| e.to_string())?;
                let interlace = ihdr.read_u8().map_err(|e| e.to_string())?;

                if interlace != 0 {
                    return Err("Interlaced PNGs are not supported".into());
                }
            }

            // actual image data required. Data Chunk
            "IDAT" => idat_data.extend(data),

            // endpoints
            "IEND" => break,
            _ => {}
        }
    }

    // currently only support 8bit rgb png
    if color_type != 2 || bit_depth != 8 {
        return Err("Only RGB 8-bit PNG supported".into());
    }

    // in png data is stored in IDAT chunks compressed using zlib which is a loseless compression format combination of LZ77 and huffman coding
    let mut decoder = ZlibDecoder::new(&idat_data[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| e.to_string())?;

    // we are assuming png to be a rgb format so bytes per pixel = 3
    let bytes_per_pixel = 3;
    // stride is number of bytes per image row
    let stride = width as usize * bytes_per_pixel;
    let mut raw_pixels = Vec::with_capacity((height as usize) * stride);

    // scanline is a single row of image pixels
    let mut prev_scanline = vec![0u8; stride];
    let mut offset = 0;

    // each scanline in png starts with 1 byte for the filter type
    for _ in 0..height {
        let filter_type = decompressed[offset];
        offset += 1;
        let scanline = &decompressed[offset..offset + stride];
        let mut unfiltered = vec![0u8; stride];

        // there exist 5 filter types in png, they are essentially used to improve compression in png
        match filter_type {
            // 0 represent no filter, data is as it is
            0 => unfiltered.copy_from_slice(scanline),

            // 1: Sub filter - each byte is stored as the difference from the corresponding byte of the pixel to the left.
            // For the first pixel in the row, no left neighbor exists, so the bytes are stored as-is.
            // Since we're working with RGB, each pixel occupies 3 bytes (R, G, B), so "left" means the value at (i - 3).
            1 => {
                for i in 0..stride {
                    let left = if i >= bytes_per_pixel {
                        unfiltered[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    unfiltered[i] = scanline[i].wrapping_add(left); // using wrapping_add to prevent the overflow so (255 + 1 = 0 & != 256)
                }
            }

            // 2: Up filter — each byte is stored as the difference from the byte above (same column, previous row).
            // During decoding, we add the value from the previous scanline to restore the original byte.
            // wrapping_add is used to handle 8-bit overflow as per PNG spec (mod 256 arithmetic).
            2 => {
                for i in 0..stride {
                    unfiltered[i] = scanline[i].wrapping_add(prev_scanline[i]);
                }
            }

            // 3: Average filter — each byte is stored as the difference from the average of the byte to the left and the byte above.
            // On decoding, we add the average back. For edge cases, missing neighbors are treated as 0.
            3 => {
                for i in 0..stride {
                    let left = if i >= bytes_per_pixel {
                        unfiltered[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    let up = prev_scanline[i];
                    unfiltered[i] = scanline[i].wrapping_add(((left as u16 + up as u16) / 2) as u8);
                }
            }

            // 4: Paeth filter — uses a nonlinear predictor based on left, above, and upper-left neighbors.
            // The predictor chooses the neighbor closest to the computed estimate (left + above - upper_left).
            // This improves compression by better approximating repeating patterns.
            4 => {
                for i in 0..stride {
                    let a = if i >= bytes_per_pixel {
                        unfiltered[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    let b = prev_scanline[i];
                    let c = if i >= bytes_per_pixel {
                        prev_scanline[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    unfiltered[i] = scanline[i].wrapping_add(paeth_predictor(a, b, c));
                }
            }

            _ => return Err("Unsupported filter type: {filter_type}".into()),
        }

        // adding the unfiltered data and managing next set of offset and scanlines
        raw_pixels.extend_from_slice(&unfiltered);
        prev_scanline.copy_from_slice(&unfiltered);
        offset += stride;
    }

    // creatung a image buffer of same size as the png for new jpeg data
    let img: RgbImage =
        ImageBuffer::from_raw(width, height, raw_pixels).ok_or("failed to create image buffer")?;

    let mut jpeg_data = Vec::new();
    // encoding obtained data to jpeg using image crate cause i got lazy
    let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut jpeg_data);
    encoder.encode_image(&img).map_err(|e| e.to_string())?;

    Ok(jpeg_data)
}

// paeth predictor used in png filtering
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}
