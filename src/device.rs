/// HID device communication for the Razer BlackShark V2 Pro dongle.

use hidapi::HidApi;
use std::thread;
use std::time::{Duration, Instant};

use crate::protocol;

const VID: u16 = 0x1532;
const PID: u16 = 0x0555;
const USAGE_PAGE_VENDOR: u16 = 0xFF00;
const SLEEP_BETWEEN_CMDS: Duration = Duration::from_millis(35);

pub struct Device {
    handle: hidapi::HidDevice,
    pub product: String,
}

impl Device {
    /// Find and open the vendor-defined HID endpoint.
    /// Retries until timeout_ms expires.
    pub fn open(timeout_ms: u32) -> Result<Self, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);

        loop {
            let api = HidApi::new().map_err(|e| format!("HID init failed: {e}"))?;

            for info in api.device_list() {
                if info.vendor_id() == VID
                    && info.product_id() == PID
                    && info.usage_page() == USAGE_PAGE_VENDOR
                {
                    let handle = info
                        .open_device(&api)
                        .map_err(|e| format!("Cannot open device: {e}"))?;
                    let product = info
                        .product_string()
                        .unwrap_or("Razer BlackShark V2 Pro")
                        .to_string();
                    return Ok(Device { handle, product });
                }
            }

            if Instant::now() >= deadline {
                return Err("Device not found. Is the dongle plugged in?".to_string());
            }

            thread::sleep(Duration::from_millis(500));
        }
    }

    /// Send a 64-byte packet.
    fn send(&self, pkt: &[u8; 64]) -> Result<(), String> {
        self.handle
            .write(pkt)
            .map_err(|e| format!("Write failed: {e}"))?;
        thread::sleep(SLEEP_BETWEEN_CMDS);
        Ok(())
    }

    /// Read a response (with timeout).
    fn read(&self, timeout_ms: i32) -> Option<Vec<u8>> {
        let mut buf = [0u8; 64];
        for _ in 0..5 {
            match self.handle.read_timeout(&mut buf, timeout_ms) {
                Ok(n) if n > 0 => return Some(buf[..n].to_vec()),
                _ => {}
            }
        }
        None
    }

    /// Drain any queued input data.
    fn drain(&self) {
        let mut buf = [0u8; 64];
        while let Ok(n) = self.handle.read_timeout(&mut buf, 50) {
            if n == 0 {
                break;
            }
        }
    }

    /// Send command and drain response.
    fn cmd(&self, pkt: &[u8; 64]) -> Result<(), String> {
        self.send(pkt)?;
        self.drain();
        Ok(())
    }

    /// Get battery percentage. Returns None if unavailable.
    pub fn get_battery(&self) -> Option<u8> {
        self.cmd(&protocol::set_remote_mode(true)).ok()?;
        self.send(&protocol::get_battery()).ok()?;
        for _ in 0..10 {
            if let Some(resp) = self.read(100) {
                for i in 0..resp.len().saturating_sub(4) {
                    if resp[i] == 0x21 && i > 8 {
                        return Some(resp[i + 3]);
                    }
                }
            }
        }
        None
    }

    /// Apply the full audio profile to the headset.
    pub fn apply_profile(
        &self,
        eq_enabled: bool,
        eq_bands: &[i8; 10],
        volume: u8,
        enhancement: bool,
    ) -> Result<(), String> {
        // Step 1: Configure audio pipeline
        self.cmd(&protocol::set_remote_mode(false))?;
        self.cmd(&protocol::set_config())?;

        // Step 2: Enable EQ system
        self.cmd(&protocol::set_remote_mode(true))?;
        self.cmd(&protocol::set_speaker_preset_eq(eq_enabled))?;

        // Step 3: Set volume
        self.cmd(&protocol::set_remote_mode(true))?;
        self.cmd(&protocol::set_volume(volume))?;

        // Step 4: Set enhancement
        self.cmd(&protocol::set_remote_mode(true))?;
        self.cmd(&protocol::set_enhancement(enhancement))?;

        // Step 5: Push EQ bands
        if eq_enabled {
            self.cmd(&protocol::set_remote_mode(true))?;
            self.cmd(&protocol::set_eq_bands(eq_bands))?;
        }

        // Repeat volume + enhancement + EQ (Synapse does this)
        thread::sleep(Duration::from_millis(300));

        self.cmd(&protocol::set_remote_mode(true))?;
        self.cmd(&protocol::set_volume(volume))?;
        self.cmd(&protocol::set_remote_mode(true))?;
        self.cmd(&protocol::set_enhancement(enhancement))?;
        if eq_enabled {
            self.cmd(&protocol::set_remote_mode(true))?;
            self.cmd(&protocol::set_eq_bands(eq_bands))?;
        }

        Ok(())
    }
}
