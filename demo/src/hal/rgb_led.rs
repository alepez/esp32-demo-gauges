pub struct RgbLedColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<u32> for RgbLedColor {
    fn from(x: u32) -> Self {
        RgbLedColor {
            r: ((x & 0xFF0000) >> 16) as u8,
            g: ((x & 0x00FF00) >> 8) as u8,
            b: (x & 0x0000FF) as u8,
        }
    }
}

pub trait RgbLed {
    fn set_color(&self, color: RgbLedColor);
}
