pub struct Telementry {
    gps: [f32; 2],
    heading: u16,
    cam_pos: [u8; 2],
    battery_charge: u8,
    speed: u8,
    latancy: u32,
}
