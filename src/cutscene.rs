//! Motor de cutscenes cinemáticas para momentos narrativos, transições de estações e eventos.
//!
//! Gerencia sequências de passos narrativos (falas de personagens, banners cinemáticos,
//! pausas dramáticas, transições de tela e controle de câmera).

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};

use bevy::prelude::*;
use serde::Deserialize;

use crate::cycle::{Clock, DayPhase, Season};
use crate::dialogue::Dialogue;
use crate::economy::Game;
use crate::map::MapState;
use crate::save::ActiveSave;
use crate::sprites::Sprites;
use crate::ui::font;
use crate::world::Travel;
use crate::AppState;

pub fn plugin(app: &mut App) {
    app.init_resource::<CutsceneManager>()
        // Depois dos mapas do editor: o `when.map` é conferido contra os mapas que existem.
        .add_systems(Startup, load_library.after(crate::save::load_map_overrides))
        .add_systems(OnEnter(AppState::Playing), setup_cutscene_ui)
        .add_systems(OnExit(AppState::Playing), |mut m: ResMut<CutsceneManager>| *m = CutsceneManager::default())
        .add_systems(
            Update,
            (fire_triggers, update_cutscene, handle_cutscene_input, render_cutscene_ui)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

    #[cfg(debug_assertions)]
    app.add_systems(Update, dev_reload.run_if(in_state(AppState::Playing)));
}

/// Passo individual dentro de uma sequência cinemática.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum CutsceneStep {
    /// Exibe uma linha ou páginas de diálogo faladas por um personagem.
    Dialogue {
        speaker: String,
        pages: Vec<String>,
    },
    /// Banner centralizado de tela cheia (ex: virada de estação ou marcos narrativos).
    Banner {
        title: String,
        subtitle: String,
        duration: f32,
    },
    /// Pausa para efeito dramático ou contemplação.
    Wait {
        duration: f32,
    },
    /// Flash ou escurecimento de tela (fade in / out).
    Fade {
        to_black: bool,
        duration: f32,
    },
    /// Ação ou gatilho customizado executado pelo motor de jogo.
    Trigger {
        tag: String,
    },
}

/// Uma cutscene completa com ID e lista de passos ordenados.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Cutscene {
    pub id: String,
    pub title: String,
    pub steps: Vec<CutsceneStep>,
    pub skippable: bool,
}

// ---------------------------------------------------------------------------
// Cutscenes em dados: assets/cutscenes/*.json (guia em docs/CUTSCENES.md)
// ---------------------------------------------------------------------------

include!(concat!(env!("OUT_DIR"), "/embedded_cutscenes.rs"));

fn yes() -> bool {
    true
}

/// Um passo como vem no arquivo: exatamente um de `say`, `banner`, `wait` ou `fade`.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct StepDef {
    say: Option<String>,
    pages: Option<Vec<String>>,
    text: Option<String>,
    banner: Option<String>,
    subtitle: Option<String>,
    seconds: Option<f32>,
    wait: Option<f32>,
    fade: Option<String>,
}

/// Condições para a cutscene começar sozinha. Todas as presentes precisam valer.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct When {
    map: Option<String>,
    phase: Option<String>,
    season: Option<String>,
    min_day: Option<u32>,
    min_curse: Option<f32>,
    has_mask: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CutsceneDef {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default = "yes")]
    skippable: bool,
    when: Option<When>,
    #[serde(default = "yes")]
    once: bool,
    steps: Vec<StepDef>,
    /// Anotação livre de quem escreveu; o jogo ignora.
    #[serde(default)]
    #[allow(dead_code)]
    notes: String,
}

/// Cutscene já conferida, com as {variáveis} ainda por preencher.
#[derive(Clone)]
struct Template {
    cutscene: Cutscene,
    when: Option<When>,
    once: bool,
}

static LIBRARY: LazyLock<RwLock<Vec<Template>>> = LazyLock::new(|| RwLock::new(Vec::new()));

fn phase_key(text: &str) -> Option<DayPhase> {
    match text.to_lowercase().as_str() {
        "day" | "dia" => Some(DayPhase::Day),
        "night" | "noite" => Some(DayPhase::Night),
        _ => None,
    }
}

fn season_from(text: &str) -> Option<Season> {
    match text.to_lowercase().as_str() {
        "spring" | "primavera" => Some(Season::Spring),
        "summer" | "verao" | "verão" => Some(Season::Summer),
        "autumn" | "outono" => Some(Season::Autumn),
        "winter" | "inverno" => Some(Season::Winter),
        _ => None,
    }
}

/// Id da cutscene de virada para cada estação (assets/cutscenes/season_*.json).
pub fn season_id(season: Season) -> &'static str {
    match season {
        Season::Spring => "season_spring",
        Season::Summer => "season_summer",
        Season::Autumn => "season_autumn",
        Season::Winter => "season_winter",
    }
}

fn parse(text: &str) -> Result<Template, String> {
    let def: CutsceneDef = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if def.id.trim().is_empty() {
        return Err("falta o \"id\"".into());
    }
    if def.steps.is_empty() {
        return Err("\"steps\" está vazio".into());
    }
    let mut steps = Vec::new();
    for (i, step) in def.steps.into_iter().enumerate() {
        let n = i + 1;
        let kinds = [step.say.is_some(), step.banner.is_some(), step.wait.is_some(), step.fade.is_some()];
        if kinds.iter().filter(|k| **k).count() != 1 {
            return Err(format!("passo {n}: use exatamente um de \"say\", \"banner\", \"wait\" ou \"fade\""));
        }
        steps.push(if let Some(speaker) = step.say {
            let pages = step.pages.or(step.text.map(|t| vec![t])).unwrap_or_default();
            if pages.is_empty() {
                return Err(format!("passo {n}: \"say\" precisa de \"text\" ou \"pages\""));
            }
            CutsceneStep::Dialogue { speaker, pages }
        } else if let Some(title) = step.banner {
            CutsceneStep::Banner { title, subtitle: step.subtitle.unwrap_or_default(), duration: step.seconds.unwrap_or(3.0) }
        } else if let Some(duration) = step.wait {
            CutsceneStep::Wait { duration }
        } else {
            let to_black = match step.fade.unwrap_or_default().to_lowercase().as_str() {
                "out" | "escurecer" => true,
                "in" | "clarear" => false,
                other => return Err(format!("passo {n}: \"fade\" é \"out\" (escurecer) ou \"in\" (clarear), não \"{other}\"")),
            };
            CutsceneStep::Fade { to_black, duration: step.seconds.unwrap_or(0.8) }
        });
    }
    if let Some(when) = &def.when {
        if let Some(phase) = &when.phase
            && phase_key(phase).is_none()
        {
            return Err(format!("\"when.phase\" é \"day\" ou \"night\", não \"{phase}\""));
        }
        if let Some(season) = &when.season
            && season_from(season).is_none()
        {
            return Err(format!("\"when.season\" é spring, summer, autumn ou winter, não \"{season}\""));
        }
        if let Some(map) = &when.map
            && !crate::worlddef::world().maps.iter().any(|m| &m.id == map)
        {
            return Err(format!("\"when.map\": não existe mapa com id \"{map}\""));
        }
    }
    let cutscene = Cutscene { id: def.id, title: def.title, steps, skippable: def.skippable };
    Ok(Template { cutscene, when: def.when, once: def.once })
}

/// (arquivo, texto) de todas as cutscenes: no nativo a pasta do disco (dá para editar com
/// o jogo aberto); no navegador, ou sem a pasta, o que foi embutido na compilação.
fn sources() -> Vec<(String, String)> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(dir) = std::fs::read_dir("assets/cutscenes") {
        let mut files: Vec<(String, String)> = dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| Some((p.file_name()?.to_string_lossy().into_owned(), std::fs::read_to_string(&p).ok()?)))
            .collect();
        files.sort();
        return files;
    }
    EMBEDDED_CUTSCENES.iter().map(|(file, text)| (file.to_string(), text.to_string())).collect()
}

/// Lê (ou relê) todas as cutscenes. Devolve quantas valeram e os erros, um por arquivo.
pub fn reload() -> (usize, Vec<String>) {
    let mut library = Vec::new();
    let mut errors = Vec::new();
    for (file, text) in sources() {
        match parse(&text) {
            Ok(template) if library.iter().any(|t: &Template| t.cutscene.id == template.cutscene.id) => {
                errors.push(format!("{file}: o id \"{}\" já existe em outro arquivo", template.cutscene.id));
            }
            Ok(template) => library.push(template),
            Err(e) => errors.push(format!("{file}: {e}")),
        }
    }
    for e in &errors {
        warn!("cutscene ignorada — {e}");
    }
    let count = library.len();
    *LIBRARY.write().unwrap() = library;
    (count, errors)
}

fn load_library() {
    let (count, errors) = reload();
    info!("cutscenes: {count} carregadas, {} com erro", errors.len());
}

/// Troca cada `{chave}` do texto pelo valor correspondente.
fn fill(text: &str, vars: &HashMap<String, String>) -> String {
    vars.iter().fold(text.to_string(), |out, (key, value)| out.replace(&format!("{{{key}}}"), value))
}

impl Cutscene {
    fn filled(&self, vars: &HashMap<String, String>) -> Cutscene {
        let steps = self
            .steps
            .iter()
            .map(|step| match step {
                CutsceneStep::Dialogue { speaker, pages } => CutsceneStep::Dialogue {
                    speaker: fill(speaker, vars),
                    pages: pages.iter().map(|p| fill(p, vars)).collect(),
                },
                CutsceneStep::Banner { title, subtitle, duration } => {
                    CutsceneStep::Banner { title: fill(title, vars), subtitle: fill(subtitle, vars), duration: *duration }
                }
                other => other.clone(),
            })
            .collect();
        Cutscene { steps, ..self.clone() }
    }
}

/// Estado de execução da cutscene ativa.
#[derive(Clone, Debug)]
pub struct ActiveCutscene {
    pub cutscene: Cutscene,
    pub step_index: usize,
    pub timer: f32,
    pub fade_alpha: f32,
}

#[derive(Resource, Default)]
pub struct CutsceneManager {
    pub active: Option<ActiveCutscene>,
    pub queue: Vec<Cutscene>,
    pub letterbox_amount: f32, // 0.0 a 1.0 (animação das faixas pretas)
    pub banner_info: Option<(String, String, f32)>, // título, subtítulo, tempo restante
    /// Valores atuais das {variáveis} dos textos (nome, sitio, dia, estacao, ouro, maldicao).
    vars: HashMap<String, String>,
    /// Gatilhos cuja condição já está valendo: só disparam de novo depois de deixar de valer.
    holding: HashSet<String>,
}

impl CutsceneManager {
    /// Toca a cutscene `id` de assets/cutscenes. `extra` são variáveis só desta vez
    /// (ex.: ("multa", "40")), que valem por cima das gerais.
    pub fn play_id(&mut self, id: &str, extra: &[(&str, String)]) {
        let template = LIBRARY.read().unwrap().iter().find(|t| t.cutscene.id == id).cloned();
        let Some(template) = template else {
            warn!("cutscene \"{id}\" não existe em assets/cutscenes (ou está com erro)");
            return;
        };
        let mut vars = self.vars.clone();
        vars.extend(extra.iter().map(|(k, v)| (k.to_string(), v.clone())));
        self.play(template.cutscene.filled(&vars));
    }

    pub fn play(&mut self, cutscene: Cutscene) {
        if self.active.is_none() {
            self.active = Some(ActiveCutscene {
                cutscene,
                step_index: 0,
                timer: 0.0,
                fade_alpha: 0.0,
            });
        } else {
            self.queue.push(cutscene);
        }
    }

    pub fn is_playing(&self) -> bool {
        self.active.is_some()
    }

    pub fn skip(&mut self, dialogue: &mut Dialogue) {
        if let Some(active) = &self.active {
            if active.cutscene.skippable {
                dialogue.close();
                self.banner_info = None;
                self.active = self.queue.pop().map(|c| ActiveCutscene {
                    cutscene: c,
                    step_index: 0,
                    timer: 0.0,
                    fade_alpha: 0.0,
                });
            }
        }
    }

    pub fn advance_step(&mut self, dialogue: &mut Dialogue) {
        let Some(active) = &mut self.active else { return };
        active.step_index += 1;
        active.timer = 0.0;
        self.banner_info = None;

        if active.step_index >= active.cutscene.steps.len() {
            dialogue.close();
            self.active = self.queue.pop().map(|c| ActiveCutscene {
                cutscene: c,
                step_index: 0,
                timer: 0.0,
                fade_alpha: 0.0,
            });
        }
    }
}

#[derive(Component)]
struct LetterboxTop;

#[derive(Component)]
struct LetterboxBottom;

#[derive(Component)]
struct CutsceneOverlayRoot;

#[derive(Component)]
struct CutsceneBannerText;

#[derive(Component)]
struct CutsceneSubtitleText;

#[derive(Component)]
struct CutsceneSkipPrompt;

fn setup_cutscene_ui(mut commands: Commands, sprites: Res<Sprites>) {
    commands
        .spawn((
            CutsceneOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(99),
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|root| {
            // Faixa preta superior cinematográfica (Letterbox)
            root.spawn((
                LetterboxTop,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));

            // Conteúdo central (Banner de título e subtítulo)
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|banner_box| {
                banner_box.spawn((
                    CutsceneBannerText,
                    Text::new(""),
                    font(&sprites, 38.0),
                    TextColor(Color::srgb(0.98, 0.94, 0.82)),
                ));
                banner_box.spawn((
                    CutsceneSubtitleText,
                    Text::new(""),
                    font(&sprites, 20.0),
                    TextColor(Color::srgb(0.85, 0.85, 0.90)),
                ));
            });

            // Faixa preta inferior cinematográfica (Letterbox)
            root.spawn((
                LetterboxBottom,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(0.0),
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    padding: UiRect::right(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ))
            .with_children(|bottom| {
                bottom.spawn((
                    CutsceneSkipPrompt,
                    Text::new("[Espaço / E] Avançar • [ESC] Pular"),
                    font(&sprites, 16.0),
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                ));
            });
        });
}

/// Atualiza as {variáveis} e começa as cutscenes cujo `when` passou a valer.
fn fire_triggers(
    mut manager: ResMut<CutsceneManager>,
    map: Option<Res<State<MapState>>>,
    phase: Option<Res<State<DayPhase>>>,
    clock: Option<Res<Clock>>,
    game: Option<ResMut<Game>>,
    active: Option<Res<ActiveSave>>,
    travel: Res<Travel>,
) {
    let (Some(map), Some(phase), Some(clock), Some(mut game), Some(active)) = (map, phase, clock, game, active) else {
        return;
    };
    let season = clock.season();
    for (key, value) in [
        ("nome", active.data.character.name.clone()),
        ("sitio", active.data.character.farm.clone()),
        ("dia", clock.day.to_string()),
        ("estacao", season.name().to_string()),
        ("ouro", game.gold.to_string()),
        ("maldicao", format!("{:.0}", game.curse)),
    ] {
        if manager.vars.get(key) != Some(&value) {
            manager.vars.insert(key.to_string(), value);
        }
    }
    // No meio de uma troca de mapa (tela escura) ou antes de o mapa existir, espera.
    if travel.busy() || map.get().0.is_empty() {
        return;
    }
    let ready: Vec<(String, bool, bool)> = LIBRARY
        .read()
        .unwrap()
        .iter()
        .filter_map(|t| {
            let when = t.when.as_ref()?;
            let holds = when.map.as_ref().is_none_or(|m| *m == map.get().0)
                && when.phase.as_ref().is_none_or(|p| phase_key(p) == Some(*phase.get()))
                && when.season.as_ref().is_none_or(|s| season_from(s) == Some(season))
                && when.min_day.is_none_or(|d| clock.day >= d)
                && when.min_curse.is_none_or(|c| game.curse >= c)
                && when.has_mask.is_none_or(|m| game.has_mask == m);
            Some((t.cutscene.id.clone(), holds, t.once))
        })
        .collect();
    for (id, holds, once) in ready {
        if !holds {
            manager.holding.remove(&id);
            continue;
        }
        if !manager.holding.insert(id.clone()) {
            continue;
        }
        if once {
            if game.seen_cutscenes.contains(&id) {
                continue;
            }
            game.seen_cutscenes.push(id.clone());
        }
        manager.play_id(&id, &[]);
    }
}

/// Dev: F5 relê assets/cutscenes e, se COZY_CUTSCENE=<id> estiver definida, toca essa.
#[cfg(debug_assertions)]
fn dev_reload(keys: Res<ButtonInput<KeyCode>>, mut manager: ResMut<CutsceneManager>, mut game: ResMut<Game>) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    let (count, errors) = reload();
    game.say(format!("Cutscenes relidas: {count} ok, {} com erro.", errors.len()));
    if let Some(first) = errors.first() {
        game.say(first.clone());
    }
    if let Ok(id) = std::env::var("COZY_CUTSCENE") {
        manager.play_id(&id, &[]);
    }
}

fn update_cutscene(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut manager: ResMut<CutsceneManager>,
    mut dialogue: ResMut<Dialogue>,
    intro_mayors: Query<Entity, With<crate::economy::IntroMayor>>,
) {
    let dt = time.delta_secs();

    // Se a cutscene de intro terminou, despawna o Prefeito que veio visitar o sítio
    if manager.active.as_ref().map_or(true, |a| a.cutscene.id != "intro") {
        for e in &intro_mayors {
            commands.entity(e).despawn();
        }
    }

    // Interpolação suave do letterboxing (barras pretas deslizando)
    let target_letterbox = if manager.is_playing() { 54.0 } else { 0.0 };
    manager.letterbox_amount += (target_letterbox - manager.letterbox_amount) * (1.0 - (-12.0 * dt).exp());

    let (current_step, timer) = {
        let Some(active) = &mut manager.active else { return };
        active.timer += dt;
        let step_index = active.step_index;
        if step_index >= active.cutscene.steps.len() {
            (None, active.timer)
        } else {
            (Some(active.cutscene.steps[step_index].clone()), active.timer)
        }
    };

    let Some(current_step) = current_step else {
        manager.advance_step(&mut dialogue);
        return;
    };

    match current_step {
        CutsceneStep::Dialogue { speaker, pages } => {
            if !dialogue.is_open() {
                dialogue.open_pages(&speaker, pages);
            } else if !dialogue.is_open() {
                // Diálogo fechou: avança o passo
                manager.advance_step(&mut dialogue);
            }
        }
        CutsceneStep::Banner { title, subtitle, duration } => {
            manager.banner_info = Some((title, subtitle, duration - timer));
            if timer >= duration {
                manager.advance_step(&mut dialogue);
            }
        }
        CutsceneStep::Wait { duration } => {
            if timer >= duration {
                manager.advance_step(&mut dialogue);
            }
        }
        CutsceneStep::Fade { to_black, duration } => {
            let progress = (timer / duration.max(0.01)).clamp(0.0, 1.0);
            if let Some(active) = &mut manager.active {
                active.fade_alpha = if to_black { progress } else { 1.0 - progress };
            }
            if timer >= duration {
                manager.advance_step(&mut dialogue);
            }
        }
        CutsceneStep::Trigger { tag: _ } => {
            manager.advance_step(&mut dialogue);
        }
    }
}

fn handle_cutscene_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut manager: ResMut<CutsceneManager>,
    mut dialogue: ResMut<Dialogue>,
) {
    if !manager.is_playing() {
        return;
    }

    // A cutscene "come" as teclas que usa: senão, no quadro em que ela termina, o mesmo E
    // ainda conta para o jogo (cochilar na cama, conversar) e o Esc abre a pausa.
    let skip = keys.clear_just_pressed(KeyCode::Escape);
    let advance = [KeyCode::Space, KeyCode::KeyE, KeyCode::Enter]
        .map(|k| keys.clear_just_pressed(k))
        .contains(&true);

    if skip {
        manager.skip(&mut dialogue);
        return;
    }

    if advance {
        if let Some(active) = &manager.active {
            if let Some(step) = active.cutscene.steps.get(active.step_index) {
                match step {
                    CutsceneStep::Dialogue { .. } => {
                        // Diálogo gerencia o avanço de páginas internamente
                        if dialogue.is_open() {
                            if dialogue.is_typing() {
                                dialogue.finish_typing();
                            } else {
                                dialogue.advance_page();
                                if !dialogue.is_open() {
                                    manager.advance_step(&mut dialogue);
                                }
                            }
                        } else {
                            manager.advance_step(&mut dialogue);
                        }
                    }
                    CutsceneStep::Banner { .. } | CutsceneStep::Wait { .. } => {
                        manager.advance_step(&mut dialogue);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render_cutscene_ui(
    manager: Res<CutsceneManager>,
    mut roots: Query<(&mut Visibility, &mut BackgroundColor), With<CutsceneOverlayRoot>>,
    mut top_bars: Query<&mut Node, (With<LetterboxTop>, Without<LetterboxBottom>)>,
    mut bottom_bars: Query<&mut Node, (With<LetterboxBottom>, Without<LetterboxTop>)>,
    mut banner_texts: Query<&mut Text, (With<CutsceneBannerText>, Without<CutsceneSubtitleText>)>,
    mut subtitle_texts: Query<&mut Text, (With<CutsceneSubtitleText>, Without<CutsceneBannerText>)>,
) {
    let visible = manager.is_playing() || manager.letterbox_amount > 1.0;
    // O passo "fade" escurece (ou clareia) a tela inteira por baixo do banner.
    let shade = Color::BLACK.with_alpha(manager.active.as_ref().map_or(0.0, |a| a.fade_alpha));
    for (mut v, mut background) in &mut roots {
        v.set_if_neq(if visible { Visibility::Inherited } else { Visibility::Hidden });
        if background.0 != shade {
            background.0 = shade;
        }
    }

    let h = manager.letterbox_amount;
    for mut node in &mut top_bars {
        node.height = Val::Px(h);
    }
    for mut node in &mut bottom_bars {
        node.height = Val::Px(h);
    }

    let (title, sub) = if let Some((t, s, _)) = &manager.banner_info {
        (t.as_str(), s.as_str())
    } else {
        ("", "")
    };

    for mut text in &mut banner_texts {
        if text.0 != title {
            text.0 = title.to_string();
        }
    }
    for mut text in &mut subtitle_texts {
        if text.0 != sub {
            text.0 = sub.to_string();
        }
    }
}
