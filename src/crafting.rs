//! Sistema de Crafting (Fabricação de Ferramentas, Estruturas, Máscaras e Alquimia).
//!
//! Permite ao jogador transformar matérias-primas da colheita, pedras, madeiras e essências
//! em itens úteis, decorações, elixires e máscaras arcanas.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::economy::Game;
use crate::inventory::{Inventory, Item, MaskKind, Quality};

pub fn plugin(app: &mut App) {
    app.init_resource::<CraftingState>();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CraftCategory {
    #[default]
    All,
    Tools,
    Structures,
    Masks,
    Alchemy,
}

impl CraftCategory {
    pub const ALL: [CraftCategory; 5] = [
        CraftCategory::All,
        CraftCategory::Tools,
        CraftCategory::Structures,
        CraftCategory::Masks,
        CraftCategory::Alchemy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CraftCategory::All => "Todos",
            CraftCategory::Tools => "Ferramentas",
            CraftCategory::Structures => "Estruturas",
            CraftCategory::Masks => "Máscaras",
            CraftCategory::Alchemy => "Alquimia",
        }
    }
}

/// Definição imutável de uma receita de fabricação.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CraftRecipe {
    pub id: &'static str,
    pub name: &'static str,
    pub category: CraftCategory,
    pub ingredients: &'static [(Item, u32)],
    pub gold_cost: i32,
    pub essence_cost: f32,
    pub result_item: Option<Item>,
    pub result_count: u32,
    pub unlock_mask: Option<MaskKind>,
    pub description: &'static str,
    pub benefits: &'static str,
}

impl CraftRecipe {
    pub const ALL: &'static [CraftRecipe] = &[
        // --- ESTRUTURAS ---
        CraftRecipe {
            id: "wood_fence",
            name: "Cerca de Madeira",
            category: CraftCategory::Structures,
            ingredients: &[(Item::Wood, 4)],
            gold_cost: 0,
            essence_cost: 0.0,
            result_item: Some(Item::WoodFence),
            result_count: 4,
            unlock_mask: None,
            description: "Cercado rústico para delimitar piquetes de pasto e canteiros de plantio.",
            benefits: "Organiza o sítio e protege canteiros contra o trânsito desordenado.",
        },
        CraftRecipe {
            id: "storage_chest",
            name: "Baú Rústico",
            category: CraftCategory::Structures,
            ingredients: &[(Item::Wood, 20), (Item::Stone, 5)],
            gold_cost: 0,
            essence_cost: 0.0,
            result_item: Some(Item::StorageChest),
            result_count: 1,
            unlock_mask: None,
            description: "Baú reforçado com ferragens rústicas para guardar excedentes de safra.",
            benefits: "Guarda ferramentas, sementes e produtos sem ocupar espaço na mochila.",
        },
        CraftRecipe {
            id: "scarecrow",
            name: "Espantalho Amigável",
            category: CraftCategory::Structures,
            ingredients: &[(Item::Wood, 10), (Item::Fiber, 15)],
            gold_cost: 0,
            essence_cost: 0.0,
            result_item: Some(Item::Scarecrow),
            result_count: 1,
            unlock_mask: None,
            description: "Boneco de palha sorridente com chapéu de aba larga.",
            benefits: "Afasta corvos e aves curiosas que tentam bicar brotos jovens.",
        },
        CraftRecipe {
            id: "sprinkler",
            name: "Aspersor de Irrigação",
            category: CraftCategory::Structures,
            ingredients: &[(Item::Stone, 10), (Item::Wood, 5)],
            gold_cost: 20,
            essence_cost: 0.0,
            result_item: Some(Item::Sprinkler),
            result_count: 1,
            unlock_mask: None,
            description: "Mecanismo hidráulico de pressão movido por gravidade.",
            benefits: "Irriga automaticamente os canteiros vizinhos todas as manhãs.",
        },

        // --- MÁSCARAS ARCANAS ---
        CraftRecipe {
            id: "mask_rustic",
            name: "Máscara Rústica",
            category: CraftCategory::Masks,
            ingredients: &[(Item::Wood, 15), (Item::Fiber, 10)],
            gold_cost: 10,
            essence_cost: 0.0,
            result_item: Some(Item::RusticStrawMask),
            result_count: 1,
            unlock_mask: Some(MaskKind::RusticStraw),
            description: "Trançada com palha encantada e talhada em madeira de carvalho.",
            benefits: "Harmonia com a terra: +15% de chance de qualidade superior nas colheitas.",
        },
        CraftRecipe {
            id: "mask_crow",
            name: "Máscara do Corvo",
            category: CraftCategory::Masks,
            ingredients: &[(Item::Fiber, 20), (Item::Stone, 10)],
            gold_cost: 30,
            essence_cost: 0.0,
            result_item: Some(Item::CrowMask),
            result_count: 1,
            unlock_mask: Some(MaskKind::Crow),
            description: "Inspirada nos médicos da peste, com bico alongado e lentes escuras.",
            benefits: "Filtro de miasmas: +35% de resistência ao acúmulo de maldição noturna.",
        },
        CraftRecipe {
            id: "mask_ghost",
            name: "Máscara da Névoa",
            category: CraftCategory::Masks,
            ingredients: &[(Item::Wood, 20), (Item::Fiber, 15)],
            gold_cost: 25,
            essence_cost: 5.0,
            result_item: Some(Item::GhostMask),
            result_count: 1,
            unlock_mask: Some(MaskKind::Ghost),
            description: "Tecida com vapores da neblina noturna e seda lunar.",
            benefits: "Passos etéreos: +20% de velocidade ao caminhar à noite e silêncio absoluto.",
        },
        CraftRecipe {
            id: "mask_dragon",
            name: "Máscara do Dragão Dourado",
            category: CraftCategory::Masks,
            ingredients: &[(Item::Stone, 25), (Item::Wood, 10)],
            gold_cost: 60,
            essence_cost: 10.0,
            result_item: Some(Item::GoldenDragonMask),
            result_count: 1,
            unlock_mask: Some(MaskKind::GoldenDragon),
            description: "Banhada em pigmento dourado reluzente e runas ancestrais de cobiça.",
            benefits: "Magnatismo sombrio: +25% de ouro em vendas para o Dragão de Ossos.",
        },

        // --- ALQUIMIA & CONSUMÍVEIS ---
        CraftRecipe {
            id: "energy_elixir",
            name: "Elixir Revigorante",
            category: CraftCategory::Alchemy,
            ingredients: &[(Item::Strawberry, 2), (Item::CowMilk, 1)],
            gold_cost: 5,
            essence_cost: 0.0,
            result_item: Some(Item::EnergyElixir),
            result_count: 1,
            unlock_mask: None,
            description: "Bebida cremosa e energizante batida com morangos frescos e leite puro.",
            benefits: "Restaura instantaneamente +50 de energia (stamina) para prolongar o dia.",
        },
        CraftRecipe {
            id: "calming_incense",
            name: "Incenso Calmante",
            category: CraftCategory::Alchemy,
            ingredients: &[(Item::Fiber, 10), (Item::Garlic, 2)],
            gold_cost: 10,
            essence_cost: 0.0,
            result_item: Some(Item::CalmingIncense),
            result_count: 1,
            unlock_mask: None,
            description: "Bastão defumador aromático com essência de alho de inverno e ervas secas.",
            benefits: "Purifica a aura do fazendeiro, reduzindo 15 pontos de maldição acumulada.",
        },
        CraftRecipe {
            id: "arcane_fertilizer",
            name: "Fertilizante Arcano",
            category: CraftCategory::Alchemy,
            ingredients: &[(Item::Carrot, 3), (Item::Fiber, 5)],
            gold_cost: 8,
            essence_cost: 2.0,
            result_item: Some(Item::ArcaneFertilizer),
            result_count: 2,
            unlock_mask: None,
            description: "Composto orgânico enriquecido com essência noturna e nutrientes terrosos.",
            benefits: "Estimula o solo, dobrando a chance de colheitas Ouro e Sobrenaturais.",
        },
    ];

    /// Verifica se o jogador tem materiais, ouro e essência suficientes para fabricar.
    pub fn can_craft(&self, inv: &Inventory, game: &Game) -> bool {
        if game.gold < self.gold_cost {
            return false;
        }
        if game.essence < self.essence_cost {
            return false;
        }
        for &(item, req) in self.ingredients {
            if inv.count(item) < req {
                return false;
            }
        }
        true
    }

    /// Executa a fabricação consumindo ingredientes e concedendo o resultado.
    pub fn craft(&self, inv: &mut Inventory, game: &mut Game) -> bool {
        if !self.can_craft(inv, game) {
            return false;
        }

        // Consome ouro e essência
        game.gold -= self.gold_cost;
        game.essence -= self.essence_cost;

        // Consome os ingredientes do inventário
        for &(item, req) in self.ingredients {
            inv.remove(item, req);
        }

        // Se desbloqueia máscara, adiciona à lista de máscaras do jogador
        if let Some(mask) = self.unlock_mask {
            game.unlock_mask(mask);
            game.say(format!("Você forjou a {}! Pressione [M] para alternar máscaras.", mask.name()));
        }

        // Se produz item físico, adiciona à mochila
        if let Some(result) = self.result_item {
            let left = inv.add_with_quality(result, self.result_count, Quality::Normal);
            if left > 0 {
                game.say(format!("Mochila cheia! {}x {} caíram no chão.", left, result.name()));
            } else {
                game.say(format!("Fabricou {}x {}!", self.result_count, result.name()));
            }
        }

        true
    }
}

/// Estado da interface de fabricação (aba do inventário ou estação de trabalho).
#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct CraftingState {
    pub selected_index: usize,
    pub category: CraftCategory,
}

impl CraftingState {
    #[allow(dead_code)]
    pub fn filtered_recipes(&self) -> Vec<&'static CraftRecipe> {
        CraftRecipe::ALL
            .iter()
            .filter(|r| self.category == CraftCategory::All || r.category == self.category)
            .collect()
    }

    #[allow(dead_code)]
    pub fn selected_recipe(&self) -> Option<&'static CraftRecipe> {
        let list = self.filtered_recipes();
        list.get(self.selected_index).copied()
    }
}
