# Cozy Curses

> 🇺🇸 A cozy farming game by day, a cursed B2B soul-furnace empire by night. Built in **Rust + Bevy, wasm-first**, with every sprite generated from declarative JSON by **pixelwind**, a small CLI I wrote that turns palette-keyed pixel maps into pixel-perfect PNGs with automatic outlines and day/night variants. Game design doc in [docs/GDD.md](docs/GDD.md).

De dia, você planta cenouras numa vila Ghibli. De noite, veste uma máscara de gárgula e
toca um império B2B de fornalhas de almas, enquanto os vizinhos viram monstros sem saber.

Rust + Bevy 0.20, **wasm first**. Design completo em [docs/GDD.md](docs/GDD.md).

## Jogar

```bash
cargo web
```

Abra http://127.0.0.1:1334. Controles: **WASD/setas** andam, **E/Espaço** interage,
**N** pula para o fim da fase.

## Arte: pixelwind

Os sprites não são desenhados em editor nem gerados por IA de imagem: são arquivos JSON
declarativos (`art/sprites/`) que usam só as chaves de cor do brand kit (`art/theme.toml`).
A CLI `pixelwind` (`tools/pixelwind`) converte tudo em PNG pixel-perfect, com contorno
automático e versões dia/noite.

```bash
cargo art                                                   # gera assets/sprites/*.png
cargo pixelwind --out target/preview --scale 8 art/sprites  # prévias ampliadas
```

Formato dos assets e protocolo de criação: [CLAUDE.md](CLAUDE.md).
