use bevy::prelude::*;
use std::collections::HashSet;
use crate::map::{LevelRes, MapGridMeta};
use crate::player::Player;
use crate::room::{LevelState, RoomVec};
use crate::{GameEntity, GameState, TILE_SIZE};

const MINIMAP_W: f32 = 420.0;
const MINIMAP_H: f32 = 420.0;

/// Coarse tile resolution for hallway fog-of-war (N tiles per tracked cell).
const HALLWAY_CELL: usize = 3;
/// Radius in coarse cells that the player "reveals" around themselves each frame.
const REVEAL_RADIUS: i32 = 3;

// Components

#[derive(Component)]
struct MinimapRoot;

#[derive(Component)]
struct MinimapRoomNode {
    room_index: usize,
}

#[derive(Component)]
struct MinimapHallwayNode {
    cell_col: i32,
    cell_row: i32,
}

#[derive(Component)]
struct MinimapPlayerDot;

// Resources

#[derive(Resource, Default)]
pub struct MinimapVisible(pub bool);

/// Coarse cells (col/HALLWAY_CELL, row/HALLWAY_CELL) that the player has explored.
#[derive(Resource, Default)]
pub struct VisitedCells(pub HashSet<(i32, i32)>);

// Plugin

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapVisible>()
            .init_resource::<VisitedCells>()
            .add_systems(OnEnter(GameState::Playing), (clear_visited_cells, setup_minimap).chain())
            .add_systems(OnExit(GameState::Playing), clear_visited_cells)
            .add_systems(
                Update,
                toggle_minimap.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                update_visited_cells.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                update_minimap
                    .run_if(in_state(GameState::Playing))
                    .run_if(|vis: Res<MinimapVisible>| vis.0),
            );
    }
}

// Setup

fn clear_visited_cells(mut visited: ResMut<VisitedCells>) {
    visited.0.clear();
}

fn setup_minimap(
    mut commands: Commands,
    rooms: Res<RoomVec>,
    level: Res<LevelRes>,
    grid: Res<MapGridMeta>,
    asset_server: Res<AssetServer>,
) {
    let map_px_w = grid.cols as f32 * TILE_SIZE;
    let map_px_h = grid.rows as f32 * TILE_SIZE;
    let world_min_x = -map_px_w * 0.5;
    let world_max_y = map_px_h * 0.5;

    let cols = grid.cols;
    let rows = grid.rows;

    let font: Handle<Font> = asset_server.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
    );

    // Precompute which coarse cells are hallway cells (floor tile outside every room).
    let mut hallway_cells: HashSet<(i32, i32)> = HashSet::new();
    let level_rows = level.level.len();
    let level_cols = level.level.first().map(|r| r.len()).unwrap_or(0);

    for cell_row in 0..(rows / HALLWAY_CELL + 1) {
        for cell_col in 0..(cols / HALLWAY_CELL + 1) {
            let tile_col = cell_col * HALLWAY_CELL + HALLWAY_CELL / 2;
            let tile_row = cell_row * HALLWAY_CELL + HALLWAY_CELL / 2;

            if tile_row >= level_rows || tile_col >= level_cols {
                continue;
            }

            let ch = level.level[tile_row].as_bytes().get(tile_col).copied().unwrap_or(b'.');
            if ch != b'#' {
                continue;
            }

            // Skip if the coarse cell's tile AABB overlaps any room (not just center tile).
            let cell_x1 = cell_col * HALLWAY_CELL;
            let cell_y1 = cell_row * HALLWAY_CELL;
            let cell_x2 = cell_x1 + HALLWAY_CELL - 1;
            let cell_y2 = cell_y1 + HALLWAY_CELL - 1;
            let in_room = rooms.0.iter().any(|r| {
                let rx1 = r.tile_top_left_corner.x as usize;
                let ry1 = r.tile_top_left_corner.y as usize;
                let rx2 = r.tile_bot_right_corner.x as usize;
                let ry2 = r.tile_bot_right_corner.y as usize;
                cell_x1 <= rx2 && cell_x2 >= rx1 && cell_y1 <= ry2 && cell_y2 >= ry1
            });

            if !in_room {
                hallway_cells.insert((cell_col as i32, cell_row as i32));
            }
        }
    }

    let cell_w = (HALLWAY_CELL as f32 / cols as f32 * MINIMAP_W).max(2.0);
    let cell_h = (HALLWAY_CELL as f32 / rows as f32 * MINIMAP_H).max(2.0);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            Visibility::Hidden,
            ZIndex(50),
            MinimapRoot,
            GameEntity,
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("MINIMAP   [TAB] to close"),
                TextFont { font, font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.65, 0.65, 0.65)),
            ));

            // Legend row
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(18.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|leg| {
                legend_item(leg, Color::srgba(0.2, 0.2, 0.2, 0.6),  "Unexplored");
                legend_item(leg, Color::srgb(1.0, 0.9, 0.0),         "Current");
                legend_item(leg, Color::srgb(0.15, 0.65, 0.15),      "Cleared");
            });

            // Map panel
            root.spawn((
                Node {
                    width: Val::Px(MINIMAP_W),
                    height: Val::Px(MINIMAP_H),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.04, 0.12, 1.0)),
            ))
            .with_children(|panel| {
                // Room nodes
                for (i, room) in rooms.0.iter().enumerate() {
                    let mini_x = (room.top_left_corner.x - world_min_x) / map_px_w * MINIMAP_W;
                    let mini_y = (world_max_y - room.top_left_corner.y) / map_px_h * MINIMAP_H;
                    let mini_w = ((room.bot_right_corner.x - room.top_left_corner.x).abs()
                        / map_px_w * MINIMAP_W).max(6.0);
                    let mini_h = ((room.top_left_corner.y - room.bot_right_corner.y).abs()
                        / map_px_h * MINIMAP_H).max(6.0);

                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(mini_x),
                            top: Val::Px(mini_y),
                            width: Val::Px(mini_w),
                            height: Val::Px(mini_h),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.5)),
                        MinimapRoomNode { room_index: i },
                    ));
                }

                // Hallway nodes (hidden until explored)
                for (cell_col, cell_row) in &hallway_cells {
                    let tile_col = *cell_col as usize * HALLWAY_CELL + HALLWAY_CELL / 2;
                    let tile_row = *cell_row as usize * HALLWAY_CELL + HALLWAY_CELL / 2;

                    let mini_x = (tile_col as f32 / cols as f32 * MINIMAP_W - cell_w * 0.5).max(0.0);
                    let mini_y = (tile_row as f32 / rows as f32 * MINIMAP_H - cell_h * 0.5).max(0.0);

                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(mini_x),
                            top: Val::Px(mini_y),
                            width: Val::Px(cell_w),
                            height: Val::Px(cell_h),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), // invisible until visited
                        Visibility::Hidden,
                        MinimapHallwayNode { cell_col: *cell_col, cell_row: *cell_row },
                    ));
                }

                // Player dot
                panel.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(8.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
                    ZIndex(1),
                    MinimapPlayerDot,
                ));
            });
        });
}

fn legend_item(parent: &mut ChildSpawnerCommands, color: Color, label: &str) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Node { width: Val::Px(12.0), height: Val::Px(12.0), ..default() },
                BackgroundColor(color),
            ));
            row.spawn((
                Text::new(label),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.75, 0.75, 0.75)),
            ));
        });
}

// Systems

fn toggle_minimap(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<MinimapVisible>,
    mut root_q: Query<&mut Visibility, With<MinimapRoot>>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    visible.0 = !visible.0;
    if let Ok(mut vis) = root_q.single_mut() {
        *vis = if visible.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

/// Marks coarse cells near the player as visited so hallways reveal on the minimap.
fn update_visited_cells(
    player_q: Query<&Transform, With<Player>>,
    grid: Res<MapGridMeta>,
    mut visited: ResMut<VisitedCells>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let px = player_tf.translation.x;
    let py = player_tf.translation.y;

    let cols = grid.cols as f32;
    let rows = grid.rows as f32;
    let _map_px_w = cols * TILE_SIZE;
    let _map_px_h = rows * TILE_SIZE;

    // Convert player world pos to tile coordinates.
    let tile_col = ((px - grid.x0) / TILE_SIZE).round() as i32;
    // y0 is the world Y of the bottom tile; row 0 is the top tile.
    let tile_row = (rows as i32 - 1) - ((py - grid.y0) / TILE_SIZE).round() as i32;

    let coarse_col = tile_col / HALLWAY_CELL as i32;
    let coarse_row = tile_row / HALLWAY_CELL as i32;

    for dr in -REVEAL_RADIUS..=REVEAL_RADIUS {
        for dc in -REVEAL_RADIUS..=REVEAL_RADIUS {
            visited.0.insert((coarse_col + dc, coarse_row + dr));
        }
    }
}

fn update_minimap(
    rooms: Res<RoomVec>,
    player_q: Query<&Transform, With<Player>>,
    grid: Res<MapGridMeta>,
    lvlstate: Res<LevelState>,
    visited: Res<VisitedCells>,
    mut room_nodes: Query<(&MinimapRoomNode, &mut BackgroundColor), Without<MinimapHallwayNode>>,
    mut hallway_nodes: Query<(&MinimapHallwayNode, &mut BackgroundColor, &mut Visibility)>,
    mut player_dot: Query<&mut Node, With<MinimapPlayerDot>>,
) {
    let map_px_w = grid.cols as f32 * TILE_SIZE;
    let map_px_h = grid.rows as f32 * TILE_SIZE;
    let world_min_x = -map_px_w * 0.5;
    let world_max_y = map_px_h * 0.5;

    let current_room = match *lvlstate {
        LevelState::InRoom(i, _) | LevelState::EnteredRoom(i) => Some(i),
        LevelState::NotRoom => None,
    };

    // Update room nodes
    for (node, mut bg) in &mut room_nodes {
        let Some(room) = rooms.0.get(node.room_index) else { continue };
        bg.0 = if current_room == Some(node.room_index) {
            Color::srgba(1.0, 0.9, 0.0, 1.0)
        } else if !room.visited {
            Color::srgba(0.04, 0.04, 0.12, 1.0)
        } else if room.cleared {
            Color::srgba(0.15, 0.65, 0.15, 0.9)
        } else {
            Color::srgba(0.35, 0.35, 0.55, 0.9)
        };
    }

    // Update hallway nodes
    for (node, mut bg, mut vis) in &mut hallway_nodes {
        if visited.0.contains(&(node.cell_col, node.cell_row)) {
            *vis = Visibility::Inherited; // inherits from parent MinimapRoot, hides when map closes
            bg.0 = Color::srgba(0.30, 0.30, 0.45, 0.85);
        }
    }

    // Update player dot
    let Ok(player_tf) = player_q.single() else { return };
    let px = player_tf.translation.x;
    let py = player_tf.translation.y;

    let dot_x = ((px - world_min_x) / map_px_w * MINIMAP_W - 4.0).clamp(0.0, MINIMAP_W - 8.0);
    let dot_y = ((world_max_y - py) / map_px_h * MINIMAP_H - 4.0).clamp(0.0, MINIMAP_H - 8.0);

    if let Ok(mut node) = player_dot.single_mut() {
        node.left = Val::Px(dot_x);
        node.top = Val::Px(dot_y);
    }
}
