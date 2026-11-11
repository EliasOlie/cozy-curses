//! Combate: o "controle de pragas" da noite.
//!
//! Você nunca luta contra os moradores (eles nem sabem o que viram à noite). Quem sai da mata
//! quando escurece são pragas: Lodos que pulam atrás de você, Corvos-de-olho que comem a
//! plantação madura e Espantalhos Errantes que dão um bote telegrafado. Quanto mais maldição na
//! vila, mais pragas. Bater protege o seu patrimônio — e rende essência, fibra e ouro.
//!
//! Golpe: Espaço (ou clique) com a foice ou a espada, em arco à sua frente. Desmaiar leva você
//! para casa e o Prefeito cobra uma "taxa de resgate". Máscaras ajudam: Gárgula reduz o dano,
//! Corvo faz os corvos te ignorarem e Palha faz o mesmo com os espantalhos.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::character::Facing;
use crate::cycle::{Clock, DayPhase};
use crate::dialogue::Dialogue;
use crate::economy::{CropView, FarmState, Game};
use crate::fx::{Pickups, Popups, spawn_burst, spawn_swing};
use crate::inventory::{Inventory, Item, MaskKind};
use crate::map::{Map, MapState, Warp, depth, tile_base, tile_center, world_to_tile};
use crate::player::Player;
use crate::sprites::{Art, Sprites, fx};
use crate::world::Travel;
use crate::{AppState, Gameplay};

pub fn plugin(app: &mut App) {
    app.init_resource::<Combat>()
        .add_systems(
            Update,
            (
                attack.after(crate::economy::interact).before(crate::tools::use_item),
                (spawn_pests, think, hurt_player, animate, faint).chain(),
            )
                .in_set(Gameplay),
        )
        .add_systems(OnEnter(DayPhase::Day), dawn_scatters_pests)
        .add_systems(OnExit(AppState::Playing), |mut c: ResMut<Combat>| *c = Combat::default());
}

/// Estado do combate do jogador.
#[derive(Resource, Default)]
struct Combat {
    /// Segundos até poder golpear de novo.
    cooldown: f32,
    /// Invencível (piscando) depois de apanhar.
    invulnerable: f32,
    /// Empurrão recebido (decai rápido).
    knock: Vec2,
    /// Até a próxima tentativa de nascer uma praga.
    spawn_in: f32,
    /// Semente do sorteio (determinístico, sem depender de crate de rand).
    seed: u32,
}

impl Combat {
    fn rand(&mut self) -> f32 {
        // xorshift: suficiente para sortear pragas e posições.
        let mut x = if self.seed == 0 { 0x9E37_79B9 } else { self.seed };
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x & 0xFFFF) as f32 / 65536.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PestKind {
    Slime,
    Crow,
    Haunt,
}

impl PestKind {
    fn name(self) -> &'static str {
        match self {
            PestKind::Slime => "Lodo",
            PestKind::Crow => "Corvo-de-olho",
            PestKind::Haunt => "Espantalho Errante",
        }
    }

    fn max_hp(self) -> f32 {
        match self {
            PestKind::Slime => 3.0,
            PestKind::Crow => 2.0,
            PestKind::Haunt => 6.0,
        }
    }

    fn damage(self) -> f32 {
        match self {
            PestKind::Slime => 8.0,
            PestKind::Crow => 5.0,
            PestKind::Haunt => 15.0,
        }
    }

    fn speed(self) -> f32 {
        match self {
            PestKind::Slime => 46.0,
            PestKind::Crow => 52.0,
            PestKind::Haunt => 26.0,
        }
    }

    /// Distância em que nota o jogador.
    fn aggro(self) -> f32 {
        match self {
            PestKind::Slime => 90.0,
            PestKind::Crow => 64.0,
            PestKind::Haunt => 110.0,
        }
    }

    /// Altura do corpo (para mirar o golpe e pôr os números em cima).
    fn height(self) -> f32 {
        match self {
            PestKind::Slime | PestKind::Crow => 12.0,
            PestKind::Haunt => 26.0,
        }
    }

    fn art(self, sprites: &Sprites) -> &Art {
        match self {
            PestKind::Slime => &sprites.slime,
            PestKind::Crow => &sprites.crow,
            PestKind::Haunt => &sprites.haunt,
        }
    }

    /// A máscara que faz esta praga te ignorar.
    fn ignored_by(self) -> Option<MaskKind> {
        match self {
            PestKind::Crow => Some(MaskKind::Crow),
            PestKind::Haunt => Some(MaskKind::RusticStraw),
            PestKind::Slime => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mood {
    /// Vagando perto de onde nasceu.
    Idle,
    /// Indo atrás do jogador.
    Chase,
    /// Corvo: indo até uma plantação madura.
    ToCrop(IVec2),
    /// Corvo: bicando a plantação (segundos bicando).
    Peck(IVec2, f32),
    /// Espantalho: braços erguidos antes do bote (direção travada).
    Windup(Vec2),
    /// Espantalho: investida.
    Lunge(Vec2),
    /// Espantalho: recuperando o fôlego depois do bote.
    Recover,
    /// Indo embora (corvo de barriga cheia).
    Leave(Vec2),
}

#[derive(Component)]
pub struct Pest {
    kind: PestKind,
    hp: f32,
    state: Mood,
    /// Segundos no estado atual.
    t: f32,
    /// Relógio próprio da animação.
    anim: f32,
    home: Vec2,
    wander: Vec2,
    knock: Vec2,
    /// Pisca branco ao apanhar.
    flash: f32,
}

/// Teto de pragas ao mesmo tempo: cresce com a maldição.
fn cap(curse: f32) -> usize {
    2 + (curse / 20.0) as usize
}

const REACH: f32 = 24.0;
const SWING_COOLDOWN: f32 = 0.35;
const INVULNERABLE: f32 = 1.0;

fn facing_dir(f: Facing) -> Vec2 {
    match f {
        Facing::Down => Vec2::NEG_Y,
        Facing::Up => Vec2::Y,
        Facing::Left => Vec2::NEG_X,
        Facing::Right => Vec2::X,
    }
}

/// Nasce uma praga de vez em quando, à noite, longe do jogador, em mapas abertos.
#[allow(clippy::too_many_arguments)]
fn spawn_pests(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    phase: Res<State<DayPhase>>,
    map: Res<Map>,
    game: Res<Game>,
    farm: Res<FarmState>,
    mut combat: ResMut<Combat>,
    player: Single<&Transform, With<Player>>,
    pests: Query<(), With<Pest>>,
) {
    if *phase.get() != DayPhase::Night || map.info.indoor {
        return;
    }
    combat.spawn_in -= time.delta_secs();
    if combat.spawn_in > 0.0 {
        return;
    }
    combat.spawn_in = 7.0 + combat.rand() * 6.0;
    if pests.iter().len() >= cap(game.curse) {
        return;
    }
    // Corvos aparecem quando há o que comer; espantalhos quando a vila já está bem amaldiçoada.
    let ripe = map.info.farm && farm.crops.iter().any(|(_, c)| c.days >= c.kind.growth_days());
    let roll = combat.rand();
    let kind = if ripe && roll < 0.4 {
        PestKind::Crow
    } else if game.curse >= 30.0 && roll > 0.7 {
        PestKind::Haunt
    } else {
        PestKind::Slime
    };
    // Um lugar andável a uma distância boa (fora da tela, de preferência).
    let you = player.translation.truncate();
    for _ in 0..24 {
        let angle = combat.rand() * std::f32::consts::TAU;
        let dist = 150.0 + combat.rand() * 90.0;
        let p = you + Vec2::new(angle.cos(), angle.sin()) * dist;
        let t = world_to_tile(p);
        if !map.walkable(t) || map.solid(Rect::from_center_size(tile_center(t), Vec2::splat(6.0))) {
            continue;
        }
        let at = tile_base(t) + Vec2::Y * 2.0;
        commands
            .spawn((
                Pest {
                    kind,
                    hp: kind.max_hp(),
                    state: Mood::Idle,
                    t: 0.0,
                    anim: combat.rand() * 3.0,
                    home: at,
                    wander: at,
                    knock: Vec2::ZERO,
                    flash: 0.0,
                },
                kind.art(&sprites).sprite(DayPhase::Night, 0),
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(at.extend(depth(at.y))),
                DespawnOnExit(MapState(map.id.clone())),
            ))
            .with_child((
                crate::sprites::DualSprite::bundle(&sprites.shadow, 0),
                Transform::from_xyz(0.0, 0.5, -0.01).with_scale(Vec3::splat(if kind == PestKind::Haunt { 0.9 } else { 0.6 })),
            ));
        spawn_burst(&mut commands, &sprites, &map.id, at + Vec2::Y * 6.0, fx::SMOKE, 3);
        break;
    }
}

/// Anda até `to` respeitando as paredes (o corvo voa e ignora).
fn step_towards(map: &Map, here: Vec2, to: Vec2, speed: f32, dt: f32, flies: bool) -> Vec2 {
    let d = to - here;
    if d.length() < 1.0 {
        return here;
    }
    let next = here + d.normalize() * (speed * dt).min(d.length());
    if flies || !map.solid(Rect::new(next.x - 3.0, next.y, next.x + 3.0, next.y + 3.0)) { next } else { here }
}

/// A cabeça de cada praga: escolhe o que fazer e se move.
#[allow(clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    time: Res<Time>,
    sprites: Res<Sprites>,
    map: Res<Map>,
    game: Res<Game>,
    mut farm: ResMut<FarmState>,
    mut popups: ResMut<Popups>,
    mut combat: ResMut<Combat>,
    player: Single<&Transform, (With<Player>, Without<Pest>)>,
    crop_views: Query<(Entity, &CropView)>,
    mut pests: Query<(Entity, &mut Pest, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    let you = player.translation.truncate();
    for (entity, mut pest, mut transform, mut sprite) in &mut pests {
        let mut here = transform.translation.truncate();
        pest.t += dt;
        pest.anim += dt;
        pest.flash = (pest.flash - dt).max(0.0);
        // Empurrão do golpe: desliza e para.
        if pest.knock.length() > 1.0 {
            let next = here + pest.knock * dt;
            if !map.solid(Rect::new(next.x - 3.0, next.y, next.x + 3.0, next.y + 3.0)) {
                here = next;
            }
            pest.knock *= (-10.0 * dt).exp();
        }
        let kind = pest.kind;
        let ignores_you = kind.ignored_by().is_some_and(|m| game.equipped_mask == Some(m));
        let near_you = !ignores_you && here.distance(you) < kind.aggro();
        let speed = kind.speed();
        let mut facing_x = None;

        match pest.state {
            Mood::Idle | Mood::Chase => {
                // Corvo prefere comida (se você não estiver em cima).
                let crop = (kind == PestKind::Crow && !near_you)
                    .then(|| {
                        farm.crops
                            .iter()
                            .filter(|(_, c)| c.days >= c.kind.growth_days())
                            .map(|(t, _)| *t)
                            .min_by(|a, b| tile_center(*a).distance(here).total_cmp(&tile_center(*b).distance(here)))
                    })
                    .flatten()
                    .filter(|_| map.info.farm);
                if let Some(t) = crop {
                    pest.state = Mood::ToCrop(t);
                    pest.t = 0.0;
                } else if near_you {
                    pest.state = Mood::Chase;
                    if kind == PestKind::Haunt && here.distance(you) < 44.0 {
                        pest.state = Mood::Windup((you - here).normalize_or(Vec2::X));
                        pest.t = 0.0;
                    } else if kind == PestKind::Slime {
                        // Pula em ciclos: parado, agacha, voa.
                        let cycle = pest.anim % 1.2;
                        if cycle > 0.7 {
                            here = step_towards(&map, here, you, speed * 1.6, dt, false);
                        }
                    } else {
                        here = step_towards(&map, here, you, speed, dt, kind == PestKind::Crow);
                    }
                    facing_x = Some(you.x - here.x);
                } else {
                    pest.state = Mood::Idle;
                    if here.distance(pest.wander) < 2.0 || pest.t > 4.0 {
                        let (a, r) = (combat.rand() * std::f32::consts::TAU, combat.rand() * 28.0);
                        pest.wander = pest.home + Vec2::new(a.cos(), a.sin() * 0.6) * r;
                        pest.t = 0.0;
                    }
                    let wander = pest.wander;
                    // Vagando: metade da velocidade; o lodo só anda no pulo.
                    if kind != PestKind::Slime || pest.anim % 1.6 > 1.1 {
                        here = step_towards(&map, here, wander, speed * 0.5, dt, kind == PestKind::Crow);
                    }
                    facing_x = Some(wander.x - here.x);
                }
            }
            Mood::ToCrop(t) => {
                let target = tile_base(t) + Vec2::Y * 3.0;
                if !farm.crops.contains_key(&t) {
                    pest.state = Mood::Idle;
                } else if near_you {
                    pest.state = Mood::Chase;
                } else if here.distance(target) < 2.0 {
                    pest.state = Mood::Peck(t, 0.0);
                } else {
                    here = step_towards(&map, here, target, speed, dt, true);
                    facing_x = Some(target.x - here.x);
                }
            }
            Mood::Peck(t, pecked) => {
                if near_you {
                    pest.state = Mood::Chase;
                } else if !farm.crops.contains_key(&t) {
                    pest.state = Mood::Idle;
                } else if pecked + dt >= 4.0 {
                    // Comeu: some a plantação e o corvo vai embora satisfeito.
                    if let Some(crop) = farm.crops.remove(&t) {
                        for (e, view) in &crop_views {
                            if view.0 == t {
                                commands.entity(e).despawn();
                            }
                        }
                        let name = crop.kind.name().to_lowercase();
                        popups.push(tile_center(t) + Vec2::Y * 12.0, format!("-1 {name} (corvo)"), Color::srgb(1.0, 0.45, 0.4));
                        spawn_burst(&mut commands, &sprites, &map.id, tile_center(t), fx::LEAF, 4);
                    }
                    let away = (here - you).normalize_or(Vec2::X);
                    pest.state = Mood::Leave(away);
                    pest.t = 0.0;
                } else {
                    pest.state = Mood::Peck(t, pecked + dt);
                }
            }
            Mood::Windup(dir) => {
                facing_x = Some(dir.x);
                if pest.t > 0.6 {
                    pest.state = Mood::Lunge(dir);
                    pest.t = 0.0;
                }
            }
            Mood::Lunge(dir) => {
                here = step_towards(&map, here, here + dir * 40.0, 120.0, dt, false);
                facing_x = Some(dir.x);
                if pest.t > 0.45 {
                    pest.state = Mood::Recover;
                    pest.t = 0.0;
                }
            }
            Mood::Recover => {
                if pest.t > 0.8 {
                    pest.state = Mood::Idle;
                    pest.t = 0.0;
                }
            }
            Mood::Leave(dir) => {
                here += dir * 70.0 * dt;
                facing_x = Some(dir.x);
                if pest.t > 3.0 {
                    commands.entity(entity).despawn();
                    continue;
                }
            }
        }
        if let Some(x) = facing_x.filter(|x| x.abs() > 0.5) {
            sprite.flip_x = x < 0.0;
        }
        // O corvo voando fica por cima de tudo.
        let flying = kind == PestKind::Crow && !matches!(pest.state, Mood::Peck(..));
        let z = if flying { depth(here.y - 40.0) } else { depth(here.y) };
        transform.translation = here.extend(z);
    }
}

/// Encostou numa praga (fora da invencibilidade): perde vida e é empurrado.
#[allow(clippy::too_many_arguments)]
fn hurt_player(
    time: Res<Time>,
    map: Res<Map>,
    mut game: ResMut<Game>,
    mut combat: ResMut<Combat>,
    mut popups: ResMut<Popups>,
    dialogue: Res<Dialogue>,
    travel: Res<Travel>,
    player: Single<(&mut Transform, &mut Visibility), (With<Player>, Without<Pest>)>,
    pests: Query<(&Pest, &Transform)>,
) {
    let dt = time.delta_secs();
    let (mut transform, mut visibility) = player.into_inner();
    let you = transform.translation.truncate();
    combat.invulnerable = (combat.invulnerable - dt).max(0.0);
    combat.cooldown = (combat.cooldown - dt).max(0.0);

    // Empurrão: desliza, respeitando paredes.
    if combat.knock.length() > 1.0 {
        let next = you + combat.knock * dt;
        if !map.solid(Rect::new(next.x - 4.0, next.y, next.x + 4.0, next.y + 4.0)) {
            transform.translation.x = next.x;
            transform.translation.y = next.y;
            transform.translation.z = depth(next.y) + 0.0005;
        }
        combat.knock *= (-12.0 * dt).exp();
    }
    // Pisca enquanto está invencível.
    let blink = combat.invulnerable > 0.0 && (combat.invulnerable * 14.0) as u32 % 2 == 0;
    visibility.set_if_neq(if blink { Visibility::Hidden } else { Visibility::Inherited });

    if combat.invulnerable > 0.0 || dialogue.is_open() || travel.busy() {
        return;
    }
    for (pest, t) in &pests {
        let p = t.translation.truncate();
        let ignores_you = pest.kind.ignored_by().is_some_and(|m| game.equipped_mask == Some(m));
        // O corvo bicando e quem está indo embora não machucam; o espantalho só no bote.
        let harmless = matches!(pest.state, Mood::Peck(..) | Mood::Leave(_) | Mood::Windup(_) | Mood::Recover)
            || (pest.kind == PestKind::Haunt && !matches!(pest.state, Mood::Lunge(_) | Mood::Chase));
        if ignores_you || harmless || p.distance(you + Vec2::Y * 4.0) > 10.0 {
            continue;
        }
        let mut dmg = pest.kind.damage();
        if game.equipped_mask == Some(MaskKind::Gargoyle) {
            dmg *= 0.5;
        }
        game.health = (game.health - dmg).max(0.0);
        combat.invulnerable = INVULNERABLE;
        combat.knock = (you - p).normalize_or(Vec2::NEG_Y) * 150.0;
        popups.push(you + Vec2::Y * 34.0, format!("-{dmg:.0} vida"), Color::srgb(1.0, 0.4, 0.4));
        break;
    }
}

/// Espaço/clique com arma: golpe em arco. Se acertar (ou se for a espada), o golpe "come" a
/// tecla para a foice não tentar cortar mato no mesmo quadro.
#[allow(clippy::too_many_arguments)]
fn attack(
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    (mut mouse, window): (ResMut<ButtonInput<MouseButton>>, Single<&Window, With<bevy::window::PrimaryWindow>>),
    sprites: Res<Sprites>,
    map: Res<Map>,
    (dialogue, travel): (Res<Dialogue>, Res<Travel>),
    cutscenes: Option<Res<crate::cutscene::CutsceneManager>>,
    (mut game, mut combat, mut popups, mut pickups): (ResMut<Game>, ResMut<Combat>, ResMut<Popups>, ResMut<Pickups>),
    mut inventory: ResMut<Inventory>,
    player: Single<(&Transform, &Player), Without<Pest>>,
    mut pests: Query<(Entity, &mut Pest, &Transform)>,
) {
    let Some(item) = inventory.selected_item() else { return };
    let Some(damage) = item.weapon_damage() else { return };
    if dialogue.is_open() || travel.busy() || cutscenes.is_some_and(|c| c.is_playing()) {
        return;
    }
    let pressed = keys.just_pressed(KeyCode::Space) || crate::player::world_click(&mouse, &window, MouseButton::Left);
    if !pressed || combat.cooldown > 0.0 {
        return;
    }
    let (transform, player) = *player;
    let you = transform.translation.truncate();
    let body = you + Vec2::Y * 10.0;
    let dir = facing_dir(player.facing);

    // Com praga por perto, a foice vira arma mesmo errando (nada de "Nada para cortar").
    let in_fight = pests.iter().any(|(_, _, t)| t.translation.truncate().distance(you) < 64.0);
    let mut hit_any = false;
    for (entity, mut pest, t) in &mut pests {
        let center = t.translation.truncate() + Vec2::Y * pest.kind.height() * 0.5;
        let to = center - body;
        // Arco à frente: perto e mais ou menos na direção para onde você olha.
        if to.length() > REACH || to.normalize_or(dir).dot(dir) < 0.15 {
            continue;
        }
        hit_any = true;
        pest.hp -= damage;
        pest.flash = 0.12;
        pest.knock = to.normalize_or(dir) * 160.0;
        // Apanhar interrompe o bote e espanta o corvo do prato.
        if matches!(pest.state, Mood::Windup(_) | Mood::Peck(..) | Mood::ToCrop(_)) {
            pest.state = Mood::Chase;
            pest.t = 0.0;
        }
        popups.push(center + Vec2::Y * 10.0, format!("-{damage:.0}"), Color::srgb(1.0, 0.95, 0.8));
        if pest.hp <= 0.0 {
            commands.entity(entity).despawn();
            spawn_burst(&mut commands, &sprites, &map.id, center, fx::SMOKE, 5);
            loot(pest.kind, center, &mut game, &mut inventory, &mut popups, &mut pickups, &mut combat);
        }
    }
    if !hit_any && !in_fight && item != Item::Sword {
        // Foice no vazio: deixa a ferramenta fazer o trabalho dela (cortar mato, colher).
        return;
    }
    keys.clear_just_pressed(KeyCode::Space);
    mouse.clear_just_pressed(MouseButton::Left);
    combat.cooldown = SWING_COOLDOWN;
    if hit_any {
        game.spend(0.5);
    }
    let side = if player.facing == Facing::Left { -1.0 } else { 1.0 };
    let hand = you + Vec2::new(7.0 * side, 14.0) + dir * 4.0;
    spawn_swing(&mut commands, &sprites, &map.id, hand, item.icon(), side < 0.0);
}

/// O que cada praga deixa: essência (lodo), fibra (corvo), ouro e palha (espantalho).
fn loot(
    kind: PestKind,
    at: Vec2,
    game: &mut Game,
    inv: &mut Inventory,
    popups: &mut Popups,
    pickups: &mut Pickups,
    combat: &mut Combat,
) {
    match kind {
        PestKind::Slime => {
            let n = 2.0 + (combat.rand() * 2.0).floor();
            game.essence += n;
            popups.push(at + Vec2::Y * 18.0, format!("+{n:.0} essência"), Popups::ESSENCE);
        }
        PestKind::Crow => {
            inv.add(Item::Fiber, 1);
            popups.push(at + Vec2::Y * 18.0, "+1 fibra", Popups::GOOD);
            pickups.0.push((at, Item::Fiber.icon()));
        }
        PestKind::Haunt => {
            let gold = 6 + (combat.rand() * 5.0) as i32;
            game.gold += gold;
            inv.add(Item::Fiber, 2);
            popups.push(at + Vec2::Y * 18.0, format!("+{gold} ouro · +2 fibra"), Popups::GOLD);
            pickups.0.push((at, Item::Fiber.icon()));
        }
    }
    game.say(format!("{} abatido. Controle de pragas é um serviço, e serviço se cobra.", kind.name()));
}

/// Quadros: lodo (0-1 parado, 2 agacha, 3 no ar), corvo (0-1 voando, 2-3 bicando),
/// espantalho (0-1 andando, 2 preparando, 3 investida). Pisca branco ao apanhar.
fn animate(mut pests: Query<(&Pest, &mut Sprite)>) {
    for (pest, mut sprite) in &mut pests {
        let a = pest.anim;
        let frame = match pest.kind {
            PestKind::Slime => {
                let cycle = a % if pest.state == Mood::Chase { 1.2 } else { 1.6 };
                let hop_at = if pest.state == Mood::Chase { 0.7 } else { 1.1 };
                if cycle > hop_at {
                    3
                } else if cycle > hop_at - 0.2 {
                    2
                } else {
                    (a * 3.0) as usize % 2
                }
            }
            PestKind::Crow => match pest.state {
                Mood::Peck(..) => 2 + (a * 5.0) as usize % 2,
                _ => (a * 10.0) as usize % 2,
            },
            PestKind::Haunt => match pest.state {
                Mood::Windup(_) => 2,
                Mood::Lunge(_) => 3,
                Mood::Recover => 0,
                _ => (a * 4.0) as usize % 2,
            },
        };
        if let Some(atlas) = &mut sprite.texture_atlas
            && atlas.index != frame
        {
            atlas.index = frame;
        }
        // Branco estourado por um instante quando apanha.
        sprite.color = if pest.flash > 0.0 { Color::srgb(4.0, 4.0, 4.0) } else { Color::WHITE };
    }
}

/// Vida zerada: o jogador desmaia, acorda em casa e o Prefeito cobra o resgate.
#[allow(clippy::too_many_arguments)]
fn faint(
    mut game: ResMut<Game>,
    mut clock: ResMut<Clock>,
    mut travel: ResMut<Travel>,
    mut combat: ResMut<Combat>,
    mut dialogue: ResMut<Dialogue>,
    map: Res<Map>,
) {
    if game.health > 0.0 || travel.busy() {
        return;
    }
    let fee = (game.gold / 10).max(10).min(game.gold.max(0));
    game.gold -= fee;
    game.health = 30.0;
    game.energy = game.energy.min(30.0);
    combat.invulnerable = 2.0;
    combat.knock = Vec2::ZERO;
    clock.skip_to_phase_end();
    let world = crate::worlddef::world();
    let home = world.start.clone();
    let arrive = crate::map::Layout::of(&home).find('@').unwrap_or(IVec2::new(4, 4));
    if map.id != home {
        travel.start(Warp { to: home, arrive, facing: Facing::Down });
    }
    game.say(format!("Você desmaiou. Acordou em casa, sem {fee} de ouro."));
    dialogue.open(
        "Prefeito Anselmo",
        &format!(
            "Encontramos você caído no mato, coitadinho! A vila te trouxe para casa. \
             A taxa de resgate ficou em {fee} de ouro — burocracia, entende? Descanse!"
        ),
    );
}

/// Ao amanhecer as pragas viram fumaça (de dia a vila é só fachada cozy).
fn dawn_scatters_pests(
    mut commands: Commands,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    pests: Query<(Entity, &Transform), With<Pest>>,
) {
    let Some(map) = map else { return };
    for (entity, t) in &pests {
        spawn_burst(&mut commands, &sprites, &map.id, t.translation.truncate() + Vec2::Y * 6.0, fx::SMOKE, 3);
        commands.entity(entity).despawn();
    }
}
