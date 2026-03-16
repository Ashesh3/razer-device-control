/// MXIC audio protocol command builders for the Razer BlackShark V2 Pro.
/// All commands are 64-byte HID interrupt OUT packets.

const PKT_SIZE: usize = 64;

/// Build a 64-byte MXIC packet.
fn mxic_packet(total_len: u8, inner_len: u8, cmd_type: u8, cmd_id: u8, params: &[u8]) -> [u8; PKT_SIZE] {
    let mut pkt = [0u8; PKT_SIZE];
    pkt[0] = 0x02; // Report type
    pkt[1] = 0x80; // Direction: output
    pkt[2] = total_len;
    pkt[5] = 0x50; // 'P'
    pkt[6] = 0x41; // 'A'
    pkt[7] = inner_len;
    pkt[9] = cmd_type;
    pkt[10] = cmd_id;
    for (i, &b) in params.iter().enumerate() {
        pkt[11 + i] = b;
    }
    pkt
}

/// Set Remote/Local mode. Must be called before most commands.
/// Remote=true gives software control, Local=false returns control to device.
pub fn set_remote_mode(enable: bool) -> [u8; PKT_SIZE] {
    mxic_packet(0x07, 0x0E, 0x02, 0xE1, &[if enable { 1 } else { 0 }])
}

/// SET_CONFIG (cmd6_0x01) — configures the audio pipeline/DSP.
/// Params from USB capture: C2 03 F8 5F 04
pub fn set_config() -> [u8; PKT_SIZE] {
    mxic_packet(0x0B, 0x08, 0x06, 0x01, &[0xC2, 0x03, 0xF8, 0x5F, 0x04])
}

/// Enable/disable speaker preset EQ (cmd4_0x9E).
pub fn set_speaker_preset_eq(enable: bool) -> [u8; PKT_SIZE] {
    mxic_packet(0x09, 0x08, 0x04, 0x9E, &[0x00, if enable { 0x01 } else { 0x00 }])
}

/// Set internal DSP volume/gain (cmd4_0x93). Value 0-255.
pub fn set_volume(value: u8) -> [u8; PKT_SIZE] {
    mxic_packet(0x09, 0x08, 0x04, 0x93, &[0x00, 0x01, value])
}

/// Enable/disable audio enhancement (cmd4_0x9D).
pub fn set_enhancement(enable: bool) -> [u8; PKT_SIZE] {
    mxic_packet(0x09, 0x08, 0x04, 0x9D, &[0x00, 0x01, if enable { 0x01 } else { 0x00 }])
}

/// Set 10-band EQ (cmd13_0x95). Bands are signed i8 values (typically -5 to +5).
pub fn set_eq_bands(bands: &[i8; 10]) -> [u8; PKT_SIZE] {
    let mut params = [0u8; 12];
    params[0] = 0x00;
    params[1] = 0x0A; // 10 bands
    for (i, &val) in bands.iter().enumerate() {
        params[2 + i] = val as u8;
    }
    mxic_packet(0x12, 0x08, 0x0D, 0x95, &params)
}

/// Get battery level (cmd3_0x21). Send this, then read response.
pub fn get_battery() -> [u8; PKT_SIZE] {
    mxic_packet(0x08, 0x08, 0x03, 0x21, &[])
}
