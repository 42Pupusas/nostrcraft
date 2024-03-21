// CYBERSPACE METHODS
// These methods are used to generate the cyberspace coordinates for the notes and avatars
// based on their content and public key respectively

pub fn extract_coordinates(hex_str: &str) -> Result<(i128, i128, i128), hex::FromHexError> {
    // Decode the hexadecimal string into bytes
    let hex_bytes = hex::decode(hex_str)?;
    let hex_bits: Vec<bool> = hex_bytes
        .iter()
        .flat_map(|byte| (0..8).map(move |i| (byte >> i) & 1 == 1))
        .collect();

    let mut x_bit_vector = Vec::new();
    let mut y_bit_vector = Vec::new();
    let mut z_bit_vector = Vec::new();

    for i in 0..255 {
        match i % 3 {
            0 => x_bit_vector.push(hex_bits[i]),
            1 => y_bit_vector.push(hex_bits[i]),
            2 => z_bit_vector.push(hex_bits[i]),
            _ => unreachable!(),
        }
    }

    let x = vec_bool_to_i128(x_bit_vector).unwrap();
    let y = vec_bool_to_i128(y_bit_vector).unwrap();
    let z = vec_bool_to_i128(z_bit_vector).unwrap();

    Ok((x, y, z))
}

fn vec_bool_to_i128(vec: Vec<bool>) -> Option<i128> {
    if vec.len() != 85 {
        return None; // Ensure the vector has exactly 85 bits
    }

    let mut result: i128 = 0;
    for (index, &bit) in vec.iter().enumerate() {
        if bit {
            result |= 1 << index;
        }
    }

    Some(result)
}

// This scale doesnt lose precision between the i128 and f32
const CYBERSPACE_SECTOR_SCALE: i128 = 2_i128.pow(60);

pub fn scale_coordinates_to_world(x: i128, y: i128, z: i128) -> (f32, f32, f32) {
    let x_scaled = x / CYBERSPACE_SECTOR_SCALE;
    let y_scaled = y / CYBERSPACE_SECTOR_SCALE;
    let z_scaled = z / CYBERSPACE_SECTOR_SCALE;

    let x_scaled = x_scaled as f32 / 10000.0;
    let y_scaled = y_scaled as f32 / 10000.0;
    let z_scaled = z_scaled as f32 / 10000.0;

    (x_scaled.round(), y_scaled.round(), z_scaled.round())
}

pub fn _coordinate_to_string(x: i128, y: i128, z: i128) -> String {
    // TODO: Implement this
    String::from("TODO")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinates_to_string() {
        let test_string = "55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106";
        // let test_string = "0000000000000000000000000000000000000000000000000000000000000000";
        let (x, y, z) = extract_coordinates(test_string).unwrap();
        let (scaled_x, scaled_y, scaled_z) = scale_coordinates_to_world(x, y, z);
        println!("{} {} {}", scaled_x, scaled_y, scaled_z);

    }
}

