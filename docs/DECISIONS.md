# Decisões tomadas

## Rodada 13 (10/11/2026): mochila mais prática e textos sem símbolos quebrados

| O quê | Decisão | Onde |
|---|---|---|
| Símbolos | A VT323 não tem emojis nem ✓ ✗ ★ ♀ ♂ (conferido no cmap da fonte); 57 textos da interface usavam. Saíram os emojis; marcas viraram `•` e `×`; ♀/♂ viraram "(fêmea)"/"(macho)"; a qualidade vai por extenso nos textos. | `inventory.rs`, `hud.rs`, `shop.rs`, `economy.rs`, `tools.rs` |
| Qualidade | Pontinho colorido no canto do espaço (bronze, prata, ouro, sobrenatural) em vez de emoji junto do número. | `hud.rs` (`SlotPip`) |
| Detalhes | Cartão na mochila com ícone grande, nome, tipo, quantidade, efeito (energia, dano, dias para colher, bônus da máscara) e preço de venda do item sob o mouse. | `hud.rs` (`bag_details`) |
| Mover | Soltar numa pilha igual junta; Shift+clique manda para o outro lado (barra <-> mochila); 1..0 com o mouse num item manda para aquele espaço da barra; espaço vazio não vai "para a mão". | `inventory.rs`, `shop.rs`, `hud.rs` (`bag_shortcuts`) |
| Organizar | Botão e tecla R: junta pilhas repetidas e ordena a mochila por tipo, nome e qualidade. A barra não é mexida. | `inventory.rs` (`sort_storage`) |
| Reposição | Quando o item em uso na barra acaba, outra pilha dele vem da mochila para o mesmo espaço. Não vale para o que foi tirado de propósito com a mochila ou a loja abertas. | `inventory.rs` (`refill_hotbar`) |

## Rodada 12 (10/11/2026): cutscenes em dados

Primeiro passo da camada de conteúdo (em vez de uma engine à parte): o que era função Rust
vira arquivo editável. Guia de uso em `docs/CUTSCENES.md`.

| O quê | Decisão | Onde |
|---|---|---|
| Formato | Um JSON por cena em `assets/cutscenes`: `id`, `steps` (`say`, `banner`, `wait`, `fade`), `skippable`, `when`, `once`. Chave desconhecida é erro; arquivo com erro é ignorado com mensagem. | `cutscene.rs` (`parse`) |
| Carga | No nativo a pasta é lida ao abrir (e com F5 em dev), sem recompilar; o `build.rs` embute os arquivos para o navegador. | `cutscene.rs` (`reload`), `build.rs` |
| Variáveis | `{nome}`, `{sitio}`, `{dia}`, `{estacao}`, `{ouro}`, `{maldicao}` e extras passadas pelo código (`{multa}`). | `cutscene.rs` (`fill`) |
| Gatilhos | `when` com `map`, `phase`, `season`, `min_day`, `min_curse`, `has_mask`; dispara quando a condição passa a valer. `once` guarda o id em `Game.seen_cutscenes` (no save). | `cutscene.rs` (`fire_triggers`) |
| Migração | As 4 cenas em código (intro, estações, máscara, Inquisidor) viraram 7 arquivos, chamados por `play_id`. | `assets/cutscenes`, `economy.rs`, `cycle.rs` |
| Fade | O passo `fade` passou a escurecer a tela de verdade (antes só calculava o valor). | `cutscene.rs` (`render_cutscene_ui`) |

Próximos: personagens em dados e passos de mover gente e câmera.

## Rodada 11 (10/11/2026): lag no Windows, flickering das árvores e sombras de chão

Medido com o perfil novo (`COZY_PROFILE=1`): a lógica do jogo gasta ~6 ms por quadro; o que
estourava era a renderização. Na MX250, maximizado (1936x1096), o menu parado custava
30–38 ms por quadro; sem MSAA caiu para ~11 ms.

| O quê | Decisão | Onde |
|---|---|---|
| MSAA | Desligado na câmera (`Msaa::Off`). O padrão do Bevy é 4x, inútil em pixel art e caro: cada camada translúcida de tela cheia custava 4 vezes mais. `COZY_MSAA=1` religa para comparar. | `cycle.rs` (`spawn_camera`) |
| Camadas invisíveis | O véu de transição e o tom da hora (sprites de 4000x4000) só são desenhados quando têm alfa > 0. | `cycle.rs` (`fade`), `lighting.rs` (`tint`) |
| Atlas único | Todos os PNGs do mundo vão para uma textura só por fase (1024x664, 86 artes), montada em `pack_atlas` ao sair da tela de carregamento. Cada `Art` aponta para o atlas e tem um layout próprio com os mesmos índices de antes. Antes cada arte era uma textura e cada troca quebrava o lote de desenho. | `sprites.rs` |
| Flickering atrás das árvores | `sync_flora_seasons` regravava a cor (alfa 1) e `fade_occluders` o alfa 0,45, um desfazendo o outro conforme a ordem do quadro. Agora a estação mexe só na cor. A copa também some e volta em ~0,15 s, e objetos na mesma linha têm desempate de profundidade por x. | `map.rs`, `player.rs` |
| Sombras de chão | Árvores ganham uma sombra larga (`shadow_big`, 32x10) e objetos soltos (pedra, arbusto, barril, caixotes, fardo, vaso, placa, poste) a sombra de contato que gente e bicho já tinham. | `map.rs` (`ground_shadow`), `art/sprites/shadow_big.json` |
| Perfil | `COZY_PROFILE=1` imprime a cada segundo ms/quadro e a divisão pre/update/post/last/fora (fora = render e espera da GPU). `COZY_NOVSYNC=1` tira o vsync para ver o custo real. | `main.rs` (`profile`) |

Não medido com a máquina livre: o ganho do atlas (as medições depois dele foram feitas com o
navegador usando a mesma GPU). O Brave roda na MX250 e, com vídeo, ocupa ~30% do motor de
cópia da placa; isso sozinho derruba o jogo para 30–40 fps.

## Rodada 10 (08/11/2026): combate — o "controle de pragas" da noite

Ninguém luta contra os moradores (eles nem sabem o que viram à noite). Quem sai da mata
quando escurece são pragas, e bater nelas é proteger patrimônio — com nota fiscal cínica.

| O quê | Decisão | Onde |
|---|---|---|
| Pragas | Lodo (pula atrás de você, 3 de vida, solta essência), Corvo-de-olho (voa até a plantação madura, bica 4 s e come; 2 de vida, solta fibra), Espantalho Errante (anda, ergue os braços 0,6 s e dá um bote; 6 de vida, solta ouro e fibra). Só à noite, em mapas abertos, longe do jogador; teto = 2 + maldição/20. Corvos só com plantação madura; espantalhos com maldição ≥ 30. Viram fumaça ao amanhecer. | `combat.rs`, `art/sprites/{slime,crow,haunt}.json` |
| Golpe | Espaço/clique com foice (1) ou Espada Enferrujada (2, 60 ouro no Seu Bartô: "era do Xerife, ele nunca usou"). Arco à frente, empurrão, piscar branco, 0,35 s entre golpes, 0,5 de energia por acerto. Com praga a menos de 64 px, a foice vira arma mesmo errando; longe, volta a cortar mato. | `combat.rs` (`attack`), `inventory.rs` (`weapon_damage`) |
| Vida | `Game.health` (100, salva no save), cheia ao amanhecer; comer cura metade da energia que dá. Encostar numa praga tira vida, empurra e deixa 1 s invencível (piscando). O painel de status aparece ao apanhar e fica à vista com menos de 35%. | `economy.rs`, `tools.rs`, `hud.rs` |
| Desmaio | Vida zerada: acorda em casa no fim da fase, com 30 de vida, e o Prefeito cobra uma "taxa de resgate" (10% do ouro, mínimo 10). | `combat.rs` (`faint`) |
| Máscaras | Gárgula corta o dano pela metade; Corvo faz os corvos te ignorarem; Palha (Rústica) faz o mesmo com os espantalhos. | `combat.rs` |

## Rodada 9 (08/11/2026): foco no desktop, animais, cultivos, ícones e moradores de verdade

O alvo agora é o **desktop**: builds e testes passam a ser nativos (`cargo run --features dev`
no WSL/WSLg). O wasm continua compilando, mas não é mais o caminho de teste.

| O quê | Decisão | Onde |
|---|---|---|
| Animais | Folhas próprias para vaca (32x24), cabra (24x24), coelho (16x16) e cavalo (32x32); galinha e ovelha ganharam a linha do macho (galo, carneiro). Todas 4x2: linha 0 fêmea, linha 1 macho; quadros 0-1 andando, 2-3 parado. Acabou a tintura/escala que reaproveitava galinha/ovelha/cachorro. Sombra proporcional ao porte. | `art/sprites/{cow,goat,rabbit,horse,chicken,sheep}.json`, `animals.rs` (`first_frame`, `shadow`), `tools.rs` |
| Cultivos | Uma folha `crops` 16x32 (3 estágios x 15 cultivos, linha = ordem de `CropKind`), base no tile e planta subindo para o de cima. Cada madura tem um detalhe amaldiçoado próprio à noite. Saiu a cenoura tingida e o caso especial da abóbora; `grow_views` só roda quando a fazenda muda. | `art/sprites/crops.json`, `economy.rs` (`crop_frame`) |
| Ícones | `items` foi de 16 para 74 quadros (8x10): semente e colheita de cada cultivo, leites, ovo, lãs, carnes, comidas, certidão, fabricáveis, alquimia e as 5 máscaras. Ícones não são mais tingidos (`Item::tint` e `CropKind::ripe_color` removidos). | `art/sprites/items.json`, `inventory.rs` (`icon`), `hud.rs`, `ui.rs` |
| Moradores | Folhas 3x3 como a do fazendeiro (frente, costas, lado; parado, passo A, passo B), com a forma noturna em todas as direções. Andam com direção certa, param de vez em quando para olhar em volta (de noite, inquietos, quase não param) e se viram para o jogador; quem está no balcão acompanha o jogador com o olhar. | `art/sprites/{baker,...}.json`, `villagers.rs` (`wander`, `facing`) |
| Retratos | `portraits` 32x32 (um por `Kind`) na caixa de diálogo, em 2x. De noite o retrato mostra a forma amaldiçoada sorrindo. | `art/sprites/portraits.json`, `dialogue.rs` |

## Rodada 8 (30/10/2026): motor de cutscenes, diálogos sazonais multi-página, inventário dedicado com equipamentos/máscaras e sistema de crafting

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Motor de Cutscenes Cinematográficas**: sequências de passos narrativos (`CutsceneStep::Dialogue`, `Banner`, `Pause`, `FadeOut`, `FadeIn`, `Trigger`), letterboxing dinâmico (faixas pretas de cinema), controles de avanço (`Espaço`/`E`/`Enter` para avançar, `ESC` para pular) | Imersão narrativa em momentos-chave sem travar a jogabilidade | `cutscene.rs` |
| **Gatilhos de Cutscenes**: Intro no Dia 1, Virada de Estação (Primavera/Verão/Outono/Inverno), Confronto com Inquisidor e Despertar da Primeira Máscara | Dar impacto dramático às mudanças de ciclo e marcos narrativos | `cutscene.rs`, `cycle.rs`, `economy.rs` |
| **Diálogos Sazonais Multi-página**: suporte a páginas encadeadas (`dialogue.open_pages`), avanço rápido de digitação com `E` (revela texto completo antes de passar de página) e indicador no HUD `[E] Mais (x/y)` / `[E] Fechar` | Diálogos ricos e naturais sem cortar o texto em uma única linha | `dialogue.rs`, `hud.rs` |
| **Roteiro Sazonal dos Moradores**: todos os 7 aldeões (Florinda, Bartô, Padeiro, Pescador, Ferreiro, Prefeito, Vovó) + Inquisidor + Necrófago com diálogos específicos para as 4 estações, além de variações diurnas, noturnas (com e sem máscara) e reação a alta maldição (>50%) | Fazer o vilarejo parecer vivo, reagindo ao clima, à noite e às ações do jogador | `villagers.rs` (`seasonal_dialogue`) |
| **Inventário Dedicado com Abas (`Overlay::Bag`)**: modal completo com abas interativas `[🎒 Mochila & Equipamento]`, `[📊 Status & Atributos]` e `[🔨 Fabricação]` | Substituir o painel simples por uma tela central de gerenciamento de RPG | `hud.rs`, `shop.rs` |
| **Equipamentos e Alternância de Máscaras**: slot de Máscara Ativa, Ferramenta na Mão e Traje no boneco de equipamento; sistema de máscaras (`MaskKind`: Gárgula, Corvo, Fantasma, Dragão Dourado, Palha Rústica) com bônus passivos e tecla rápida `M` para alternar | Customização de gameplay, redução de maldição ou bônus de venda com feedback visual | `inventory.rs`, `economy.rs`, `hud.rs` |
| **Painel Visual de Status e Atributos**: barras estilizadas de Energia e Maldição, exibição de Estação atual, Dia e resumo econômico | Visualização clara do estado do avatar e riscos de maldição | `hud.rs` (`redraw_bag`) |
| **Sistema de Crafting Integrado**: bancada com receitas divididas em Estruturas (Cerca, Baú, Espantalho, Aspersor), Máscaras e Alquimia (Elixir de Energia, Incenso Calmante, Fertilizante Arcano); verificação em tempo real de materiais (`✓`/`✗`) e tecla rápida `C` | Permitir progressão autônoma do jogador forjando utilidades e itens arcanos | `crafting.rs`, `hud.rs`, `shop.rs` |

## Rodada 7 (29/10/2026): sazonalidade, estações do ano, reflorestamento e coexistência B2C/B2B

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Estações do ano (Primavera, Verão, Outono, Inverno)**: 7 dias por estação (28 dias = 1 ano) | Ritmo ágil para ver as quatro estações sem grind de meses | `cycle.rs` (`DAYS_PER_SEASON`) |
| **Paleta sazonal nos tiles de chão assado**: a grama ganha tons quentes no verão, âmbar/acobreado no outono e geada fria/esbranquiçada no inverno | Atmosfera viva sem recriar dezenas de PNGs; o chão é repintado dinamicamente na virada de estação | `terrain.rs` (`apply_season_ground`) |
| **Vegetação sazonal (copas de árvores e arbustos)**: carvalhos (`tree_oak`, `tree_big`) ficam dourados/laranja no outono e com geada no inverno; pinheiros e arbustos adaptam-se | Clima autêntico de colheita e passagem de tempo | `map.rs` (`FloraKind`, `sync_flora_seasons`) |
| **Reflorestamento na virada de estação**: árvores e arbustos cortados com machado na fazenda voltam a brotar na mudança de estação | A natureza se regenera, mantendo suprimento de madeira e vida na fazenda | `economy.rs` (`FarmState::reforest`) |
| **Coexistência contínua B2C e B2B**: as economias e atividades diurnas e noturnas coexistem abertamente, sem a dicotomia rígida de "fachada escondida em feno" | Pedido do Elias (Decisão Janus #20): alinhar a visão do jogo com coexistência orgânica dos sistemas | `economy.rs` / `docs/GDD.md` |
| **HUD com Estação e Calendário**: o relógio e o cartão do amanhecer mostram a estação atual e o dia correspondente (ex: `Primavera · Dia 1`) | Feedback claro do calendário | `hud.rs` / `economy.rs` (`DawnCard`) |
| **Dev Cheat de Estação (`P`)**: pula instantaneamente para o primeiro dia da próxima estação | Testar e demonstrar todas as quatro estações rapidamente | `cycle.rs` (`skip_milestone`) |

## Rodada 6 (27/10/2026): direção de arte das imagens de inspiração

As duas imagens que você mandou no começo foram recuperadas do histórico e ficam em
`target/inspiration/` (fora do git) como referência para quem desenha.

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Mantidos os tiles de 16 px** | O que dava cara de "grade" não era o tamanho, e sim as bordas retas entre terrenos; 32 px exigiria redesenhar tudo e manteria a grade | Se ainda quiser mais detalhe, o próximo passo é desenhar os objetos maiores (a escala dos tiles não precisa mudar) |
| **Terreno orgânico gerado em código**: caminho, água e canteiro têm bordas curvas com ruído, friso escuro e sombrinha na grama | Igual às imagens, sem desenhar dezenas de tiles de transição | `terrain.rs` (`wobble`, limites dos frisos) |
| Texturas de chão sem borda (grama "pintada" com florzinhas, areia com pedrinhas, terra fofa, água turquesa) | O terreno orgânico precisa de texturas que emendem | `art/sprites/tile_*.json` |
| **Água animada** numa camada própria: 4 quadros por fase, com reflexos ondulados que deslizam, brilhinhos e espuma na margem | Pedido; trocar a imagem é mais barato que repintar o chão | `terrain::water_pixel` |
| Cenário da inspiração: celeiro, galinheiro, **moinho com pás girando**, carroça, fardos, píer (anda-se sobre a água), barco, taboas, árvore grande, abóboras, flores | Densidade de cenário das imagens | Pincéis no grupo "Fazenda" do editor |
| **Animais**: galinhas e ovelhas passeiam e bicam/pastam; o cachorro senta abanando o rabo e vem até você; dá para fazer carinho (E) | Vida no cenário, como nas imagens | `animals.rs`; marcadores `a`, `e`, `j` |
| **Fazenda redesenhada** no espírito da imagem (caminhos curvos, cercado de bichos, canteiros cercados, lago com píer) | — | `assets/maps/farm.txt` (ou o editor); o gerador ficou fora do repo |
| Botões com a moldura numa camada atrás do texto e folga proporcional à letra | O padding não funcionava com a imagem no próprio nó | `spawn_button` em `ui.rs` |
| Lojas com **seletor de quantidade** (−, +, Máx) e o total no botão | Pedido | `shop.rs` |
| Saves antigos: plantações e cortes da fazenda antiga podem cair em lugares estranhos do mapa novo | O mapa mudou de lugar as coisas | Comece um jogo novo para ver a fazenda limpa |

## Rodada 5 (27/10/2026): fundação otimizada, mundo editável, HUD contextual e polimento cozy

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Mundo como dados**: `assets/maps/world.json` lista os mapas (nome, interior, fazenda, balconista, loja), as **portas** (tile do prédio → interior) e as **bordas** (`<` `>` `^` `v` → borda de outro mapa) | Criar áreas e ligar interiores pelo editor, sem mexer em código | `worlddef.rs`; o `MapId` fixo virou o estado `MapState(String)` |
| `build.rs` **embute tudo o que estiver em assets/maps** | Mapas novos salvos no editor nativo entram no jogo web no próximo build | — |
| Editor: **+ Exterior / + Interior**, renomear, marcar interior/fazenda/mapa inicial, balconista e loja, **porta → interior** e **borda → mapa/borda** por botões | Pedido: interior da florista, áreas novas, tudo no editor | `editor.rs` |
| Só **uma fazenda** (o mapa marcado "Fazenda"): plantações, ferramentas, fornalhas e o que se corta/ara são guardados nela | Simples; várias fazendas exigiriam estado por mapa | `FarmState` |
| **Chão "assado"**: cada mapa vira 2 imagens (dia/noite) montadas em CPU; arar/regar/caverna repintam só o tile | ~3 mil sprites de chão viram 2 (a água animada e as bordas continuam sprites) | `bake.rs` |
| **Grade espacial** para colisão (tile → colisores próximos) | Testa só o que está perto do pé | `Map::solid` |
| **Tela de carregamento**: o jogo espera todas as imagens antes do menu | Nada aparece pela metade; o chão assado precisa dos pixels dos tiles | `loading.rs` |
| Vento só anima o que está na tela | Menos trabalho por quadro | `ViewVisibility` em `sway` |
| **HUD contextual**: relógio sempre; números do império só quando mudam (4 s) ou com Alt; diário some 6 s após a última mensagem; ajuda nos primeiros 45 s ou com H; nome do item ao trocar; ação "só informativa" mais apagada | Pedido: esconder o HUD de forma inteligente | `hud.rs` (constantes no topo) |
| **Alvo inteligente**: gente ganha balão ("!" age, "..." só olha); objetos/prédios uma setinha quicando; chão os colchetes, que **deslizam** entre alvos; o cursor de ferramenta só aparece quando ela funciona ali | Feedback de onde vai interagir sem poluir | `draw_cursor` em `fx.rs` |
| **Kit de UI de madeira** (9-slice, `art/ui` → `assets/ui` em 3x): painéis, botões (normal/hover/apertado), slots, diálogo de pergaminho com plaquinha do nome | Mais cozy no desenho, não na cor | `cargo ui-art`; `ui.rs` |
| Micro-interações: botões crescem no hover e afundam no clique; slot escolhido dá pulinho; diálogo abre com pulinho; ouro "conta"; itens **voam até a barra**; arco da ferramenta na mão; mato/flores balançam quando você passa; jogador estica/achata ao andar/parar; sombras sob personagens; moradores **param e olham** quando você chega; câmera olha à frente | Pedido: investir em micro-interações | `ui.rs`, `fx.rs`, `player.rs`, `villagers.rs` |
| QoL: **Shift corre**; **clique direito interage / esquerdo usa** (fora da barra); **cartão do amanhecer** com o resumo de ontem; **F3** mostra FPS, entidades, mapa e tile | Variedade de QoL pedida | — |

## Rodada 4 (27/10/2026): hitboxes, editor, tempo em marcos, inventário, ferramentas e lojas

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Colisão por caixas em pixels** do tamanho da base do desenho (tronco, pé do poste, frente da casa), em vez de tiles inteiros bloqueados | "As coisas são maiores do que parecem" | `collider_size` em `map.rs` (largura x altura em px, a partir da base) |
| Portas com interior têm uma **abertura de 14 px** no colisor | Entrar andando, sem ficar preso no batente | `colliders_of` |
| **Editor de mapas no jogo** (menu → Editor de mapas): pinta os caracteres do .txt e mostra o mapa na hora com o mesmo `map::build` | Um editor externo exigiria manter outro formato; assim o .txt continua a fonte de verdade | `editor.rs`; pincéis em `BRUSHES` |
| Editor salva no **localStorage** (navegador) ou **direto em assets/maps/*.txt** (nativo); o mapa salvo vale no próximo jogo | No navegador não dá para escrever no repositório; "Copiar" leva o texto para a área de transferência | `save::save_map` / `reset_map` |
| Relógio em **minutos de jogo**: 12 h = 10 min reais (antes, 2 min) | Ciclo bem maior | `MINUTES_PER_SECOND` em `cycle.rs` |
| Dia e noite divididos em **6 marcos de 4 h** (Manhã, Meio-dia, Tarde, Anoitecer, Madrugada, Alvorada); atividades "marco" pulam para o próximo | Sua ideia de avançar em passos; primeiro rascunho, ainda sem saber quais atividades | Hoje são marcos: **almoçar na padaria** (pula o período), **cochilar** de dia (vai ao anoitecer) e **dormir** à noite (vai ao amanhecer) |
| **Celular / doomscroll** (F): cada post rolado = +10 min, feed de fofoca de dia e posts amaldiçoados à noite; o total de posts rolados fica salvo | Avançar o tempo manualmente, já com gancho para a mecânica de doomscroll | `phone.rs` (`MINUTES_PER_SCROLL`, listas de posts) |
| Mochila, loja e celular **pausam o tempo** (como no Stardew) | Comprar/organizar não deve custar o dia | `Overlay` + set `Gameplay` em `main.rs` |
| Mochila de 30 espaços; os 10 primeiros são a **barra de itens** (1-0 ou rodinha); trocar = clicar em dois espaços | Padrão Stardew com o mínimo de interface | `inventory.rs`, `hud.rs` |
| **Ferramentas no tile à frente** (Espaço): enxada ara grama, regador molha, sementes plantam, foice corta mato (fibra) e colhe, machado derruba árvore/arbusto (4 golpes, madeira), picareta quebra pedra (3 golpes) | Ferramentas básicas pedidas; só na fazenda ("melhor não mexer no que não é seu") | `tools.rs` |
| **Energia** (100): ferramentas gastam 1-2; comida recupera; dormir enche | Dá sentido à comida e às lojas | `MAX_ENERGY`, `Item::energy` |
| Plantação **cresce por dia regado** (cenoura: 2 dias); a terra seca ao amanhecer | Ritmo Stardew; casa com o ciclo maior | `CARROT_DAYS` |
| Sementes agora são **compradas no Armazém** (antes eram automáticas a 2 ouro); começa-se com 10 | Loja com função | `Shop::stock` |
| Armazém compra cenoura/madeira/pedra/fibra; Padaria vende pão/bolo, dá o pão do dia e serve o almoço-marco | Lojas básicas pedidas | `shop.rs` |
| Cortar árvores, quebrar pedras e arar ficam **salvos** na fazenda | A fazenda é sua | `FarmState.edits` |
| Saves antigos continuam abrindo (campos novos com padrão: mochila inicial, energia cheia) | Não perder seus testes | `#[serde(default)]` em `SaveData` |

## Rodada 3 (27/10/2026): vila, interiores, feedback visual e atmosfera

| Decisão | Por quê | Para mudar |
|---|---|---|
| Vários mapas como **sub-estado `MapId`** (fazenda, vila, casa, padaria, armazém); entidades somem com `DespawnOnExit(mapa)` | O Bevy já faz a limpeza na troca; nada de rastrear entidades à mão | Novo mapa = `.txt` em `assets/maps/` + variante em `MapId` |
| Portas e saídas definidas **no próprio mapa**: porta do prédio (`H`, `1`, `2`) leva ao interior; capacho `E` volta; `>`/`<` ligam fazenda e vila | O mapa é a fonte de verdade; sem tabela de coordenadas para manter | `warp_for` em `map.rs` |
| **FarmState** guarda plantações/fornalhas como dados; os sprites só existem com o jogador na fazenda | As cenouras crescem enquanto você está na vila; o save fica trivial | — |
| Jogo novo e save **acordam em casa, ao lado da cama**; a cama, à noite, pula para o amanhecer (e salva) | Ritual do Stardew; evita jogar a noite inteira esperando | `Prop::Bed` em `economy.rs` |
| A forja de máscaras saiu da fachada e virou **a bancada dentro de casa** | Mais coerente; dá função ao interior | — |
| Moradores: 7 pecados com **falas de dia, de noite com máscara e de noite sem máscara**; só o Padeiro tem mecânica (pão / Ghoul) | Pedido: mecânicas de dia/noite ficam para depois; as falas já plantam os ganchos do GDD | `Kind::line` em `villagers.rs` |
| Padeiro e Mercador ficam **atrás do balcão de dia** (`N` no mapa do interior) e saem à noite; os outros ficam na frente dos prédios | Lojas com gente dentro; noite com monstros na praça | `Kind::shop` |
| Prédios sem interior (ferraria, prefeitura, biblioteca, floricultura, guarita) mostram um **aviso na porta** com piada do morador | Gancho para a próxima rodada sem portas "mortas" | `closed_door` |
| **Cursor de interação**: 4 cantos pulsando em volta do alvo (tile, móvel, prédio ou pessoa); apagado quando a ação é só informativa | Pedido "desenhar nas bordas da interação" | `fx.rs` (`Focus`) |
| Números subindo ("+12 ouro"), pulinho ao plantar/construir, partículas de poeira/folha/fumaça/moeda/coração/"!" | Retorno imediato de cada ação | `fx.rs` |
| Vento como **uma onda que atravessa o mapa**: árvores e enfeites trocam de quadro quando ela passa | Balanço coerente (não tudo ao mesmo tempo) e barato | `wind()` em `fx.rs` |
| Borboletas de dia, vaga-lumes à noite, folhas caindo, fumaça das chaminés, brilho na água, sombras de nuvens | Atmosfera pedida | `fx.rs` |
| Luz **sem shaders**: tom de cor da hora por cima da tela (manhã rosada, tarde dourada, noite azul), vinheta pontilhada e poças de luz quente (lampião, postes, forno, forja) | Funciona igual no WebGL2 do navegador; o pontilhado (Bayer) combina com pixel art | `lighting.rs` |
| Caixa de diálogo com texto "digitando"; enquanto aberta o jogador não anda | Padrão do gênero | `dialogue.rs` |
| Arte feita por 4 agentes em paralelo a partir de **`art/CONTRACT.md`** (nomes, tamanhos, quadros, pegadas) | O contrato evita desencontro entre arte e código; fica como referência | Mudou o contrato? Mude `sprites.rs` junto |

## Rodada 2 (27/10/2026): top-down, menu, saves e criador de personagem

| Decisão | Por quê | Para mudar |
|---|---|---|
| **Top-down 16x16** (estilo Stardew) no lugar do isométrico 2:1 | Pedido seu; tiles quadrados emendam fácil e a arte rende mais detalhe | — (os itens marcados como isométricos abaixo ficaram obsoletos) |
| Mapa **64x48** em `assets/maps/farm.txt` (1 caractere por tile), embutido no binário | Editável à mão, sem editor de mapas; sem carregamento assíncrono | Trocar `include_str!` por um asset ou por um editor (ex.: LDtk) |
| pixelwind ganhou **spritesheets** (`frames`, `columns`, `from`, `offset`) | Animação de caminhada e variações de tile sem duplicar JSON | Recurso opcional |
| Personagem em **4 peças tingíveis** (corpo, calça, camisa, cabelo) multiplicadas pela cor escolhida | 6 peles × 3 cabelos × 8 cores × 8 camisas × 6 calças com só 6 PNGs | Cores em `src/character.rs` (`SKINS`, `HAIR_COLORS`...) — ficam no código, não no tema |
| Animação: 3 direções × 3 quadros (lado espelhado) | Padrão Stardew com o mínimo de desenho | Mais quadros = mais colunas na folha + `Facing::frame` |
| **3 slots** de save; **salva sozinho ao amanhecer** (como dormir no Stardew); sair pela pausa perde o dia em curso | Um ponto de save só = sem estado no meio da noite para restaurar (fornalhas ligadas, Nêmesis etc.) | Salvar a qualquer hora exige guardar `Clock.t` e a fase, e pular os efeitos de entrada da noite ao carregar |
| Save em JSON: `localStorage` no navegador, `saves/slotN.json` no nativo | Zero infraestrutura; sobrevive a recarregar a página | Nuvem/conta exigiria backend (o Auth MS do grupo, por exemplo) |
| localStorage via `wasm-bindgen` direto, sem a feature `Storage` do `web-sys` | Ligar uma feature nova do `web-sys` recompila o Bevy inteiro para wasm | Se um dia precisar de mais APIs do navegador, ligue as features de uma vez só |
| Saves trazem `version`; versão diferente aparece como slot vazio | Simples enquanto o jogo muda rápido | Escrever migração quando houver jogadores de verdade |
| Menu com a **fazenda de fundo** e câmera passeando | Clima Stardew sem arte extra | `menu.rs` (`pan_camera`) |
| Telas do menu são **redesenhadas inteiras** quando algo muda | Pouco código, sem sincronizar widgets | Se ficar lento (não deve), atualizar só os textos |
| Zoom inteiro automático (~270 px de altura visível) e câmera presa às bordas do mapa | Pixel perfeito em qualquer janela | `VIEW_HEIGHT` em `cycle.rs` |
| Árvores e casa ficam **translúcidas** com o jogador atrás | Resolve a pendência da rodada 1 | `Occluder` em `map.rs` |
| Arte feita por dois agentes em paralelo (cenário e personagens) seguindo um contrato de nomes/quadros | Tempo | Os JSONs são a fonte; o gerador Python que fez árvores/casa ficou fora do repo |
| Placa na saída leste: "Vila Serena (estrada em obras)" | Gancho para a próxima rodada (vila e interiores) | `farm.txt` (`g`) |

# Rodada 1 (26–27/10/2026)

Feitas sem consulta, seguindo "a abordagem mais simples e econômica". Cada item diz o que foi
decidido, por quê, e o que muda se você quiser outra coisa.

## Ferramenta de arte (pixelwind)

| Decisão | Por quê | Para mudar |
|---|---|---|
| Tema em **TOML** (`art/theme.toml`), assets em **JSON** | TOML é legível para o brand kit; JSON é fácil de gerar/colar para assets | Trocar o parser em `tools/pixelwind/src/lib.rs` (`Theme::parse` / `Asset::parse`) |
| Paletas dia/noite com **as mesmas chaves semânticas** (validado) | Um desenho gera as duas versões: a dualidade vira regra da ferramenta | Remover a validação em `Theme::validate` se quiser cores exclusivas de uma fase |
| Matriz 2D como **lista de strings + legenda de caracteres** | Muito mais compacta e legível que uma matriz de chaves `[["stone","eye"]]` | Dá para aceitar os dois formatos adicionando um campo `grid` ao `Layer` |
| Extras além do pedido: `mirror`, `only` (camada só de dia/noite), formas (`rect`, `ellipse`, `iso_diamond`) com `noise` determinístico | Simetria corta erros e metade do texto; `only` permite transformações (Padeiro → Ghoul) no mesmo asset; formas geram tiles sem desenhar pixel a pixel | Todos opcionais; assets antigos continuam válidos |
| Contorno 4-vizinhos, **dentro** do canvas (não aumenta o tamanho) | Mantém o tamanho do grid previsível | Deixe 1 px de margem nos desenhos |
| CLI sem `clap` (parse manual) | Uma dependência a menos; são só 5 opções | — |
| PNGs gerados **são commitados** em `assets/sprites/` | O jogo roda sem precisar compilar a ferramenta | Ignorar no git e rodar `cargo art` no build |
| Personagens desenhados **de frente** (16x20), não em 3/4 isométrico | Isométrico real de personagem dobra o trabalho de arte; o resultado lê bem sobre tiles 2:1 | Redesenhar os JSONs de personagem |

## Jogo

| Decisão | Por quê | Para mudar |
|---|---|---|
| Bevy com features `2d`, `ui`, `audio` (sem 3D) | `.wasm` muito menor e build mais rápido | `Cargo.toml` |
| Perfil `web` (dev sem debuginfo) + runner que remove debuginfo/nomes do `.wasm` | Com debuginfo o `.wasm` passava de 1 GB e travava o link (WSL com ~5 GB) | `Cargo.toml`, `tools/wasm-runner.sh` |
| `.wslconfig`: WSL com 5 GB de RAM e 8 GB de swap | O link do Bevy estourava a memória | `C:\Users\eliasolie\.wslconfig` + `wsl --shutdown` |
| Fonte **VT323** (OFL) em `assets/fonts/` | A fonte padrão do Bevy não tem acentos | Qualquer TTF com Latin-1 |
| Vertical slice com **1 morador (Padeiro/Gula)** e **1 máscara (Gárgula)** | Prova todos os sistemas do GDD (dualidade, B2C/B2B, máscara metroidvania, Inquisidor, Nêmesis) com escopo de uma noite | Os outros 6 pecados e 2 máscaras estão no roadmap |
| Dia 120 s, noite 90 s; `N` pula a fase | Ciclo rápido para testar | `cycle.rs` (`DAY_LEN`, `NIGHT_LEN`) |
| Economia: semente 2, cenoura 6 (3 com Inquisidor), máscara 40, fornalha 60 × 1,6ⁿ, essência 4 ouro, produção `n · 0,5 · 1,15ⁿ`/s só à noite | Dia linear e limitado, noite exponencial (como no GDD) | Constantes no topo de `economy.rs` |
| Maldição 0–100: exportar essência sobe (+0,35/un.); vender cenoura (−1/un.), pão do Padeiro (−5/dia) e amanhecer (−5) descem | Equilíbrio "beirando o colapso" | `economy.rs` |
| Inquisidor ao amanhecer se maldição ≥ 60: preço B2C pela metade + sementes confiscadas; sai pagando multa (50 + 20% do ouro) ou se a maldição cair abaixo de 35 | GDD §5 | `INQUISITOR_AT`, `INQUISITOR_LEAVES_AT`, `Game::fine` |
| Nêmesis ao anoitecer se maldição ≥ 85: uma fornalha é sabotada | GDD §5 (sabotagem de infraestrutura) | `NEMESIS_AT` |
| Ghoul: cada cenoura dada à noite = +12 essência; 3 numa noite fecham a padaria no dia seguinte (sem pão, sem −5 de maldição) | GDD §6 (Gula) | `GHOUL_GLUTTONY` |
| Fornalhas de dia aparecem como **montes de feno** | A fachada perfeita; reforça a dualidade sem código extra | `art/sprites/factory.json` |
| Piso rachado: com a Gárgula, à noite, vira caverna (+40 essência, depois +15/noite) | Mecânica metroidvania da Gárgula, em versão mínima | `economy.rs` (`BreakFloor`, `Mine`) |
| Meta de 1000 de ouro | Dá um objetivo ao vertical slice | `GOAL` |
| `G` (+100 ouro) e `C` (+30 maldição) **só em builds de desenvolvimento** | Testar o loop noturno, Inquisidor e Nêmesis sem grind | Somem sozinhos no `--release` |
| Sazonalidade (Primavera, Verão, Outono, Inverno) com 7 dias/estação (28 dias/ano) | Ciclo de ano rural clássico, tiles de chão e árvores mudam por estação, reflorestamento de árvores cortadas na virada | `cycle.rs`, `terrain.rs`, `map.rs` |
| Florinda vendendo sementes sazonais (`Shop::Florist`) | Cada estação tem 3–4 culturas exclusivas (15 espécies no total) com tempos de maturação e cores próprias | `shop.rs`, `inventory.rs`, `economy.rs` |
| Seu Bartô vendendo animais da fazenda e carnes do açougue (`Shop::Store`) | Certidões de animais (Galinha, Galo, Vaca, Touro, Ovelha, Carneiro, Cabra, Bode, Coelho, Coelha, Égua, Cavalo) e cortes de açougue (Bovino, Frango, Cordeiro, Coelho) | `shop.rs`, `animals.rs`, `inventory.rs` |
| Produtos animais de coleta diária | Ordenha de vacas e cabras, tosquia de ovelhas, ovos de galinha e lã de coelho ao interagir com fêmeas/machos com produto; reset ao amanhecer | `animals.rs`, `economy.rs` |
| Culinária no fogão a lenha (`Shop::Kitchen` / `Prop::Oven`) | 8 receitas nutritivas (Salada da Estação, Ensopado Campeiro, Torta de Morango, Omelete, Sopa de Abóbora, Frango Assado, Assado de Cordeiro, Geleia de Mirtilo) consumindo colheitas e carnes | `shop.rs`, `economy.rs` |
| Proteção animal cozy e maldições em receitas/carnes/vegetais | Animais **nunca** viram monstros à noite; colheitas, carnes e receitas feitas/vendidas à noite interagem com os fluxos sombrios e maldição sem afetar os animais | Diretriz Janus #21 |
| Qualidade (Quality) em colheitas, animais e pratos | Bronze (1.25x 🥉★), Prata (1.5x 🥈★★), Ouro (2.0x 🥇★★★) e Sobrenatural (3.0x 🔮✦✦✦✦); `Quality::roll` probabilístico desacoplado do período dia/noite (52% normal, 25% bronze, 13% prata, 7% ouro, 3% sobrenatural) | `inventory.rs`, `shop.rs`, `economy.rs`, `tools.rs` |
| Banca da Fazenda (`Shop::Stand`) | Barraca com popup para venda direta de qualquer colheita, corte ou prato da mochila com multiplicador de qualidade, além de exportação de essência à noite | `shop.rs`, `economy.rs` |
| QoL contextual (`WorldBalloon`) | Balão flutuante em mundo real sobre plantações (tempo restante e estado regado ou pronto) e sobre animais (prontidão de leite/lã ou carinho) | `hud.rs`, `economy.rs` |
| Scroll e abas no Armazém & Lojas | Abas [Comprar] e [Vender] no Armazém, scroll suave via rodinha do mouse e botões de navegação [▲ Subir] e [▼ Descer] | `shop.rs` |
| Alto contraste e TextShadow na UI | TextShadow preto em todas as fontes brancas, números da toolbar ampliados (20px) e indicadores rápidos de atalho (1..0) | `ui.rs`, `hud.rs`, `shop.rs` |
| Correção do Crash de Tela Branca | Remoção de parâmetro conflitante `Res<Clock>` + `ResMut<Clock>` em `game_actions` (`error[B0002]`) que causava pânico na inicialização do Bevy | `shop.rs` |
| Pipeline Windows & Aceleração GPU Nativa | WSLg com aceleração direta via GPU NVIDIA (Direct3D 12 pass-through) e cross-compilação Windows nativa via `cargo-xwin` (`x86_64-pc-windows-msvc`) | Toolchain, `cargo-xwin` |
| Alinhamento de Grid & Atlas Slicing | Correção do bug onde sprites adultos/tintados sobrescreviam o `Sprite` e perdiam o `texture_atlas`, esticando 48px fora do grid. Bundling direto com preservação de atlas | `economy.rs`, `animals.rs`, `tools.rs` |
| Z-Index de Diálogo & Letterbox | Elevação da `DialogueBox` para `GlobalZIndex(105)` e `bottom: 80px` evitando sobreposição da tarja preta de cinema (`GlobalZIndex(99)`). Sistema de retratos/avatares e avanço ágil de texto | `dialogue.rs`, `cutscene.rs` |
| Otimização de Ações & Repaint Seletivo | Eliminação do stutter/peso em ações de arar/regar: repintura pontual 1x1 (`repaint_exact`) em vez de 3x3 (redução de 89% em custo de CPU) e guardas de detecção de mudança no balão QoL | `map.rs`, `economy.rs`, `hud.rs`, `Cargo.toml` |

## Testado no navegador (wasm)

Plantar → colher → vender (B2C) · ciclo dia/noite com troca de todos os sprites · forjar
máscara · erguer fornalha (feno de dia) · produção noturna · exportar essência (B2B) ·
maldição subindo/descendo · Inquisidor chegando ao amanhecer e sumindo à noite · Nêmesis
sabotando uma fornalha. Console sem erros.

Não testado na prática (só o código): alimentar o Ghoul/padaria fechada, quebrar o piso
rachado/minerar, pagar a multa do Inquisidor.

## Pendências conhecidas

- **No WSL o jogo roda a ~24 fps com engasgos** (medido 08/11/2026 com o registrador de
  quadros lentos de dev, `log_spikes` em `main.rs`). Quadros de ~41 ms até no menu, com a CPU
  ~75% ociosa, e igual com vsync ligado, desligado ou mailbox. O gargalo é o caminho gráfico do
  WSLg (Vulkan Dozen → D3D12 na MX250 → cópia da janela para o Windows), não o código do jogo.
  **Resolvido com o build nativo de Windows** (`tools/windows.ps1`, Rust 1.99 MSVC + VS Build
  Tools): 0 quadros lentos em 45 s no menu (contra ~300 no WSL) e a fazenda roda lisa.
- **Troca de mapa engasga ~1 s** (946 ms ao sair de casa para a fazenda, no Windows): é o chão
  assado do mapa grande sendo pintado na CPU. Dá para cachear os mapas já assados ou assar em
  pedaços ao longo de alguns quadros.

- **Árvores escondem o jogador** quando ele passa atrás delas (a ordenação isométrica está
  certa, mas as copas são altas). Solução comum: deixar o objeto translúcido quando o jogador
  está atrás.
- **`cargo web` leva ~5 min para subir** mesmo com o build em ~1 min: o wasm-bindgen processa o
  `.wasm` de desenvolvimento (~60 MB). Testei o `wasm-bindgen` direto e foi mais lento (~7 min).
  Saídas possíveis: build release com LTO para a web (`.wasm` bem menor), ou iterar no nativo.

- **Build nativo (`cargo run --features dev`) compila sem erros, mas o primeiro build levou
  ~2h44** (dividiu CPU/RAM com o servidor wasm e compila o Bevy inteiro com debuginfo). Os
  builds seguintes são incrementais. Se ficar lento, `debug = "line-tables-only"` no `profile.dev`.

- **Sem save** (recarregar a página reinicia). Próximo passo natural: salvar `Game` + ocupantes do mapa no `localStorage`.
- **Sem som** ainda (a feature `audio` já está ligada para a trilha acústica/jazz).
- **Deploy web**: `cargo web` é só para desenvolvimento. Para publicar: build `--release`,
  `wasm-bindgen --target web` e um `index.html` estático (ou usar o `bevy` CLI / `trunk`).
  O `wasm-bindgen-cli` 0.2.129 (mesma versão do Cargo.lock) já está instalado.
