use bevy::prelude::*;


// to treat as a collision obstacle.
#[derive(Component)]
pub struct Collidable;

// If you omit this, you can assume the sprite’s custom_size or TILE_SIZE.
#[derive(Component, Copy, Clone)]
pub struct Collider {
    pub half_extents: Vec2,
}

impl Collider {
    pub fn square(side: f32) -> Self {
        Self { half_extents: Vec2::splat(side * 0.5) }
    }
    pub fn from_size(size: Vec2) -> Self {
        Self { half_extents: size * 0.5 }
    }
}


