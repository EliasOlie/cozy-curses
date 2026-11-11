//! Esc durante o jogo: pausa com opção de voltar ao menu.

use bevy::prelude::*;

use crate::menu::UiAction;
use crate::sprites::Sprites;
use crate::ui::{MUTED, font, panel, spawn_button};
use crate::{AppState, Overlay, Pause};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, toggle.run_if(in_state(AppState::Playing)))
        .add_systems(OnEnter(Pause::Paused), (spawn_overlay, |mut t: ResMut<Time<Virtual>>| t.pause()))
        .add_systems(OnExit(Pause::Paused), |mut t: ResMut<Time<Virtual>>| t.unpause());
}

fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    pause: Res<State<Pause>>,
    mut next: ResMut<NextState<Pause>>,
    mut overlay: ResMut<Overlay>,
    cutscenes: Option<Res<crate::cutscene::CutsceneManager>>,
) {
    // Na cutscene o Esc é "pular", não pausar.
    if cutscenes.is_some_and(|c| c.is_playing()) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        // Esc fecha primeiro a janela aberta (loja, mochila, celular).
        if *overlay != Overlay::None {
            *overlay = Overlay::None;
            return;
        }
        next.set(match pause.get() {
            Pause::Running => Pause::Paused,
            Pause::Paused => Pause::Running,
        });
    }
}

fn spawn_overlay(mut commands: Commands, sprites: Res<Sprites>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.01, 0.06, 0.6)),
            GlobalZIndex(10),
            DespawnOnExit(Pause::Paused),
        ))
        .with_children(|ui| {
            ui.spawn(panel(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            }))
            .with_children(|card| {
                card.spawn((Text::new("Pausado"), font(&sprites, 56.0), TextColor(Color::srgb(1.0, 0.86, 0.55))));
                spawn_button(card, &sprites, "Continuar", 32.0, UiAction::Resume);
                spawn_button(card, &sprites, "Sair para o menu", 32.0, UiAction::QuitToMenu);
                card.spawn((
                    Text::new("O jogo é salvo sozinho a cada amanhecer.\nO que aconteceu desde então se perde ao sair."),
                    font(&sprites, 20.0),
                    TextColor(MUTED),
                    TextLayout::justify(Justify::Center),
                ));
            });
        });
}
