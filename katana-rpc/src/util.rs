use anyhow::{Ok, Result};
use starknet::core::types::FieldElement;
use starknet_api::hash::StarkFelt;

pub fn to_trimmed_hex_string(bytes: &[u8]) -> String {
    let hex_str = hex::encode(bytes);
    let trimmed_hex_str = hex_str.trim_start_matches('0');
    if trimmed_hex_str.is_empty() {
        "0x0".to_string()
    } else {
        format!("0x{}", trimmed_hex_str)
    }
}

pub fn stark_felt_to_field_element(felt: StarkFelt) -> Result<FieldElement> {
    Ok(FieldElement::from_byte_slice_be(felt.bytes())?)
}
