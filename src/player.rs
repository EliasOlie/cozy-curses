//! O forasteiro imune à maldição: peças tingidas, animação de caminhada e câmera.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::character::{Character, Facing, NIGHT_TINT};
use crate::cycle::{DayPhase, half_view};
use crate::economy::Game;
use crate::dialogue::Dialogue;
use crate::map::{Map, Occluder, depth, tile_center, world_size, world_to_tile};
use crate::save::ActiveSave;
use crate::world::{Travel, check_warp};
use crate::sprites::{DualSprite, Sprites};
use crate::{AppState, Gameplay};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (walk, animate.after(walk), fade_occluders).in_set(Gameplay))
        .add_systems(Update, (night_look, follow_camera.after(walk)).run_if(in_state(AppState::Playing)));
}

const SPEED: f32 = 64.0;
/// Shift corre.
const RUN: f32 = 1.6;
/// Caixa dos pés usada na colisão: meia largura e altura, a partir da base do sprite.
const FEET_HALF_WIDTH: f32 = 4.0;
const FEET_HEIGHT: f32 = 4.0;
const STEP_FPS: f32 = 8.0;

#[derive(Component)]
pub struct Player {
    pub facing: Facing,
    pub moving: bool,
    anim: f32,
    /// Quadro do ciclo de passos (0 parado; 1 e 2 descem o tronco 1 px).
    step: usize,
    /// Achatar/esticar ao começar (+) e ao parar (-) de andar; decai sozinho.
    squash: f32,
}

/// Uma peça tingida do fazendeiro, com sua cor de dia.
#[derive(Component)]
pub struct FarmerPart(pub Color);

#[derive(Component)]
struct MaskOverlay;

#[derive(Component)]
struct Lantern;

/// Desenha o fazendeiro como filhos de `parent`: as 4 peças empilhadas.
/// Usado também pela prévia do menu (via UI) com o mesmo `Character`.
pub fn spawn_farmer_parts(parent: &mut ChildSpawnerCommands, character: &Character, sprites: &Sprites) {
    for (i, (art, color)) in character.parts(sprites).into_iter().enumerate() {
        let mut sprite = art.sprite(DayPhase::Day, 0);
        sprite.color = color;
        parent.spawn((
            FarmerPart(color),
            sprite,
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, 0.0, i as f32 * 0.0001),
        ));
    }
}

/// O jogador nasce na origem; `world::enter_map` o coloca no lugar certo do mapa.
pub fn spawn_player(mut commands: Commands, sprites: Res<Sprites>, active: Res<ActiveSave>) {
    commands
        .spawn((
            Player { facing: Facing::Down, moving: false, anim: 0.0, step: 0, squash: 0.0 },
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|player| {
            // Sombra suave sob os pés: "assenta" o personagem no chão.
            player.spawn((
                DualSprite::bundle(&sprites.shadow, 0),
                Transform::from_xyz(0.0, 1.0, -0.01),
            ));
            spawn_farmer_parts(player, &active.data.character, &sprites);
            // Máscara e lampião são posicionados a cada frame por `night_look`.
            player.spawn((
                MaskOverlay,
                Sprite::from_image(sprites.mask_worn.clone()),
                Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.001),
                Visibility::Hidden,
            ));
            player.spawn((
                Lantern,
                Sprite::from_image(sprites.lantern.clone()),
                Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.002),
                Visibility::Hidden,
            ));
        });
}

fn feet_clear(map: &Map, p: Vec2) -> bool {
    !map.solid(Rect::new(p.x - FEET_HALF_WIDTH, p.y, p.x + FEET_HALF_WIDTH, p.y + FEET_HEIGHT))
}

/// Tile logo à frente do jogador: onde ferramentas e ações de chão acontecem.
pub fn target_tile(transform: &Transform, facing: Facing) -> IVec2 {
    let feet = world_to_tile(transform.translation.truncate() + Vec2::new(0.0, FEET_HEIGHT / 2.0));
    feet + match facing {
        Facing::Down => IVec2::Y,
        Facing::Up => IVec2::NEG_Y,
        Facing::Left => IVec2::NEG_X,
        Facing::Right => IVec2::X,
    }
}

fn walk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    map: Res<Map>,
    dialogue: Res<Dialogue>,
    mut travel: ResMut<Travel>,
    player: Single<(&mut Transform, &mut Player)>,
) {
    let (mut transform, mut player) = player.into_inner();
    if travel.busy() || dialogue.is_open() {
        player.moving = false;
        return;
    }
    let pressed = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let mut dir = Vec2::ZERO;
    if pressed(KeyCode::KeyW, KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if pressed(KeyCode::KeyS, KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if pressed(KeyCode::KeyA, KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if pressed(KeyCode::KeyD, KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    let was_moving = player.moving;
    player.moving = dir != Vec2::ZERO;
    if player.moving != was_moving {
        player.squash = if player.moving { 1.0 } else { -0.7 };
    }
    if !player.moving {
        return;
    }
    // Na diagonal, vale o eixo horizontal (como no Stardew).
    player.facing = if dir.x > 0.0 {
        Facing::Right
    } else if dir.x < 0.0 {
        Facing::Left
    } else if dir.y > 0.0 {
        Facing::Up
    } else {
        Facing::Down
    };

    // Move eixo a eixo para deslizar nas paredes em vez de travar.
    let run = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) { RUN } else { 1.0 };
    let step = dir.normalize() * SPEED * run * time.delta_secs();
    let mut pos = transform.translation.truncate();
    for delta in [Vec2::new(step.x, 0.0), Vec2::new(0.0, step.y)] {
        if delta == Vec2::ZERO {
            continue;
        }
        if feet_clear(&map, pos + delta) {
            pos += delta;
            continue;
        }
        // Ajuste de quina: travado de raspão, escorrega para o centro do tile para
        // passar por vãos de 1 tile (porteira, entre árvores) sem alinhar no pixel.
        let center = tile_center(world_to_tile(pos + Vec2::new(0.0, FEET_HEIGHT / 2.0)));
        let feet_center = pos + Vec2::new(0.0, FEET_HEIGHT / 2.0);
        let off = center - feet_center;
        let nudge = if delta.x != 0.0 { Vec2::new(0.0, off.y) } else { Vec2::new(off.x, 0.0) };
        let nudge = nudge.clamp_length_max(delta.length());
        if nudge.length() > 0.01 && nudge.length() < 7.0 && feet_clear(&map, pos + nudge) {
            pos += nudge;
        }
    }
    transform.translation = pos.extend(depth(pos.y) + 0.0005);
    check_warp(&map, &mut travel, world_to_tile(pos + Vec2::new(0.0, FEET_HEIGHT / 2.0)));
}

fn animate(
    time: Res<Time>,
    player: Single<(&mut Player, &mut Transform, &Children)>,
    mut parts: Query<&mut Sprite, With<FarmerPart>>,
) {
    let (mut player, mut transform, children) = player.into_inner();
    // Partida: estica para a frente; parada: achata um pouquinho.
    player.squash *= (-12.0 * time.delta_secs()).exp();
    let s = player.squash * 0.08;
    transform.scale = Vec3::new(1.0 + s, 1.0 - s, 1.0);
    // Ciclo de passos: parado, passo A, parado, passo B.
    let step = if player.moving {
        player.anim += time.delta_secs() * STEP_FPS;
        [0, 1, 0, 2][player.anim as usize % 4]
    } else {
        player.anim = 0.0;
        0
    };
    player.step = step;
    let frame = player.facing.frame(step);
    for child in children.iter() {
        if let Ok(mut sprite) = parts.get_mut(child) {
            sprite.flip_x = player.facing.flip();
            if let Some(atlas) = &mut sprite.texture_atlas
                && atlas.index != frame
            {
                atlas.index = frame;
            }
        }
    }
}

/// De noite: peças azuladas, lampião aceso e, com a máscara, o rosto de gárgula.
fn night_look(
    game: Option<Res<Game>>,
    phase: Option<Res<State<DayPhase>>>,
    player: Single<&Player>,
    mut parts: Query<(&FarmerPart, &mut Sprite)>,
    mut mask: Query<(&mut Visibility, &mut Transform), (With<MaskOverlay>, Without<Lantern>)>,
    mut lantern: Query<
        (&mut Visibility, &mut Transform, &mut Sprite),
        (With<Lantern>, Without<FarmerPart>, Without<MaskOverlay>),
    >,
) {
    let night = phase.is_some_and(|p| *p.get() == DayPhase::Night);
    for (part, mut sprite) in &mut parts {
        let color = if night { multiply(part.0, NIGHT_TINT) } else { part.0 };
        if sprite.color != color {
            sprite.color = color;
        }
    }
    // Pontos medidos no farmer_body (canto superior esquerdo do quadro 16x32, y para baixo).
    let bob = if player.step == 0 { 0.0 } else { 1.0 };
    let worn = night && game.is_some_and(|g| g.has_mask) && player.facing == Facing::Down;
    for (mut v, mut transform) in &mut mask {
        v.set_if_neq(if worn { Visibility::Inherited } else { Visibility::Hidden });
        transform.translation = frame_point(2.0, 7.0 + bob).extend(0.001);
    }
    for (mut v, mut transform, mut sprite) in &mut lantern {
        v.set_if_neq(if night && player.facing != Facing::Up {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        // Lampião de 8 px pendurado na mão da frente.
        let x = match player.facing {
            Facing::Right => 5.0,
            Facing::Left => 16.0 - 5.0 - 8.0,
            _ => 9.0,
        };
        transform.translation = frame_point(x, 22.0 + bob).extend(0.002);
        sprite.flip_x = player.facing == Facing::Left;
    }
}

/// Converte um pixel do quadro (origem no canto superior esquerdo) para o espaço do
/// jogador, cuja origem é o meio da borda de baixo do quadro.
fn frame_point(x: f32, y: f32) -> Vec2 {
    Vec2::new(x - 8.0, 32.0 - y)
}

fn multiply(a: Color, b: Color) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    Color::srgba(a.red * b.red, a.green * b.green, a.blue * b.blue, a.alpha)
}

/// Árvores e a casa ficam translúcidas quando o jogador passa por trás.
fn fade_occluders(
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut occluders: Query<(&Occluder, &Transform, &mut Sprite), Without<Player>>,
) {
    let p = player.translation.truncate();
    // A copa some e volta em ~0,15 s em vez de trocar de uma vez.
    let step = time.delta_secs() * 4.0;
    for (occluder, transform, mut sprite) in &mut occluders {
        let base = transform.translation.truncate();
        let behind = p.y > base.y + 2.0
            && p.y < base.y + occluder.height - 6.0
            && (p.x - base.x).abs() < occluder.half_width;
        let target: f32 = if behind { 0.45 } else { 1.0 };
        let alpha = sprite.color.alpha();
        if alpha != target {
            let next = if alpha < target { (alpha + step).min(target) } else { (alpha - step).max(target) };
            sprite.color.set_alpha(next);
        }
    }
}

/// Segue o jogador sem mostrar o vazio fora do mapa.
fn follow_camera(
    time: Res<Time<Real>>,
    map: Option<Res<Map>>,
    window: Single<&Window, With<PrimaryWindow>>,
    player: Single<&Transform, With<Player>>,
    facing: Single<&Player>,
    camera: Single<(&mut Transform, &Projection), (With<Camera2d>, Without<Player>)>,
) {
    let Some(map) = map else { return };
    let (mut camera, projection) = camera.into_inner();
    // Olha um pouco à frente para onde o jogador anda.
    let ahead = match facing.facing {
        Facing::Down => Vec2::new(0.0, -10.0),
        Facing::Up => Vec2::new(0.0, 10.0),
        Facing::Left => Vec2::new(-14.0, 0.0),
        Facing::Right => Vec2::new(14.0, 0.0),
    };
    let target = player.translation.truncate() + Vec2::new(0.0, 12.0) + ahead;
    let current = camera.translation.truncate();
    let next = current.lerp(target, 1.0 - (-5.0 * time.delta_secs()).exp());
    let clamped = clamp_to_map(next, half_view(&window, projection), world_size(map.size));
    camera.translation = clamped.round().extend(camera.translation.z);
}

/// Mantém a câmera dentro de [0, w] x [-h, 0]; se o mapa for menor que a tela, centraliza.
pub fn clamp_to_map(p: Vec2, half: Vec2, size: Vec2) -> Vec2 {
    let axis = |v: f32, half: f32, lo: f32, hi: f32| {
        if hi - lo <= half * 2.0 { (lo + hi) / 2.0 } else { v.clamp(lo + half, hi - half) }
    };
    Vec2::new(axis(p.x, half.x, 0.0, size.x), axis(p.y, half.y, -size.y, 0.0))
}

/// Clique no mundo (não na barra de itens nem nos painéis de baixo).
pub fn world_click(mouse: &ButtonInput<MouseButton>, window: &Window, button: MouseButton) -> bool {
    mouse.just_pressed(button) && window.cursor_position().is_some_and(|c| c.y < window.height() - 130.0 && c.y > 90.0)
}
