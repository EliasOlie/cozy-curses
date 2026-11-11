//! Viagem entre mapas: portas, capachos e bordas, com um fade curto no meio.

use bevy::prelude::*;

use crate::AppState;
use crate::bake::Painter;
use crate::cycle::Clock;
use crate::economy::FarmState;
use crate::map::{Layout, Map, MapEdits, MapState, Warp, build, depth, tile_base};
use crate::player::Player;
use crate::sprites::Sprites;
use crate::worlddef;

pub fn plugin(app: &mut App) {
    app.init_resource::<Travel>()
        .add_systems(OnEnter(AppState::Playing), go_to_start)
        .add_systems(Update, travel.run_if(in_state(AppState::Playing)))
        .add_systems(Update, enter_map.in_set(MapEntered).run_if(state_changed::<MapState>))
        .configure_sets(Update, MapEntered.run_if(in_state(AppState::Playing)))
        .add_systems(OnExit(AppState::Playing), |mut t: ResMut<Travel>| *t = Travel::default());
}

/// Sistemas que montam o que mora no mapa recém-aberto (rodam depois de `enter_map`).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapEntered;

/// Metade do fade (escurece, troca o mapa, clareia).
const HALF: f32 = 0.22;

#[derive(Resource, Default)]
pub struct Travel {
    pending: Option<Warp>,
    t: f32,
    switched: bool,
    /// Onde o jogador aparece no próximo mapa (sem isso: no `@` do mapa).
    arrival: Option<Warp>,
}

impl Travel {
    pub fn start(&mut self, warp: Warp) {
        if self.pending.is_none() {
            *self = Travel { pending: Some(warp), ..default() };
        }
    }

    /// Viajando: o jogador não anda nem interage.
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Opacidade do véu preto da transição.
    pub fn veil(&self) -> f32 {
        if self.pending.is_none() {
            return 0.0;
        }
        (1.0 - (self.t - HALF).abs() / HALF).clamp(0.0, 1.0)
    }
}

/// O jogo (novo ou carregado) começa no mapa inicial do world.json.
fn go_to_start(mut next: ResMut<NextState<MapState>>) {
    next.set(MapState(worlddef::world().start));
}

fn travel(time: Res<Time<Real>>, mut travel: ResMut<Travel>, mut next: ResMut<NextState<MapState>>) {
    let Some(warp) = travel.pending.clone() else { return };
    travel.t += time.delta_secs();
    if !travel.switched && travel.t >= HALF {
        travel.switched = true;
        next.set(MapState(warp.to.clone()));
        travel.arrival = Some(warp);
    }
    if travel.t >= HALF * 2.0 {
        travel.pending = None;
    }
}

/// Monta o mapa novo e põe o jogador (e a câmera) no lugar de chegada.
#[allow(clippy::too_many_arguments)]
pub fn enter_map(
    mut commands: Commands,
    sprites: Res<Sprites>,
    state: Res<State<MapState>>,
    clock: Res<Clock>,
    farm: Res<FarmState>,
    mut travel: ResMut<Travel>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    player: Single<(&mut Transform, &mut Player)>,
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let id = state.get().0.clone();
    if id.is_empty() {
        return;
    }
    let info = worlddef::info(&id);
    let edits = if info.farm { farm.edits.clone() } else { MapEdits::default() };
    let mut painter = Painter { images: &mut images, layouts: &layouts };
    let map = build(&mut commands, &sprites, &mut painter, &info, &Layout::of(&id), DespawnOnExit(state.get().clone()), &edits, clock.season());

    let (tile, facing) = match travel.arrival.take() {
        Some(warp) => (warp.arrive, warp.facing),
        None => (map.spawn.unwrap_or(map.size / 2), crate::character::Facing::Down),
    };
    let p = tile_base(tile) + Vec2::new(0.0, 2.0);
    let (mut transform, mut player) = player.into_inner();
    transform.translation = p.extend(depth(p.y));
    player.facing = facing;
    camera.translation = p.extend(camera.translation.z);
    commands.insert_resource(map);
}

/// Verifica se o jogador pisou numa porta/saída.
pub fn check_warp(map: &Map, travel: &mut Travel, feet_tile: IVec2) {
    if let Some(warp) = map.warps.get(&feet_tile) {
        travel.start(warp.clone());
    }
}
