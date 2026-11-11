//! Feedback visual e vida no cenário: cursor de interação, números subindo, partículas,
//! vento, bichinhos, fumaça e sombras de nuvem.

use std::sync::atomic::{AtomicU32, Ordering};

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::AppState;
use crate::cycle::{DayPhase, half_view};
use crate::map::{Ground, Map, MapState, Sway, SwayKind, depth, tile_center, world_size, world_to_tile};
use crate::player::Player;
use crate::sprites::{DualSprite, Sprites, fx};
use crate::ui::font;

pub fn plugin(app: &mut App) {
    app.init_resource::<Focus>()
        .init_resource::<Popups>()
        .init_resource::<Pickups>()
        .add_systems(OnEnter(AppState::Playing), spawn_cursor)
        .add_systems(Update, (sway, particles, pop))
        .add_systems(
            Update,
            (draw_cursor, show_popups, footsteps, critters, chimney_smoke, falling_leaves, water_sparkles, clouds, swing, fly_to_bar)
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnExit(AppState::Playing), |mut f: ResMut<Focus>, mut p: ResMut<Popups>| {
            f.0 = None;
            p.0.clear();
        });
    app.add_systems(
        Update,
        spawn_clouds.in_set(crate::world::MapEntered).after(crate::world::enter_map).run_if(state_changed::<MapState>),
    );
}

/// Aleatório barato e suficiente para efeitos (sem dependência extra).
fn rand01() -> f32 {
    static STATE: AtomicU32 = AtomicU32::new(0x1234_5678);
    let mut x = STATE.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    STATE.store(x, Ordering::Relaxed);
    (x & 0xFFFF) as f32 / 65536.0
}

fn rand_range(lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * rand01()
}

/// Vento: uma onda que atravessa o mapa da esquerda para a direita.
fn wind(t: f32, x: f32) -> f32 {
    (t * 1.3 - x * 0.02).sin() + 0.5 * (t * 0.37 + x * 0.005).sin()
}

/// Folhagem mexida pelo jogador (segundos restantes).
#[derive(Component)]
pub struct Rustle(pub f32);

fn sway(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut q: Query<(Entity, &Sway, &Transform, &mut Sprite, &ViewVisibility, Option<&mut Rustle>)>,
) {
    let t = time.elapsed_secs();
    for (entity, sway, transform, mut sprite, visible, rustle) in &mut q {
        // Fora da tela não precisa balançar.
        if !visible.get() {
            continue;
        }
        let mut w = wind(t, transform.translation.x);
        if let Some(mut rustle) = rustle {
            rustle.0 -= time.delta_secs();
            // Mexe rápido para os dois lados enquanto dura.
            w = if (rustle.0 * 20.0).sin() > 0.0 { 1.0 } else { -1.0 };
            if rustle.0 <= 0.0 {
                commands.entity(entity).remove::<Rustle>();
            }
        }
        let index = match sway.kind {
            SwayKind::Tree if w > 0.85 => 2,
            SwayKind::Tree if w < -0.95 => 1,
            SwayKind::Tree => 0,
            SwayKind::Decor(offset) if w > 0.7 => sway.base + offset,
            SwayKind::Decor(_) => sway.base,
        };
        if let Some(atlas) = &mut sprite.texture_atlas
            && atlas.index != index
        {
            atlas.index = index;
        }
    }
}

// ---------------------------------------------------------------------------
// Cursor de interação: quatro cantos de colchete em volta do alvo
// ---------------------------------------------------------------------------

/// Alvo da ação disponível: retângulo no mundo, se a ação faz algo (ou é só informação)
/// e que tipo de coisa é (muda o jeito de apontar).
#[derive(Resource, Default)]
pub struct Focus(pub Option<(Rect, bool, FocusKind)>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusKind {
    /// Chão: colchetes nos cantos do tile.
    Tile,
    /// Objeto ou prédio: setinha quicando em cima.
    Thing,
    /// Gente: balão em cima da cabeça.
    Person,
}

#[derive(Component)]
struct CursorCorner(usize);

#[derive(Component)]
struct PointerArrow;

#[derive(Component)]
struct TalkBubble;

/// Retângulo atual dos colchetes (desliza até o alvo em vez de pular).
#[derive(Resource, Default)]
struct CursorRect(Option<Rect>);

fn spawn_cursor(mut commands: Commands, sprites: Res<Sprites>) {
    for (marker, art) in [(0, &sprites.arrow), (1, &sprites.bubble)] {
        let mut entity = commands.spawn((
            DualSprite::bundle(art, 0),
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, 0.0, 31.0),
            Visibility::Hidden,
            DespawnOnExit(AppState::Playing),
        ));
        if marker == 0 {
            entity.insert(PointerArrow);
        } else {
            entity.insert(TalkBubble);
        }
    }
    let anchors = [Anchor::TOP_LEFT, Anchor::TOP_RIGHT, Anchor::BOTTOM_LEFT, Anchor::BOTTOM_RIGHT];
    for (i, anchor) in anchors.into_iter().enumerate() {
        let (mut sprite, dual) = DualSprite::bundle(&sprites.cursor, 0);
        sprite.flip_x = i % 2 == 1;
        sprite.flip_y = i >= 2;
        commands.spawn((
            CursorCorner(i),
            sprite,
            dual,
            anchor,
            Transform::from_xyz(0.0, 0.0, 30.0),
            Visibility::Hidden,
            DespawnOnExit(AppState::Playing),
        ));
    }
}

#[allow(clippy::type_complexity)]
fn draw_cursor(
    time: Res<Time<Real>>,
    focus: Res<Focus>,
    mut current: Local<CursorRect>,
    mut corners: Query<(&CursorCorner, &mut Transform, &mut Visibility, &mut Sprite), (Without<PointerArrow>, Without<TalkBubble>)>,
    mut arrow: Query<(&mut Transform, &mut Visibility, &mut Sprite), (With<PointerArrow>, Without<TalkBubble>)>,
    mut bubble: Query<(&mut Transform, &mut Visibility, &mut Sprite), (With<TalkBubble>, Without<PointerArrow>)>,
) {
    let t = time.elapsed_secs();
    let target = focus.0;
    let tile_target = target.filter(|(_, _, k)| *k == FocusKind::Tile);

    // Colchetes: só para o chão; deslizam do alvo anterior.
    current.0 = match (current.0, tile_target) {
        (Some(from), Some((to, ..))) => {
            let k = 1.0 - (-22.0 * time.delta_secs()).exp();
            Some(Rect::from_corners(from.min.lerp(to.min, k), from.max.lerp(to.max, k)))
        }
        (None, Some((to, ..))) => Some(to),
        (_, None) => None,
    };
    let pulse = ((t * 6.0).sin() * 1.2 + 1.5).round();
    for (corner, mut transform, mut visibility, mut sprite) in &mut corners {
        let (Some(rect), Some((_, active, _))) = (current.0, tile_target) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);
        let r = rect.inflate(pulse);
        let p = match corner.0 {
            0 => Vec2::new(r.min.x, r.max.y),
            1 => Vec2::new(r.max.x, r.max.y),
            2 => Vec2::new(r.min.x, r.min.y),
            _ => Vec2::new(r.max.x, r.min.y),
        };
        transform.translation = p.round().extend(30.0);
        sprite.color = Color::WHITE.with_alpha(if active { 1.0 } else { 0.4 });
    }

    // Setinha quicando em cima de objetos e prédios.
    let bob = ((t * 5.0).sin() * 2.0).round();
    for (mut transform, mut visibility, mut sprite) in &mut arrow {
        match target {
            Some((rect, active, FocusKind::Thing)) => {
                visibility.set_if_neq(Visibility::Inherited);
                let top = Vec2::new(rect.center().x, (rect.max.y + 6.0).min(rect.min.y + 40.0));
                transform.translation = (top + Vec2::Y * bob).round().extend(31.0);
                sprite.color = Color::WHITE.with_alpha(if active { 1.0 } else { 0.5 });
            }
            _ => _ = visibility.set_if_neq(Visibility::Hidden),
        }
    }
    // Balão em cima de quem dá para conversar: "!" para ação, "..." para só olhar.
    for (mut transform, mut visibility, mut sprite) in &mut bubble {
        match target {
            Some((rect, active, FocusKind::Person)) => {
                visibility.set_if_neq(Visibility::Inherited);
                let top = Vec2::new(rect.center().x + 5.0, rect.max.y + 2.0);
                transform.translation = (top + Vec2::Y * (bob * 0.5)).round().extend(31.0);
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = if active { 0 } else { 1 };
                }
            }
            _ => _ = visibility.set_if_neq(Visibility::Hidden),
        }
    }
}

// ---------------------------------------------------------------------------
// Números subindo ("+12 ouro")
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct Popups(Vec<(Vec2, String, Color)>);

impl Popups {
    pub const GOLD: Color = Color::srgb(1.0, 0.86, 0.35);
    pub const GOOD: Color = Color::srgb(0.6, 1.0, 0.55);
    pub const ESSENCE: Color = Color::srgb(0.8, 0.55, 1.0);

    pub fn push(&mut self, world: Vec2, text: impl Into<String>, color: Color) {
        self.0.push((world, text.into(), color));
    }
}

#[derive(Component)]
struct Popup {
    world: Vec2,
    age: f32,
    color: Color,
}

const POPUP_LIFE: f32 = 1.2;

fn show_popups(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    mut queue: ResMut<Popups>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut popups: Query<(Entity, &mut Popup, &mut Node, &mut TextColor)>,
) {
    for (world, text, color) in queue.0.drain(..) {
        commands.spawn((
            Popup { world, age: 0.0, color },
            Text::new(text),
            font(&sprites, 22.0),
            TextColor(color),
            TextShadow { offset: Vec2::splat(2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8) },
            Node { position_type: PositionType::Absolute, ..default() },
            DespawnOnExit(AppState::Playing),
        ));
    }
    let (camera, transform) = *camera;
    for (entity, mut popup, mut node, mut color) in &mut popups {
        popup.age += time.delta_secs();
        if popup.age > POPUP_LIFE {
            commands.entity(entity).despawn();
            continue;
        }
        let k = popup.age / POPUP_LIFE;
        let world = popup.world + Vec2::new(0.0, 16.0 * k.sqrt());
        if let Ok(screen) = camera.world_to_viewport(transform, world.extend(0.0)) {
            node.left = Val::Px(screen.x - 30.0);
            node.top = Val::Px(screen.y);
        }
        color.0 = popup.color.with_alpha(if k < 0.6 { 1.0 } else { 1.0 - (k - 0.6) / 0.4 });
    }
}

// ---------------------------------------------------------------------------
// "Pulinho" de algo que acabou de aparecer
// ---------------------------------------------------------------------------

#[derive(Component, Default)]
pub struct Pop(f32);

const POP_TIME: f32 = 0.3;

fn pop(mut commands: Commands, time: Res<Time<Real>>, mut q: Query<(Entity, &mut Pop, &mut Transform)>) {
    for (entity, mut pop, mut transform) in &mut q {
        pop.0 += time.delta_secs();
        let k = (pop.0 / POP_TIME).min(1.0);
        let bump = (k * std::f32::consts::PI).sin() * 0.3;
        transform.scale = Vec3::new(1.0 - bump * 0.5, 1.0 + bump, 1.0);
        if k >= 1.0 {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<Pop>();
        }
    }
}

// ---------------------------------------------------------------------------
// Partículas (quadros da spritesheet fx)
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Particle {
    vel: Vec2,
    age: f32,
    life: f32,
    first: usize,
    frames: usize,
    gravity: f32,
    /// Alterna os quadros em vez de avançar (folha girando, vaga-lume piscando).
    flicker: bool,
}

fn spawn_particle(commands: &mut Commands, sprites: &Sprites, scope: &str, pos: Vec2, z: f32, p: Particle) {
    commands.spawn((
        DualSprite::bundle(&sprites.fx, p.first),
        p,
        Transform::from_translation(pos.extend(z)),
        DespawnOnExit(MapState(scope.to_string())),
    ));
}

/// Explosão pequena de efeitos em volta de um ponto (plantar, colher, construir...).
pub fn spawn_burst(commands: &mut Commands, sprites: &Sprites, scope: &str, at: Vec2, first: usize, n: usize) {
    for _ in 0..n {
        let angle = rand_range(0.3, std::f32::consts::PI - 0.3);
        let speed = rand_range(10.0, 26.0);
        let (frames, life, gravity, flicker) = match first {
            fx::LEAF => (2, 0.9, 50.0, true),
            fx::SMOKE => (3, 1.1, -6.0, false),
            fx::SPARKLE => (2, 0.6, 0.0, true),
            fx::COIN | fx::HEART | fx::TALK => (1, 0.8, 40.0, false),
            _ => (2, 0.45, 20.0, false),
        };
        let particle = Particle {
            vel: Vec2::new(angle.cos(), angle.sin()) * speed,
            age: 0.0,
            life,
            first,
            frames,
            gravity,
            flicker,
        };
        spawn_particle(commands, sprites, scope, at + Vec2::new(0.0, 4.0), 25.0, particle);
    }
}

fn particles(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>) {
    let dt = time.delta_secs();
    for (entity, mut p, mut transform, mut sprite) in &mut q {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(entity).despawn();
            continue;
        }
        let gravity = p.gravity;
        p.vel.y -= gravity * dt;
        transform.translation += (p.vel * dt).extend(0.0);
        let k = p.age / p.life;
        let frame = if p.flicker {
            (p.age * 6.0) as usize % p.frames
        } else {
            ((k * p.frames as f32) as usize).min(p.frames - 1)
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = p.first + frame;
        }
        sprite.color = Color::WHITE.with_alpha(if k < 0.6 { 1.0 } else { 1.0 - (k - 0.6) / 0.4 });
    }
}

/// Poeirinha nos passos sobre terra, caminho e pedra.
fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    player: Single<(&Player, &Transform)>,
    mut timer: Local<f32>,
) {
    let (player, transform) = *player;
    let Some(map) = map else { return };
    *timer -= time.delta_secs();
    if !player.moving || *timer > 0.0 {
        return;
    }
    *timer = 0.28;
    let feet = transform.translation.truncate();
    if let Some(decor) = map.decor_at(world_to_tile(feet + Vec2::Y * 2.0)) {
        commands.entity(decor).insert(Rustle(0.35));
    }
    if !matches!(map.ground(world_to_tile(feet + Vec2::Y * 2.0)), Some(Ground::Path | Ground::Soil | Ground::Cobble)) {
        return;
    }
    let particle = Particle {
        vel: Vec2::new(rand_range(-6.0, 6.0), 4.0),
        age: 0.0,
        life: 0.4,
        first: fx::DUST,
        frames: 2,
        gravity: 0.0,
        flicker: false,
    };
    spawn_particle(&mut commands, &sprites, &map.id, feet + Vec2::new(0.0, 2.0), depth(feet.y) - 0.001, particle);
}

fn chimney_smoke(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    mut timer: Local<f32>,
) {
    let Some(map) = map else { return };
    *timer -= time.delta_secs();
    if *timer > 0.0 || map.chimneys.is_empty() {
        return;
    }
    *timer = 0.5;
    for chimney in &map.chimneys {
        let particle = Particle {
            vel: Vec2::new(rand_range(3.0, 8.0), rand_range(10.0, 16.0)),
            age: 0.0,
            life: 2.4,
            first: fx::SMOKE,
            frames: 3,
            gravity: -2.0,
            flicker: false,
        };
        spawn_particle(&mut commands, &sprites, &map.id, *chimney + Vec2::new(rand_range(-2.0, 2.0), 0.0), 26.0, particle);
    }
}

/// Folhas que se soltam das árvores visíveis.
fn falling_leaves(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Transform, &Projection), With<Camera2d>>,
    trees: Query<(&Transform, &Sway), Without<Camera2d>>,
    mut timer: Local<f32>,
) {
    let Some(map) = map else { return };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = rand_range(0.4, 1.0);
    let (cam, projection) = *camera;
    let view = Rect::from_center_half_size(cam.translation.truncate(), half_view(&window, projection));
    let visible: Vec<Vec2> = trees
        .iter()
        .filter(|(_, s)| s.kind == SwayKind::Tree)
        .map(|(t, _)| t.translation.truncate())
        .filter(|p| view.contains(*p))
        .collect();
    if visible.is_empty() {
        return;
    }
    let tree = visible[(rand01() * visible.len() as f32) as usize % visible.len()];
    let particle = Particle {
        vel: Vec2::new(rand_range(4.0, 14.0), rand_range(-14.0, -8.0)),
        age: 0.0,
        life: 2.6,
        first: fx::LEAF,
        frames: 2,
        gravity: 0.0,
        flicker: true,
    };
    let at = tree + Vec2::new(rand_range(-12.0, 12.0), rand_range(24.0, 40.0));
    spawn_particle(&mut commands, &sprites, &map.id, at, 24.0, particle);
}

fn water_sparkles(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Transform, &Projection), With<Camera2d>>,
    mut timer: Local<f32>,
) {
    let Some(map) = map else { return };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.12;
    let (cam, projection) = *camera;
    let half = half_view(&window, projection);
    let p = cam.translation.truncate() + Vec2::new(rand_range(-half.x, half.x), rand_range(-half.y, half.y));
    let tile = world_to_tile(p);
    if map.ground(tile) != Some(Ground::Water) {
        return;
    }
    let particle = Particle { vel: Vec2::ZERO, age: 0.0, life: 0.7, first: fx::SPARKLE, frames: 2, gravity: 0.0, flicker: false };
    let at = tile_center(tile) + Vec2::new(rand_range(-5.0, 5.0), rand_range(-5.0, 5.0));
    spawn_particle(&mut commands, &sprites, &map.id, at, 0.3, particle);
}

// ---------------------------------------------------------------------------
// Borboletas (dia) e vaga-lumes (noite)
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Critter {
    night: bool,
    origin: Vec2,
    t: f32,
    seed: f32,
}

fn critters(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    phase: Option<Res<State<DayPhase>>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Transform, &Projection), With<Camera2d>>,
    mut q: Query<(Entity, &mut Critter, &mut Transform, &mut Sprite), Without<Camera2d>>,
) {
    let (Some(map), Some(phase)) = (map, phase) else { return };
    let night = *phase.get() == DayPhase::Night;
    let (cam, projection) = *camera;
    let half = half_view(&window, projection);
    let center = cam.translation.truncate();
    let wanted = match (map.info.indoor, night) {
        (true, _) => 0,
        (false, false) => 5,
        (false, true) => 16,
    };

    let mut count = 0;
    for (entity, mut c, mut transform, mut sprite) in &mut q {
        let far = (c.origin - center).abs().cmpgt(half + Vec2::splat(60.0)).any();
        if c.night != night || far || count >= wanted {
            commands.entity(entity).despawn();
            continue;
        }
        count += 1;
        c.t += time.delta_secs();
        let t = c.t + c.seed * 10.0;
        let (offset, frame, alpha) = if night {
            // Flutua devagar e pisca.
            let glow = ((t * 1.7).sin() * 0.5 + 0.5).powf(1.5);
            (Vec2::new((t * 0.5).sin() * 18.0, (t * 0.33).cos() * 12.0 + (t * 2.0).sin() * 2.0), fx::FIREFLY + (glow > 0.6) as usize, 0.3 + glow * 0.7)
        } else {
            // Voo em zigue-zague, batendo as asas.
            c.origin.x += time.delta_secs() * 8.0 * (c.seed - 0.5).signum();
            (Vec2::new((t * 0.8).sin() * 24.0, (t * 1.9).sin() * 8.0 + (t * 0.6).cos() * 10.0), fx::BUTTERFLY + ((t * 9.0) as usize % 2), 1.0)
        };
        transform.translation = (c.origin + offset).round().extend(28.0);
        sprite.color = Color::WHITE.with_alpha(alpha);
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = frame;
        }
    }
    for _ in count..wanted {
        let origin = center + Vec2::new(rand_range(-half.x, half.x), rand_range(-half.y, half.y));
        commands.spawn((
            Critter { night, origin, t: 0.0, seed: rand01() },
            DualSprite::bundle(&sprites.fx, if night { fx::FIREFLY } else { fx::BUTTERFLY }),
            Transform::from_translation(origin.extend(28.0)),
            DespawnOnExit(MapState(map.id.clone())),
        ));
    }
}

// ---------------------------------------------------------------------------
// Sombras de nuvem passando (só de dia, ao ar livre)
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Cloud(f32);

/// O mapa pode ainda não existir no primeiro quadro do jogo (estado vazio).
fn spawn_clouds(mut commands: Commands, sprites: Res<Sprites>, map: Option<Res<Map>>) {
    let Some(map) = map else { return };
    if map.info.indoor {
        return;
    }
    let size = world_size(map.size);
    for _ in 0..(size.x * size.y / 90_000.0).ceil() as usize + 2 {
        let p = Vec2::new(rand_range(0.0, size.x), -rand_range(0.0, size.y));
        commands.spawn((
            Cloud(rand_range(5.0, 10.0)),
            Sprite::from_image(sprites.cloud_shadow.clone()),
            Transform::from_translation(p.extend(29.0)).with_scale(Vec3::splat(2.0)),
            DespawnOnExit(MapState(map.id.clone())),
        ));
    }
}

fn clouds(
    time: Res<Time>,
    map: Option<Res<Map>>,
    phase: Option<Res<State<DayPhase>>>,
    mut q: Query<(&Cloud, &mut Transform, &mut Visibility)>,
) {
    let Some(map) = map else { return };
    let night = phase.is_some_and(|p| *p.get() == DayPhase::Night);
    let width = world_size(map.size).x;
    for (cloud, mut transform, mut visibility) in &mut q {
        visibility.set_if_neq(if night { Visibility::Hidden } else { Visibility::Inherited });
        transform.translation.x += cloud.0 * time.delta_secs();
        if transform.translation.x > width + 120.0 {
            transform.translation.x = -120.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Arco da ferramenta e itens voando para a barra
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Swing {
    t: f32,
    flip: bool,
}

const SWING_TIME: f32 = 0.2;

/// O ícone da ferramenta faz um arco na mão do jogador.
pub fn spawn_swing(commands: &mut Commands, sprites: &Sprites, scope: &str, hand: Vec2, icon: usize, flip: bool) {
    let mut sprite = sprites.items.sprite(DayPhase::Day, icon);
    sprite.flip_x = flip;
    commands.spawn((
        Swing { t: 0.0, flip },
        sprite,
        Transform::from_translation(hand.extend(26.0)),
        DespawnOnExit(MapState(scope.to_string())),
    ));
}

fn swing(mut commands: Commands, time: Res<Time<Real>>, mut q: Query<(Entity, &mut Swing, &mut Transform)>) {
    for (entity, mut swing, mut transform) in &mut q {
        swing.t += time.delta_secs();
        let k = (swing.t / SWING_TIME).min(1.0);
        let sign = if swing.flip { -1.0 } else { 1.0 };
        // De cima para a frente, como uma enxadada.
        transform.rotation = Quat::from_rotation_z(sign * (0.9 - k * 1.8));
        if k >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Itens que acabaram de entrar na mochila (posição no mundo, ícone).
#[derive(Resource, Default)]
pub struct Pickups(pub Vec<(Vec2, usize)>);

#[derive(Component)]
struct Flying {
    from: Vec2,
    t: f32,
}

const FLY_TIME: f32 = 0.55;

fn fly_to_bar(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    mut pickups: ResMut<Pickups>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut flying: Query<(Entity, &mut Flying, &mut Node, &mut UiTransform)>,
) {
    let (camera, cam_transform) = *camera;
    for (world, icon) in pickups.0.drain(..) {
        let Ok(screen) = camera.world_to_viewport(cam_transform, world.extend(0.0)) else { continue };
        commands.spawn((
            Flying { from: screen, t: 0.0 },
            ImageNode {
                image: sprites.items.day.clone(),
                texture_atlas: sprites.items.layout.clone().map(|layout| TextureAtlas { layout, index: icon }),
                ..default()
            },
            Node { position_type: PositionType::Absolute, width: Val::Px(32.0), height: Val::Px(32.0), ..default() },
            UiTransform::IDENTITY,
            GlobalZIndex(8),
            Pickable::IGNORE,
            DespawnOnExit(AppState::Playing),
        ));
    }
    // Destino: o meio da barra de itens.
    let to = Vec2::new(window.width() / 2.0, window.height() - 40.0);
    for (entity, mut fly, mut node, mut transform) in &mut flying {
        fly.t += time.delta_secs();
        let k = (fly.t / FLY_TIME).min(1.0);
        let ease = 1.0 - (1.0 - k).powi(3);
        // Arco: sobe um pouco antes de cair na barra.
        let p = fly.from.lerp(to, ease) - Vec2::Y * (k * std::f32::consts::PI).sin() * 60.0;
        node.left = Val::Px(p.x - 16.0);
        node.top = Val::Px(p.y - 16.0);
        transform.scale = Vec2::splat(1.0 + (k * std::f32::consts::PI).sin() * 0.4 - k * 0.3);
        if k >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}
