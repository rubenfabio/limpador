# Estratégia de testes

Um app que **apaga coisas** precisa provar que só apaga o que deve. A estratégia combina três tipos de teste, cada um com um papel.

## Decisões

| Tema | Decisão |
|---|---|
| Disciplina | **Testes primeiro**: cada critério de aceite (CA) vira um teste que falha antes de existir código |
| Unitários | Núcleo em Rust (`cargo test`) e interface (Vitest) |
| Mutação | `cargo-mutants` (Rust) e StrykerJS (TypeScript), com **metas por criticidade** |
| E2E | **Duas camadas**: interface com núcleo simulado (Playwright) + app real (WebDriver: `tauri-driver` + WebdriverIO) |
| Execução | Local (verificações rápidas antes do commit) + CI no GitHub Actions em Windows |

## A pirâmide

```
            ▲   E2E app real (WebDriver)        ~10 jornadas críticas · lento · máxima fidelidade
           ▲▲▲  E2E interface (núcleo simulado) telas e fluxos · rápido
         ▲▲▲▲▲▲ Integração do núcleo            coletores contra a "máquina falsa" (git real, Docker simulado)
      ▲▲▲▲▲▲▲▲▲ Unitários                       regras puras: risco, ganho, vereditos, formatação
  ───────────── Mutação                         mede se os testes acima realmente pegam defeitos
```

| Camada | O que prova | Onde roda |
|---|---|---|
| **Unitário** | Regras puras de negócio, isoladas (classificação de risco, ganho seguro, vereditos de merge, formatação) | Local e CI, a cada commit |
| **Integração do núcleo** | Cada coletor contra a máquina falsa: encontra, classifica, limpa, coloca na quarentena e restaura de verdade em pastas temporárias | Local e CI |
| **E2E interface** | Telas, filtros, seleção, confirmações e estados, com o núcleo simulado (respostas fixas via mock do IPC do Tauri) | Local e CI |
| **E2E app real** | O binário compilado, interface + núcleo juntos, contra uma máquina falsa completa | CI (Windows) e local sob demanda |
| **Mutação** | Que os testes detectam defeitos introduzidos de propósito | CI: incremental no PR, completa semanal |

## A "máquina falsa"

Os testes de integração e o e2e do app real nunca tocam a máquina de verdade (I1–I10). Eles usam um **gerador de cenários** que monta, numa pasta temporária, uma máquina completa:

- pastas raiz com projetos de vários ecossistemas, ativos e parados (datas controladas);
- repositórios git **reais** com merge normal, squash, rebase, branches abandonadas, worktrees (limpas, sujas, travadas, órfãs), stashes, remotos locais simulando o GitHub;
- caches globais, Downloads, Temp e locais de ferramentas de IA nos caminhos do catálogo, apontados para a pasta temporária;
- **Docker e WSL simulados**: dublês que respondem como os comandos reais (imagens, containers, volumes, discos virtuais) e registram o que foi pedido;
- respostas simuladas da API do GitHub;
- relógio controlado.

O app entra no "modo sandbox" por configuração **disponível só em builds de teste**. O build de produção não aceita redirecionar o catálogo nem os dublês.

## Metas de mutação

| Nível | Meta | Código coberto |
|---|---|---|
| **Crítico** | **≥ 90%** | Classificação de risco, destino e elegibilidade (T-01.02) · ganho seguro (T-01.03) · proteções (T-02.04) · vereditos de merge, worktrees e stashes (T-05.02–T-05.06) · revalidação (T-09.02) · quarentena: entrada, restauração e purga (T-09.04–T-09.06) · elegibilidade da limpeza automática (T-10.03) · permissões de agentes (T-12.02) |
| **Padrão** | **≥ 70%** | Todo o resto do núcleo e da lógica da interface |
| Fora da medição | — | Código gerado, ponto de entrada do app, componentes puramente visuais |

- Meta não atingida **bloqueia** o merge. A nota é calculada **por arquivo**: (mortos + tempo esgotado) ÷ (mortos + tempo esgotado + sobreviventes). Mutantes que não compilam ou estão ignorados não contam.
- A lista de módulos críticos e o que fica fora da medição estão em [`mutation/politica.json`](../mutation/politica.json). Mudar a lista exige justificativa no PR.
- Mutantes sobreviventes que são equivalentes (não mudam comportamento) podem ser ignorados, **sempre com justificativa**. Sem ela, a mutação falha por configuração inválida:

| Onde | Como ignorar |
|---|---|
| Rust, uma função | `// motivo: ...` na linha de cima de `#[cfg_attr(test, mutants::skip)]` |
| Rust, um mutante específico | entrada em `exclude_re` de `.cargo/mutants.toml`, com `# motivo: ...` na linha de cima |
| TypeScript | `// Stryker disable next-line <Mutador>: <motivo>` |

**Comandos:** `pnpm mutation` (incremental: só o que mudou desde `origin/main`; outra base via `LIMPADOR_MUTATION_BASE`) e `pnpm mutation:full` (tudo). O resumo fica em `reports/mutation/resumo.md`, e os relatórios detalhados de cada ferramenta ficam ao lado.

**Notas de ferramenta:** o Stryker usa o executor `command` (roda `vitest run` por mutante), porque o plugin do Vitest ainda não lê os resultados do Vitest 5. O `cargo-mutants` copia a árvore respeitando o `.gitignore`, porque os links simbólicos do pnpm exigiriam privilégio de administrador no Windows.

## Jornadas E2E do app real

Poucas e críticas. Cada uma roda contra a máquina falsa:

| # | Jornada | Tarefas cobertas |
|---|---|---|
| J1 | Primeira execução: sugere pastas → confirma → analisa → ranking aparece | T-02.01, T-01.05, T-08.01, T-08.02 |
| J2 | "Limpar o seguro agora": seleciona 🟢 → confirma → espaço liberado e log registrado | T-08.05, T-09.01, T-09.03, T-11.01 |
| J3 | Worktree mergeada 🔴 → quarentena → restaurada idêntica | T-05.05, T-09.04, T-09.05 |
| J4 | Item muda entre análise e execução (commit novo) → é pulado com motivo | T-09.02 |
| J5 | Item protegido nunca aparece como limpável | T-02.04 |
| J6 | Docker parado: cartão com aviso, discos virtuais medidos, nada é iniciado | T-03.01, T-03.06 |
| J7 | Análise agendada + limpeza automática: só 🟢 é tocado | T-10.02, T-10.03 |
| J8 | Quarentena expira → purga → espaço liberado e log | T-09.06 |
| J9 | Cancelar análise e cancelar limpeza no meio | T-01.05, T-09.03 |

Jornadas da Fase 2 (agentes via MCP) entram quando F12 começar.

## Rastreabilidade CA → teste

- Todo teste que cobre um critério de aceite leva o identificador no nome. Exemplo: `t01_02_ca3_protected_item_leaves_ranking`.
- Um verificador lê todas as tarefas `concluída`, extrai os CAs e falha se algum CA não tiver teste correspondente.

## Fluxo de uma tarefa (testes primeiro)

1. Ler o processo da feature e a tarefa.
2. Escrever um teste por CA, todos falhando (vermelho).
3. Implementar até todos passarem (verde).
4. Refatorar mantendo verde.
5. Rodar a mutação nos módulos tocados e matar os mutantes sobreviventes relevantes.
6. Se a tarefa toca uma jornada E2E, atualizar ou criar o cenário.
7. Marcar a tarefa como `concluída`.

## Pipeline

| Quando | O que roda | Tempo alvo |
|---|---|---|
| Antes do commit (local) | Formatação, lint, unitários dos módulos alterados | < 1 min |
| Cada push (CI, Windows) | Formatação, lint, unitários, integração do núcleo, E2E interface, E2E app real, rastreabilidade | < 20 min |
| Cada PR (CI) | Tudo acima + **mutação incremental** só no código alterado, com as metas | < 30 min |
| Semanal (CI) | Mutação completa + relatório de tendência | sem limite rígido |

## Cuidados específicos do Windows

- O `tauri-driver` precisa de um `msedgedriver` na mesma versão do WebView2 da máquina.
- E2E do app real roda no Windows e no Linux, **não no macOS** (limitação do WebDriver do Tauri). Quando houver suporte a Mac, o Mac fica coberto pelas outras camadas.
- Fixtures cobrem caminhos longos (> 260), junções, arquivos em uso, nomes com espaço e acento, e diferenças de maiúsculas e minúsculas.
