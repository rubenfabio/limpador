# ADR-0023 — Portas e adaptadores no núcleo (Adapter e Repository)

- **Status:** Aceita
- **Data:** 2026-10-18
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-04.01, T-09.04, T-11.01 e toda tarefa que toque o mundo externo

## Contexto

O núcleo já recebia dependências por traits (`Clock`, `CommandRunner`, ADR-0004), mas sem uma regra explícita: as implementações reais conviviam com as traits em `ports.rs`, e a configuração era uma struct concreta (`ConfigStore`) usada direto pela casca. Com a quarentena e o histórico em SQLite (ADR-0021) e as APIs do Windows (ADR-0022) chegando, o núcleo passa a ter vários pontos de contato com tecnologias específicas, e cada um precisa poder ser trocado por um dublê nos testes e reutilizado pela CLI e pelo MCP (ADR-0003).

## Decisão

Adotar **portas e adaptadores** (arquitetura hexagonal) no núcleo:

- **`crates/core/src/ports/`**: só traits e tipos de valor, sem tecnologia. Exemplos: `Clock`, `CommandRunner`, `Platform` (arquivos no sistema operacional) e os **Repositories** da persistência (`ConfigRepository`; depois os da quarentena e do histórico).
- **`crates/core/src/adapters/`**: as implementações reais, **nomeadas pela tecnologia** que usam: `SystemClock`, `SystemCommandRunner`, `TomlConfigRepository`, `WindowsPlatform`, `SqliteQuarantineRepository`… Nunca sufixo `Impl`.
- **Dublês de teste** ficam no `limpador-testkit` ou junto dos testes (`Scripted`, `SandboxCommands`, `FixedClock`).
- O domínio, o motor e os coletores dependem **só das portas**; quem escolhe o adaptador é a borda (a casca Tauri, a CLI, o MCP ou o teste).
- Os **coletores** continuam plugáveis como no ADR-0004: são o núcleo do domínio e recebem as portas que usam no construtor.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Portas e adaptadores no núcleo** | Uma regra só para tudo que toca o mundo externo; dublês triviais | Uma camada de trait a mais em cada dependência |
| Só Repository para a persistência | Menor mudança | Plataforma e comandos ficariam sem regra |
| Hexagonal completo, coletores como adaptadores | Separação máxima | Reorganiza todos os coletores sem ganho para o usuário |
| Sufixo `Impl` nos nomes | Familiar em Java/C# | Ambíguo quando há duas implementações (real e dublê) |

## Consequências

**Positivas**
- Trocar SQLite, TOML ou a API do Windows afeta só o seu adaptador.
- A casca depende de `dyn ConfigRepository`, não do formato do arquivo.

**Negativas / custos aceitos**
- Uma indireção (`Arc<dyn …>`/`Box<dyn …>`) em cada dependência externa.

**O que passa a ser obrigatório**
- Código novo que fale com arquivo de dados, banco, processo, relógio ou API do sistema entra como **porta + adaptador**.
- `unsafe` só em adaptador, com justificativa em cada bloco (ADR-0022).
- Testes de adaptador usam recursos reais em pastas temporárias; testes de domínio usam dublês.

## Referências

- ADR-0003, ADR-0004, ADR-0012, ADR-0018, ADR-0021, ADR-0022.
