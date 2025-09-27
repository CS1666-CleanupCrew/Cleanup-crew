use bevy::prelude::*;

/// Window bounds for clamping positions.
#[derive(Resource, Clone, Copy)]
struct ScreenBounds {
    min: Vec3,
    max: Vec3,
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity(pub Vec2);

impl Default for Velocity {
    fn default() -> Self {
        Self(Vec2::ZERO)
    }
}

pub struct MotionStruct {
    pub width: f32,
    pub height: f32,
    pub player_size: f32,
    pub max_speed: f32,
    pub accel_rate: f32,
    pub sprite_path: &'static str, // <-- path to your player sprite
}

impl motionStruct {
    pub fn new(
        width: f32,
        height: f32,
        player_size: f32,
        max_speed: f32,
        accel_rate: f32,
        sprite_path: &'static str,
    ) -> Self {
        Self { width, height, player_size, max_speed, accel_rate, sprite_path }
    }
}

#[derive(Resource)] struct MaxSpeed(pub f32);
#[derive(Resource)] struct AccelRate(pub f32);
#[derive(Resource)] struct SpritePath(&'static str);

impl Plugin for motionStruct {
    fn build(&self, app: &mut App) {
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;
        let r = self.player_size * 0.5;

        app.insert_resource(ScreenBounds {
            min: Vec3::new(-half_w + r, -half_h + r, 0.0),
            max: Vec3::new( half_w - r,  half_h - r, 0.0),
        });
        app.insert_resource(MaxSpeed(self.max_speed));
        app.insert_resource(AccelRate(self.accel_rate));
        app.insert_resource(SpritePath(self.sprite_path));

        app.add_systems(Startup, setup_player)
           .add_systems(Update, move_player);
    }
}

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sprite_path: Res<SpritePath>,
    plugin: Res<MotionStruct>,
) {
    commands.spawn(Camera2d);

    // Load your sprite
    let texture_handle = asset_server.load(sprite_path.0);

    commands.spawn((
        Sprite::from_image(texture_handle).with_custom_size(Vec2::splat(plugin.player_size)),
        Velocity::default(),
        Player,
        Transform::default(),
    ));
}

fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    bounds: Res<ScreenBounds>,
    ms: Res<MaxSpeed>,
    ar: Res<AccelRate>,
    mut q: Query<(&mut Transform, &mut Velocity), With<Player>>,
) {
    let (mut transform, mut vel) = match q.get_single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }

    let dt = time.delta_secs();
    let accel = ar.0 * dt;

    **vel = if dir.length() > 0.0 {
        (**vel + dir.normalize() * accel).clamp_length_max(ms.0)
    } else if vel.length() > accel {
        **vel + vel.normalize() * -accel
    } else {
        Vec2::ZERO
    };

    transform.translation += (**vel * dt).extend(0.0);
    transform.translation = transform.translation.clamp(bounds.min, bounds.max);
}