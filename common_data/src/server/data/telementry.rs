use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Telementry {
    pub gps: [f32; 2],
    pub heading: u16,
    pub cam_pos: [u8; 2],
    pub battery_charge: u8,
    pub speed: u8,
    pub latancy: u32,
    pub last_changed: i64,
}
