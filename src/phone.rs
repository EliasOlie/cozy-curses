//! O celular: rolar o feed da vila faz o tempo passar (doomscroll).
//! Cada rolada adianta 10 minutos de jogo. O contador fica guardado para virar mecânica.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::cycle::{Clock, DayPhase};
use crate::economy::Game;
use crate::sprites::Sprites;
use crate::ui::{TEXT, font};
use crate::{AppState, Overlay};

pub fn plugin(app: &mut App) {
    app.init_resource::<Feed>().add_systems(
        Update,
        (toggle, scroll, redraw).chain().run_if(in_state(crate::Pause::Running)),
    );
}

const MINUTES_PER_SCROLL: f32 = 10.0;
const SHOWN: usize = 4;
/// O desenho do celular (art/sprites/phone.json) ampliado 3x; a tela é x 8..71, y 16..111.
const SCALE: f32 = 3.0;

const DAY_POSTS: &[&str] = &[
    "Seu Tobias: Fornada nova! Pão grátis pra quem passar. #gratidão",
    "Prefeito Anselmo: 312 carimbos hoje. Quem dorme não carimba! #produtividade",
    "Florinda: as margaridas acordaram tão felizes (e eu toda suja de pólen?)",
    "Xerife Bento: Reunião sobre segurança CANCELADA. Motivo: medo.",
    "Dona Celeste: Peguei 40 livros 'emprestados' pra arrumar. Sou tão bagunceira!",
    "Seu Bartô: PROMOÇÃO: tudo fiado! Pague quando puder (ou nunca, amigo).",
    "Tião Ferreiro: fiz uma ferradura torta. de novo. desculpa, cavalos.",
    "ANÚNCIO: Sementes de cenoura no Armazém. Cenouras não têm olhos. Garantido.",
    "Vila Serena: nossa vila foi eleita a mais gentil da região pelo 99º ano!",
    "Alguém viu minhas galinhas? Elas sumiram de novo depois da meia-noite.",
    "Dica do dia: regue suas plantas! Plantas regadas não sussurram.",
    "Prefeito Anselmo: alguém deixou lama no meu travesseiro. Quem foi??",
    "Florinda: um moço me elogiou e eu corei até a raiz do cabelo",
    "Enquete: o que é melhor, pão de queijo ou pão de queijo?",
];

const NIGHT_POSTS: &[&str] = &[
    "███: a fome não dorme. a fome nunca dorme.",
    "A CONDESSA: Tributos até o amanhecer. Seda, ouro ou sangue nobre.",
    "[post apagado pelo Inquisidor]",
    "O DRAGÃO DE OSSOS: COMPRO ESSÊNCIA. PAGO EM OURO DA VILA. NÃO PERGUNTE.",
    "O Cavaleiro Sem Cabeça curtiu sua foto. E a foto da sua cabeça.",
    "anúncio: Trem Fantasma parte às 3h. Exportações B2B com 30% de desconto.",
    "A Dríade: respire fundo... meus esporos fazem tudo trabalhar o dobro",
    "croooac... pedágio... cogumelos... zzz",
    "O Mímico postou uma foto sua. Você não lembra de ter tirado.",
    "Trending: #NinguémLembraDeNada #VilaSerena #DormirÉPraFracos",
    "Lembrete: o sol nasce às 6h. Até lá, tudo é permitido.",
    "Sua fornalha favoritou este post",
];

#[derive(Resource, Default)]
struct Feed {
    posts: Vec<String>,
    seed: usize,
}

impl Feed {
    fn push_post(&mut self, night: bool) {
        let list = if night { NIGHT_POSTS } else { DAY_POSTS };
        self.seed = self.seed.wrapping_mul(31).wrapping_add(17);
        self.posts.push(list[self.seed % list.len()].to_string());
        if self.posts.len() > SHOWN {
            self.posts.remove(0);
        }
    }
}

#[derive(Component)]
struct PhoneUi;

/// F abre/fecha o celular.
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<Overlay>,
    mut feed: ResMut<Feed>,
    phase: Res<State<DayPhase>>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    *overlay = match *overlay {
        Overlay::None => {
            if feed.posts.is_empty() {
                let night = *phase.get() == DayPhase::Night;
                for _ in 0..SHOWN {
                    feed.push_post(night);
                }
            }
            Overlay::Phone
        }
        Overlay::Phone => Overlay::None,
        other => other,
    };
}

/// Rolar o feed: cada passo adianta o relógio.
fn scroll(
    overlay: Res<Overlay>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut feed: ResMut<Feed>,
    mut clock: ResMut<Clock>,
    mut game: ResMut<Game>,
) {
    let wheel_down = wheel.read().filter(|w| w.y < 0.0).count();
    if *overlay != Overlay::Phone {
        return;
    }
    let keys_down = [KeyCode::ArrowDown, KeyCode::KeyS].iter().filter(|k| keys.just_pressed(**k)).count();
    for _ in 0..(wheel_down + keys_down) {
        clock.advance(MINUTES_PER_SCROLL);
        game.doomscrolls += 1;
        feed.push_post(clock.minutes >= crate::cycle::HALF_DAY);
    }
}

fn redraw(
    mut commands: Commands,
    sprites: Res<Sprites>,
    overlay: Res<Overlay>,
    feed: Res<Feed>,
    clock: Res<Clock>,
    game: Res<Game>,
    ui: Query<Entity, With<PhoneUi>>,
) {
    if !(overlay.is_changed() || feed.is_changed()) {
        return;
    }
    for entity in &ui {
        commands.entity(entity).despawn();
    }
    if *overlay != Overlay::Phone {
        return;
    }
    commands
        .spawn((
            PhoneUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            GlobalZIndex(7),
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|root| {
            root.spawn(Node { width: Val::Px(80.0 * SCALE), height: Val::Px(128.0 * SCALE), ..default() })
                .with_children(|phone| {
                    // Fundo da tela, o desenho por cima e o feed dentro da tela.
                    let screen = |node: Node| Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(8.0 * SCALE),
                        top: Val::Px(16.0 * SCALE),
                        width: Val::Px(64.0 * SCALE),
                        height: Val::Px(96.0 * SCALE),
                        ..node
                    };
                    phone.spawn((screen(Node::default()), BackgroundColor(Color::srgb(0.95, 0.93, 0.88))));
                    phone.spawn((
                        ImageNode::new(sprites.phone.clone()),
                        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    ));
                    phone
                        .spawn(screen(Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(6.0)),
                            row_gap: Val::Px(6.0),
                            overflow: Overflow::clip(),
                            ..default()
                        }))
                        .with_children(|feed_ui| {
                            feed_ui.spawn((
                                Text::new(format!("Feed da Vila · {}", clock.hour_label())),
                                font(&sprites, 18.0),
                                TextColor(Color::srgb(0.45, 0.3, 0.6)),
                            ));
                            for post in feed.posts.iter().rev() {
                                feed_ui.spawn((
                                    Text::new(post.clone()),
                                    font(&sprites, 17.0),
                                    TextColor(Color::srgb(0.18, 0.12, 0.1)),
                                ));
                            }
                        });
                });
            root.spawn((
                Text::new(format!(
                    "Role para baixo: +10 min por post · {} posts rolados\n[F] guardar o celular",
                    game.doomscrolls
                )),
                font(&sprites, 20.0),
                TextColor(TEXT),
                TextShadow { offset: Vec2::splat(2.0), color: Color::BLACK },
                TextLayout::justify(Justify::Center),
                Node { margin: UiRect::left(Val::Px(18.0)), ..default() },
            ));
        });
}
