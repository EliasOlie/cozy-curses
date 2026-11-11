# Contrato de arte (assets ↔ código)

Lista de todo asset que o jogo carrega: nome do JSON em `art/sprites/`, tamanho do quadro,
paleta, quadros (ordem = índice usado no código) e onde ele encosta no chão.
Mudou algo aqui? Mude também `src/sprites.rs` (e o código que usa o índice).

Convenções:
- **Âncora**: objetos e personagens são desenhados a partir do **meio da borda de baixo** do
  sprite, que fica na borda de baixo do tile onde estão (a "base"). Deixe a última linha livre
  para o contorno quando usar contorno automático.
- **Pegada** (footprint): tiles que o objeto bloqueia, em largura × altura, contados a partir
  da base. A porta, quando há, é o tile do meio da fileira de baixo.
- Spritesheets: `frames` em ordem, `columns` por linha (ver CLAUDE.md).
- Só chaves de `art/theme.toml`. Pode criar chaves novas, sempre nas duas paletas.
- Estilo: Stardew Valley — 3/4 de cima, 3-4 tons por material, highlight, contorno escuro;
  noite = mesma forma com a verdade bioluminescente (olhos, brilhos, espinhos), via paleta e
  camadas `only: night`.

## Fazenda (prontos)

| Asset | Quadro | Paleta | Quadros | Notas |
|---|---|---|---|---|
| tile_grass | 16x16 | both | 4 | variações, sem costura |
| tile_path | 16x16 | both | 2 | |
| tile_soil, tile_cracked, tile_cavern | 16x16 | both | 1 | |
| tile_water | 16x16 | both | 2 | animação |
| water_edge | 16x16 | both | 4 | N, L, S, O (overlay sobre a água) |
| decor | 16x16 | both | 12, columns 6 | linha 0: tuft, tuft_tall, flowers_pink, flowers_yellow, pebbles, mushrooms; linha 1: o mesmo, **balançando** (índice + 6) |
| rock, bush | 16x16 | both | 1 | pegada 1x1 |
| fence | 16x16 | both | 5 | post, rail_e, rail_w, rail_n, rail_s |
| tree_oak | 32x48 | both | 3, columns 3 | parado, copa 1 px p/ esquerda, copa 1 px p/ direita (tronco fixo) |
| tree_pine | 24x40 | both | 3, columns 3 | idem; 2 linhas vazias embaixo |
| house | 80x80 | both | 1 | pegada 5x3, porta no meio de baixo; chaminé ~ (60, 3) |
| stand | 32x32 | both | 1 | pegada 2x1 |
| signpost | 16x24 | both | 1 | |
| crop_carrot | 16x16 | both | 3 | broto, crescendo, madura |
| crops | 16x32 | both | 45, columns 3 | linha = cultivo (ordem do enum `CropKind`): 0 Carrot, 1 Strawberry, 2 Cauliflower, 3 Potato, 4 Corn, 5 Tomato, 6 Blueberry, 7 Melon, 8 Pumpkin, 9 Eggplant, 10 Grape, 11 SweetPotato, 12 WinterRoot, 13 WinterCabbage, 14 Garlic; coluna 0 broto, 1 crescendo, 2 madura; índice = linha*3 + coluna. Base da planta em y=28, montinho de terra em y=29 (x 5..10): desenhe com a borda de baixo do quadro na borda de baixo do tile (planta sobe para o tile de cima). Noite: só a madura muda (detalhe amaldiçoado por cultivo) |
| factory | 16x32 | both | 1 | feno de dia, fornalha de noite |
| barn | 80x80 | both | 1 | celeiro, pegada 5x3; portão duplo x 30..49 encostando em y 78; palheiro acima do portão |
| coop | 48x48 | both | 1 | galinheiro, pegada 3x2; porta x 20..27 com rampinha até y 46 |
| windmill | 48x80 | both | 4, columns 4 | pás giram 22.5° por quadro (torre fixa); eixo ~ (23.5, 23.5); pegada 3x2; porta x 20..27 em y 78 |
| cart | 48x32 | both | 1 | carroça com 2 fardos; pegada 3x1 |
| haybale | 16x16 | both | 1 | fardo amarrado; pegada 1x1 |
| haystack | 32x24 | both | 1 | pilha de 3 fardos; pegada 2x1 |
| dock | 16x16 | both | 1 | tábuas de píer sem contorno, emenda nas 4 direções; última linha = sombra (translúcida) |
| rowboat | 32x16 | both | 1 | barquinho a remo (de cima/3-4) |
| reeds | 16x16 | both | 2, columns 2 | taboas; quadro 1 = metade de cima 1 px p/ direita |
| tree_big | 48x64 | both | 3, columns 3 | parado, copa 1 px p/ esquerda, copa 1 px p/ direita (tronco fixo); base do tronco y≈59, x 24 |
| pumpkin | 16x16 | both | 1 | abóbora de enfeite; noite: rosto aceso |
| flowerbed | 16x16 | both | 1 | moita de flores de enfeite |

## Efeitos e interface

| Asset | Quadro | Paleta | Quadros | Notas |
|---|---|---|---|---|
| fx | 8x8 | both | 16, columns 8 | 0-1 borboleta (asas abertas/fechadas; à noite vira mariposa brilhante) · 2-3 folha caindo (2 rotações) · 4-5 vaga-lume (fraco/forte) · 6-8 fumaça (pequena, média, grande) · 9-10 poeira de passo (2 tamanhos) · 11-12 brilho na água (2) · 13 "!" de conversa · 14 coração · 15 moedinha |
| cursor | 6x6 | both | 1 | canto superior esquerdo de um colchete "⌜" (o jogo espelha para os outros 3 cantos) |
| cloud_shadow | 96x48 | day | 1 | mancha suave e irregular só com a chave `shadow` (translúcida) |
| items | 16x16 | day | 75, columns 8 (10 linhas, a última com 3) | ícones de inventário, **sem tintura** (cada item tem o seu): 0 enxada · 1 regador · 2 machado · 3 picareta · 4 foice · 5 sementes de cenoura · 6 cenoura · 7 madeira · 8 pedra · 9 fibra · 10 pão · 11 bolo · 12 essência · 13 moeda · 14 energia (raio) · 15 almoço (sopa) · sementes (pacote com o cultivo): 16 morango · 17 couve-flor · 18 batata · 19 milho · 20 tomate · 21 mirtilo · 22 melão · 23 abóbora · 24 berinjela · 25 uva · 26 batata-doce · 27 raiz de inverno · 28 repolho de inverno · 29 alho · colheitas: 30 morango · 31 couve-flor · 32 batata · 33 milho · 34 tomate · 35 mirtilo · 36 melão · 37 abóbora · 38 berinjela · 39 uva · 40 batata-doce · 41 raiz de inverno · 42 repolho de inverno · 43 alho · animais: 44 leite de vaca (garrafa) · 45 leite de cabra (jarrinha) · 46 ovo · 47 lã (novelo) · 48 pelo de coelho (tufo) · carnes: 49 bovina · 50 frango · 51 cordeiro · 52 coelho · comidas: 53 torta de morango · 54 salada · 55 ensopado · 56 omelete de ervas · 57 sopa de abóbora · 58 frango assado · 59 assado campeiro · 60 geleia de mirtilo · 61 certidão de animal (todas) · 62 cerca · 63 baú · 64 espantalho · 65 aspersor · 66 elixir · 67 incenso · 68 fertilizante arcano · máscaras: 69 gárgula · 70 corvo · 71 névoa · 72 dragão dourado · 73 rústica · armas: 74 espada enferrujada (diagonal, cabo de couro embaixo à esquerda) |
| phone | 80x128 | day | 1 | celular com capinha de cenoura; **tela transparente** em x 8..71, y 16..111 (64x96), onde o jogo desenha o feed |
| shadow | 16x6 | both | 1 | sombra de contato (chave `shadow`, borda pontilhada) sob personagens e itens |
| bubble | 12x12 | both | 3, columns 3 | balão de fala sobre moradores: 0 "!" · 1 "..." · 2 "?"; rabinho embaixo |
| arrow | 7x6 | both | 1 | setinha para baixo que quica sobre o alvo da interação |

## Interface (assets/ui, 3x, 9-slice)

JSONs em `art/ui/`, paleta `day`, gerados com `cargo pixelwind --out assets/ui --scale 3 art/ui`
(o PNG sai 3x; tamanhos e bordas abaixo em 1x — multiplique por 3 no código).

| Asset | Tamanho | Borda 9-slice | Notas |
|---|---|---|---|
| panel | 24x24 | 8 | moldura de madeira com pregos, miolo pergaminho creme |
| panel_dark | 24x24 | 8 | mesma moldura, miolo `ui_glass` translúcido (HUD) |
| button, button_hover, button_pressed | 16x16 | 5 | madeira clara; hover mais claro com brilho; pressed 1 px mais baixo, chanfro invertido |
| slot, slot_selected | 16x16 | — (fixo) | inventário; selecionado com moldura dourada e cantinhos brilhantes |
| dialog | 32x32 | 12 | moldura com florzinha e folhas nos cantos, filete interno |
| tag | 16x12 | 4 | plaquinha de madeira para o nome de quem fala |

## Vila Serena (exterior)

| Asset | Quadro | Paleta | Pegada | Notas |
|---|---|---|---|---|
| tile_cobble | 16x16 | both | — | 2 quadros, calçamento de pedra sem costura |
| bakery | 80x72 | both | 5x2 | Padaria (Padeiro/Gula): pão na vitrine, placa |
| store | 80x72 | both | 5x2 | Armazém (Mercador/Avareza): toldo, caixotes |
| smithy | 80x64 | both | 5x2 | Ferraria (Ferreiro/Inveja): bigorna, fornalha |
| townhall | 112x96 | both | 7x3 | Prefeitura (Prefeito/Preguiça): relógio, bandeira |
| library | 80x80 | both | 5x3 | Biblioteca/escola (Professora/Orgulho) |
| flower_shop | 48x56 | both | 3x2 | Floricultura (Florista/Luxúria): vasos, trepadeiras |
| guardpost | 48x56 | both | 3x2 | Guarita (Xerife/Ira): escudo, lanças |
| fountain | 48x40 | both | 3x2 | chafariz da praça (noite: água bioluminescente) |
| lamppost | 16x32 | both | 1x1 | poste (noite: aceso) |
| bench | 32x16 | both | 2x1 | banco de praça |
| barrel | 16x16 | both | 1x1 | |
| crates | 16x16 | both | 1x1 | pilha de caixotes |
| planter | 16x16 | both | 1x1 | floreira (noite: flores com olhos) |

Todas as construções: porta no meio da borda de baixo, encostando na última linha útil.

## Interiores

| Asset | Quadro | Paleta | Pegada | Notas |
|---|---|---|---|---|
| floor_wood | 16x16 | both | — | 2 quadros, assoalho sem costura |
| wall | 16x16 | both | — | 2 quadros: face da parede (lambri + papel de parede), topo/borda escura |
| rug | 16x16 | both | — | tapete que emenda em qualquer direção |
| doormat | 16x16 | both | — | capacho da saída |
| bed | 16x32 | both | 1x2 | cama de solteiro vista de cima |
| forge | 32x32 | both | 2x1 | bancada de forjar máscaras (bigorna + moldes; noite: máscaras olhando) |
| table | 32x24 | both | 2x1 | |
| oven | 32x32 | both | 2x1 | forno de padaria |
| counter | 48x24 | both | 3x1 | balcão de loja |
| shelf | 32x32 | both | 2x1 | prateleira com potes/mercadorias |
| plant | 16x24 | both | 1x1 | vaso |

## Personagens

| Asset | Quadro | Paleta | Quadros | Notas |
|---|---|---|---|---|
| farmer_body/pants/shirt/hair_0..2 | 16x32 | day | 9, columns 3 | baixo, cima, lado × parado, passo A, passo B; tons tint_* |
| baker | 16x32 | both | 9, columns 3 | morador (ver nota abaixo). Padeiro de touca e avental / Ghoul magro, babando |
| guard | 16x32 | both | 9, columns 3 | morador. Xerife medroso (chapéu, espada na bainha) / Dullahan com elmo debaixo do braço |
| mayor | 16x32 | both | 9, columns 3 | morador. Prefeito de cartola e prancheta / Sapo-gigante (cartolinha, cogumelos, zzz) |
| teacher | 16x32 | both | 9, columns 3 | morador. Professora de coque e livros / Vampira aristocrata com leque |
| merchant | 16x32 | both | 9, columns 3 | morador. Mercador de boina e barbão / Dragão de ossos agarrado ao saco de ouro |
| smith | 16x32 | both | 9, columns 3 | morador. Ferreiro de bandana e avental de couro / Mímico sem rosto, baú de dentes |
| florist | 16x32 | both | 9, columns 3 | morador. Florista de chapéu de palha e cesta / Dríade de esporos com raízes |
| portraits | 32x32 | both | 7, columns 7 | retratos do diálogo (busto, fundo transparente), ordem do enum Kind: 0 Baker, 1 Guard, 2 Mayor, 3 Teacher, 4 Merchant, 5 Smith, 6 Florist. Noite: forma amaldiçoada sorrindo |
| inquisitor | 16x32 | day | 1 | |
| lantern | 8x12 | night | 1 | |
| mask_gargoyle_worn | 12x10 | night | 1 | desenhada em (2, 7) do quadro do fazendeiro |
| mask_gargoyle | 16x16 | night | 1 | ícone |
| chicken | 16x16 | both | 8, columns 4 (4x2) | olhando p/ a direita (o jogo espelha); linha 0 galinha, linha 1 galo (índice + 4); por linha 0-1 andando, 2-3 bicando; pés em y=14 |
| sheep | 24x24 | both | 8, columns 4 (4x2) | olhando p/ a direita; linha 0 ovelha, linha 1 carneiro de chifre enrolado (índice + 4); por linha 0-1 andando, 2-3 pastando (cabeça baixa); pés em y=22 |
| cow | 32x24 | both | 8, columns 4 (4x2) | olhando p/ a direita; linha 0 vaca malhada, linha 1 touro (índice + 4); por linha 0-1 andando, 2-3 pastando (focinho no capim / mastigando); pés em y=22 |
| goat | 24x24 | both | 8, columns 4 (4x2) | olhando p/ a direita; linha 0 cabra, linha 1 bode (chifres grandes, barbicha); por linha 0-1 andando, 2-3 pastando; pés em y=22 |
| rabbit | 16x16 | both | 8, columns 4 (4x2) | olhando p/ a direita; linha 0 coelha branca, linha 1 coelho caramelo malhado; por linha 0-1 pulando (agachado / no ar), 2-3 parado farejando (orelha mexe); pés em y=14 (no quadro 1 está no ar) |
| horse | 32x32 | both | 8, columns 4 (4x2) | olhando p/ a direita; linha 0 égua alazã, linha 1 garanhão baio de crina cheia; por linha 0-1 andando, 2-3 pastando (cabeça no chão, rabo abana); pés em y=30 |
| dog | 16x16 | both | 4, columns 4 | caramelo, olhando p/ a direita; 0-1 sentado abanando o rabo, 2-3 andando; pés em y=14 |

Moradores: folha 3x3 igual à do fazendeiro — linha 0 baixo (de frente), linha 1 cima (de
costas), linha 2 lado olhando para a **direita** (o jogo espelha para a esquerda); colunas
parado, passo A, passo B (índice = linha × 3 + coluna). Nos passos o tronco desce 1 px e as
pernas alternam; pés em y≈30 (o sapo e a dríade usam pés/raízes próprios, mesma altura). A
forma noturna é o mesmo arquivo com camadas `only: day` / `only: night` em todas as direções;
ela pode ser maior/torta, mas cabe no quadro.

## Inimigos noturnos

Só aparecem à noite: paleta `night` gera um PNG só, `assets/sprites/<nome>.png` (sem sufixo).

| Asset | Quadro | Paleta | Quadros | Notas |
|---|---|---|---|---|
| slime | 16x16 | night | 4, columns 4 (64x16) | Lodo: gosma ciano bioluminescente com olhinhos e bolhas verde-ácido. 0-1 parado balançando (0 alto, 1 achatado 1 px e mais largo) · 2 agachado antes do pulo (olhos apertados) · 3 no ar (esticado, base em y=12). Base em y=14 nos quadros 0-2 |
| crow | 16x16 | night | 4, columns 4 (64x16) | Corvo-de-olho: corvo preto com reflexo roxo e olho grande amarelo (pálpebra vermelha) no peito/asa; olhando p/ a **direita** (o jogo espelha). 0 voando asas em cima · 1 voando asas embaixo · 2 no chão bicando (cabeça alta) · 3 no chão bicando (bico no chão). Pés em y=14 nos quadros 2-3 |
| haunt | 16x32 | night | 4, columns 4 (64x32) | Espantalho Errante: chapéu de palha torto, cabeça de saco com sorriso costurado e olhos acesos, flanela vermelha, pernas de graveto; olhando p/ a **direita**. 0-1 andando (cambaleia: no 1 desce 1 px, chapéu atrasa) · 2 preparando o bote (braços erguidos, inclinado p/ trás, boca rasgada brilhando — telegrafado) · 3 investida (inclinado p/ frente, braço esticado, base mais baixa). Pés em y=30 |
