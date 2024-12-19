//! This example shows how to enable pixel art anti aliasing for a material, and
//! demonstrates its effect on how the material looks.
//!
//! By pressing 1, 2, or 3, the sprite can be switched between pixel art anti
//! aliasing, standard linear filtering, and standard nearest filtering respectively.
//!
//! With nearest filtering, the edges between colors look inconsistent. Pixel art
//! anti aliasing causes these artifacts to disappear while preserving discrete
//! color boundaries, unlike linear filtering.

use bevy::{image::ImageSampler, pbr::TextureSampler, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (spin_camera, (change_mode, swap_material).chain()))
        .run();
}

const IMAGE_PATH: &str = "pixel/bevy_pixel_dark.png";

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pixel_art_material = materials.add(StandardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        base_color_texture: Some(asset_server.load(IMAGE_PATH)),
        texture_sampler: TextureSampler::PixelArt,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(PixelArtMaterial(pixel_art_material.clone()));
    commands.insert_resource(NormalMaterial(materials.add(StandardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        base_color_texture: Some(asset_server.load(IMAGE_PATH)),
        cull_mode: None,
        ..default()
    })));

    // camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.3, 0.5)),
            ..default()
        },
        SpinCamera {
            offset: Vec3::new(-2.0, 1.5, 2.0),
            target: Vec3::ZERO,
            angle: 0.0,
        },
        Transform::from_xyz(-2.0, 1.5, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d {
            normal: Dir3::Y,
            half_size: Vec2::splat(2.5),
        })),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // pixel art sprite
    commands.spawn((
        DemoMaterial,
        Mesh3d(meshes.add(Plane3d {
            normal: Dir3::Z,
            half_size: Vec2::splat(0.5),
        })),
        MeshMaterial3d(pixel_art_material),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    commands.insert_resource(CurrentMode::PixelArt);

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

#[derive(Resource, Deref)]
struct PixelArtMaterial(Handle<StandardMaterial>);
#[derive(Resource, Deref)]
struct NormalMaterial(Handle<StandardMaterial>);

#[derive(Component)]
struct DemoMaterial;

#[derive(Resource)]
enum CurrentMode {
    PixelArt,
    Linear,
    Nearest,
}

fn change_mode(mut mode: ResMut<CurrentMode>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Digit1) {
        *mode = CurrentMode::PixelArt;
    } else if input.just_pressed(KeyCode::Digit2) {
        *mode = CurrentMode::Linear;
    } else if input.just_pressed(KeyCode::Digit3) {
        *mode = CurrentMode::Nearest;
    }
}

fn swap_material(
    mode: Res<CurrentMode>,
    mut demo_materials: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemoMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pixel_art_material: Res<PixelArtMaterial>,
    normal_material: Res<NormalMaterial>,
) {
    if !mode.is_changed() {
        return;
    }

    for mut material in &mut demo_materials {
        match *mode {
            CurrentMode::PixelArt => {
                material.0 = pixel_art_material.clone();
            }
            CurrentMode::Linear => {
                material.0 = normal_material.clone();

                let Some(texture) = materials
                    .get_mut(&*material)
                    .and_then(|m| m.base_color_texture.as_ref())
                else {
                    continue;
                };
                let Some(texture) = images.get_mut(texture) else {
                    continue;
                };
                texture.sampler = ImageSampler::linear();
            }
            CurrentMode::Nearest => {
                material.0 = normal_material.clone();

                let Some(texture) = materials
                    .get_mut(&*material)
                    .and_then(|m| m.base_color_texture.as_ref())
                else {
                    continue;
                };
                let Some(texture) = images.get_mut(texture) else {
                    continue;
                };
                texture.sampler = ImageSampler::nearest();
            }
        }
    }
}

#[derive(Component, Default)]
struct SpinCamera {
    target: Vec3,
    offset: Vec3,
    angle: f32,
}

fn spin_camera(mut cameras: Query<(&mut SpinCamera, &mut Transform)>, time: Res<Time>) {
    for (mut spin, mut transform) in &mut cameras {
        spin.angle += std::f32::consts::FRAC_PI_8 * time.delta_secs();

        let rotation = Quat::from_axis_angle(Vec3::Y, spin.angle);

        transform.translation = spin.target + rotation * spin.offset;
        transform.look_at(spin.target, Vec3::Y);
    }
}
