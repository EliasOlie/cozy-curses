//! Tela de carregamento: espera todas as imagens e a fonte antes do menu.
//! Assim nada aparece "pela metade" e o chão assado já tem os pixels dos tiles.

use bevy::prelude::*;

use crate::AppState;
use crate::sprites::Sprites;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), spawn_screen)
        .add_systems(Update, wait.run_if(in_state(AppState::Loading)));
}

#[derive(Component)]
struct Progress;

fn spawn_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.08, 0.06)),
            DespawnOnExit(AppState::Loading),
        ))
        .with_child((Progress, Text::new("Carregando..."), TextColor(Color::srgb(1.0, 0.86, 0.55))));
}

fn wait(
    server: Res<AssetServer>,
    sprites: Res<Sprites>,
    mut text: Query<&mut Text, With<Progress>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let done = sprites.pending.iter().filter(|h| server.is_loaded_with_dependencies(h.id())).count();
    let failed = sprites.pending.iter().any(|h| server.load_state(h.id()).is_failed());
    for mut text in &mut text {
        text.0 = format!("Carregando... {done}/{}", sprites.pending.len());
    }
    // Um arquivo faltando não pode travar o jogo: o aviso fica no console.
    if done == sprites.pending.len() || failed {
        if failed {
            warn!("algum asset não carregou; seguindo mesmo assim");
        }
        next.set(AppState::Menu);
    }
}
