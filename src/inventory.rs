//! Itens, mochila (30 espaços) e a barra de itens (os 10 primeiros espaços).

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Gameplay;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (select_slot, refill_hotbar).in_set(Gameplay));
}

/// Repõe sozinho o espaço da barra que esvaziou enquanto estava em uso.
fn refill_hotbar(
    mut inv: ResMut<Inventory>,
    mut game: Option<ResMut<crate::economy::Game>>,
    overlay: Res<crate::Overlay>,
    mut last: Local<Option<(usize, Item)>>,
) {
    let now = inv.selected_item().map(|item| (inv.selected, item));
    // Acabou de fechar a mochila ou a loja: o espaço pode ter sido esvaziado de propósito.
    if !overlay.is_changed()
        && now.is_none()
        && let Some((slot, item)) = *last
        && slot == inv.selected
        && !item.is_tool()
        && inv.refill_selected(item)
        && let Some(game) = &mut game
    {
        game.say(format!("{}: peguei mais da mochila.", item.name()));
    }
    *last = inv.selected_item().map(|item| (inv.selected, item));
}

pub const SLOTS: usize = 30;
pub const HOTBAR: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Item {
    // Ferramentas
    Hoe,
    WateringCan,
    Axe,
    Pickaxe,
    Scythe,
    /// Arma: a espada que o Xerife nunca usou (de dia).
    Sword,

    // Sementes por estação
    // Primavera
    CarrotSeeds,
    StrawberrySeeds,
    CauliflowerSeeds,
    PotatoSeeds,
    // Verão
    CornSeeds,
    TomatoSeeds,
    BlueberrySeeds,
    MelonSeeds,
    // Outono
    PumpkinSeeds,
    EggplantSeeds,
    GrapeSeeds,
    SweetPotatoSeeds,
    // Inverno
    WinterRootSeeds,
    WinterCabbageSeeds,
    GarlicSeeds,

    // Colheitas e frutos
    Carrot,
    Strawberry,
    Cauliflower,
    Potato,
    Corn,
    Tomato,
    Blueberry,
    Melon,
    Pumpkin,
    Eggplant,
    Grape,
    SweetPotato,
    WinterRoot,
    WinterCabbage,
    Garlic,

    // Recursos
    Wood,
    Stone,
    Fiber,

    // Produtos de animais
    CowMilk,
    GoatMilk,
    Egg,
    Wool,
    RabbitWool,

    // Açougue e carnes
    Beef,
    ChickenMeat,
    Mutton,
    RabbitMeat,

    // Culinária (comidas prontas)
    Bread,
    Cake,
    GardenSalad,
    CountryStew,
    StrawberryPie,
    HerbOmelet,
    PumpkinSoup,
    RoastChicken,
    HeartyRoast,
    BlueberryJam,

    // Certidões de animais (para comprar e colocar na fazenda)
    ChickenDeed,
    RoosterDeed,
    CowDeed,
    BullDeed,
    SheepDeed,
    RamDeed,
    GoatDeed,
    BuckDeed,
    DoeRabbitDeed,
    BuckRabbitDeed,
    MareDeed,
    StallionDeed,

    // Estruturas e ferramentas fabricáveis
    WoodFence,
    StorageChest,
    Scarecrow,
    Sprinkler,

    // Alquimia e consumíveis arcanos
    EnergyElixir,
    CalmingIncense,
    ArcaneFertilizer,

    // Máscaras arcanas colecionáveis e equipáveis
    GargoyleMask,
    CrowMask,
    GhostMask,
    GoldenDragonMask,
    RusticStrawMask,
}

/// Variantes de máscaras arcanas com bônus e lore distintos.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum MaskKind {
    #[default]
    Gargoyle,
    Crow,
    Ghost,
    GoldenDragon,
    RusticStraw,
}

impl MaskKind {
    pub const ALL: [MaskKind; 5] = [
        MaskKind::Gargoyle,
        MaskKind::Crow,
        MaskKind::Ghost,
        MaskKind::GoldenDragon,
        MaskKind::RusticStraw,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MaskKind::Gargoyle => "Máscara de Gárgula",
            MaskKind::Crow => "Máscara do Corvo",
            MaskKind::Ghost => "Máscara da Névoa",
            MaskKind::GoldenDragon => "Máscara do Dragão Dourado",
            MaskKind::RusticStraw => "Máscara Rústica",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MaskKind::Gargoyle => "Entalhada em pedra sombria. Oculta sua face humana para circular à noite.",
            MaskKind::Crow => "Inspirada nos médicos da peste. Bico com filtro de miasmas e ervas purificadoras.",
            MaskKind::Ghost => "Tecida com névoas lunares. Torna seus passos etéreos e velozes no escuro.",
            MaskKind::GoldenDragon => "Folheada em ouro reluzente. Encanta os olhos de mercadores gananciosos.",
            MaskKind::RusticStraw => "Trançada em palha mágica e carvalho. Harmoniza seu trabalho com o solo fértil.",
        }
    }

    pub fn bonus_text(self) -> &'static str {
        match self {
            MaskKind::Gargoyle => "Comércio Noturno livre e imunidade a hostilidade.",
            MaskKind::Crow => "+35% de resistência ao acúmulo de maldição noturna.",
            MaskKind::Ghost => "+20% de velocidade de movimento ao caminhar à noite.",
            MaskKind::GoldenDragon => "+25% de ouro em vendas noturnas para o Dragão de Ossos.",
            MaskKind::RusticStraw => "+15% de chance de qualidade superior em colheitas.",
        }
    }

    /// Quadro da folha `items` (ordem dos quadros 69-73 = ordem do enum).
    pub fn icon(self) -> usize {
        69 + self as usize
    }

    pub fn color(self) -> Color {
        match self {
            MaskKind::Gargoyle => Color::srgb(0.70, 0.72, 0.75),
            MaskKind::Crow => Color::srgb(0.28, 0.22, 0.38),
            MaskKind::Ghost => Color::srgb(0.65, 0.85, 0.95),
            MaskKind::GoldenDragon => Color::srgb(0.95, 0.80, 0.20),
            MaskKind::RusticStraw => Color::srgb(0.85, 0.70, 0.35),
        }
    }
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Hoe => "Enxada",
            Item::WateringCan => "Regador",
            Item::Axe => "Machado",
            Item::Pickaxe => "Picareta",
            Item::Scythe => "Foice",
            Item::Sword => "Espada Enferrujada",

            Item::CarrotSeeds => "Sementes de Cenoura",
            Item::StrawberrySeeds => "Mudas de Morango",
            Item::CauliflowerSeeds => "Sementes de Couve-flor",
            Item::PotatoSeeds => "Batatas para Plantar",

            Item::CornSeeds => "Sementes de Milho",
            Item::TomatoSeeds => "Mudas de Tomate",
            Item::BlueberrySeeds => "Mudas de Mirtilo",
            Item::MelonSeeds => "Sementes de Melão",

            Item::PumpkinSeeds => "Sementes de Abóbora",
            Item::EggplantSeeds => "Sementes de Berinjela",
            Item::GrapeSeeds => "Mudas de Videira (Uva)",
            Item::SweetPotatoSeeds => "Ramas de Batata-Doce",

            Item::WinterRootSeeds => "Sementes de Raiz de Inverno",
            Item::WinterCabbageSeeds => "Sementes de Repolho de Inverno",
            Item::GarlicSeeds => "Dentes de Alho de Inverno",

            Item::Carrot => "Cenoura",
            Item::Strawberry => "Morango",
            Item::Cauliflower => "Couve-flor",
            Item::Potato => "Batata",
            Item::Corn => "Milho",
            Item::Tomato => "Tomate",
            Item::Blueberry => "Mirtilo",
            Item::Melon => "Melão",
            Item::Pumpkin => "Abóbora",
            Item::Eggplant => "Berinjela",
            Item::Grape => "Uva",
            Item::SweetPotato => "Batata-Doce",
            Item::WinterRoot => "Raiz de Inverno",
            Item::WinterCabbage => "Repolho de Inverno",
            Item::Garlic => "Alho",

            Item::Wood => "Madeira",
            Item::Stone => "Pedra",
            Item::Fiber => "Fibra",

            Item::CowMilk => "Leite de Vaca",
            Item::GoatMilk => "Leite de Cabra",
            Item::Egg => "Ovo de Galinha",
            Item::Wool => "Lã de Ovelha",
            Item::RabbitWool => "Pêlo Macio de Coelho",

            Item::Beef => "Carne Bovina",
            Item::ChickenMeat => "Carne de Frango",
            Item::Mutton => "Carne de Cordeiro",
            Item::RabbitMeat => "Carne de Coelho",

            Item::Bread => "Pão",
            Item::Cake => "Bolo",
            Item::GardenSalad => "Salada da Estação",
            Item::CountryStew => "Ensopado Campeiro",
            Item::StrawberryPie => "Torta de Morango",
            Item::HerbOmelet => "Omelete de Ervas",
            Item::PumpkinSoup => "Sopa de Abóbora",
            Item::RoastChicken => "Frango Assado com Batatas",
            Item::HeartyRoast => "Assado Campeiro de Cordeiro",
            Item::BlueberryJam => "Geleia de Mirtilo",

            Item::ChickenDeed => "Galinha Poedeira (fêmea)",
            Item::RoosterDeed => "Galo Cantor (macho)",
            Item::CowDeed => "Vaca Leiteira (fêmea)",
            Item::BullDeed => "Touro Reprodutor (macho)",
            Item::SheepDeed => "Ovelha de Lã (fêmea)",
            Item::RamDeed => "Carneiro (macho)",
            Item::GoatDeed => "Cabra Leiteira (fêmea)",
            Item::BuckDeed => "Bode Montanhês (macho)",
            Item::DoeRabbitDeed => "Coelha Fofa (fêmea)",
            Item::BuckRabbitDeed => "Coelho Saltitante (macho)",
            Item::MareDeed => "Égua de Campo (fêmea)",
            Item::StallionDeed => "Garanhão (macho)",

            Item::WoodFence => "Cerca de Madeira",
            Item::StorageChest => "Baú Rústico",
            Item::Scarecrow => "Espantalho Amigável",
            Item::Sprinkler => "Aspersor de Irrigação",

            Item::EnergyElixir => "Elixir Revigorante",
            Item::CalmingIncense => "Incenso Calmante",
            Item::ArcaneFertilizer => "Fertilizante Arcano",

            Item::GargoyleMask => "Máscara de Gárgula",
            Item::CrowMask => "Máscara do Corvo",
            Item::GhostMask => "Máscara da Névoa",
            Item::GoldenDragonMask => "Máscara do Dragão Dourado",
            Item::RusticStrawMask => "Máscara Rústica",
        }
    }

    /// Quadro da spritesheet `items` (art/CONTRACT.md).
    pub fn icon(self) -> usize {
        match self {
            Item::Hoe => 0,
            Item::WateringCan => 1,
            Item::Axe => 2,
            Item::Pickaxe => 3,
            Item::Scythe => 4,
            Item::Sword => 74,

            Item::CarrotSeeds => 5,
            Item::StrawberrySeeds => 16,
            Item::CauliflowerSeeds => 17,
            Item::PotatoSeeds => 18,
            Item::CornSeeds => 19,
            Item::TomatoSeeds => 20,
            Item::BlueberrySeeds => 21,
            Item::MelonSeeds => 22,
            Item::PumpkinSeeds => 23,
            Item::EggplantSeeds => 24,
            Item::GrapeSeeds => 25,
            Item::SweetPotatoSeeds => 26,
            Item::WinterRootSeeds => 27,
            Item::WinterCabbageSeeds => 28,
            Item::GarlicSeeds => 29,

            Item::Carrot => 6,
            Item::Strawberry => 30,
            Item::Cauliflower => 31,
            Item::Potato => 32,
            Item::Corn => 33,
            Item::Tomato => 34,
            Item::Blueberry => 35,
            Item::Melon => 36,
            Item::Pumpkin => 37,
            Item::Eggplant => 38,
            Item::Grape => 39,
            Item::SweetPotato => 40,
            Item::WinterRoot => 41,
            Item::WinterCabbage => 42,
            Item::Garlic => 43,

            Item::Wood => 7,
            Item::Stone => 8,
            Item::Fiber => 9,

            Item::CowMilk => 44,
            Item::GoatMilk => 45,
            Item::Egg => 46,
            Item::Wool => 47,
            Item::RabbitWool => 48,

            Item::Beef => 49,
            Item::ChickenMeat => 50,
            Item::Mutton => 51,
            Item::RabbitMeat => 52,

            Item::Bread => 10,
            Item::Cake => 11,
            Item::StrawberryPie => 53,
            Item::GardenSalad => 54,
            Item::CountryStew => 55,
            Item::HerbOmelet => 56,
            Item::PumpkinSoup => 57,
            Item::RoastChicken => 58,
            Item::HeartyRoast => 59,
            Item::BlueberryJam => 60,

            Item::ChickenDeed
            | Item::RoosterDeed
            | Item::CowDeed
            | Item::BullDeed
            | Item::SheepDeed
            | Item::RamDeed
            | Item::GoatDeed
            | Item::BuckDeed
            | Item::DoeRabbitDeed
            | Item::BuckRabbitDeed
            | Item::MareDeed
            | Item::StallionDeed => 61,

            Item::WoodFence => 62,
            Item::StorageChest => 63,
            Item::Scarecrow => 64,
            Item::Sprinkler => 65,

            Item::EnergyElixir => 66,
            Item::CalmingIncense => 67,
            Item::ArcaneFertilizer => 68,

            Item::GargoyleMask => 69,
            Item::CrowMask => 70,
            Item::GhostMask => 71,
            Item::GoldenDragonMask => 72,
            Item::RusticStrawMask => 73,
        }
    }

    pub fn is_tool(self) -> bool {
        matches!(self, Item::Hoe | Item::WateringCan | Item::Axe | Item::Pickaxe | Item::Scythe | Item::Sword)
    }

    /// Dano por golpe, se o item serve de arma (a foice também corta pragas).
    pub fn weapon_damage(self) -> Option<f32> {
        match self {
            Item::Sword => Some(2.0),
            Item::Scythe => Some(1.0),
            _ => None,
        }
    }

    pub fn is_mask(self) -> bool {
        matches!(
            self,
            Item::GargoyleMask
                | Item::CrowMask
                | Item::GhostMask
                | Item::GoldenDragonMask
                | Item::RusticStrawMask
        )
    }

    pub fn mask_kind(self) -> Option<MaskKind> {
        match self {
            Item::GargoyleMask => Some(MaskKind::Gargoyle),
            Item::CrowMask => Some(MaskKind::Crow),
            Item::GhostMask => Some(MaskKind::Ghost),
            Item::GoldenDragonMask => Some(MaskKind::GoldenDragon),
            Item::RusticStrawMask => Some(MaskKind::RusticStraw),
            _ => None,
        }
    }

    pub fn is_consumable(self) -> bool {
        self.energy().is_some() || matches!(self, Item::EnergyElixir | Item::CalmingIncense | Item::ArcaneFertilizer)
    }

    pub fn is_structure(self) -> bool {
        matches!(self, Item::WoodFence | Item::StorageChest | Item::Scarecrow | Item::Sprinkler)
    }

    pub fn is_animal_deed(self) -> bool {
        matches!(
            self,
            Item::ChickenDeed
                | Item::RoosterDeed
                | Item::CowDeed
                | Item::BullDeed
                | Item::SheepDeed
                | Item::RamDeed
                | Item::GoatDeed
                | Item::BuckDeed
                | Item::DoeRabbitDeed
                | Item::BuckRabbitDeed
                | Item::MareDeed
                | Item::StallionDeed
        )
    }

    /// Ordem e nome do tipo do item (para organizar a mochila e para o cartão de detalhes).
    pub fn category(self) -> (u8, &'static str) {
        if self.is_tool() {
            (0, "Ferramenta")
        } else if self.seed_crop().is_some() {
            (1, "Semente")
        } else if self.is_structure() {
            (2, "Estrutura")
        } else if matches!(self, Item::Wood | Item::Stone | Item::Fiber) {
            (3, "Recurso")
        } else if matches!(self, Item::CowMilk | Item::GoatMilk | Item::Egg | Item::Wool | Item::RabbitWool) {
            (5, "Produto animal")
        } else if matches!(self, Item::Beef | Item::ChickenMeat | Item::Mutton | Item::RabbitMeat) {
            (6, "Carne")
        } else if matches!(self, Item::EnergyElixir | Item::CalmingIncense | Item::ArcaneFertilizer) {
            (8, "Alquimia")
        } else if self.is_animal_deed() {
            (9, "Certidão de animal")
        } else if self.is_mask() {
            (10, "Máscara")
        } else if self.energy().is_some_and(|e| e >= 25.0) {
            (7, "Comida")
        } else {
            (4, "Colheita")
        }
    }

    fn max_stack(self) -> u32 {
        if self.is_tool() || self.is_animal_deed() || self.is_mask() { 1 } else { 99 }
    }

    /// Quanto a loja paga (itens sem preço não são vendidos).
    pub fn sell_price(self) -> Option<i32> {
        match self {
            Item::Carrot => Some(6),
            Item::Strawberry => Some(12),
            Item::Cauliflower => Some(15),
            Item::Potato => Some(8),
            Item::Corn => Some(10),
            Item::Tomato => Some(9),
            Item::Blueberry => Some(14),
            Item::Melon => Some(22),
            Item::Pumpkin => Some(24),
            Item::Eggplant => Some(11),
            Item::Grape => Some(13),
            Item::SweetPotato => Some(9),
            Item::WinterRoot => Some(10),
            Item::WinterCabbage => Some(14),
            Item::Garlic => Some(16),

            Item::Wood | Item::Stone => Some(2),
            Item::Fiber => Some(1),

            Item::WoodFence => Some(6),
            Item::StorageChest => Some(25),
            Item::Scarecrow => Some(20),
            Item::Sprinkler => Some(35),

            Item::EnergyElixir => Some(18),
            Item::CalmingIncense => Some(22),
            Item::ArcaneFertilizer => Some(20),

            Item::GargoyleMask => Some(45),
            Item::CrowMask => Some(60),
            Item::GhostMask => Some(70),
            Item::GoldenDragonMask => Some(110),
            Item::RusticStrawMask => Some(40),

            Item::CowMilk => Some(14),
            Item::GoatMilk => Some(18),
            Item::Egg => Some(8),
            Item::Wool => Some(20),
            Item::RabbitWool => Some(22),

            Item::Beef => Some(28),
            Item::ChickenMeat => Some(16),
            Item::Mutton => Some(24),
            Item::RabbitMeat => Some(18),

            Item::GardenSalad => Some(32),
            Item::CountryStew => Some(55),
            Item::StrawberryPie => Some(42),
            Item::HerbOmelet => Some(30),
            Item::PumpkinSoup => Some(45),
            Item::RoastChicken => Some(50),
            Item::HeartyRoast => Some(65),
            Item::BlueberryJam => Some(28),
            Item::Bread => Some(5),
            Item::Cake => Some(15),

            _ => None,
        }
    }

    /// Energia recuperada ao comer.
    pub fn energy(self) -> Option<f32> {
        match self {
            // Pratos prontos
            Item::HeartyRoast => Some(100.0),
            Item::CountryStew => Some(95.0),
            Item::RoastChicken => Some(90.0),
            Item::StrawberryPie => Some(75.0),
            Item::PumpkinSoup => Some(65.0),
            Item::Cake => Some(60.0),
            Item::HerbOmelet => Some(50.0),
            Item::GardenSalad => Some(45.0),
            Item::BlueberryJam => Some(40.0),
            Item::Bread => Some(25.0),
            Item::EnergyElixir => Some(50.0),

            // Laticínios e ovos
            Item::GoatMilk => Some(22.0),
            Item::CowMilk => Some(18.0),
            Item::Egg => Some(12.0),

            // Frutas e vegetais frescos
            Item::Melon => Some(20.0),
            Item::Blueberry => Some(12.0),
            Item::Grape => Some(12.0),
            Item::Strawberry => Some(10.0),
            Item::Cauliflower => Some(10.0),
            Item::Corn => Some(10.0),
            Item::SweetPotato => Some(10.0),
            Item::WinterCabbage => Some(10.0),
            Item::Carrot => Some(8.0),
            Item::Tomato => Some(8.0),
            Item::WinterRoot => Some(8.0),
            Item::Potato => Some(6.0),
            Item::Garlic => Some(6.0),

            _ => None,
        }
    }

    pub fn seed_crop(self) -> Option<CropKind> {
        match self {
            Item::CarrotSeeds => Some(CropKind::Carrot),
            Item::StrawberrySeeds => Some(CropKind::Strawberry),
            Item::CauliflowerSeeds => Some(CropKind::Cauliflower),
            Item::PotatoSeeds => Some(CropKind::Potato),
            Item::CornSeeds => Some(CropKind::Corn),
            Item::TomatoSeeds => Some(CropKind::Tomato),
            Item::BlueberrySeeds => Some(CropKind::Blueberry),
            Item::MelonSeeds => Some(CropKind::Melon),
            Item::PumpkinSeeds => Some(CropKind::Pumpkin),
            Item::EggplantSeeds => Some(CropKind::Eggplant),
            Item::GrapeSeeds => Some(CropKind::Grape),
            Item::SweetPotatoSeeds => Some(CropKind::SweetPotato),
            Item::WinterRootSeeds => Some(CropKind::WinterRoot),
            Item::WinterCabbageSeeds => Some(CropKind::WinterCabbage),
            Item::GarlicSeeds => Some(CropKind::Garlic),
            _ => None,
        }
    }

    pub fn deed_animal(self) -> Option<(crate::animals::Species, crate::animals::Gender)> {
        use crate::animals::{Gender, Species};
        match self {
            Item::ChickenDeed => Some((Species::Chicken, Gender::Female)),
            Item::RoosterDeed => Some((Species::Chicken, Gender::Male)),
            Item::CowDeed => Some((Species::Cow, Gender::Female)),
            Item::BullDeed => Some((Species::Cow, Gender::Male)),
            Item::SheepDeed => Some((Species::Sheep, Gender::Female)),
            Item::RamDeed => Some((Species::Sheep, Gender::Male)),
            Item::GoatDeed => Some((Species::Goat, Gender::Female)),
            Item::BuckDeed => Some((Species::Goat, Gender::Male)),
            Item::DoeRabbitDeed => Some((Species::Rabbit, Gender::Female)),
            Item::BuckRabbitDeed => Some((Species::Rabbit, Gender::Male)),
            Item::MareDeed => Some((Species::Horse, Gender::Female)),
            Item::StallionDeed => Some((Species::Horse, Gender::Male)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CropKind {
    Carrot,
    Strawberry,
    Cauliflower,
    Potato,
    Corn,
    Tomato,
    Blueberry,
    Melon,
    Pumpkin,
    Eggplant,
    Grape,
    SweetPotato,
    WinterRoot,
    WinterCabbage,
    Garlic,
}

impl CropKind {
    pub fn name(self) -> &'static str {
        match self {
            CropKind::Carrot => "Cenoura",
            CropKind::Strawberry => "Morango",
            CropKind::Cauliflower => "Couve-flor",
            CropKind::Potato => "Batata",
            CropKind::Corn => "Milho",
            CropKind::Tomato => "Tomate",
            CropKind::Blueberry => "Mirtilo",
            CropKind::Melon => "Melão",
            CropKind::Pumpkin => "Abóbora",
            CropKind::Eggplant => "Berinjela",
            CropKind::Grape => "Uva",
            CropKind::SweetPotato => "Batata-Doce",
            CropKind::WinterRoot => "Raiz de Inverno",
            CropKind::WinterCabbage => "Repolho de Inverno",
            CropKind::Garlic => "Alho",
        }
    }

    pub fn harvest_item(self) -> Item {
        match self {
            CropKind::Carrot => Item::Carrot,
            CropKind::Strawberry => Item::Strawberry,
            CropKind::Cauliflower => Item::Cauliflower,
            CropKind::Potato => Item::Potato,
            CropKind::Corn => Item::Corn,
            CropKind::Tomato => Item::Tomato,
            CropKind::Blueberry => Item::Blueberry,
            CropKind::Melon => Item::Melon,
            CropKind::Pumpkin => Item::Pumpkin,
            CropKind::Eggplant => Item::Eggplant,
            CropKind::Grape => Item::Grape,
            CropKind::SweetPotato => Item::SweetPotato,
            CropKind::WinterRoot => Item::WinterRoot,
            CropKind::WinterCabbage => Item::WinterCabbage,
            CropKind::Garlic => Item::Garlic,
        }
    }

    pub fn growth_days(self) -> u32 {
        match self {
            CropKind::Carrot | CropKind::Potato | CropKind::SweetPotato | CropKind::WinterRoot => 3,
            CropKind::Strawberry | CropKind::Tomato | CropKind::Blueberry | CropKind::Eggplant | CropKind::Grape | CropKind::WinterCabbage | CropKind::Garlic => 4,
            CropKind::Cauliflower | CropKind::Corn | CropKind::Melon | CropKind::Pumpkin => 5,
        }
    }

}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CropData {
    pub kind: CropKind,
    pub days: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Quality {
    #[default]
    Normal,
    Bronze,
    Silver,
    Gold,
    Supernatural,
}

impl Quality {
    pub fn name(self) -> &'static str {
        match self {
            Quality::Normal => "Normal",
            Quality::Bronze => "Bronze",
            Quality::Silver => "Prata",
            Quality::Gold => "Ouro",
            Quality::Supernatural => "Sobrenatural",
        }
    }

    /// A fonte do jogo não tem estrelas nem emojis: a qualidade vai por extenso nos textos
    /// (`badge`) e como um pontinho colorido nos espaços da mochila (`color`).
    pub fn star_symbol(self) -> &'static str {
        ""
    }

    pub fn badge(self) -> &'static str {
        match self {
            Quality::Normal => "",
            Quality::Bronze => "(Bronze)",
            Quality::Silver => "(Prata)",
            Quality::Gold => "(Ouro)",
            Quality::Supernatural => "(Sobrenatural)",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Quality::Normal => Color::WHITE,
            Quality::Bronze => Color::srgb(0.82, 0.52, 0.25),
            Quality::Silver => Color::srgb(0.85, 0.90, 0.96),
            Quality::Gold => Color::srgb(1.0, 0.84, 0.0),
            Quality::Supernatural => Color::srgb(0.88, 0.38, 1.0),
        }
    }

    pub fn multiplier(self) -> f32 {
        match self {
            Quality::Normal => 1.0,
            Quality::Bronze => 1.25,
            Quality::Silver => 1.5,
            Quality::Gold => 2.0,
            Quality::Supernatural => 3.0,
        }
    }

    /// Sorteia a qualidade por pura probabilidade (chance percentual):
    /// Normal: 52%, Bronze: 25%, Prata: 13%, Ouro: 7%, Sobrenatural: 3%
    pub fn roll(seed: u32) -> Quality {
        match seed % 100 {
            0..52 => Quality::Normal,
            52..77 => Quality::Bronze,
            77..90 => Quality::Silver,
            90..97 => Quality::Gold,
            _ => Quality::Supernatural,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub item: Item,
    pub count: u32,
    #[serde(default)]
    pub quality: Quality,
}

impl Stack {
    #[allow(dead_code)]
    pub fn price(&self) -> Option<i32> {
        self.item.sell_price().map(|p| (p as f32 * self.quality.multiplier()).round() as i32)
    }

    #[allow(dead_code)]
    pub fn energy(&self) -> Option<f32> {
        self.item.energy().map(|e| e * self.quality.multiplier())
    }
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Stack>>,
    /// Espaço selecionado na barra (0..HOTBAR).
    #[serde(default)]
    pub selected: usize,
}

impl Default for Inventory {
    /// O kit de quem chega ao sítio: as ferramentas e um punhado de sementes.
    fn default() -> Self {
        let mut inv = Inventory { slots: vec![None; SLOTS], selected: 0 };
        for item in [Item::Hoe, Item::WateringCan, Item::CarrotSeeds, Item::Scythe, Item::Axe, Item::Pickaxe] {
            let n = if item == Item::CarrotSeeds { 10 } else { 1 };
            inv.add(item, n);
        }
        inv
    }
}

impl Inventory {
    /// Guarda `n` itens com qualidade Normal; devolve quantos não couberam.
    pub fn add(&mut self, item: Item, n: u32) -> u32 {
        self.add_with_quality(item, n, Quality::Normal)
    }

    /// Guarda `n` itens com a qualidade especificada.
    pub fn add_with_quality(&mut self, item: Item, mut n: u32, quality: Quality) -> u32 {
        for slot in self.slots.iter_mut().flatten() {
            if slot.item == item && slot.quality == quality && slot.count < item.max_stack() {
                let put = n.min(item.max_stack() - slot.count);
                slot.count += put;
                n -= put;
                if n == 0 {
                    return 0;
                }
            }
        }
        for slot in &mut self.slots {
            if slot.is_none() {
                let put = n.min(item.max_stack());
                *slot = Some(Stack { item, count: put, quality });
                n -= put;
                if n == 0 {
                    return 0;
                }
            }
        }
        n
    }

    pub fn count(&self, item: Item) -> u32 {
        self.slots.iter().flatten().filter(|s| s.item == item).map(|s| s.count).sum()
    }

    pub fn count_quality(&self, item: Item, quality: Quality) -> u32 {
        self.slots.iter().flatten().filter(|s| s.item == item && s.quality == quality).map(|s| s.count).sum()
    }

    /// Tira `n` itens de qualquer qualidade, se houver todos.
    pub fn remove(&mut self, item: Item, mut n: u32) -> bool {
        if self.count(item) < n {
            return false;
        }
        for slot in &mut self.slots {
            if let Some(stack) = slot
                && stack.item == item
            {
                let take = n.min(stack.count);
                stack.count -= take;
                n -= take;
                if stack.count == 0 {
                    *slot = None;
                }
                if n == 0 {
                    break;
                }
            }
        }
        true
    }

    /// Tira `n` itens com qualidade específica.
    pub fn remove_quality(&mut self, item: Item, quality: Quality, mut n: u32) -> bool {
        if self.count_quality(item, quality) < n {
            return false;
        }
        for slot in &mut self.slots {
            if let Some(stack) = slot
                && stack.item == item
                && stack.quality == quality
            {
                let take = n.min(stack.count);
                stack.count -= take;
                n -= take;
                if stack.count == 0 {
                    *slot = None;
                }
                if n == 0 {
                    break;
                }
            }
        }
        true
    }

    pub fn selected_item(&self) -> Option<Item> {
        self.slots.get(self.selected).copied().flatten().map(|s| s.item)
    }

    pub fn selected_stack(&self) -> Option<Stack> {
        self.slots.get(self.selected).copied().flatten()
    }

    /// Solta o espaço `from` em cima do `to`: pilhas iguais se juntam (o que não couber
    /// fica onde estava); o resto troca de lugar.
    pub fn merge_or_swap(&mut self, from: usize, to: usize) {
        if from == to || from >= self.slots.len() || to >= self.slots.len() {
            return;
        }
        if let (Some(a), Some(b)) = (self.slots[from], self.slots[to])
            && a.item == b.item
            && a.quality == b.quality
            && b.count < b.item.max_stack()
        {
            let put = a.count.min(b.item.max_stack() - b.count);
            self.slots[to] = Some(Stack { count: b.count + put, ..b });
            self.slots[from] = (a.count > put).then_some(Stack { count: a.count - put, ..a });
            return;
        }
        self.slots.swap(from, to);
    }

    /// Manda o espaço `i` para o outro lado (barra <-> mochila): junta com uma pilha igual
    /// ou ocupa o primeiro espaço vazio. Devolve se algo saiu do lugar.
    pub fn quick_move(&mut self, i: usize) -> bool {
        let Some(stack) = self.slots.get(i).copied().flatten() else { return false };
        let range = if i < HOTBAR { HOTBAR..self.slots.len() } else { 0..HOTBAR };
        let same = range.clone().find(|&j| {
            self.slots[j].is_some_and(|s| s.item == stack.item && s.quality == stack.quality && s.count < s.item.max_stack())
        });
        let Some(to) = same.or_else(|| range.clone().find(|&j| self.slots[j].is_none())) else { return false };
        self.merge_or_swap(i, to);
        true
    }

    /// Arruma a mochila (não mexe na barra): junta pilhas repetidas e ordena por tipo,
    /// nome e qualidade.
    pub fn sort_storage(&mut self) {
        let mut stacks: Vec<Stack> = Vec::new();
        for stack in self.slots[HOTBAR..].iter().flatten() {
            let mut left = stack.count;
            for other in stacks.iter_mut().filter(|o| o.item == stack.item && o.quality == stack.quality) {
                let put = left.min(other.item.max_stack() - other.count);
                other.count += put;
                left -= put;
            }
            if left > 0 {
                stacks.push(Stack { count: left, ..*stack });
            }
        }
        stacks.sort_by_key(|s| (s.item.category().0, s.item.name(), std::cmp::Reverse(s.quality), std::cmp::Reverse(s.count)));
        let mut stacks = stacks.into_iter();
        for slot in &mut self.slots[HOTBAR..] {
            *slot = stacks.next();
        }
    }

    /// Acabou o que estava na mão (sementes, comida): puxa da mochila outra pilha do mesmo
    /// item para o mesmo espaço da barra. Devolve se repôs.
    pub fn refill_selected(&mut self, item: Item) -> bool {
        if self.slots.get(self.selected).is_none_or(|s| s.is_some()) {
            return false;
        }
        let Some(from) = (HOTBAR..self.slots.len()).find(|&j| self.slots[j].is_some_and(|s| s.item == item)) else {
            return false;
        };
        self.slots.swap(from, self.selected);
        true
    }

    /// Retorna todos os tipos distintos de itens presentes na mochila.
    #[allow(dead_code)]
    pub fn items_present(&self) -> Vec<Item> {
        let mut list = Vec::new();
        for slot in self.slots.iter().flatten() {
            if !list.contains(&slot.item) {
                list.push(slot.item);
            }
        }
        list
    }

    /// Retorna todos os pares (Item, Quality) presentes na mochila.
    pub fn stacks_present(&self) -> Vec<(Item, Quality)> {
        let mut list = Vec::new();
        for slot in self.slots.iter().flatten() {
            if !list.contains(&(slot.item, slot.quality)) {
                list.push((slot.item, slot.quality));
            }
        }
        list
    }
}

/// Teclas 1..0 e a rodinha do mouse escolhem o espaço da barra.
fn select_slot(keys: Res<ButtonInput<KeyCode>>, mut wheel: MessageReader<MouseWheel>, mut inv: ResMut<Inventory>) {
    const DIGITS: [KeyCode; HOTBAR] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ];
    for (i, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            inv.selected = i;
        }
    }
    let scroll: f32 = wheel.read().map(|w| w.y).sum();
    if scroll != 0.0 {
        let step = if scroll > 0.0 { HOTBAR - 1 } else { 1 };
        inv.selected = (inv.selected + step) % HOTBAR;
    }
}
