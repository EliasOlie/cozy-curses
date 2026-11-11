//! Ciclo dia/noite: a ilusão (Ghibli) e a verdade (Over the Garden Wall).
//!
//! O relógio anda em minutos de jogo (06:00 -> 06:00 do dia seguinte). Além do tempo
//! correndo, ele pode saltar: atividades "marco" (almoçar, cochilar, dormir) pulam para o
//! próximo período, e o celular adianta 10 minutos a cada rolada do feed.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};

use crate::world::Travel;
use crate::{AppState, Gameplay};

pub fn plugin(app: &mut App) {
    app.add_sub_state::<DayPhase>()
        .insert_resource(ClearColor(DAY_SKY))
        .init_resource::<PhaseFade>()
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, tick.in_set(Gameplay))
        .add_systems(Update, sync_phase.run_if(in_state(AppState::Playing)))
        .add_systems(Update, (fade, pixel_zoom.run_if(not(in_state(AppState::Editor)))));

    #[cfg(debug_assertions)]
    app.add_systems(Update, skip_milestone.in_set(Gameplay));
}

/// As quatro estações do ano. Cada uma dura 7 dias de jogo (28 dias = 1 ano).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}

pub const DAYS_PER_SEASON: u32 = 7;

impl Season {
    pub fn from_day(day: u32) -> Self {
        let idx = ((day.saturating_sub(1)) / DAYS_PER_SEASON) % 4;
        match idx {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Season::Spring => "Primavera",
            Season::Summer => "Verão",
            Season::Autumn => "Outono",
            Season::Winter => "Inverno",
        }
    }

    pub fn greeting(&self) -> &'static str {
        match self {
            Season::Spring => "Chegou a Primavera! A brisa suave desperta as flores e a vida na vila.",
            Season::Summer => "Chegou o Verão! O sol radiante aquece os campos e o lago.",
            Season::Autumn => "Chegou o Outono! Folhas douradas caem e a floresta se tinge de âmbar.",
            Season::Winter => "Chegou o Inverno! Uma geada fina prateia o solo e o vento sopra frio.",
        }
    }

    pub fn day_sky(&self) -> Color {
        match self {
            Season::Spring => Color::srgb(0.25, 0.46, 0.22),
            Season::Summer => Color::srgb(0.22, 0.48, 0.26),
            Season::Autumn => Color::srgb(0.35, 0.38, 0.18),
            Season::Winter => Color::srgb(0.28, 0.35, 0.40),
        }
    }

    #[allow(dead_code)]
    pub fn next(&self) -> Self {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }
}

/// Só existe durante o jogo. Saves são gravados ao amanhecer, então todo jogo
/// (novo ou carregado) começa de dia.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum DayPhase {
    #[default]
    Day,
    Night,
}

/// Minutos de jogo por segundo real: 12 h de jogo = 10 min reais.
const MINUTES_PER_SECOND: f32 = 1.2;
/// 06:00 -> 18:00 é dia; 18:00 -> 06:00 é noite.
pub const HALF_DAY: f32 = 720.0;
pub const FULL_DAY: f32 = 1440.0;
/// Cada marco dura 4 horas de jogo.
pub const MILESTONE: f32 = 240.0;
const FADE: f32 = 1.2;
pub const DAY_SKY: Color = Color::srgb(0.27, 0.45, 0.2);
pub const NIGHT_SKY: Color = Color::srgb(0.07, 0.05, 0.14);
/// Altura aproximada, em pixels do jogo, da área visível (o zoom é sempre inteiro).
const VIEW_HEIGHT: f32 = 270.0;

const MILESTONES: [&str; 6] = ["Manhã", "Meio-dia", "Tarde", "Anoitecer", "Madrugada", "Alvorada"];

#[derive(Resource)]
pub struct Clock {
    /// Minutos desde as 06:00 do dia atual (0..1440).
    pub minutes: f32,
    pub day: u32,
}

impl Clock {
    pub fn new(day: u32) -> Self {
        Clock { minutes: 0.0, day }
    }

    pub fn phase(&self) -> DayPhase {
        if self.minutes < HALF_DAY { DayPhase::Day } else { DayPhase::Night }
    }

    /// 0..1 dentro da fase atual (para a cor do céu).
    pub fn phase_progress(&self) -> f32 {
        (self.minutes % HALF_DAY) / HALF_DAY
    }

    pub fn hour_label(&self) -> String {
        let total = (6.0 * 60.0 + self.minutes) as u32 % (24 * 60);
        format!("{:02}:{:02}", total / 60, (total % 60) / 10 * 10)
    }

    /// Nome do marco atual (Manhã, Tarde...).
    pub fn milestone(&self) -> &'static str {
        MILESTONES[((self.minutes / MILESTONE) as usize).min(MILESTONES.len() - 1)]
    }


    /// Avança `minutes` (doomscroll, atividades). A virada de fase é tratada em `sync_phase`.
    pub fn advance(&mut self, minutes: f32) {
        self.minutes = (self.minutes + minutes).min(FULL_DAY);
    }

    /// Pula para o começo do próximo marco.
    pub fn skip_to_next_milestone(&mut self) {
        self.minutes = ((self.minutes / MILESTONE).floor() + 1.0) * MILESTONE;
    }

    /// Pula direto para o anoitecer (cochilo) ou para o amanhecer (sono).
    pub fn skip_to_phase_end(&mut self) {
        self.minutes = if self.minutes < HALF_DAY { HALF_DAY } else { FULL_DAY };
    }

    /// Estação atual correspondente ao dia do jogo.
    pub fn season(&self) -> Season {
        Season::from_day(self.day)
    }

    /// Dia dentro da estação atual (1..=7).
    pub fn day_of_season(&self) -> u32 {
        ((self.day.saturating_sub(1)) % DAYS_PER_SEASON) + 1
    }

    /// Ano atual da fazenda (ano 1 a cada 28 dias).
    #[allow(dead_code)]
    pub fn year(&self) -> u32 {
        ((self.day.saturating_sub(1)) / (DAYS_PER_SEASON * 4)) + 1
    }

    /// Pula para o primeiro dia da próxima estação (ao amanhecer).
    pub fn skip_to_next_season(&mut self) {
        let current_season_idx = ((self.day.saturating_sub(1)) / DAYS_PER_SEASON) as u32;
        self.day = (current_season_idx + 1) * DAYS_PER_SEASON + 1;
        self.minutes = 0.0;
    }
}

/// Véu escuro preso à câmera, usado nas transições.
#[derive(Component)]
struct FadeVeil;

/// Segundos restantes do clarear depois de uma virada de fase.
#[derive(Resource, Default)]
pub struct PhaseFade(f32);

fn spawn_camera(mut commands: Commands) {
    commands
        .spawn((
            Camera2d,
            // Pixel art não usa anti-aliasing; o MSAA 4x (padrão do Bevy) quadruplica o custo
            // de cada camada translúcida de tela cheia e derruba GPUs modestas.
            if std::env::var_os("COZY_MSAA").is_some() { Msaa::Sample4 } else { Msaa::Off },
            Projection::Orthographic(OrthographicProjection {
                scale: 1.0 / 3.0,
                ..OrthographicProjection::default_2d()
            }),
        ))
        .with_children(|camera| {
            camera.spawn((
                FadeVeil,
                Sprite::from_color(Color::NONE, Vec2::splat(4000.0)),
                Transform::from_xyz(0.0, 0.0, 500.0),
            ));
        });
}

/// Zoom inteiro (pixel perfeito) que mostra ~VIEW_HEIGHT pixels de altura.
fn pixel_zoom(
    window: Single<&Window, With<PrimaryWindow>>,
    mut projection: Single<&mut Projection, With<Camera2d>>,
) {
    let zoom = (window.height() / VIEW_HEIGHT).round().max(1.0);
    if let Projection::Orthographic(ortho) = projection.as_mut()
        && ortho.scale != 1.0 / zoom
    {
        ortho.scale = 1.0 / zoom;
    }
}

/// Metade do tamanho visível, em unidades do mundo.
pub fn half_view(window: &Window, projection: &Projection) -> Vec2 {
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    Vec2::new(window.width(), window.height()) * scale / 2.0
}

fn tick(time: Res<Time>, mut clock: ResMut<Clock>) {
    clock.advance(time.delta_secs() * MINUTES_PER_SECOND);
}

/// Mantém o estado DayPhase igual ao relógio (vale também para os saltos).
fn sync_phase(
    mut clock: ResMut<Clock>,
    phase: Res<State<DayPhase>>,
    mut next: ResMut<NextState<DayPhase>>,
    mut fade: ResMut<PhaseFade>,
    mut cutscenes: Option<ResMut<crate::cutscene::CutsceneManager>>,
) {
    if clock.minutes >= FULL_DAY {
        let old_season = clock.season();
        clock.minutes = 0.0;
        clock.day += 1;
        let new_season = clock.season();
        if new_season != old_season {
            if let Some(cutscenes) = &mut cutscenes {
                cutscenes.play_id(crate::cutscene::season_id(new_season), &[("dia", clock.day.to_string())]);
            }
        }
    }
    let wanted = clock.phase();
    if wanted != *phase.get() {
        next.set(wanted);
        fade.0 = FADE;
    }
}

/// [N] pula para o próximo marco · [P] pula para a próxima estação (atalhos de desenvolvimento).
#[cfg(debug_assertions)]
fn skip_milestone(
    keys: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<Clock>,
    mut cutscenes: Option<ResMut<crate::cutscene::CutsceneManager>>,
) {
    if keys.just_pressed(KeyCode::KeyN) {
        clock.skip_to_next_milestone();
    }
    if keys.just_pressed(KeyCode::KeyP) {
        clock.skip_to_next_season();
        if let Some(cutscenes) = &mut cutscenes {
            cutscenes.play_id(crate::cutscene::season_id(clock.season()), &[("dia", clock.day.to_string())]);
        }
    }
}

fn fade(
    time: Res<Time<Real>>,
    mut phase_fade: ResMut<PhaseFade>,
    travel: Res<Travel>,
    veil: Single<(&mut Sprite, &mut Visibility), With<FadeVeil>>,
) {
    phase_fade.0 = (phase_fade.0 - time.delta_secs()).max(0.0);
    let alpha = (phase_fade.0 / FADE).max(travel.veil()).clamp(0.0, 1.0);
    let (mut sprite, mut visibility) = veil.into_inner();
    // Invisível não é desenhado: uma camada de tela cheia a menos na GPU.
    visibility.set_if_neq(if alpha > 0.0 { Visibility::Inherited } else { Visibility::Hidden });
    let color = Color::srgba(0.03, 0.01, 0.06, alpha);
    if sprite.color != color {
        sprite.color = color;
    }
}
