# ADR-0022 — APIs do Windows via windows-sys: hardlinks e arquivo em uso

- **Status:** Aceita
- **Data:** 2026-10-18
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-04.01, T-04.03, T-09.02, T-06.04

## Contexto

Dois dados do Windows não estão na biblioteca padrão estável do Rust:

- **Hardlinks**: o pnpm instala `node_modules` com hardlinks do store; o tamanho liberável não pode contar arquivos que continuam existindo em outro lugar (T-04.01). É preciso o número de links e a identidade do arquivo (volume + índice) para contar cada arquivo uma vez.
- **Arquivo em uso**: um item com arquivo aberto por outro processo é ⛔ (I5), e a interface deve dizer qual processo (ex.: `code.exe`).

## Decisão

- Usar a crate oficial da Microsoft **`windows-sys`** no núcleo, só em `cfg(windows)` e isolada no adaptador `WindowsPlatform` da porta `Platform` (ADR-0013, ADR-0023).
- **Hardlinks**: `GetFileInformationByHandle` (número de links e índice do arquivo). Um arquivo com mais de um link só conta como liberável se todos os seus links estiverem dentro do próprio item.
- **Arquivo em uso** (combinado, só na revalidação, imediatamente antes de agir — I6):
  1. abrir cada arquivo do item pedindo **acesso de exclusão** (sem impedir os outros de ler ou escrever) até achar o primeiro que falhe por violação de compartilhamento, que é exatamente quando removê-lo falharia;
  2. perguntar ao **Restart Manager** (`RmGetList`) qual processo usa esse arquivo, para citar o nome no motivo do ⛔.
- Fora do Windows, as duas funções têm implementação neutra (sem hardlinks compartilhados; nada em uso), até haver suporte de outra plataforma.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **windows-sys, abertura com acesso de exclusão + Restart Manager** | Barato até achar o arquivo travado; diz o processo | Uma dependência nova, código `unsafe` isolado |
| Só abertura exclusiva (std) | Sem dependência para "em uso" | Não diz o processo; hardlinks ainda precisariam da API |
| Só Restart Manager | Diz o processo | Registrar milhares de arquivos é lento |

## Consequências

**Positivas**
- Tamanho liberável correto para pnpm; ⛔ com o nome do processo.

**Negativas / custos aceitos**
- `unsafe` passa a existir num único adaptador do núcleo, com justificativa em cada bloco; o lint do workspace vai de `forbid` para `deny`, e só esse módulo o libera.
- A checagem de uso percorre os arquivos do item na revalidação (custo proporcional ao item, só ao limpar).

**O que passa a ser obrigatório**
- Toda chamada à API do Windows fica no adaptador `WindowsPlatform`, atrás da porta `Platform`, testada com recursos reais em pasta temporária (`LockedFile`, hardlinks).

## Referências

- ADR-0013 (Windows primeiro), I3, I5, I6.
