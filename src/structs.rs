#[derive(Debug)]
pub struct Tile {
    pub z: usize,
    pub x: usize,
    pub y: usize,
}

impl Tile {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Tile { x, y, z }
    }
}
