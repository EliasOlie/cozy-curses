//! Caixa de diálogo: nome de quem fala e a fala, com efeito de digitação e suporte a múltiplas páginas.

use bevy::prelude::*;

use crate::AppState;
use crate::cycle::DayPhase;
use crate::sprites::Sprites;
use crate::ui::{font, CozyPanel, INK};

pub fn plugin(app: &mut App) {
    app.init_resource::<Dialogue>()
        .add_systems(OnEnter(AppState::Playing), spawn_box)
        .add_systems(OnExit(AppState::Playing), |mut d: ResMut<Dialogue>| d.close())
        .add_systems(Update, show.run_if(in_state(AppState::Playing)));
}

/// Letras por segundo do efeito de digitação.
const TYPE_SPEED: f32 = 60.0;

#[derive(Resource, Default)]
pub struct Dialogue {
    /// Quem fala e a lista de páginas de falas.
    pub current: Option<(String, Vec<String>)>,
    /// Página atual exibida (0..pages.len()).
    pub page: usize,
    /// Segundos desde que a página atual abriu.
    pub t: f32,
}

impl Dialogue {
    pub fn open(&mut self, speaker: &str, text: &str) {
        self.open_pages(speaker, vec![text.to_string()]);
    }

    pub fn open_pages(&mut self, speaker: &str, pages: Vec<String>) {
        if pages.is_empty() {
            self.current = None;
            return;
        }
        self.current = Some((speaker.to_string(), pages));
        self.page = 0;
        self.t = 0.0;
    }

    pub fn advance_page(&mut self) {
        let Some((_, pages)) = &self.current else { return };
        if self.page + 1 < pages.len() {
            self.page += 1;
            self.t = 0.0;
        } else {
            self.close();
        }
    }

    pub fn is_typing(&self) -> bool {
        let Some((_, pages)) = &self.current else { return false };
        let Some(line) = pages.get(self.page) else { return false };
        (self.t * TYPE_SPEED) < line.chars().count() as f32
    }

    pub fn finish_typing(&mut self) {
        let Some((_, pages)) = &self.current else { return };
        let Some(line) = pages.get(self.page) else { return };
        self.t = (line.chars().count() as f32) / TYPE_SPEED + 0.1;
    }

    pub fn close(&mut self) {
        self.current = None;
        self.page = 0;
        self.t = 0.0;
    }

    pub fn is_open(&self) -> bool {
        self.current.is_some()
    }
}

#[derive(Component)]
struct DialogueBox;

/// O cartão de pergaminho (recebe o pulinho ao abrir).
#[derive(Component)]
struct DialogueCard;

#[derive(Component)]
struct DialoguePortrait;

#[derive(Component)]
enum DialogueText {
    Speaker,
    Line,
    Prompt,
}

fn spawn_box(mut commands: Commands, sprites: Res<Sprites>) {
    commands
        .spawn((
            DialogueBox,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(105),
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|row| {
            // Pergaminho com moldura de flores e foto do interlocutor na esquerda
            row.spawn((
                DialogueCard,
                Node {
                    width: Val::Px(760.0),
                    min_height: Val::Px(150.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(16.0),
                    padding: UiRect::new(Val::Px(24.0), Val::Px(28.0), Val::Px(20.0), Val::Px(16.0)),
                    ..default()
                },
                CozyPanel::Dialog,
                UiTransform::IDENTITY,
            ))
            .with_children(|card| {
                // Moldura do Retrato do NPC
                card.spawn((
                    Node {
                        width: Val::Px(84.0),
                        height: Val::Px(84.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(4.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    CozyPanel::Dark,
                ))
                .with_child((
                    DialoguePortrait,
                    ImageNode {
                        image: sprites.portraits.day.clone(),
                        texture_atlas: sprites.portraits.layout.clone().map(|layout| TextureAtlas { layout, index: 2 }),
                        ..default()
                    },
                    Node {
                        // Retratos 32x32 em 2x; os outros (corpo 16x32) mantêm a proporção.
                        height: Val::Px(64.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));

                // Conteúdo da conversa (Tag, Fala e Tecla)
                card.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: Val::Px(8.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|body| {
                    body.spawn((
                        Node {
                            padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(4.0), Val::Px(6.0)),
                            align_self: AlignSelf::FlexStart,
                            ..default()
                        },
                        crate::ui::sliced(sprites.ui.tag.clone(), 4.0),
                    ))
                    .with_child((DialogueText::Speaker, Text::new(""), font(&sprites, 22.0), TextColor(INK)));

                    body.spawn((
                        DialogueText::Line,
                        Text::new(""),
                        font(&sprites, 22.0),
                        TextColor(INK),
                        Node {
                            max_width: Val::Px(610.0),
                            min_height: Val::Px(50.0),
                            ..default()
                        },
                    ));

                    body.spawn((
                        DialogueText::Prompt,
                        Text::new("[E]"),
                        font(&sprites, 18.0),
                        TextColor(INK),
                        Node { align_self: AlignSelf::FlexEnd, ..default() },
                    ));
                });
            });
        });
}

fn show(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    mut dialogue: ResMut<Dialogue>,
    cards: Query<Entity, With<DialogueCard>>,
    mut boxes: Query<&mut Visibility, With<DialogueBox>>,
    mut portraits: Query<(&mut ImageNode, &mut Visibility), (With<DialoguePortrait>, Without<DialogueBox>)>,
    mut texts: Query<(&DialogueText, &mut Text, &mut TextColor)>,
    phase: Option<Res<State<DayPhase>>>,
) {
    let open = dialogue.is_open();
    for mut v in &mut boxes {
        v.set_if_neq(if open { Visibility::Inherited } else { Visibility::Hidden });
    }
    let Some((speaker, pages)) = dialogue.current.clone() else { return };
    let page = dialogue.page;
    let total = pages.len();
    let line = pages.get(page).cloned().unwrap_or_default();

    if dialogue.t == 0.0 {
        for card in &cards {
            commands.entity(card).insert(crate::ui::UiBounce::new(0.06));
        }
    }
    dialogue.t += time.delta_secs();
    let shown: String = line.chars().take((dialogue.t * TYPE_SPEED) as usize).collect();
    let blink = (dialogue.t * 3.0) as u32 % 2 == 0;

    // Atualiza o retrato do falante: moradores têm retrato próprio (de noite, a forma
    // amaldiçoada que eles mesmos não enxergam).
    let night = phase.is_some_and(|p| *p.get() == DayPhase::Night);
    let has = |names: &[&str]| names.iter().any(|n| speaker.contains(n));
    let portrait = |index: usize, night: bool| {
        let art = &sprites.portraits;
        (if night { art.night.clone() } else { art.day.clone() }, art.layout.clone(), index)
    };
    for (mut img, mut vis) in &mut portraits {
        let (art_image, art_layout, index) = if has(&["Prefeito", "Anselmo"]) {
            portrait(2, night)
        } else if has(&["Florinda", "Florista"]) {
            portrait(6, night)
        } else if has(&["Bartô", "Mercador"]) {
            portrait(4, night)
        } else if has(&["Tobias", "Padeiro"]) {
            portrait(0, night)
        } else if has(&["Tião", "Ferreiro"]) {
            portrait(5, night)
        } else if has(&["Bento", "Xerife", "Guarda"]) {
            portrait(1, night)
        } else if has(&["Celeste", "Professora"]) {
            portrait(3, night)
        } else if has(&["Ghoul", "Necrófago"]) {
            portrait(0, true)
        } else if has(&["Inquisidor"]) {
            (sprites.inquisitor.day.clone(), sprites.inquisitor.layout.clone(), 0)
        } else if has(&["Vento", "Sombra", "Voz", "Máscara"]) {
            (sprites.items.day.clone(), sprites.items.layout.clone(), 12)
        } else {
            (sprites.farmer_body.day.clone(), sprites.farmer_body.layout.clone(), 0)
        };

        if img.image != art_image {
            img.image = art_image;
        }
        if let Some(layout) = art_layout {
            img.texture_atlas = Some(TextureAtlas { layout, index });
        } else {
            img.texture_atlas = None;
        }
        vis.set_if_neq(Visibility::Inherited);
    }

    let prompt_str = if page + 1 < total {
        format!("[E] Mais ({}/{})", page + 1, total)
    } else {
        "[E] Fechar".to_string()
    };

    for (kind, mut text, mut color) in &mut texts {
        match kind {
            DialogueText::Speaker => text.0.clone_from(&speaker),
            DialogueText::Line => {
                if text.0 != shown {
                    text.0.clone_from(&shown);
                }
            }
            DialogueText::Prompt => {
                if text.0 != prompt_str {
                    text.0 = prompt_str.clone();
                }
                color.0 = if blink { INK } else { INK.with_alpha(0.25) };
            }
        }
    }
}
