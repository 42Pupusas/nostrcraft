// CYBERSPACE METHODS
// These methods are used to generate the cyberspace coordinates for the notes and avatars
// based on their content and public key respectively

use bevy::log::info;
use bevy::utils::HashMap;
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha256;
use primitive_types::U256;

pub fn hex_string_to_i_space(hex: &str) -> (f32, f32, f32) {
    info!("hex: {:?}", hex);
    // this hex string has 256 bits
    let bytes = hex::decode(hex).unwrap();
    info!("bytes: {:?}", bytes);

    // i need to split it into 3 with one bit left over 
    // then i need the omst significant 32 bits of each slice 
    // from thosbuild an f32

    let x = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let y = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let z = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let x_f32 = f32::from_bits(x);
    let y_f32 = f32::from_bits(y);
    let z_f32 = f32::from_bits(z);

    (x_f32, y_f32, z_f32)

}

pub fn i_space_to_hex_string(x: f32, y: f32, z: f32) -> String {
    let x_bytes = x.to_bits().to_le_bytes();
    let y_bytes = y.to_bits().to_le_bytes();
    let z_bytes = z.to_bits().to_le_bytes();
    let mut result = [0; 13];
    result[0..4].copy_from_slice(&x_bytes);
    result[4..8].copy_from_slice(&y_bytes);
    result[8..12].copy_from_slice(&z_bytes);
    result[12] = 1;
    hex::encode(&result)
}

// Simhash is a hashing algorithm that is used to generate a 256 bit hash from a string
pub fn simhash(input: &str) -> U256 {
    // Initialize a vector of 256 zeros
    let mut vectors: Vec<i32> = vec![0; 256];

    // Initialize a hashmap to count the occurrences of each shingle
    let mut shingle_count = HashMap::new();

    // Convert input to a vector of characters
    let chars: Vec<char> = input.chars().collect();

    if chars.len() > 1 {
        for i in 0..chars.len() - 1 {
            let shingle = chars[i].to_string() + &chars[i + 1].to_string();
            *shingle_count.entry(shingle).or_insert(0) += 1;
        }
    }
    // Hash each shingle and add/subtract from the vector
    for (shingle, count) in shingle_count {
        // Hash the shingle
        let hash = hash_word(&shingle); // Assuming hash_word can hash a shingle
        for i in 0..256 {
            // Add or subtract from the vector based on the bit at index i
            if get_bit(hash, i) {
                vectors[i] += count;
            } else {
                vectors[i] -= count;
            }
        }
    }

    // Construct the final hash
    let mut final_hash = U256::zero();
    for i in 0..256 {
        if vectors[i] > 0 {
            final_hash = final_hash.overflowing_add(U256::one() << i).0;
        }
    }

    final_hash
}

pub fn normal_hash(input: &str) -> U256 {
    let hash = hash_word(input);
    let mut result = U256::zero();
    for i in 0..256 {
        if get_bit(hash, i) {
            result = result.overflowing_add(U256::one() << i).0;
        }
    }
    result
}

// Get the bit at index i from a 256 bit hash
fn get_bit(hash: [u8; 32], index: usize) -> bool {
    let byte = index / 8;
    let bit = index % 8;
    hash[byte] & (1 << bit) != 0
}

// Hash a word using SHA256
fn hash_word(word: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.input_str(word);

    // Get the hash as a 32 byte array
    let mut result = [0u8; 32];
    hasher.result(&mut result);
    result
}

pub fn map_hash_to_coordinates(hash: U256) -> (U256, U256, U256) {
    // Split the hash into 3 parts of 85 bits each
    let mut x = U256::zero();
    let mut y = U256::zero();
    let mut z = U256::zero();

    // Map the first 85 bits to x, the next 85 to y, and the last 85 to z
    for i in 0..255 {
        match i % 3 {
            0 => x = (x << 1) | get_bit_as_u256(&hash, i),
            1 => y = (y << 1) | get_bit_as_u256(&hash, i),
            2 => z = (z << 1) | get_bit_as_u256(&hash, i),
            _ => unreachable!(),
        }
    }

    (x, y, z)
}

// U256 type comes from the primitive_types crate
// It's a 256 bit unsigned integer
fn get_bit_as_u256(hash: &U256, index: usize) -> U256 {
    if get_bit_from_primitive(hash, index) {
        U256::one()
    } else {
        U256::zero()
    }
}

fn get_bit_from_primitive(hash: &U256, index: usize) -> bool {
    let byte_index = index / 8;
    let bit_index = index % 8;
    let byte = hash.byte(byte_index);
    (byte & (1 << bit_index)) != 0
}

// This one is used for notes, because we want to offset them from the avatar
// we also want to scale them down to a smaller size
fn map_hash_to_coordinates_with_offset(hash: U256, origin_hash: U256) -> (f32, f32, f32) {
    // Calculate coordinates for the hash
    let (x, y, z) = map_hash_to_coordinates(hash);
    // Calculate coordinates for the origin hash
    let (origin_x, origin_y, origin_z) = map_hash_to_coordinates(origin_hash);

    let (scaled_x, scaled_y, scaled_z) = scale_down_coordinates_to_f32(x, y, z);
    let (scaled_origin_x, scaled_origin_y, scaled_origin_z) =
        scale_down_coordinates_to_f32(origin_x, origin_y, origin_z);

    let extra_scale = 42.0;

    let x_f32 = (scaled_x / extra_scale) + scaled_origin_x;
    let y_f32 = (scaled_y / extra_scale) + scaled_origin_y;
    let z_f32 = (scaled_z / extra_scale) + scaled_origin_z;

    (x_f32, y_f32, z_f32)
}

// This function scales down the coordinates to a smaller usize
// so that we can fit them into a f32 and then scale them up to the desired
// scene
pub fn scale_down_coordinates_to_f32(x: U256, y: U256, z: U256) -> (f32, f32, f32) {
    // Max value is 2^85
    let max_value = U256::from(1u128) << 85;

    // Extract sign and absolute value
    // The sign is the 85th bit
    // I did this so I could get negative values as well
    // and make the scene more interesting
    let x_sign = if x.bit(84) { -1.0 } else { 1.0 };
    let y_sign = if y.bit(84) { -1.0 } else { 1.0 };
    let z_sign = if z.bit(84) { -1.0 } else { 1.0 };

    let x_abs = x & ((U256::from(1u128) << 84) - 1);
    let y_abs = y & ((U256::from(1u128) << 84) - 1);
    let z_abs = z & ((U256::from(1u128) << 84) - 1);

    // Helper function below
    // From testing, the coordinates are always between 0 and 1.6
    // with 12 decimal points of precision
    let x_f32 = u256_to_f32(x_abs, max_value);
    let y_f32 = u256_to_f32(y_abs, max_value);
    let z_f32 = u256_to_f32(z_abs, max_value);

    // Scale the coordinates up to the desired scene size
    // This is a magic number that I picked because it looked good
    // and gave the scene a good size
    let scale = 8400.0;
    (
        x_f32 * scale * x_sign,
        y_f32 * scale * y_sign,
        z_f32 * scale * z_sign,
    )
}

fn u256_to_f32(value: U256, max_value: U256) -> f32 {
    // Convert U256 to f32 by first converting to a smaller integer (like u64) and then to f32
    // This is because U256 doesn't implement From<f32> for some reason
    //
    // We divide by max_value / u64::MAX to get a value between 0 and 1
    // and then multiply by u64::MAX to get a value between 0 and u64::MAX
    let value_u64 = value / (max_value / U256::from(u64::MAX));
    // Once we have a value between 0 and u64::MAX, we can convert it to f32
    // by dividing by u64::MAX to get a value between 0 and 1.6
    value_u64.as_u64() as f32 / u64::MAX as f32
}

fn f32_to_bytes_with_dspace(xyz: (f32, f32, f32)) -> [u8; 13] {
    let (x, y, z) = xyz;
    let x_bytes = x.to_le_bytes();
    let y_bytes = y.to_le_bytes();
    let z_bytes = z.to_le_bytes();
    let mut result = [0; 13];
    // Set the first bit to 0 for d-space
    result[0] = 0;
    result[1..5].copy_from_slice(&x_bytes);
    result[5..9].copy_from_slice(&y_bytes);
    result[9..13].copy_from_slice(&z_bytes);
    result
}

pub fn coordinates_to_hash(xyz: (f32, f32, f32)) -> String {
    let bytes = f32_to_bytes_with_dspace(xyz);
    hex::encode(&bytes)
}

use rand::Rng;

pub fn generate_nonce() -> [u8; 16] {
    // Define the symbols allowed in the nonce
    let symbols: [u8; 16] = [
        b'!', b'"', b'#', b'$', b'%', b'&', b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.', b'/',
        b'0',
    ];

    let mut rng = rand::thread_rng();
    let mut nonce: [u8; 16] = [0; 16];

    for i in 0..16 {
        // Generate a random index to select a symbol from the array
        let index = rng.gen_range(0..16);
        // Assign the selected symbol to the nonce buffer
        nonce[i] = symbols[index];
    }

    nonce
}
