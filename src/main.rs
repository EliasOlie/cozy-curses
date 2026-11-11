//! Cozy Curses — farming sim de dia, império B2B amaldiçoado de noite.
//! Fonte de verdade do design: docs/GDD.md

mod animals;
mod bake;
mod character;
mod combat;
mod crafting;
mod cutscene;
mod cycle;
mod dialogue;
mod economy;
mod editor;
mod fx;
mod hud;
mod inventory;
mod lighting;
mod loading;
mod map;
mod menu;
mod pause;
mod phone;
mod player;
mod save;
mod shop;
mod sprites;
mod terrain;
mod tips;
mod tools;
mod ui;
mod villagers;
mod world;
mod worlddef;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;

/// Carregando, menu principal, jogo em andamento ou editor de mapas.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Menu,
    Playing,
    Editor,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum Pause {
    #[default]
    Running,
    Paused,
}

/// Janela aberta por cima do jogo. Com ela aberta o tempo para (como no Stardew).
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Overlay {
    #[default]
    None,
    Shop(shop::Shop),
    Phone,
    Bag,
}

/// Sistemas da simulação: só rodam com o jogo aberto, sem pausa e sem janela por cima.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Gameplay;

fn no_overlay(overlay: Res<Overlay>) -> bool {
    *overlay == Overlay::None
}

/// Quadro acima de 35 ms (menos de ~28 fps naquele instante) vira uma linha no log,
/// com o estado do jogo e quantas entidades existem, para achar o padrão dos engasgos.
fn log_spikes(
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    entities: Query<()>,
    mut frame: Local<u64>,
) {
    *frame += 1;
    let ms = time.delta_secs() * 1000.0;
    // Os primeiros quadros (carregamento) são lentos mesmo.
    if *frame > 120 && ms > 35.0 && std::env::var_os("COZY_PROFILE").is_none() {
        warn!("engasgo: {ms:.0} ms · {:?} · {} entidades · t={:.1}s", state.get(), entities.iter().len(), time.elapsed_secs());
    }
}

/// Perfil simples (COZY_PROFILE=1): a cada segundo, a média de ms por quadro e quanto
/// disso ficou em cada trecho do laço principal. O que sobra é render/espera da GPU.
mod profile {
    use bevy::app::MainScheduleOrder;
    use bevy::ecs::schedule::ScheduleLabel;
    use bevy::prelude::*;
    use std::time::Instant;

    #[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
    struct Mark(u8);

    #[derive(Resource)]
    struct Prof {
        last: Instant,
        sums: [f64; 5],
        frames: u32,
        worst: f64,
        frame_start: Instant,
        since: Instant,
    }

    const NAMES: [&str; 5] = ["pre", "update", "post", "last", "fora"];

    pub fn plugin(app: &mut App) {
        if std::env::var_os("COZY_PROFILE").is_none() {
            return;
        }
        let now = Instant::now();
        app.insert_resource(Prof { last: now, sums: [0.0; 5], frames: 0, worst: 0.0, frame_start: now, since: now });
        for i in 0..5u8 {
            app.init_schedule(Mark(i));
            app.add_systems(Mark(i), move |mut p: ResMut<Prof>, entities: Query<()>| {
                let now = Instant::now();
                let ms = now.duration_since(p.last).as_secs_f64() * 1000.0;
                p.last = now;
                // Mark(0) roda antes de First: o intervalo até ele é o que ficou fora do laço.
                let slot = if i == 0 { 4 } else { i as usize - 1 };
                p.sums[slot] += ms;
                if i == 0 {
                    let frame = now.duration_since(p.frame_start).as_secs_f64() * 1000.0;
                    p.frame_start = now;
                    p.worst = p.worst.max(frame);
                    p.frames += 1;
                    if now.duration_since(p.since).as_secs_f64() >= 1.0 {
                        let n = p.frames as f64;
                        let parts: Vec<String> =
                            (0..5).map(|k| format!("{} {:.1}", NAMES[k], p.sums[k] / n)).collect();
                        let total: f64 = p.sums.iter().sum::<f64>() / n;
                        info!(
                            "perfil: {:.1} ms/quadro ({:.0} fps, pior {:.0}) · {} · {} entidades",
                            total,
                            1000.0 / total,
                            p.worst,
                            parts.join(" · "),
                            entities.iter().len()
                        );
                        p.sums = [0.0; 5];
                        p.frames = 0;
                        p.worst = 0.0;
                        p.since = now;
                    }
                }
            });
        }
        let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
        order.insert_before(First, Mark(0));
        order.insert_after(PreUpdate, Mark(1));
        order.insert_after(Update, Mark(2));
        order.insert_after(PostUpdate, Mark(3));
        order.insert_after(Last, Mark(4));
    }
}

fn main() {
    // No WSL a GPU chega pelo Vulkan "Dozen" (sobre D3D12), que o wgpu esconde por não ser
    // 100% conforme. O Bevy 2D funciona bem nele; sem isso o jogo nem abre ("Unable to find a GPU").
    #[cfg(target_os = "linux")]
    if std::env::var_os("WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER").is_none() {
        // SAFETY: primeira coisa do programa, antes de qualquer thread existir.
        unsafe { std::env::set_var("WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER", "1") };
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cozy Curses".into(),
                        // COZY_NOVSYNC=1: sem vsync, para medir o custo real de um quadro.
                        present_mode: if std::env::var_os("COZY_NOVSYNC").is_some() {
                            bevy::window::PresentMode::AutoNoVsync
                        } else {
                            bevy::window::PresentMode::AutoVsync
                        },
                        // wasm: o canvas ocupa a página e as setas não rolam a página
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                // pixel art: amostragem sem interpolação
                .set(ImagePlugin::default_nearest())
                // wasm: não procurar arquivos .meta (evita 404 para cada asset)
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .init_state::<AppState>()
        .add_sub_state::<Pause>()
        .init_resource::<Overlay>()
        // O mapa é montado um quadro depois de entrar no jogo (world::enter_map).
        .configure_sets(
            Update,
            Gameplay.run_if(in_state(Pause::Running)).run_if(no_overlay).run_if(resource_exists::<map::Map>),
        )
        .add_plugins((
            sprites::plugin,
            cycle::plugin,
            map::plugin,
            world::plugin,
            player::plugin,
            economy::plugin,
            villagers::plugin,
            dialogue::plugin,
            fx::plugin,
            lighting::plugin,
            ui::plugin,
            menu::plugin,
            pause::plugin,
        ))
        .add_plugins((
            hud::plugin,
            inventory::plugin,
            tools::plugin,
            shop::plugin,
            phone::plugin,
            editor::plugin,
            loading::plugin,
            animals::plugin,
            tips::plugin,
            combat::plugin,
            crafting::plugin,
            cutscene::plugin,
        ))
        .add_plugins(profile::plugin)
        // Dev: anota quadros lentos no terminal (para caçar engasgos).
        .add_systems(Last, log_spikes.run_if(|| cfg!(debug_assertions)))
        // Mapas editados no editor valem no lugar dos originais.
        .add_systems(Startup, save::load_map_overrides)
        // Recursos e jogador primeiro; o mapa (sub-estado MapState) é montado logo depois,
        // em world::enter_map, já com tudo isso aplicado.
        .add_systems(OnEnter(AppState::Playing), (economy::restore, player::spawn_player, hud::spawn_hud).chain())
        .add_systems(OnExit(AppState::Playing), (economy::cleanup, |mut o: ResMut<Overlay>| *o = Overlay::None))
        .run();
}
