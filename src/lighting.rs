//! Clima de luz: tom da hora do dia, vinheta e poças de luz quente à noite.
//! Tudo com sprites translúcidos e texturas geradas em código (sem shaders).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::AppState;
use crate::cycle::{Clock, DAY_SKY, DayPhase, NIGHT_SKY, half_view};
use crate::map::Map;
use crate::player::Player;

pub fn plugin(app: &mut App) {
    // PostStartup: a câmera nasce no Startup (cycle.rs).
    app.add_systems(PostStartup, setup)
        .add_systems(OnEnter(AppState::Playing), lantern_glow)
        .add_systems(Update, (sky, tint, light_sources));
}

/// Algo que brilha à noite (postes, forno, forja).
#[derive(Component)]
pub struct LightSource;

#[derive(Component)]
struct Tint;

#[derive(Component)]
struct Vignette;

/// Poça de luz; só aparece à noite.
#[derive(Component)]
struct Glow;

#[derive(Resource)]
struct LightImages {
    glow: Handle<Image>,
}

/// Matriz de Bayer 4x4: degradês em degraus pontilhados, no estilo pixel art.
const BAYER: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];

/// Textura RGBA a partir de uma função (x, y normalizados em -1..1) -> alfa 0..1,
/// com o alfa quantizado em `levels` degraus pontilhados.
fn dithered(w: u32, h: u32, levels: f32, alpha: impl Fn(f32, f32) -> f32) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = (x as f32 / (w - 1) as f32 * 2.0 - 1.0, y as f32 / (h - 1) as f32 * 2.0 - 1.0);
            let a = alpha(nx, ny).clamp(0.0, 1.0) * levels;
            let threshold = (BAYER[(y % 4) as usize][(x % 4) as usize] + 0.5) / 16.0;
            let q = (a.floor() + if a.fract() > threshold { 1.0 } else { 0.0 }) / levels;
            data.extend_from_slice(&[255, 255, 255, (q * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, camera: Single<Entity, With<Camera2d>>) {
    let vignette = images.add(dithered(160, 90, 4.0, |x, y| {
        let d = (x * x + y * y).sqrt() / std::f32::consts::SQRT_2;
        ((d - 0.45) / 0.55).powf(1.4)
    }));
    // Queda rápida: miolo claro e borda bem fraca, para não virar uma mancha.
    let glow = images.add(dithered(64, 64, 4.0, |x, y| (1.0 - (x * x + y * y).sqrt()).max(0.0).powf(1.8)));
    commands.insert_resource(LightImages { glow });

    commands.entity(*camera).with_children(|camera| {
        camera.spawn((Tint, Sprite::from_color(Color::NONE, Vec2::splat(4000.0)), Transform::from_xyz(0.0, 0.0, 380.0)));
        camera.spawn((Vignette, Sprite::from_image(vignette), Transform::from_xyz(0.0, 0.0, 390.0)));
    });
}

fn phase_of(phase: &Option<Res<State<DayPhase>>>) -> Option<DayPhase> {
    phase.as_ref().map(|p| *p.get())
}

/// Fundo da janela: céu ao ar livre, preto dentro de casa.
fn sky(mut clear: ResMut<ClearColor>, map: Option<Res<Map>>, phase: Option<Res<State<DayPhase>>>, clock: Option<Res<Clock>>) {
    let day_sky = clock.as_ref().map_or(DAY_SKY, |c| c.season().day_sky());
    let color = match (map.map(|m| m.info.indoor), phase_of(&phase)) {
        (Some(true), _) => Color::BLACK,
        (_, Some(DayPhase::Night)) => NIGHT_SKY,
        _ => day_sky,
    };
    if clear.0 != color {
        clear.0 = color;
    }
}

/// Cor (com alfa) por cima de tudo, conforme a hora e o lugar.
fn tint_color(indoor: bool, phase: Option<DayPhase>, f: f32) -> (Color, f32) {
    // (cor do tom, força da vinheta)
    match (indoor, phase) {
        (_, None) => (Color::NONE, 0.35),
        (true, Some(DayPhase::Day)) => (Color::srgba(1.0, 0.85, 0.6, 0.06), 0.5),
        (true, Some(DayPhase::Night)) => (Color::srgba(0.3, 0.13, 0.04, 0.18), 0.8),
        (false, Some(DayPhase::Night)) => (Color::srgba(0.12, 0.08, 0.35, 0.2), 0.85),
        (false, Some(DayPhase::Day)) => {
            if f < 0.15 {
                // Manhã rosada que vai clareando.
                (Color::srgba(1.0, 0.7, 0.62, 0.2 * (1.0 - f / 0.15)), 0.3)
            } else if f > 0.6 {
                // Fim de tarde dourado, cada vez mais alaranjado.
                let k = (f - 0.6) / 0.4;
                (Color::srgba(1.0, 0.62 - 0.2 * k, 0.25, 0.26 * k), 0.3 + 0.3 * k)
            } else {
                (Color::NONE, 0.3)
            }
        }
    }
}

fn tint(
    clock: Option<Res<Clock>>,
    map: Option<Res<Map>>,
    phase: Option<Res<State<DayPhase>>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<&Projection, With<Camera2d>>,
    tint: Single<(&mut Sprite, &mut Visibility), (With<Tint>, Without<Vignette>)>,
    mut vignette: Single<(&mut Sprite, &mut Visibility), (With<Vignette>, Without<Tint>)>,
) {
    let phase = phase_of(&phase);
    let indoor = map.is_some_and(|m| m.info.indoor);
    let f = match (&clock, phase) {
        (Some(clock), Some(_)) => clock.phase_progress(),
        _ => 0.3,
    };
    let (color, strength) = tint_color(indoor, phase, f);
    let (mut tint, mut tint_visibility) = tint.into_inner();
    // Sem tom (meio do dia) a camada nem é desenhada.
    tint_visibility.set_if_neq(if color.alpha() > 0.0 { Visibility::Inherited } else { Visibility::Hidden });
    if tint.color != color {
        tint.color = color;
    }
    let (sprite, visibility) = &mut *vignette;
    let size = Some(half_view(&window, &camera) * 2.0 + Vec2::splat(2.0));
    if sprite.custom_size != size {
        sprite.custom_size = size;
    }
    let shade = Color::srgba(0.03, 0.02, 0.08, strength);
    if sprite.color != shade {
        sprite.color = shade;
    }
    visibility.set_if_neq(Visibility::Inherited);
}

fn lantern_glow(mut commands: Commands, images: Res<LightImages>, player: Single<Entity, With<Player>>) {
    commands.entity(*player).with_children(|p| {
        p.spawn(glow_bundle(&images, 1.2));
    });
}

fn glow_bundle(images: &LightImages, scale: f32) -> impl Bundle {
    (
        Glow,
        Sprite { image: images.glow.clone(), color: Color::srgba(1.0, 0.86, 0.55, 0.3), ..default() },
        // Logo acima do chão, abaixo de gente e objetos.
        Transform::from_xyz(0.0, 4.0, -9.0).with_scale(Vec3::splat(scale)),
        Visibility::Hidden,
    )
}

/// Dá uma poça de luz a cada fonte nova e acende tudo só à noite.
fn light_sources(
    mut commands: Commands,
    images: Option<Res<LightImages>>,
    map: Option<Res<Map>>,
    phase: Option<Res<State<DayPhase>>>,
    new_sources: Query<Entity, Added<LightSource>>,
    mut glows: Query<&mut Visibility, With<Glow>>,
) {
    let Some(images) = images else { return };
    for entity in &new_sources {
        commands.entity(entity).with_children(|p| {
            p.spawn(glow_bundle(&images, 1.1));
        });
    }
    let lit = map.is_some() && phase_of(&phase) == Some(DayPhase::Night);
    for mut v in &mut glows {
        v.set_if_neq(if lit { Visibility::Inherited } else { Visibility::Hidden });
    }
}
