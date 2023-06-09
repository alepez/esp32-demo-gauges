use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemState {
    acc: Vec3,
}
pub mod app;
pub mod hal;
pub mod svc;

pub use svc::CoordinatedInstant;
