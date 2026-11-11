# Cutscenes em dados

Cada cutscene é um arquivo em `assets/cutscenes/<id>.json`. Criar, mudar ou apagar um arquivo
não exige recompilar no desktop: o jogo lê a pasta ao abrir (e de novo com **F5**, em builds
de dev). No navegador vale o que estava na pasta na hora da compilação.

## O menor exemplo

```json
{
  "id": "minha_cena",
  "steps": [
    { "say": "Prefeito Anselmo", "text": "Bom dia, {nome}." }
  ]
}
```

## Campos do arquivo

| Campo | Obrigatório | O que faz |
|---|---|---|
| `id` | sim | Nome único. É por ele que o código e o `COZY_CUTSCENE` chamam a cena. |
| `steps` | sim | Lista de passos, tocados em ordem. |
| `title` | não | Nome legível, só para organização. |
| `notes` | não | Anotação livre; o jogo ignora. |
| `skippable` | não | `false` impede pular com Esc. Padrão: `true`. |
| `when` | não | Condições para a cena começar sozinha (veja abaixo). Sem `when`, só o código a toca. |
| `once` | não | Com `when`: `true` toca uma vez por save (padrão); `false` toca toda vez que a condição voltar a valer. |

## Passos

Cada passo usa exatamente uma destas chaves:

| Passo | Exemplo | Observações |
|---|---|---|
| Fala | `{ "say": "Florinda", "text": "Uma frase." }` | Para várias páginas use `"pages": ["...", "..."]` no lugar de `text`. O retrato vem do nome de quem fala. |
| Banner | `{ "banner": "TÍTULO", "subtitle": "linha de baixo", "seconds": 3.5 }` | `subtitle` e `seconds` são opcionais (padrão: 3 s). |
| Pausa | `{ "wait": 1.5 }` | Segundos. |
| Escurecer / clarear | `{ "fade": "out", "seconds": 0.8 }` | `"out"` escurece, `"in"` clareia (padrão: 0,8 s). Um banner entre os dois aparece sobre a tela preta. |

O jogador avança falas e banners com Espaço, E ou Enter, e pula a cena com Esc.

## Variáveis nos textos

Escreva `{chave}` em qualquer fala, nome ou banner:

| Chave | Valor |
|---|---|
| `{nome}` | Nome do personagem |
| `{sitio}` | Nome do sítio |
| `{dia}` | Dia atual |
| `{estacao}` | Estação atual |
| `{ouro}` | Ouro atual |
| `{maldicao}` | Maldição atual, em % |
| `{multa}` | Só em `inquisitor_visit`: valor cobrado |

## Gatilhos (`when`)

Todas as condições escritas precisam valer ao mesmo tempo. A cena toca no momento em que
elas passam a valer.

| Condição | Exemplo | Vale quando |
|---|---|---|
| `map` | `"map": "village"` | O jogador está nesse mapa (o `id` do `world.json`). |
| `phase` | `"phase": "night"` | É `day` ou `night`. |
| `season` | `"season": "autumn"` | É `spring`, `summer`, `autumn` ou `winter`. |
| `min_day` | `"min_day": 3` | Do dia 3 em diante. |
| `min_curse` | `"min_curse": 30` | Maldição em 30% ou mais. |
| `has_mask` | `"has_mask": true` | O jogador está (ou não) de máscara. |

Exemplo completo: `assets/cutscenes/village_first_night.json`.

As cenas já vistas ficam no save (`seen_cutscenes`), que é gravado ao amanhecer.

## Cenas que o código toca

Estas não têm `when`: o jogo as chama pelo `id` em momentos específicos. Dá para reescrever
os passos à vontade, mas não mude o `id`.

| `id` | Quando toca |
|---|---|
| `intro` | Dia 1 de um jogo novo |
| `season_spring`, `season_summer`, `season_autumn`, `season_winter` | Virada de estação |
| `first_mask` | Primeira máscara |
| `inquisitor_visit` | O Inquisidor cobra a multa |

## Testar sem jogar até o momento certo

No PowerShell, antes de abrir o jogo:

```powershell
$env:COZY_CUTSCENE = "minha_cena"
.\tools\windows.ps1
```

Dentro do jogo, **F5** relê a pasta e toca essa cena. O ciclo fica: editar o arquivo, salvar,
F5. Sem a variável, o F5 só relê os arquivos.

## Erros

Um arquivo com erro é ignorado e os outros continuam valendo. A mensagem aparece no terminal
ao abrir o jogo e no registro do jogo depois de um F5, com o nome do arquivo e o motivo, por
exemplo `minha_cena.json: passo 2: "say" precisa de "text" ou "pages"`. Chave desconhecida
(erro de digitação) também é erro.

## O que ainda não existe

Mover personagens, mover a câmera, dar itens e mudar a hora dentro de uma cutscene. O formato
foi feito para receber esses passos depois.
