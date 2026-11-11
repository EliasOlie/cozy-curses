//! Moradores: virtude de dia, pecado de noite — e eles não sabem.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::{Deserialize, Serialize};

use crate::cycle::{DayPhase, Season};
use crate::map::{Building, Map, MapState, Prop, depth, standing_at, tile_base};
use crate::sprites::{Art, DualSprite, Sprites};
use crate::Gameplay;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (wander, hide_inquisitor_at_night).in_set(Gameplay))
        .add_systems(OnEnter(DayPhase::Day), respawn)
        .add_systems(OnEnter(DayPhase::Night), respawn);
    app.add_systems(
        Update,
        respawn.in_set(crate::world::MapEntered).after(crate::world::enter_map).run_if(state_changed::<MapState>),
    );
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Baker,
    Guard,
    Mayor,
    Teacher,
    Merchant,
    Smith,
    Florist,
}

impl Kind {
    pub const ALL: [Kind; 7] =
        [Kind::Baker, Kind::Guard, Kind::Mayor, Kind::Teacher, Kind::Merchant, Kind::Smith, Kind::Florist];

    pub fn name(self, night: bool) -> &'static str {
        match (self, night) {
            (Kind::Baker, false) => "Seu Tobias, o Padeiro",
            (Kind::Baker, true) => "O Ghoul",
            (Kind::Guard, false) => "Xerife Bento",
            (Kind::Guard, true) => "O Cavaleiro Sem Cabeça",
            (Kind::Mayor, false) => "Prefeito Anselmo",
            (Kind::Mayor, true) => "O Sapo do Pântano",
            (Kind::Teacher, false) => "Dona Celeste, a Professora",
            (Kind::Teacher, true) => "A Condessa",
            (Kind::Merchant, false) => "Seu Bartô, o Mercador",
            (Kind::Merchant, true) => "O Dragão de Ossos",
            (Kind::Smith, false) => "Tião, o Ferreiro",
            (Kind::Smith, true) => "O Mímico",
            (Kind::Florist, false) => "Florinda, a Florista",
            (Kind::Florist, true) => "A Dríade",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Kind::Baker => "o Padeiro",
            Kind::Guard => "o Xerife",
            Kind::Mayor => "o Prefeito",
            Kind::Teacher => "a Professora",
            Kind::Merchant => "o Mercador",
            Kind::Smith => "o Ferreiro",
            Kind::Florist => "a Florista",
        }
    }

    fn building(self) -> Building {
        match self {
            Kind::Baker => Building::Bakery,
            Kind::Guard => Building::GuardPost,
            Kind::Mayor => Building::TownHall,
            Kind::Teacher => Building::Library,
            Kind::Merchant => Building::Store,
            Kind::Smith => Building::Smithy,
            Kind::Florist => Building::Florist,
        }
    }

    fn art(self, sprites: &Sprites) -> &Art {
        match self {
            Kind::Baker => &sprites.baker,
            Kind::Guard => &sprites.guard,
            Kind::Mayor => &sprites.mayor,
            Kind::Teacher => &sprites.teacher,
            Kind::Merchant => &sprites.merchant,
            Kind::Smith => &sprites.smith,
            Kind::Florist => &sprites.florist,
        }
    }

    /// Diálogos sazonais com múltiplas páginas, variando por estação, noite/dia, máscara e nível de maldição.
    pub fn seasonal_dialogue(
        self,
        season: Season,
        night: bool,
        masked: bool,
        _day: u32,
        curse: f32,
    ) -> Vec<String> {
        if night && !masked {
            return match self {
                Kind::Baker => vec!["Uma coisa magra e faminta fareja o ar na penumbra.".into(), "Seus dentes estalam: ela não te vê como gente, apenas como carne desprotegida.".into()],
                Kind::Guard => vec!["Uma armadura passa montada num corcel espectral, segurando a própria cabeça sob o braço.".into(), "Sem máscara, ela nem te nota como presença digna de atenção.".into()],
                Kind::Mayor => vec!["Um sapo colossal e viscoso ronca no meio do caminho.".into(), "Ele abre um olho amarelo fosforescente: 'Croaac... sem máscara você é só comida de pântano.'".into()],
                Kind::Teacher => vec!["Uma dama pálida em trajes carmesim te fita com desdém gélido:".into(), "\"Plebeu sem máscara vagando no meu domínio? Que falta de modos deplorável.\"".into()],
                Kind::Merchant => vec!["Ossos de dragão tilintam sobre pilhas de ouro e joias antigas.".into(), "Ele ruge baixo: 'Sem máscara? Carniça não negocia com o Dragão de Ossos.'".into()],
                Kind::Smith => vec!["Uma criatura com o seu exato jeito de andar te imita no escuro, um passo atrasada.".into(), "Sem máscara, seus movimentos são lentos e zombados pela sombra.".into()],
                Kind::Florist => vec!["Esporos dourados dançam em volta de uma silhueta de folhas sussurrantes.".into(), "A Dríade te fita: 'Sem máscara, mortal? O pólen noturno vai consumir sua mente.'".into()],
            };
        }

        if night && masked {
            return match self {
                Kind::Baker => vec![
                    "O Ghoul fareja sua máscara e rosna com satisfação:".into(),
                    "\"Você tem o cheiro do banquete proibido... Traga carnes assadas e pratos fortes da noite.\"".into(),
                ],
                Kind::Guard => vec![
                    "A cabeça, debaixo do braço da armadura, abre um sorriso flamejante:".into(),
                    "\"ORDEM! Traga-me essência e armas forjadas na escuridão. Esta vila precisa de disciplina de ferro.\"".into(),
                ],
                Kind::Mayor => vec![
                    "O Sapo do Pântano boceja e cospe bolhas de névoa escura:".into(),
                    "\"Croooac... você traz a face das sombras. Pague o pedágio dos pântanos ou traga essência arcanas.\"".into(),
                ],
                Kind::Teacher => vec![
                    "A Condessa curva a cabeça com elegância aristocrática:".into(),
                    "\"Agrade-me com produtos nobres da noite e eu tornarei suas máquinas e produções eficientes além da imaginação.\"".into(),
                ],
                Kind::Merchant => vec![
                    "O Dragão de Ossos afunda as garras na montanha de moedas com estalos metálicos:".into(),
                    "\"OURO! Traga sua essência e colheitas da noite e eu pago o triplo. O que a vila comerá amanhã não me importa!\"".into(),
                ],
                Kind::Smith => vec![
                    "O Mímico fala com a SUA própria voz distorcida:".into(),
                    "\"Adoro essa face... uma cópia perfeita! Me dê mais criações da forja e ferramentas afiadas.\"".into(),
                ],
                Kind::Florist => vec![
                    "A Dríade estende gavinhas brilhantes em sua direção:".into(),
                    "\"Respire fundo, mercador mascarado. Meus esporos arcanos fazem qualquer plantação render o dobro no escuro.\"".into(),
                ],
            };
        }

        // Diálogos diurnos por Estação do Ano
        let mut pages = match (self, season) {
            (Kind::Florist, Season::Spring) => vec![
                "Que manhã radiante! O orvalho da primavera desperta as sementes de morango e cenoura.".into(),
                "Se precisar de mudas novas para encher sua horta, passe na floricultura que separei as melhores!".into(),
            ],
            (Kind::Florist, Season::Summer) => vec![
                "O sol de verão está maravilhoso, mas esquenta rápido! As vinhas de uva e o milharal crescem a olhos vistos.".into(),
                "Lembre-se de regar a lavoura bem cedinho para a terra segurar a umidade.".into(),
            ],
            (Kind::Florist, Season::Autumn) => vec![
                "O vento de outono trouxe um tapete de folhas douradas. As abóboras e berinjelas estão no ponto ideal!".into(),
                "Tem algo de místico no outono... as flores da floresta parecem sussurrar canções ao entardecer.".into(),
            ],
            (Kind::Florist, Season::Winter) => vec![
                "Um frio de congelar as bochechas! A geada cobre os canteiros, mas as raízes de inverno e alho resistem bravamente.".into(),
                "Cobri o jardim com palha seca para as mudas não sofrerem com a neve.".into(),
            ],

            (Kind::Merchant, Season::Spring) => vec![
                "Feira aberta para a nova temporada! Primavera traz pastos verdes e animais jovens e saudáveis.".into(),
                "Tenho certidões de galinhas poedeiras, vacas e ovelhas. Paga quando puder, fiado de amigo é tradição!".into(),
            ],
            (Kind::Merchant, Season::Summer) => vec![
                "Calor danado no vale! O gado precisa de sombra e água limpa no cocho.".into(),
                "Minha loja está cheia de mantimentos e cortes frescos de açougue. Se tiver frutas sobrando, eu compro tudo!".into(),
            ],
            (Kind::Merchant, Season::Autumn) => vec![
                "Tempo de estocagem! O outono pede carnes defumadas e feno seco antes que os passos da serra congelem.".into(),
                "Engraçado... moedas do cofre sumiram de novo essa semana. Devo estar ficando esquecido com a idade.".into(),
            ],
            (Kind::Merchant, Season::Winter) => vec![
                "Brrr! A serra congelou inteira, nenhuma carroça sobe até o degelo. Nos viramos com o comércio local.".into(),
                "Mantenha o celeiro bem fechado e a lareira acesa. O vento de inverno não perdoa descuido.".into(),
            ],

            (Kind::Baker, Season::Spring) => vec![
                "O cheirinho de pão fresco pela manhã é a alma da primavera na vila!".into(),
                "Passe na padaria sempre que a energia fraquejar. Uma fornada quentinha renova as forças de qualquer fazendeiro.".into(),
            ],
            (Kind::Baker, Season::Summer) => vec![
                "O fogão a lenha no verão vira uma fornalha, mas as tortas de morango e mirtilo saem perfeitas!".into(),
                "Cozinhar com ingredientes frescos da sua horta é o que mantém este vilarejo feliz e bem nutrido.".into(),
            ],
            (Kind::Baker, Season::Autumn) => vec![
                "Outono é época da famosa torta de abóbora com canela e ensopados campeiros!".into(),
                "Não deixe faltar colheitas na despensa, a vila inteira vem atrás dos meus assados nessa época.".into(),
            ],
            (Kind::Baker, Season::Winter) => vec![
                "Aqueça as mãos no balcão, vizinho! No inverno rigoroso, uma sopa fumegante ou caldo quente salva vidas.".into(),
                "Ultimamente sinto um vazio estranho no estômago à noite... como uma fome antiga que não passa com pão comum.".into(),
            ],

            (Kind::Smith, Season::Spring) => vec![
                "Suas ferramentas estão prontas para a lida pesada da primavera? Se a enxada cegar, a forja está acesa!".into(),
                "Você cuida da terra com tanto talento. Meu ferro às vezes sai meio rústico, mas tem alma e dureza.".into(),
            ],
            (Kind::Smith, Season::Summer) => vec![
                "O fogo da forja com o sol de verão quase me derrete! Mas as lâminas de foice precisam de têmpera rigorosa.".into(),
                "Acordei com seu chapéu na minha oficina de novo... juro que não lembro como ele veio parar na bigorna!".into(),
            ],
            (Kind::Smith, Season::Autumn) => vec![
                "Machados bem afiados para colher madeira de outono! O inverno vem aí e lenha seca é riqueza pura.".into(),
                "Se encontrar pedras e minérios nas clareiras, traga para cá que reforçamos seu maquinário.".into(),
            ],
            (Kind::Smith, Season::Winter) => vec![
                "A bigorna fica gelada como gelo de manhã, mas quando o fole sopra carvão, este é o lugar mais acolhedor do vale.".into(),
                "Ferro bom nasce da paciência no frio e da fúria no fogo.".into(),
            ],

            (Kind::Teacher, Season::Spring) => vec![
                "A primavera sempre renova o espírito e os estudos botânicos na nossa biblioteca.".into(),
                "Gosto de servir às crianças e organizar nossos registros... embora às vezes sinta um gosto metálico estranho ao acordar.".into(),
            ],
            (Kind::Teacher, Season::Summer) => vec![
                "As crianças leem sob a copa fresca dos salgueiros no verão. Os anais de Vila Serena escondem lendas muito antigas.".into(),
                "Você já reparou naquelas ruínas misteriosas na encosta? Dizem que reis de eras esquecidas repousam ali.".into(),
            ],
            (Kind::Teacher, Season::Autumn) => vec![
                "Folhas secas marcam os capítulos dos meus alfarrábios favoritos. O outono inspira silêncio e memória.".into(),
                "Se sentir uma presença fria nas clareiras perto do sexto toque do sino, volte logo para casa.".into(),
            ],
            (Kind::Teacher, Season::Winter) => vec![
                "Lendo à luz de velas enquanto a neve suave bate nas vidraças. Histórias de condessas imortais aquecem o inverno.".into(),
                "O sangue e a história deste vale são nobres e profundos... nunca se esqueça de quem veio antes de nós.".into(),
            ],

            (Kind::Mayor, Season::Spring) => vec![
                "Bom dia, honrado produtor! Já carimbei 300 despachos de plantio hoje, o vale vai florescer com vigor!".into(),
                "Na Vila Serena o trabalho é constante! Só preciso descobrir por que meus sapatos amanhecem cheirando a lodo de brejo.".into(),
            ],
            (Kind::Mayor, Season::Summer) => vec![
                "Organizando o festival de verão da vila! Barracas de comida típica, desfile de animais e música até o entardecer.".into(),
                "Seus impostos e colheitas mantêm a ordem em dia. Continue assim e nossa comunidade será modelo em todo o reino!".into(),
            ],
            (Kind::Mayor, Season::Autumn) => vec![
                "Pilhas de inventários de safra para auditar neste outono. Burocracia bem feita é garantia de inverno sem fome!".into(),
                "Se notar algum sapo fora do comum perto da represa, não se espante... são apenas os guardiões da várzea.".into(),
            ],
            (Kind::Mayor, Season::Winter) => vec![
                "O fechamento fiscal de inverno exige atenção redobrada. Contas limpas evitam visitas indesejadas de fiscais reais.".into(),
                "Mantenha a fazenda próspera e afastada de boatos estranhos... não queremos o Inquisidor rondando nossos portões.".into(),
            ],

            (Kind::Guard, Season::Spring) => vec![
                "Patrulha tranquila na primavera! Os caminhos estão floridos e seguros, minha espada segue lustrosa na bainha.".into(),
                "Dormi pesado de novo e acordei com manchas de lama na couraça. Estranho, mas a brisa da manhã compensa tudo.".into(),
            ],
            (Kind::Guard, Season::Summer) => vec![
                "O sol refletindo no aço da armadura parece uma brasa viva! Mas o Xerife Bento nunca abandona o posto.".into(),
                "Fique atento com viajantes e tropeiros que passam ao crepúsculo. Nem tudo que vaga ao anoitecer carrega boas intenções.".into(),
            ],
            (Kind::Guard, Season::Autumn) => vec![
                "Névoa espessa rondando as encostas ao entardecer. Minha vista fica turva e as sombras parecem se mexer sozinhas.".into(),
                "Tranque o galinheiro e os portões quando o sino bater seis horas. O xerife vigia as ruas com honra.".into(),
            ],
            (Kind::Guard, Season::Winter) => vec![
                "O vento uiva como lobo faminto entre as guaritas. O chão congelado dificulta as rondas a cavalo.".into(),
                "Se ouvir cascos trotando na escuridão sem nenhum cavaleiro sobre a sela... apenas feche as cortinas e espere o dia clarear.".into(),
            ],
        };

        // Alerta especial de Maldição Alta (> 50%)
        if curse >= 50.0 {
            pages.push("...Você sente esse ar pesado e gélido que arrepia a nuca? A maldição do vale está se acumulando. Ouço dizer que o Inquisidor já prepara sua descida da serra.".into());
        }

        pages
    }

    /// Fala do dia (muda a cada dia) ou da noite (com ou sem máscara).
    #[allow(dead_code)]
    pub fn line(self, night: bool, masked: bool, day: u32) -> &'static str {
        let pick = |lines: &[&'static str]| lines[day as usize % lines.len()];
        if night && !masked {
            return match self {
                Kind::Baker => "Uma coisa magra e faminta fareja o ar. Ela não te vê como gente.",
                Kind::Guard => "Uma armadura passa carregando a própria cabeça. Sem máscara, ela nem te nota.",
                Kind::Mayor => "Um sapo enorme ronca no meio do caminho. Ele abre um olho e fecha de novo.",
                Kind::Teacher => "Uma dama pálida te olha de cima a baixo. \"Plebeu sem máscara. Que tédio.\"",
                Kind::Merchant => "Ossos de dragão tilintam sobre uma pilha de ouro. Ele não negocia com rostos.",
                Kind::Smith => "Uma coisa com o seu jeito de andar te imita, um passo atrasada.",
                Kind::Florist => "Esporos dourados dançam em volta de uma silhueta de folhas. Melhor prender a respiração.",
            };
        }
        match (self, night) {
            (Kind::Baker, _) => "...",
            (Kind::Guard, false) => pick(&[
                "Eu? Usar a espada? Deus me livre, ela é só enfeite.",
                "Dormi mal de novo... acordei com a armadura suja de lama. Estranho, né?",
                "Se acontecer alguma coisa, me chama! Eu chamo alguém. Alguém corajoso.",
            ]),
            (Kind::Guard, true) => pick(&[
                "A cabeça, debaixo do braço, sorri: \"ARMAS. Traga-me armas amaldiçoadas.\"",
                "\"Esta vila precisa de ORDEM. E de sangue. Mais ordem que sangue. Talvez.\"",
            ]),
            (Kind::Mayor, false) => pick(&[
                "Bom dia! Já carimbei 300 documentos hoje. Faltam só 4 mil!",
                "Descansar? Na Vila Serena a prefeitura nunca fecha. Nunca!",
                "Engraçado, toda manhã acordo com os pés molhados... e cheirando a pântano.",
            ]),
            (Kind::Mayor, true) => pick(&[
                "\"Croooac... pedágio... cogumelos... depois... zzzz.\"",
                "O sapo boceja. A ponte inteira treme. \"Volte... amanhã... ou nunca...\"",
            ]),
            (Kind::Teacher, false) => pick(&[
                "Ah, eu? Não sou ninguém especial. Só gosto de servir as crianças.",
                "Peguei mais um livro emprestado só para arrumar. Sou muito desorganizada.",
                "Acordei com um gosto metálico na boca... deve ser anemia, né?",
            ]),
            (Kind::Teacher, true) => pick(&[
                "\"Tributos, querido. Seda, ouro, sangue nobre. O resto é para os camponeses.\"",
                "\"Agrade-me e eu tornarei suas máquinas... eficientes. Desagrade-me e...\"",
            ]),
            (Kind::Merchant, false) => pick(&[
                "Leva, leva! Paga quando puder. Fiado é amizade!",
                "Preço justo é preço de amigo. E aqui todo mundo é amigo.",
                "Sumiram as moedas do cofre de novo. Devo estar ficando esquecido.",
            ]),
            (Kind::Merchant, true) => pick(&[
                "\"OURO. Traga sua essência e eu pago o triplo. O que a vila vai comer amanhã? Não me importa.\"",
                "O dragão afunda as garras na pilha de moedas. \"Meu. Meu. Tudo meu.\"",
            ]),
            (Kind::Smith, false) => pick(&[
                "Suas ferramentas são incríveis! As minhas? Ah, são só... ferro torto.",
                "Você é tão bom nisso tudo. Eu nunca vou ser assim.",
                "Acordei com seu chapéu na minha cabeça. Desculpa! Não sei como foi parar lá.",
            ]),
            (Kind::Smith, true) => pick(&[
                "O Mímico fala com a SUA voz: \"Me dá uma cópia. Uma cópia perfeita. Senão...\"",
                "\"Sua melhor cenoura. Igualzinha. Eu QUERO.\"",
            ]),
            (Kind::Florist, false) => pick(&[
                "Que dia lindo! As flores estão tão felizes hoje.",
                "Um rapaz me elogiou ontem e eu corei até as orelhas. Que vergonha!",
                "Às vezes acordo coberta de pólen. Deve ser o vento, né?",
            ]),
            (Kind::Florist, true) => pick(&[
                "\"Respire fundo, meu bem. Meus esporos fazem qualquer monstro trabalhar o dobro...\"",
                "\"Eles sempre voltam pedindo mais. Sempre.\"",
            ]),
        }
    }
}

#[derive(Component)]
pub struct Villager {
    pub kind: Kind,
    home: Vec2,
    t: f32,
    /// Raio do passeio em volta de casa (0 = parado atrás do balcão).
    roam: f32,
    /// Segundos parado antes de voltar a andar, e quanto já andou desde a última parada.
    rest: f32,
    walked: f32,
}

/// Forasteiro que audita suas finanças quando a vila amaldiçoa demais.
#[derive(Component)]
pub struct Inquisitor;

/// Recoloca os moradores do mapa atual (ao chegar num mapa e quando a fase muda).
fn respawn(
    mut commands: Commands,
    sprites: Res<Sprites>,
    map: Option<Res<Map>>,
    map_state: Option<Res<State<MapState>>>,
    phase: Option<Res<State<DayPhase>>>,
    existing: Query<Entity, With<Villager>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(map), Some(map_state), Some(phase)) = (map, map_state, phase) else { return };
    // O recurso pode ser de um mapa anterior durante a troca: confia só no estado.
    if map.id != map_state.get().0 {
        return;
    }
    let night = *phase.get() == DayPhase::Night;
    let world = crate::worlddef::world();
    for (i, kind) in Kind::ALL.into_iter().enumerate() {
        // De dia, quem atende num balcão fica lá dentro; à noite, todos saem.
        let works_inside = world.maps.iter().any(|m| m.keeper == Some(kind));
        let spot = if !night && map.info.keeper == Some(kind) {
            map.keeper_spot.map(|t| (tile_base(t), 0.0))
        } else if (!night && works_inside) || map.info.indoor {
            None
        } else {
            map.find(Prop::Building(kind.building()))
                .map(|door| (tile_base(door.anchor + IVec2::new(0, 2)), if night { 30.0 } else { 18.0 }))
        };
        let Some((home, roam)) = spot else { continue };
        commands
            .spawn((
                Villager { kind, home, t: i as f32 * 1.7, roam, rest: 0.0, walked: i as f32 * 0.6 },
                DualSprite::bundle(kind.art(&sprites), 0),
                Anchor::BOTTOM_CENTER,
                standing_at(home),
                DespawnOnExit(MapState(map.id.clone())),
            ))
            .with_child((DualSprite::bundle(&sprites.shadow, 0), Transform::from_xyz(0.0, 1.0, -0.01)));
    }
}

/// Passeiam em volta de casa; de noite, ficam inquietos e andam mais rápido.
/// Perto disso o morador para e olha para o jogador (dá tempo de conversar).
const NOTICE: f32 = 34.0;

fn wander(
    time: Res<Time>,
    phase: Res<State<DayPhase>>,
    player: Single<&Transform, (With<crate::player::Player>, Without<Villager>)>,
    mut villagers: Query<(&mut Villager, &mut Transform, &mut Sprite)>,
) {
    let you = player.translation.truncate();
    let speed = match phase.get() {
        DayPhase::Day => 0.4,
        DayPhase::Night => 0.9,
    };
    let dt = time.delta_secs();
    let night = *phase.get() == DayPhase::Night;
    for (mut v, mut transform, mut sprite) in &mut villagers {
        let here = transform.translation.truncate();
        // Linha atual da folha (para continuar olhando para o mesmo lado quando para).
        let row = sprite.texture_atlas.as_ref().map_or(0, |a| a.index / 3);
        let (row, col, flip) = if v.roam <= 0.0 {
            // Atrás do balcão: de frente, olhando para o jogador quando ele chega perto.
            let (row, flip) = if here.distance(you) < NOTICE * 2.0 { facing(you - here) } else { (0, false) };
            (row, 0, flip)
        } else if here.distance(you) < NOTICE {
            let (row, flip) = facing(you - here);
            (row, 0, flip)
        } else if v.rest > 0.0 {
            // Parado um pouco: olha em volta.
            v.rest -= dt;
            (row, 0, sprite.flip_x)
        } else {
            v.t += dt * speed;
            v.walked += dt;
            let p = v.home + Vec2::new(v.t.cos() * v.roam, v.t.sin() * v.roam * 0.4);
            let velocity = Vec2::new(-v.t.sin() * v.roam, v.t.cos() * v.roam * 0.4);
            transform.translation = p.extend(depth(p.y));
            // De vez em quando para (de noite, inquietos, quase não param).
            let walk_for = if night { 9.0 } else { 4.0 };
            if v.walked > walk_for {
                v.walked = 0.0;
                let r = (v.t * 12.9898).sin() * 43758.547;
                v.rest = if night { 0.6 } else { 1.5 } + (r - r.floor()) * 2.5;
            }
            let (row, flip) = facing(velocity);
            let step = [0, 1, 0, 2][(v.t / speed * 6.0) as usize % 4];
            (row, step, flip)
        };
        sprite.flip_x = flip;
        if let Some(atlas) = &mut sprite.texture_atlas {
            let frame = row * 3 + col;
            if atlas.index != frame {
                atlas.index = frame;
            }
        }
    }
}

/// Linha da folha 3x3 (0 frente, 1 costas, 2 lado direito) e se espelha, para andar/olhar em `dir`.
fn facing(dir: Vec2) -> (usize, bool) {
    if dir.y.abs() > dir.x.abs() {
        (if dir.y < 0.0 { 0 } else { 1 }, false)
    } else {
        (2, dir.x < 0.0)
    }
}

pub fn spawn_inquisitor(commands: &mut Commands, sprites: &Sprites, scope: &str, tile: IVec2) {
    commands.spawn((
        Inquisitor,
        sprites.inquisitor.sprite(DayPhase::Day, 0),
        Anchor::BOTTOM_CENTER,
        standing_at(tile_base(tile)),
        DespawnOnExit(MapState(scope.to_string())),
    ));
}

fn hide_inquisitor_at_night(
    phase: Res<State<DayPhase>>,
    mut inquisitors: Query<&mut Visibility, With<Inquisitor>>,
) {
    let visibility = match phase.get() {
        DayPhase::Day => Visibility::Inherited,
        DayPhase::Night => Visibility::Hidden,
    };
    for mut v in &mut inquisitors {
        v.set_if_neq(visibility);
    }
}
