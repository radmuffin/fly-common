/// Pure Rust, zero-dependency Scalable Vector Graphics (SVG) QR Code Generator.
/// Generates crisp vector QR codes for offline pairing, multi-device sync, and mobile scanning.

#[derive(Debug, Clone)]
pub struct QrSvgResult {
    pub svg: String,
    pub data_url: String,
    pub size: usize,
    pub module_count: usize,
}

/// Generates a scalable SVG QR code string and standard Data URL.
pub fn generate_qr_svg(data: &str, size: usize, margin: usize) -> QrSvgResult {
    let raw_data = if data.is_empty() { " " } else { data };
    let bytes = raw_data.as_bytes();

    // Determine minimal QR Version (1, 2, 3, or 4 with Byte encoding)
    let (version, total_data_bytes, ec_bytes_per_block) = match bytes.len() {
        0..=14 => (1, 19, 10),
        15..=26 => (2, 34, 16),
        27..=42 => (3, 55, 26),
        43..=62 => (4, 80, 18),
        63..=84 => (5, 108, 24),
        85..=106 => (6, 136, 16),
        107..=122 => (7, 156, 18),
        _ => (8, 194, 22),
    };

    let module_count = version * 4 + 17;
    let mut matrix = vec![vec![None; module_count]; module_count];

    // 1. Finder patterns (Top-left, Top-right, Bottom-left)
    draw_finder_pattern(&mut matrix, 0, 0);
    draw_finder_pattern(&mut matrix, module_count - 7, 0);
    draw_finder_pattern(&mut matrix, 0, module_count - 7);

    // 2. Timing patterns
    #[allow(clippy::needless_range_loop)]
    for i in 8..(module_count - 8) {
        let bit = (i % 2) == 0;
        if matrix[6][i].is_none() {
            matrix[6][i] = Some(bit);
        }
        if matrix[i][6].is_none() {
            matrix[i][6] = Some(bit);
        }
    }

    // 3. Dark module
    let dark_row = 4 * version + 9;
    if dark_row < module_count {
        matrix[dark_row][8] = Some(true);
    }

    // 4. Reserve format info areas
    for i in 0..8 {
        if matrix[8][i].is_none() {
            matrix[8][i] = Some(false);
        }
        if matrix[i][8].is_none() {
            matrix[i][8] = Some(false);
        }
        if matrix[8][module_count - 1 - i].is_none() {
            matrix[8][module_count - 1 - i] = Some(false);
        }
        if matrix[module_count - 1 - i][8].is_none() {
            matrix[module_count - 1 - i][8] = Some(false);
        }
    }
    matrix[8][8] = Some(false);

    // 5. Build bitstream
    let mut bitstream = Vec::new();
    push_bits(&mut bitstream, 0b0100, 4);
    let count_bits = if version <= 9 { 8 } else { 16 };
    push_bits(&mut bitstream, bytes.len() as u32, count_bits);

    for &b in bytes {
        push_bits(&mut bitstream, b as u32, 8);
    }

    // Terminator (up to 4 zeroes)
    let max_data_bits: usize = total_data_bytes * 8;
    let term_len = 4.min(max_data_bits.saturating_sub(bitstream.len()));
    push_bits(&mut bitstream, 0, term_len);

    // Byte alignment
    while bitstream.len() % 8 != 0 && bitstream.len() < max_data_bits {
        bitstream.push(false);
    }

    // Pad bytes 0xEC, 0x11
    let pad_bytes = [0xEC, 0x11];
    let mut pad_idx = 0;
    while bitstream.len() < max_data_bits {
        push_bits(&mut bitstream, pad_bytes[pad_idx % 2], 8);
        pad_idx += 1;
    }

    // Group into data codewords
    let mut data_codewords = Vec::new();
    for chunk in bitstream.chunks(8) {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | (if bit { 1 } else { 0 });
        }
        data_codewords.push(byte);
    }

    // Reed-Solomon Error Correction Codewords
    let ec_codewords = generate_reed_solomon_ec(&data_codewords, ec_bytes_per_block);
    let mut final_codewords = data_codewords;
    final_codewords.extend(ec_codewords);

    // 6. Placement in zigzag pattern
    let mut all_bits = Vec::new();
    for b in final_codewords {
        for i in (0..8).rev() {
            all_bits.push(((b >> i) & 1) == 1);
        }
    }

    let mut bit_idx = 0;
    let mut right = module_count as isize - 1;
    let mut upward = true;

    while right > 0 {
        if right == 6 {
            right -= 1; // Skip vertical timing line
        }
        let rows: Vec<usize> = if upward {
            (0..module_count).rev().collect()
        } else {
            (0..module_count).collect()
        };

        for r in rows {
            for col_offset in 0..=1 {
                let c = (right - col_offset) as usize;
                if matrix[r][c].is_none() {
                    let bit = if bit_idx < all_bits.len() {
                        all_bits[bit_idx]
                    } else {
                        false
                    };
                    bit_idx += 1;

                    // Apply standard mask pattern 0 ((row + col) % 2 == 0)
                    let mask = (r + c).is_multiple_of(2);
                    matrix[r][c] = Some(bit ^ mask);
                }
            }
        }
        upward = !upward;
        right -= 2;
    }

    // 7. Format Information (Mask 0 + Error Level Medium => format bits 0x5412)
    let format_bits: u16 = 0x5412;
    for i in 0..15 {
        let bit = ((format_bits >> i) & 1) == 1;
        if i <= 5 {
            matrix[8][i] = Some(bit);
        } else if i == 6 {
            matrix[8][7] = Some(bit);
        } else if i == 7 {
            matrix[8][8] = Some(bit);
        } else if i == 8 {
            matrix[7][8] = Some(bit);
        } else {
            matrix[14 - i][8] = Some(bit);
        }

        if i < 8 {
            matrix[module_count - 1 - i][8] = Some(bit);
        } else {
            matrix[8][module_count - 15 + i] = Some(bit);
        }
    }

    // 8. Build Crisp Scalable SVG
    let total_dim = module_count + margin * 2;
    let mut path_d = String::new();

    #[allow(clippy::needless_range_loop)]
    for r in 0..module_count {
        for c in 0..module_count {
            if matrix[r][c].unwrap_or(false) {
                let x = c + margin;
                let y = r + margin;
                path_d.push_str(&format!("M{},{}h1v1h-1z ", x, y));
            }
        }
    }

    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" shape-rendering=\"crispEdges\"><rect width=\"{}\" height=\"{}\" fill=\"#ffffff\"/><path d=\"{}\" fill=\"#000000\"/></svg>",
        total_dim, total_dim, size, size, total_dim, total_dim, path_d.trim()
    );

    let data_url = format!(
        "data:image/svg+xml;charset=utf-8,{}",
        utf8_percent_encode(&svg)
    );

    QrSvgResult {
        svg,
        data_url,
        size,
        module_count: total_dim,
    }
}

fn draw_finder_pattern(matrix: &mut [Vec<Option<bool>>], row: usize, col: usize) {
    for r in 0..7 {
        for c in 0..7 {
            let is_border = r == 0 || r == 6 || c == 0 || c == 6;
            let is_center = (2..=4).contains(&r) && (2..=4).contains(&c);
            matrix[row + r][col + c] = Some(is_border || is_center);
        }
    }
}

fn push_bits(stream: &mut Vec<bool>, val: u32, count: usize) {
    for i in (0..count).rev() {
        stream.push(((val >> i) & 1) == 1);
    }
}

fn generate_reed_solomon_ec(data: &[u8], ec_count: usize) -> Vec<u8> {
    // Galois Field GF(256) primitive polynomial 0x11D
    let mut gf_exp = [0u8; 512];
    let mut gf_log = [0u8; 256];
    let mut x = 1u16;
    for i in 0..255 {
        gf_exp[i] = x as u8;
        gf_exp[i + 255] = x as u8;
        gf_log[x as usize] = i as u8;
        x <<= 1;
        if x >= 256 {
            x ^= 0x11D;
        }
    }

    let gf_mul = |a: u8, b: u8| -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let idx = (gf_log[a as usize] as usize) + (gf_log[b as usize] as usize);
            gf_exp[idx]
        }
    };

    let mut gen = vec![1u8];
    for &factor in gf_exp.iter().take(ec_count) {
        let mut next_gen = vec![0u8; gen.len() + 1];
        for j in 0..gen.len() {
            next_gen[j] ^= gf_mul(gen[j], factor);
            next_gen[j + 1] ^= gen[j];
        }
        gen = next_gen;
    }

    let mut msg = vec![0u8; data.len() + ec_count];
    msg[..data.len()].copy_from_slice(data);

    #[allow(clippy::needless_range_loop)]
    for i in 0..data.len() {
        let coef = msg[i];
        if coef != 0 {
            for j in 0..gen.len() {
                msg[i + j] ^= gf_mul(gen[j], coef);
            }
        }
    }

    msg[data.len()..].to_vec()
}

fn utf8_percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_qr_svg_output() {
        let res = generate_qr_svg("https://example.com/sync?token=usr_123", 240, 2);
        assert!(res.svg.starts_with("<svg"));
        assert!(res.svg.ends_with("</svg>"));
        assert!(res
            .data_url
            .starts_with("data:image/svg+xml;charset=utf-8,"));
        assert!(res.module_count > 20);
        assert_eq!(res.size, 240);
    }
}
