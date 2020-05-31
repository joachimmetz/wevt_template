use std::{fs, io::prelude::*};

use log::{debug, error};
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilError {
    #[error("insufficient file access")]
    FileAccess,
    #[error("invalid file format")]
    FileFormat,
}

/// Static cast the given 64-bit unsigned integer to a 64-bit signed integer.
/// This is probably only useful when some other code provides you a u64
///  that is meant to be an i64 (aka. uncommon).
///
/// In C: `*(int64_t *)&i`
///
/// # Examples
///
/// ```
/// use wevt_template::util::*;
/// assert_eq!(0, u64_i64(0));
/// assert_eq!(1, u64_i64(0x1));
/// assert_eq!(-1, u64_i64(0xFFFF_FFFF_FFFF_FFFF));
/// ```
pub fn u64_i64(i: u64) -> i64 {
    if i & 1 << 63 > 0 {
        // TODO: there's probably some elegant rust-way to do this.
        // in the meantime, manually compute twos-complement.
        let bits = i & 0x7FFF_FFFF_FFFF_FFFF;
        let bits = !bits;
        let bits = bits + 1;
        let bits = bits & 0x7FFF_FFFF_FFFF_FFFF;
        -(bits as i64)
    } else {
        i as i64
    }
}

/// Round the given value up to the next multiple of the given base.
///
/// # Panics
///
///   - Base `b` must be at least `2`.
///
/// # Examples
///
/// ```
/// use wevt_template::util::*;
/// assert_eq!(align(0, 2), 0);
/// assert_eq!(align(1, 2), 2);
/// assert_eq!(align(2, 2), 2);
/// assert_eq!(align(3, 2), 4);
/// assert_eq!(align(4, 2), 4);
/// ```
pub fn align(i: u64, b: u64) -> u64 {
    if b < 2 {
        panic!("base `b` must be at least: 2");
    }
    let rem = i % b;
    if rem == 0 {
        i
    } else {
        i + (b - rem)
    }
}

pub fn hexdump_ascii(b: u8) -> char {
    if b.is_ascii_graphic() || b == b' ' {
        b as char
    } else {
        '.'
    }
}

pub fn hexdump(buf: &[u8], offset: usize) -> String {
    // 01234567:  00 01 02 03 04 05 06 07  ...............
    // <prefix>   <hex col>                <ascii col>

    let padding = "  ";

    let padding_size = 2;
    let hex_col_size = 3;
    let ascii_col_size = 1;
    let prefix_size = 8 + 1;
    let newline_size = 1;
    let line_size = (prefix_size
        + padding_size
        + 16 * hex_col_size
        + padding_size
        + 16 * ascii_col_size
        + newline_size) as usize;
    let line_count = (align(buf.len() as u64, 0x10) / 0x10) as usize;

    let mut ret = String::with_capacity(line_count * line_size);

    let mut line = String::with_capacity(line_size);
    let mut remaining_count = buf.len();
    for line_index in 0..line_count {
        let line_elem_count = 0x10.min(remaining_count);
        let padding_elem_count = 0x10 - line_elem_count;

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        // ^^^^^^^^^
        line.push_str(format!("{:08x}:", offset + 0x10 * line_index).as_str());

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        //          ^^
        line.push_str(padding);

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        //            ^^^
        for elem in &buf[(line_index * 0x10)..(line_index * 0x10) + line_elem_count] {
            line.push_str(format!("{:02x} ", elem).as_str());
        }
        for _ in 0..padding_elem_count {
            line.push_str("   ");
        }

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        //                                   ^^
        line.push_str(padding);

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        //                                     ^
        for elem in &buf[(line_index * 0x10)..(line_index * 0x10) + line_elem_count] {
            line.push(hexdump_ascii(*elem))
        }
        for _ in 0..padding_elem_count {
            line.push(' ');
        }
        line.push_str(padding);

        // 01234567:  00 01 02 03 04 05 06 07  ...............
        //                                                    ^
        line.push('\n');

        ret.push_str(line.as_str());
        line.truncate(0x0);
        remaining_count -= line_elem_count;
    }

    ret
}

pub fn read_file(filename: &str) -> Result<Vec<u8>> {
    debug!("read_file: {:?}", filename);

    let mut buf = Vec::new();
    {
        debug!("reading file: {}", filename);
        let mut f = match fs::File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                error!("failed to open file: {}", filename);
                return Err(UtilError::FileAccess.into());
            }
        };
        let bytes_read = match f.read_to_end(&mut buf) {
            Ok(c) => c,
            Err(_) => {
                error!("failed to read entire file: {}", filename);
                return Err(UtilError::FileAccess.into());
            }
        };
        debug!("read {} bytes", bytes_read);
        if bytes_read < 0x10 {
            error!("file too small: {}", filename);
            return Err(UtilError::FileFormat.into());
        }
    }

    Ok(buf)
}