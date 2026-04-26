pub mod zapper;
pub mod beam_rifle;

pub use beam_rifle::BeamRifleRes;

use bevy::prelude::*;
use crate::GameEntity;
use crate::bullet::{Bullet, BulletOwner, Velocity, AnimationTimer, AnimationFrameCount, Piercing, HitEnemies};
use crate::collidable::Collider;

#[derive(Component, Clone)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub fire_rate: f32,
    pub bullet_speed: f32,
    pub damage: f32,
    pub bullet_size: f32,
    pub shoot_timer: Timer,
    pub piercing_pickups: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WeaponType {
    Zapper,
    BeamRifle,
}

impl WeaponType {
    pub fn name(self) -> &'static str {
        match self {
            WeaponType::Zapper => "Zapper",
            WeaponType::BeamRifle => "Beam Rifle",
        }
    }
}

impl Weapon {
    pub fn new(weapon_type: WeaponType) -> Self {
        match weapon_type {
            WeaponType::Zapper => zapper::new(),
            WeaponType::BeamRifle => beam_rifle::new(),
        }
    }

    pub fn can_shoot(&self) -> bool {
        self.shoot_timer.finished()
    }

    pub fn reset_timer(&mut self) {
        self.shoot_timer.reset();
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.shoot_timer.tick(delta);
    }

    pub fn effective_pierce_count(&self) -> u32 {
        let p = self.piercing_pickups;
        if p <= 4 { p } else { 4 + (p - 4) / 2 }
    }
}

#[derive(Component)]
pub struct WeaponInventory {
    pub weapons: Vec<Weapon>,
    pub equipped: usize,
}

impl WeaponInventory {
    pub fn new(weapon: Weapon) -> Self {
        Self { weapons: vec![weapon], equipped: 0 }
    }

    pub fn current(&self) -> &Weapon {
        &self.weapons[self.equipped]
    }

    pub fn current_mut(&mut self) -> &mut Weapon {
        &mut self.weapons[self.equipped]
    }

    pub fn cycle_next(&mut self) {
        if self.weapons.len() > 1 {
            self.equipped = (self.equipped + 1) % self.weapons.len();
        }
    }

    pub fn equipped_name(&self) -> &'static str {
        self.current().weapon_type.name()
    }
}

#[derive(Resource)]
pub struct BulletRes(pub Handle<Image>, pub Handle<TextureAtlasLayout>);

#[derive(Resource)]
pub struct EnemyBulletRes(pub Handle<Image>, pub Handle<TextureAtlasLayout>);

#[derive(Resource)]
pub struct WeaponSounds {
    pub laser: Handle<AudioSource>,
    pub shoot: Handle<AudioSource>,
}

#[derive(Component)]
pub struct WeaponNameDisplay;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_weapon_assets)
            .add_systems(OnEnter(crate::GameState::Playing), spawn_weapon_hud)
            .add_systems(
                Update,
                (update_weapon_timers, update_weapon_hud, cycle_weapons)
                    .run_if(in_state(crate::GameState::Playing))
                    .run_if(not(resource_exists::<crate::pause::IsPaused>)),
            );
    }
}

fn load_weapon_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let bullet_animate_image: Handle<Image> = asset_server.load("bullet_animation.png");
    let bullet_animate_layout = TextureAtlasLayout::from_grid(UVec2::splat(100), 3, 1, None, None);
    let bullet_animate_handle = texture_atlases.add(bullet_animate_layout);
    commands.insert_resource(BulletRes(bullet_animate_image, bullet_animate_handle));

    let enemy_bullet_image: Handle<Image> = asset_server.load("enemy_bullet_animation.png");
    let enemy_bullet_layout = TextureAtlasLayout::from_grid(UVec2::splat(100), 3, 1, None, None);
    let enemy_bullet_handle = texture_atlases.add(enemy_bullet_layout);
    commands.insert_resource(EnemyBulletRes(enemy_bullet_image, enemy_bullet_handle));

    let laser_sound: Handle<AudioSource> = asset_server.load("audio/laser_zap.ogg");
    let shoot_sound: Handle<AudioSource> = asset_server.load("audio/shoot.ogg");
    commands.insert_resource(WeaponSounds { laser: laser_sound, shoot: shoot_sound });

    let beam_bullet = asset_server.load("beam.png");
    commands.insert_resource(BeamRifleRes { bullet: beam_bullet });
}

fn spawn_weapon_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load(crate::FONT_PATH);
    commands.spawn((
        Text::new("Zapper"),
        TextFont { font, font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        WeaponNameDisplay,
        GameEntity,
    ));
}

fn update_weapon_timers(
    time: Res<Time>,
    mut inventories: Query<&mut WeaponInventory>,
) {
    for mut inv in &mut inventories {
        for weapon in &mut inv.weapons {
            weapon.tick(time.delta());
        }
    }
}

fn update_weapon_hud(
    player_q: Query<&WeaponInventory, With<crate::player::Player>>,
    mut text_q: Query<&mut Text, With<WeaponNameDisplay>>,
) {
    let Ok(inv) = player_q.single() else { return; };
    let Ok(mut text) = text_q.single_mut() else { return; };
    text.0 = inv.equipped_name().to_string();
}

fn cycle_weapons(
    input: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<&mut WeaponInventory, With<crate::player::Player>>,
) {
    if !input.just_pressed(KeyCode::KeyQ) { return; }
    let Ok(mut inv) = player_q.single_mut() else { return; };
    inv.cycle_next();
}

pub fn fire_weapon(
    commands: &mut Commands,
    inventory: &mut WeaponInventory,
    bullet_res: &BulletRes,
    beam_res: &BeamRifleRes,
    weapon_sounds: &WeaponSounds,
    pos: Vec2,
    dir: Vec2,
) {
    let sound = {
        let weapon = inventory.current();
        match weapon.weapon_type {
            WeaponType::Zapper => {
                spawn_bullet(commands, bullet_res, weapon, pos, dir);
                weapon_sounds.laser.clone()
            }
            WeaponType::BeamRifle => {
                beam_rifle::spawn_bullet(commands, beam_res, weapon, pos, dir);
                weapon_sounds.shoot.clone()
            }
        }
    };
    commands.spawn((AudioPlayer::new(sound), PlaybackSettings::DESPAWN));
    inventory.current_mut().reset_timer();
}

pub fn spawn_bullet(
    commands: &mut Commands,
    bullet_res: &BulletRes,
    weapon: &Weapon,
    pos: Vec2,
    dir: Vec2,
) {
    let normalized_dir = dir.normalize_or_zero();

    let mut bullet = commands.spawn((
        Sprite::from_atlas_image(
            bullet_res.0.clone(),
            TextureAtlas {
                layout: bullet_res.1.clone(),
                index: 0,
            },
        ),
        Transform {
            translation: Vec3::new(pos.x, pos.y, 910.0),
            scale: Vec3::splat(weapon.bullet_size),
            ..Default::default()
        },
        AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
        AnimationFrameCount(3),
        Velocity(normalized_dir * weapon.bullet_speed),
        Bullet,
        BulletOwner::Player,
        Collider {
            half_extents: Vec2::splat(5.0),
        },
        BulletDamage(weapon.damage),
        HitEnemies::default(),
        GameEntity,
    ));
    let pierce = weapon.effective_pierce_count();
    if pierce > 0 {
        bullet.insert(Piercing(pierce));
    }
}

#[derive(Component)]
pub struct BulletDamage(pub f32);
