// CYBERSPACE METHODS
// These methods are used to generate the cyberspace coordinates for the notes and avatars
// based on their content and public key respectively

pub fn extract_coordinates(hex_str: &str) -> Result<(i128, i128, i128), hex::FromHexError> {
    // Decode the hexadecimal string into bytes
    let hex_bytes = hex::decode(hex_str)?;

    // Convert the bytes into a vector of bits represented as bools
    let hex_bits: Vec<bool> = hex_bytes
        .iter()
        .flat_map(|byte| (0..8).map(move |i| (byte >> i) & 1 == 1))
        .collect();

    // Initialize the vectors to store the bits for each coordinate
    let mut x_bit_vector = Vec::new();
    let mut y_bit_vector = Vec::new();
    let mut z_bit_vector = Vec::new();

    // Split the bits into the x, y, and z vectors
    // The bits are split into 3 parts, each part is 85 bits long
    // Shuffle the bits using modulo 3 to give one bit to each coordinate in order
    for i in 0..255 {
        match i % 3 {
            0 => x_bit_vector.push(hex_bits[i]),
            1 => y_bit_vector.push(hex_bits[i]),
            2 => z_bit_vector.push(hex_bits[i]),
            _ => unreachable!(),
        }
    }

    // Last bit is for i-space o d-space, we are using i-space here so always 1

    // Convert the bit vectors into i128 values
    let x = vec_bool_to_i128(x_bit_vector).unwrap();
    let y = vec_bool_to_i128(y_bit_vector).unwrap();
    let z = vec_bool_to_i128(z_bit_vector).unwrap();

    Ok((x, y, z))
}

pub fn encode_coordinates(x: i128, y: i128, z: i128) -> String {
    // Convert the coordinates into a vector of bits
    let x_bits = i128_to_vec_bool(x);
    let y_bits = i128_to_vec_bool(y);
    let z_bits = i128_to_vec_bool(z);

    // Combine the bits into a single vector
    let mut combined_bits = Vec::new();
    for i in 0..85 {
        combined_bits.push(x_bits[i]);
        combined_bits.push(y_bits[i]);
        combined_bits.push(z_bits[i]);
    }

    combined_bits.push(true); // Always 1 for i-space

    // Convert the bits into bytes
    let mut bytes = Vec::new();
    for i in 0..combined_bits.len() / 8 {
        let mut byte = 0;
        for j in 0..8 {
            if combined_bits[i * 8 + j] {
                byte |= 1 << j;
            }
        }
        bytes.push(byte);
    }

    // Encode the bytes as a hexadecimal string
    hex::encode(bytes)
}

fn vec_bool_to_i128(vec: Vec<bool>) -> Option<i128> {
    // initialize the result as a zeroed out i128
    let mut result: i128 = 0;

    // Each true bit represe a power of 2
    // Least significants bits are stored first in the array
    // so if bit is true, we set the corresponding bit in the i128 and shift left
    // so the first bot will always be the least significant bit
    for (index, &bit) in vec.iter().enumerate() {
        if bit {
            // SHift left result, adding a true bit at the bit index
            result |= 1 << index;
        }
    }

    Some(result)
}

fn i128_to_vec_bool(num: i128) -> Vec<bool> {
    let mut result = Vec::new();

    // We iterate over the 128 bits of the i128 number
    // and check if the bit is set, if it is we add a true to the result vector
    for i in 0..128 {
        let bit = num & (1 << i) != 0;
        result.push(bit);
    }

    result
}

// This scale doesnt lose precision between the i128 and f32
const CYBERSPACE_SECTOR_SCALE: i128 = 2_i128.pow(71);

pub fn scale_coordinates_to_world(x: i128, y: i128, z: i128) -> (f32, f32, f32) {
    let x_scaled = x / CYBERSPACE_SECTOR_SCALE;
    let y_scaled = y / CYBERSPACE_SECTOR_SCALE;
    let z_scaled = z / CYBERSPACE_SECTOR_SCALE;

    let x_scaled = x_scaled as f32;
    let y_scaled = y_scaled as f32;
    let z_scaled = z_scaled as f32;

    (x_scaled.round(), y_scaled.round(), z_scaled.round())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1010101010
    #[test]
    fn test_vec_bool_to_i128() {
        let test_vec = vec![
            false, true, false, true, false, true, false, true, false, true,
        ];
        let result = vec_bool_to_i128(test_vec).unwrap();
        assert_eq!(result, 682);
    }

    #[test]
    fn test_i128_to_vec_bool() {
        let test_num = 682;
        let result = i128_to_vec_bool(test_num);
        let expected = vec![
            false, true, false, true, false, true, false, true, false, true, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn i_128_to_bit_vector_and_back() {
        let test_num = 69420;
        let bit_vector = i128_to_vec_bool(test_num);
        let result = vec_bool_to_i128(bit_vector).unwrap();
        assert_eq!(result, test_num);
    }

    #[test]
    fn test_extract_coordinates() {
        let hex_str = "b722c93ee3be55e782a2d14378dd2b47e3a7faf08f5e5d79e34911fcf9b8409b";
        let result = extract_coordinates(hex_str).unwrap();
        assert_eq!(
            result,
            (
                34709496724926780557617673,
                406823014141971989681143,
                15561938306656479869269891
            )
        );
    }

    #[test]
    fn test_encode_coordinates() {
        let x = 34709496724926780557617673;
        let y = 406823014141971989681143;
        let z = 15561938306656479869269891;
        let result = encode_coordinates(x, y, z);
        let expected = "b722c93ee3be55e782a2d14378dd2b47e3a7faf08f5e5d79e34911fcf9b8409b";
        assert_eq!(result, expected);
    }
    
    #[test]
    fn encode_coordinates_and_back() {
        let x = 69;
        let y = 420;
        let z = 50;
        let encoded = encode_coordinates(x, y, z);
        println!("{}", encoded);
        let result = extract_coordinates(&encoded).unwrap();
        assert_eq!(result, (x, y, z));
    }
}
