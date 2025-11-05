use rand::{Rng, RngCore, distr::Alphanumeric};

/// Generates a random alphanumeric string of the specified length.
///
/// # Arguments
///
/// * `length` - The length of the random string to generate
///
/// # Returns
///
/// A randomly generated string of alphanumeric characters.
///
/// # Examples
///
/// ```
/// let token = generate_random_string(8); // e.g., "A1b2C3d4"
/// ```
pub fn generate_random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>()
}

#[allow(dead_code)]
pub fn fill_random_bytes(buffer: &mut [u8]) {
    rand::rng().fill_bytes(buffer);
}

#[allow(dead_code)]
pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    fill_random_bytes(&mut bytes);
    bytes
}
