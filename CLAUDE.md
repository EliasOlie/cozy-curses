# Cozy Curses

Farming sim em Rust + Bevy 0.20, **desktop first** (o wasm ainda compila, mas builds e testes
são nativos). Projeto pessoal do Elias: responda em PT-BR,
direto, sem a estrutura "Problema Comercial / Conceito / ..." das instruções corporativas.

- Design (fonte de verdade): `docs/GDD.md`. Mantenha o contraste "Cozy" × "Curse", o cinismo
  do capitalismo do jogador e a ironia trágica dos moradores que não sabem o que são à noite.
- Decisões técnicas e estado atual: `docs/DECISIONS.md`.

## Comandos

- `.\tools\windows.ps1` (PowerShell do Windows) — **alvo principal**: build nativo de Windows
  (MSVC, Vulkan direto no driver da NVIDIA), roda liso. Artefatos em `%USERPROFILE%\cozy-target`.
  `.\tools\windows.ps1 build` só compila. O Smart App Control pode barrar um script de build na
  primeira vez: rode de novo.
  Para medir desempenho: `$env:COZY_PROFILE = "1"` antes de rodar imprime ms/quadro por
  segundo (e onde o tempo foi gasto); `$env:COZY_NOVSYNC = "1"` tira o vsync.
- `cargo run --features dev` — janela pelo WSLg: serve para testar rápido, mas fica em ~24 fps
  com engasgos (limite do caminho gráfico do WSL, não do jogo)
- `cargo web` — compila para wasm (perfil `web`) e serve em http://127.0.0.1:1334 (secundário)
- `cargo art` — regenera todos os sprites (`art/sprites/*.json` -> `assets/sprites/*.png`)
- `cargo pixelwind --out target/preview --scale 8 art/sprites` — prévias ampliadas
- `cargo test -p pixelwind --target-dir target/tools` — testes da ferramenta

Controles: WASD anda (Shift corre) · E ou clique direito interage · Espaço ou clique esquerdo
usa o item da barra no tile à frente (com foice/espada e praga por perto, golpeia) · 1-0/rodinha escolhem o item · I/Tab mochila · F celular
(rolar = +10 min) · Alt mostra os números · H ajuda · F3 diagnóstico · Esc fecha/pausa.

Mundo: `assets/maps/world.json` (mapas, portas → interiores, bordas → outros mapas) + um `.txt`
por mapa; o `build.rs` embute a pasta inteira. Tudo isso é editável no editor do jogo.
Interface: `cargo ui-art` gera as molduras 9-slice de `art/ui` em `assets/ui` (3x).

Atalhos de dev no jogo (só em builds não-release): `G` +100 ouro, `C` +30 maldição, `N` pula
para o próximo marco.

Cutscenes: um JSON por cena em `assets/cutscenes` (formato, gatilhos e como testar em
`docs/CUTSCENES.md`). No desktop não precisa recompilar; F5 relê a pasta em builds de dev.

Editor de mapas: menu principal → "Editor de mapas". No nativo, salvar grava em
`assets/maps/*.txt`; no navegador, no localStorage (e vale no próximo jogo).

Mapas: `assets/maps/*.txt` (um caractere por tile; a legenda está em `src/map.rs`, função
`build`). Portas, capachos `E` e as saídas `<`/`>` são lidos do próprio mapa. Contrato de todos
os assets (nomes, tamanhos, quadros, pegadas): `art/CONTRACT.md`. Saves: `localStorage` no
navegador, `saves/` no nativo.

A máquina tem pouca RAM: `.cargo/config.toml` limita `jobs = 4`. Não rode dois builds do Bevy
ao mesmo tempo. O pixelwind usa `--target-dir target/tools` para não disputar o lock.

## Protocolo de geração de assets (OBRIGATÓRIO)

Quando o usuário mandar uma imagem de referência ou descrever um item, roupa, monstro, tile ou
personagem, **não gere imagens**. Atue como tradutor visual:

1. Analise a referência.
2. Escreva o asset declarativo JSON do pixelwind que representa aquele sprite (vista de
   cima/3-4 estilo Stardew Valley, tiles 16x16), usando **apenas as chaves de cor de
   `art/theme.toml`**. Nunca invente hex.
3. Entregue **apenas o bloco de código JSON** do asset, pronto para salvar em
   `art/sprites/<nome>.json` e rodar `cargo art`.

Se estiver trabalhando direto no repositório, salve o arquivo, rode `cargo art`, confira a
prévia com `--scale 8` e corrija o que o validador apontar.

### Formato do asset

```json
{
  "name": "nome_do_arquivo",
  "size": [16, 16],
  "palette": "both",
  "outline": true,
  "notes": "o que é / de onde veio a referência",
  "legend": { "s": "stone", "e": "eye" },
  "layers": [
    { "name": "base", "mirror": true, "rows": ["..ss", ".see"] },
    { "only": "night", "offset": [6, 10], "rows": ["ee"] },
    { "shape": "iso_diamond", "rect": [0, 0, 32, 16], "color": "grass",
      "noise": [{ "color": "grass_light", "density": 0.12, "seed": 1 }] }
  ]
}
```

- `palette`: `day`, `night` ou `both`. `both` gera `<nome>_day.png` e `<nome>_night.png` com o
  mesmo desenho; as paletas compartilham chaves, então a versão noturna sai "de graça".
- `.` e espaço são transparentes. Cada caractere de `rows` precisa estar na `legend`.
- `mirror: true`: escreva só a metade esquerda; ela é espelhada (largura dobra).
- `only: "day" | "night"`: camada só existe naquela fase (olhos, brilhos, transformações).
- `shape`: `rect`, `ellipse` ou `iso_diamond` em `rect: [x, y, w, h]`; `noise` pontilha a forma
  de modo determinístico (mesmo `seed`, mesmo resultado).
- Contorno automático com a cor `outline` da paleta da fase (desligue com `"outline": false`,
  por exemplo em tiles de chão). Deixe 1 px de margem para o contorno caber.
- Tamanhos: tiles de chão 16x16 (precisam emendar sem costura); personagens 16x32 (pés em
  y≈30); ícones 16x16; árvores e casas maiores, ancoradas pelo meio da borda de baixo.

### Spritesheets (animação e variações)

```json
{ "name": "farmer_body", "size": [16, 32], "palette": "day", "columns": 3,
  "layers": [ /* desenhadas em todo quadro */ ],
  "frames": [
    { "name": "down_0", "layers": [ ... ] },
    { "name": "down_1", "from": "down_0", "offset": [0, 1], "layers": [ /* pernas */ ] }
  ] }
```

- Cada quadro tem o tamanho `size`; a folha é montada em grade com `columns` quadros por linha.
- `from` herda tudo o que outro quadro desenha; `offset` desloca o herdado (as camadas
  próprias do quadro não). O contorno é calculado por quadro.
- A ordem dos quadros é contrato com o código (`src/sprites.rs`, `src/character.rs`).

### Peças tingíveis do personagem

`farmer_body/pants/shirt/hair_N` usam só `tint_light/tint/tint_dark/tint_deep` nas partes que
mudam de cor; o jogo multiplica pela cor escolhida no criador (`src/character.rs`). Detalhes
fixos (olhos, sapatos) usam cores escuras (`ink`, `outline`). Folha 3x3: linhas baixo, cima,
lado (olhando para a direita); colunas parado, passo A, passo B (passos descem o tronco 1 px).
