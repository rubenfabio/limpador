# AGENTS.md — Limpador

Instruções para qualquer agente de código (Claude Code, Codex, Cursor etc.) que trabalhe neste repositório.

## O que é

App desktop que encontra, ranqueia e limpa com segurança o lixo que o desenvolvimento acumula: Docker e discos virtuais do WSL, dependências e builds, Git (branches, worktrees, stashes), ferramentas de IA e editores, e arquivos gerais (Downloads, Temp). Visão completa em `docs/visao.md`. Catálogo do que se limpa em `docs/catalogo-de-limpeza.md`.

## Como o trabalho está organizado

```
docs/visao.md                    problema, decisões, princípios, fases
docs/catalogo-de-limpeza.md      ranking e locais de tudo o que pode ser limpo
docs/estrategia-de-testes.md     pirâmide de testes, máquina falsa, metas de mutação, jornadas e2e
docs/adr/                        decisões de arquitetura (ADRs) — índice em docs/adr/README.md
features/FNN-<nome>/processo.md  plano de processo de cada feature (fluxo, regras, ordem)
tarefas/README.md                índice de todas as tarefas, dependências e status
tarefas/FNN-<nome>/T-NN.MM-*.md  uma tarefa: regras de negócio + critérios de aceite
```

As tarefas são **agnósticas de tecnologia**: descrevem regra de negócio e critério de aceite. As decisões técnicas ficam nos **ADRs** e no código.

## Fluxo para executar uma tarefa

1. Escolha em `tarefas/README.md` uma tarefa `pendente` com todas as dependências `concluída`. A feature F00 (qualidade e testes) vem antes de todas.
2. Leia `features/<feature>/processo.md`, depois a tarefa e os ADRs relacionados a ela.
3. **Testes primeiro**: escreva um teste por critério de aceite, com o identificador no nome (`tNN_MM_caK`), e veja todos falharem.
4. Implemente até todos passarem; refatore mantendo verde.
5. Rode a mutação nos módulos tocados e atinja as metas (ver Testes).
6. Se a tarefa habilita uma jornada E2E (J1–J9 em `docs/estrategia-de-testes.md`), crie ou atualize o cenário.
7. Se uma regra estiver ambígua ou conflitar com outra, **pare e pergunte**. Não invente regra de negócio.
8. Atualize o status da tarefa em `tarefas/README.md`.

## Decisões de arquitetura (ADRs)

As decisões técnicas e de arquitetura estão em [`docs/adr/`](docs/adr/README.md), um arquivo por decisão, seguindo o [modelo](docs/adr/000-modelo.md).

- **Antes** de implementar, leia os ADRs relacionados à tarefa. ADRs com status **Aceita** são obrigatórios.
- **Ao precisar de uma decisão nova** (tecnologia, biblioteca, formato, fronteira entre módulos, algo que afete I1–I10 ou seja difícil de reverter): crie um ADR com status **Proposta**, apresente as opções ao dono do produto com uma recomendada, e só implemente depois que ele estiver **Aceita**.
- **Nunca** edite o conteúdo de um ADR aceito. Para mudar uma decisão, crie um ADR novo que a substitua.
- Este arquivo não repete os ADRs; aponta para eles.

## Stack (resumo — detalhes nos ADRs)

- **Tauri 2**: núcleo em **Rust** e interface em **React + TypeScript** ([ADR-0002](docs/adr/0002-tauri-como-stack-desktop.md)).
- **Núcleo independente da interface**, reutilizado por CLI e MCP na Fase 2 ([ADR-0003](docs/adr/0003-nucleo-independente-da-interface.md)).
- **Coletores plugáveis** com dependências injetadas: sistema de arquivos, comandos `git`/`docker`/`wsl`, GitHub e relógio ([ADR-0004](docs/adr/0004-coletores-plugaveis.md)).
- **Catálogo de locais** declarativo, sem caminhos de terceiros fixos no código ([ADR-0005](docs/adr/0005-catalogo-de-locais-orientado-a-dados.md)).
- **Persistência apenas local**; o formato ainda será decidido ([ADR-0010](docs/adr/0010-persistencia-local.md)).
- **Textos por chave de tradução**, inclusive os motivos gerados pelo núcleo ([ADR-0014](docs/adr/0014-textos-por-chave-de-traducao.md)).

## Invariantes de segurança (inegociáveis)

Qualquer código que remova algo **deve** respeitar:

- **I1** A análise nunca altera nada no sistema.
- **I2** Nunca remover alterações não commitadas, commits que não existem em nenhum remoto, nem arquivos ignorados não recriáveis (ex.: `.env*`, chaves, bancos locais).
- **I3** Nunca seguir links simbólicos ou junções para fora do item. Remover o link, nunca o alvo (o pnpm usa junções em `node_modules`).
- **I4** Nunca agir fora das pastas raiz configuradas e dos locais do catálogo.
- **I5** Nunca agir em recurso em uso (arquivo aberto, container rodando, editor aberto sobre o cache).
- **I6** Revalidar o estado de cada item imediatamente antes de removê-lo.
- **I7** Toda ação gera entrada no log de ações, inclusive falhas.
- **I8** A automação só executa itens 🟢 Seguros.
- **I9** Itens do tipo "conteúdo do usuário" (branch, worktree, stash, volume, container com dados, arquivo pessoal, transcrição) ou 🔴 sempre vão para a quarentena, nunca são excluídos direto.
- **I10** Itens protegidos (projetos fixados, caminhos protegidos, branches protegidas) nunca aparecem como limpáveis.

## Testes

Estratégia completa em [`docs/estrategia-de-testes.md`](docs/estrategia-de-testes.md); decisões em [ADR-0011](docs/adr/0011-estrategia-de-testes.md) e [ADR-0012](docs/adr/0012-maquina-falsa-e-sandbox.md).

| Camada | Ferramenta | Papel |
|---|---|---|
| Unitário | `cargo test` (núcleo) · Vitest (interface) | Regras puras |
| Integração do núcleo | `cargo test` + máquina falsa | Coletores limpando e restaurando de verdade em pastas temporárias |
| E2E interface | Playwright, com o IPC do Tauri simulado | Telas, seleção, confirmações |
| E2E app real | WebdriverIO + `tauri-driver` (Windows) | Jornadas J1–J9 com o binário real |
| Mutação | `cargo-mutants` · StrykerJS | **≥ 90%** em código crítico, **≥ 70%** no resto; abaixo disso, bloqueia |

- **Testes primeiro.** Todo critério de aceite tem teste com o identificador `tNN_MM_caK`. O verificador de rastreabilidade falha se faltar algum.
- **Código crítico** (mutação ≥ 90%): risco, destino e elegibilidade · ganho seguro · proteções · vereditos de merge, worktrees e stashes · revalidação · quarentena · limpeza automática · permissões de agentes. A lista vive num arquivo versionado.
- **Nunca** rode limpeza real nos diretórios do usuário durante testes. Use a **máquina falsa** (pastas temporárias, git real, Docker/WSL/GitHub simulados, relógio controlado).
- O modo sandbox existe **só** em builds de teste.
- Testes com Docker real só sob variável de ambiente explícita.
- Cubra os casos do Windows: caminhos longos (> 260), junções, arquivos em uso, nomes com espaço e acento, diferenças de maiúsculas e minúsculas.
- Mutantes equivalentes só podem ser ignorados com justificativa no código.

## Convenções

- Código e identificadores em inglês; documentação e textos da interface em pt-BR.
- Tamanhos sempre em bytes internamente; formatação só na interface (GB com 1 casa decimal).
- Datas internas em UTC; exibição no fuso local.
- Não adicione dependência nova sem necessidade clara.

