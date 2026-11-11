//! Peças de interface compartilhadas pelos menus e pelo HUD: molduras de madeira (9-slice),
//! botões com micro-interações, ícones de item.

use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::widget::NodeImageMode;

use crate::inventory::Item;
use crate::menu::UiAction;
use crate::sprites::Sprites;

pub fn plugin(app: &mut App) {
    app.init_resource::<Clicked>().add_systems(Update, (dress_panels, button_feel, ui_bounce));
}

pub const PANEL_SOLID: Color = Color::srgb(0.16, 0.1, 0.07);
pub const PANEL_BORDER: Color = Color::srgb(0.55, 0.36, 0.2);
pub const TEXT: Color = Color::srgb(0.96, 0.93, 0.86);
pub const MUTED: Color = Color::srgb(0.72, 0.66, 0.58);
pub const HINT: Color = Color::srgb(1.0, 0.95, 0.66);
/// Texto escuro sobre madeira clara e pergaminho.
pub const INK: Color = Color::srgb(0.27, 0.16, 0.09);
/// As artes de interface vêm ampliadas 3x (assets/ui).
const UI_SCALE: f32 = 3.0;

/// Cliques de botões desde o último frame, para os menus tratarem num sistema normal.
#[derive(Resource, Default)]
pub struct Clicked(pub Vec<UiAction>);

pub fn font(sprites: &Sprites, size: f32) -> TextFont {
    TextFont::from_font_size(size).with_font(sprites.font.clone())
}

/// Moldura fatiada em 9 (cantos fixos, bordas e miolo esticam).
pub fn sliced(image: Handle<Image>, border_px: f32) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(border_px * UI_SCALE),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        }),
        ..default()
    }
}

/// Que moldura um painel usa (a arte entra em `dress_panels`).
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum CozyPanel {
    /// Vidro escuro sobre o mundo (HUD, menus).
    Dark,
    /// Pergaminho claro (diálogo) — texto escuro.
    Dialog,
}

/// Painel com moldura de madeira. A borda do 9-slice já faz o papel de borda/padding mínimo.
pub fn panel(node: Node) -> impl Bundle {
    let padding = node.padding;
    (
        Node {
            padding: UiRect {
                // A moldura tem 24 px (8 px ampliados 3x): o texto começa depois dela.
                left: max_px(padding.left, 24.0),
                right: max_px(padding.right, 24.0),
                top: max_px(padding.top, 18.0),
                bottom: max_px(padding.bottom, 20.0),
            },
            ..node
        },
        CozyPanel::Dark,
    )
}

fn max_px(v: Val, min: f32) -> Val {
    match v {
        Val::Px(p) => Val::Px(p.max(min)),
        _ => Val::Px(min),
    }
}

/// Põe a arte das molduras assim que o painel aparece.
fn dress_panels(mut commands: Commands, sprites: Res<Sprites>, panels: Query<(Entity, &CozyPanel), Added<CozyPanel>>) {
    for (entity, kind) in &panels {
        let (image, border) = match kind {
            CozyPanel::Dark => (sprites.ui.panel_dark.clone(), 8.0),
            CozyPanel::Dialog => (sprites.ui.dialog.clone(), 12.0),
        };
        // A moldura é uma camada atrás do conteúdo, do tamanho do painel: assim a imagem
        // não interfere no layout (padding e tamanho continuam sendo do painel).
        commands.entity(entity).remove::<(BackgroundColor, BorderColor)>().with_child((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            sliced(image, border),
            ZIndex(-1),
            Pickable::IGNORE,
        ));
    }
}

/// Ícone de item (spritesheet `items`) num quadrado de `size` px, com coloração própria.
pub fn item_icon(sprites: &Sprites, item: Item, size: f32) -> impl Bundle {
    (
        ImageNode {
            image: sprites.items.day.clone(),
            texture_atlas: sprites.items.layout.clone().map(|layout| TextureAtlas { layout, index: item.icon() }),
            ..default()
        },
        Node { width: Val::Px(size), height: Val::Px(size), ..default() },
    )
}

pub fn text_shadow() -> TextShadow {
    TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::BLACK }
}

#[derive(Component)]
pub struct UiButton {
    /// Segundos restantes do "afundado" depois de um clique.
    pressed: f32,
}

/// A madeira do botão: camada atrás do texto, do tamanho do botão (assim a imagem não
/// atrapalha o padding).
#[derive(Component)]
struct ButtonFrame;

/// Borda do botão no 9-slice: 5 px ampliados 3x. O padding fica maior que ela.
const BUTTON_BORDER: f32 = 5.0;

pub fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    sprites: &Sprites,
    label: impl Into<String>,
    size: f32,
    action: UiAction,
) {
    parent
        .spawn((
            UiButton { pressed: 0.0 },
            Hovered::default(),
            Node {
                // Folga proporcional à letra: o texto nunca encosta na moldura (embaixo fica a
                // "espessura" da madeira).
                padding: UiRect::new(
                    Val::Px((size * 0.85).max(BUTTON_BORDER * 3.0)),
                    Val::Px((size * 0.85).max(BUTTON_BORDER * 3.0)),
                    Val::Px(size * 0.35 + 3.0),
                    Val::Px(size * 0.5 + 5.0),
                ),
                min_width: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            UiTransform::IDENTITY,
        ))
        .observe(move |press: On<PointerPress>, mut clicked: ResMut<Clicked>, mut buttons: Query<&mut UiButton>| {
            clicked.0.push(action.clone());
            if let Ok(mut button) = buttons.get_mut(press.event_target()) {
                button.pressed = 0.12;
            }
        })
        .with_children(|b| {
            b.spawn((
                ButtonFrame,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                sliced(sprites.ui.button.clone(), BUTTON_BORDER),
                ZIndex(-1),
                Pickable::IGNORE,
            ));
            b.spawn((Text::new(label), font(sprites, size), TextColor(INK), Pickable::IGNORE));
        });
}

/// Botões: madeira clara no hover (crescendo um tiquinho), afundada no clique.
fn button_feel(
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    mut buttons: Query<(&mut UiButton, &Hovered, &mut UiTransform, &Children)>,
    mut frames: Query<&mut ImageNode, With<ButtonFrame>>,
) {
    for (mut button, hovered, mut transform, children) in &mut buttons {
        button.pressed = (button.pressed - time.delta_secs()).max(0.0);
        let (art, scale) = if button.pressed > 0.0 {
            (&sprites.ui.button_pressed, 0.96)
        } else if hovered.get() {
            (&sprites.ui.button_hover, 1.04)
        } else {
            (&sprites.ui.button, 1.0)
        };
        for child in children.iter() {
            if let Ok(mut image) = frames.get_mut(child)
                && image.image != *art
            {
                image.image = art.clone();
            }
        }
        let k = 1.0 - (-18.0 * time.delta_secs()).exp();
        transform.scale = transform.scale.lerp(Vec2::splat(scale), k);
    }
}

/// "Pulinho" de um elemento de interface (slot escolhido, item chegando, caixa abrindo).
#[derive(Component)]
pub struct UiBounce {
    pub t: f32,
    /// Tamanho do pulo (0.15 = 15% maior no pico).
    pub amount: f32,
}

impl UiBounce {
    pub fn new(amount: f32) -> Self {
        UiBounce { t: 0.0, amount }
    }
}

const BOUNCE_TIME: f32 = 0.25;

fn ui_bounce(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut q: Query<(Entity, &mut UiBounce, Option<&mut UiTransform>)>,
) {
    for (entity, mut bounce, transform) in &mut q {
        bounce.t += time.delta_secs();
        let k = (bounce.t / BOUNCE_TIME).min(1.0);
        let scale = 1.0 + (k * std::f32::consts::PI).sin() * bounce.amount;
        match transform {
            Some(mut t) => t.scale = Vec2::splat(scale),
            None => _ = commands.entity(entity).insert(UiTransform::from_scale(Vec2::splat(scale))),
        }
        if k >= 1.0 {
            commands.entity(entity).remove::<UiBounce>();
            commands.entity(entity).insert(UiTransform::IDENTITY);
        }
    }
}
