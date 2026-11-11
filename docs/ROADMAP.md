# Roteiro: o que foi feito e o que vem

Atualizado em 10/11/2026. O detalhe de cada decisão está em `docs/DECISIONS.md`; aqui fica
só o panorama.

## Feito

| Rodada | Entrega | Estado |
|---|---|---|
| 9 | Foco no desktop, animais, cultivos, ícones, moradores animados | Testado no jogo |
| 10 | Combate: pragas noturnas, foice/espada, vida, desmaio, máscaras | Lodo e Espantalho testados; espada, corvo, desmaio e máscaras não |
| — | Build nativo de Windows (`tools/windows.ps1`) | Testado |
| 11 | Desempenho: MSAA desligado, camadas invisíveis, atlas único de sprites | MSAA medido (30–38 ms → ~11 ms); atlas não medido com a máquina livre |
| 11 | Flickering atrás das árvores, fade da copa, sombras de chão | Sombras conferidas na tela; fade e flickering não testados andando |
| 12 | Cutscenes em dados (`assets/cutscenes`, guia em `docs/CUTSCENES.md`) | Os 8 arquivos carregam; nenhuma cena foi assistida no jogo |
| 13 | Mochila: detalhes do item, juntar pilhas, Shift+clique, organizar, reposição da barra, qualidade por cor; textos sem símbolos que a fonte não tem | Compila; não testado no jogo |

## Para testar no jogo (pendente)

1. Andar atrás das árvores: a copa deve sumir e voltar suave, sem piscar.
2. Uma cutscene do começo ao fim (jogo novo toca a `intro`; o fade agora escurece a tela).
3. A cena de exemplo `village_first_night`: ir à vila de noite.
4. Mochila: passar o mouse nos itens, Shift+clique, tecla R, teclas 1..0 sobre um item,
   gastar todas as sementes da barra com mais na mochila.
5. Combate: espada, corvo comendo plantação, desmaio, efeito das máscaras.
6. Desempenho com o navegador fechado (`$env:COZY_PROFILE = "1"`).

## Próximos passos

### Camada de conteúdo (world building sem recompilar)

1. **Personagens em dados**: `assets/npcs/<nome>.json` com nome, arte, retrato, casa, rotina
   de dia e de noite e falas por estação e condição. Hoje estão em `villagers.rs` e
   `dialogue.rs`.
2. **Mais passos de cutscene**: mover personagem, mover câmera, dar item, mudar a hora.
3. **Editor ampliado**: abas de Personagens, Cutscenes (com botão de testar) e Gatilhos
   (zonas no mapa), ao lado do editor de mapas.
4. **Itens, lojas e cultivos em dados**: por último, porque mexem em mais código.

### Jogo

- Baú de armazenamento (a `StorageChest` já existe como item, mas não guarda nada).
- Dividir pilha e jogar item fora na mochila.
- Atalho de dev `C` (+30 de maldição) conflita com `C` da bancada de criação.
- Chaves de cor roxa/azul na paleta para berinjela, uva e mirtilo.

### Técnico

- Troca de mapa engasga ~1 s (o chão do mapa é pintado na CPU): cachear ou fatiar.
- Executável único para distribuir (release, assets embutidos, sem console, ícone).
- `art/CONTRACT.md` não lista a `shadow_big`.
