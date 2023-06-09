pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

pub struct SystemState {
    acc: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
