# Tarefas

Cada tarefa descreve **regra de negócio e critérios de aceite**, sem decisão de tecnologia. Antes de executar uma tarefa, leia o `processo.md` da feature em `features/`.

**Status:** `pendente` · `em andamento` · `concluída` · `bloqueada`

Uma tarefa só começa quando todas as dependências estão `concluída`. Ao concluir, atualize o status aqui.

**Definição de pronto de qualquer tarefa:** todos os CAs têm teste com o identificador `tNN_MM_caK` (escrito antes do código), todos passam, as metas de mutação dos módulos tocados são atingidas e, se a tarefa habilita uma jornada E2E, a jornada existe e passa. Ver `docs/estrategia-de-testes.md`.

## Ordem sugerida de execução (MVP)

0. **Fundação de qualidade**: F00 (esqueleto, máquina falsa, e2e, mutação, CI, rastreabilidade)
1. **Base**: F01 + F02 (motor, configuração, projetos)
2. **Primeiro valor real**: T-08.07, T-08.06, T-08.01, T-08.02 (ver o ranking) + F03, exceto T-03.04 e T-03.07 (Docker e discos virtuais, o maior ganho na máquina do usuário)
3. **Limpeza segura**: T-08.05 + F09 (confirmação, revalidação, execução, quarentena); depois T-03.04 e T-03.07
4. **Demais coletores**: F04 → F05 → F06 → F07
5. **Interface completa**: T-08.03 → T-08.05
6. **Automação e histórico**: F10 → F11
7. **Fase 2**: F12

## F00 — Qualidade e testes

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-00.01 | [Esqueleto e testes unitários](F00-qualidade-e-testes/T-00.01-esqueleto-e-testes-unitarios.md) | — | concluída |
| T-00.02 | [Máquina falsa](F00-qualidade-e-testes/T-00.02-maquina-falsa.md) | T-00.01 | concluída |
| T-00.03 | [E2E da interface](F00-qualidade-e-testes/T-00.03-e2e-de-interface.md) | T-00.01 | concluída |
| T-00.04 | [E2E do app real](F00-qualidade-e-testes/T-00.04-e2e-do-app-real.md) | T-00.01, T-00.02 | concluída |
| T-00.05 | [Testes de mutação](F00-qualidade-e-testes/T-00.05-testes-de-mutacao.md) | T-00.01 | concluída |
| T-00.06 | [Verificações locais e CI](F00-qualidade-e-testes/T-00.06-verificacoes-locais-e-ci.md) | T-00.03, T-00.04, T-00.05 | pendente |
| T-00.07 | [Rastreabilidade CA → teste](F00-qualidade-e-testes/T-00.07-rastreabilidade.md) | T-00.01 | pendente |

## F01 — Motor de análise

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-01.01 | [Modelo de item limpável](F01-motor-de-analise/T-01.01-modelo-de-item.md) | — | pendente |
| T-01.02 | [Classificação de risco e destino](F01-motor-de-analise/T-01.02-classificacao-de-risco.md) | T-01.01 | pendente |
| T-01.03 | [Ganho seguro e ranking](F01-motor-de-analise/T-01.03-ganho-seguro-e-ranking.md) | T-01.02, T-01.06 | pendente |
| T-01.04 | [Contrato de coletor e registro](F01-motor-de-analise/T-01.04-contrato-de-coletor.md) | T-01.01, T-01.02 | pendente |
| T-01.05 | [Orquestração da análise](F01-motor-de-analise/T-01.05-orquestracao-da-analise.md) | T-01.03, T-01.04 | pendente |
| T-01.06 | [Atividade de projeto e associação](F01-motor-de-analise/T-01.06-atividade-de-projeto.md) | T-01.01, T-02.03 | pendente |

## F02 — Configuração e descoberta

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-02.01 | [Primeira execução](F02-configuracao-e-descoberta/T-02.01-primeira-execucao.md) | T-02.05, T-02.03 | pendente |
| T-02.02 | [Pastas raiz e exclusões](F02-configuracao-e-descoberta/T-02.02-pastas-raiz-e-exclusoes.md) | T-02.05 | pendente |
| T-02.03 | [Identificação de projetos](F02-configuracao-e-descoberta/T-02.03-identificacao-de-projetos.md) | T-02.02 | pendente |
| T-02.04 | [Proteções](F02-configuracao-e-descoberta/T-02.04-protecoes.md) | T-02.03 | pendente |
| T-02.05 | [Preferências e limiares](F02-configuracao-e-descoberta/T-02.05-preferencias-e-limiares.md) | — | pendente |

## F03 — Docker e discos virtuais

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-03.01 | [Detecção do Docker e do WSL](F03-docker-e-discos-virtuais/T-03.01-deteccao-do-docker.md) | T-01.04 | pendente |
| T-03.02 | [Imagens](F03-docker-e-discos-virtuais/T-03.02-imagens.md) | T-03.01, T-01.06 | pendente |
| T-03.03 | [Containers parados](F03-docker-e-discos-virtuais/T-03.03-containers-parados.md) | T-03.01, T-01.06 | pendente |
| T-03.04 | [Volumes órfãos](F03-docker-e-discos-virtuais/T-03.04-volumes.md) | T-03.01, T-09.04 | pendente |
| T-03.05 | [Cache de build e redes](F03-docker-e-discos-virtuais/T-03.05-cache-de-build-e-redes.md) | T-03.01 | pendente |
| T-03.06 | [Espaço fantasma dos discos virtuais](F03-docker-e-discos-virtuais/T-03.06-espaco-fantasma-dos-discos-virtuais.md) | T-03.01 | pendente |
| T-03.07 | [Compactação dos discos virtuais](F03-docker-e-discos-virtuais/T-03.07-compactacao-dos-discos-virtuais.md) | T-03.06, T-09.01 | pendente |

## F04 — Dependências e builds

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-04.01 | [node_modules](F04-dependencias-e-builds/T-04.01-node-modules.md) | T-02.03, T-01.06 | pendente |
| T-04.02 | [Saídas de build](F04-dependencias-e-builds/T-04.02-saidas-de-build.md) | T-02.03 | pendente |
| T-04.03 | [Caches de gerenciadores JS](F04-dependencias-e-builds/T-04.03-caches-de-gerenciadores-js.md) | T-01.04 | pendente |
| T-04.04 | [Python](F04-dependencias-e-builds/T-04.04-python.md) | T-02.03, T-01.06 | pendente |
| T-04.05 | [Outros ecossistemas](F04-dependencias-e-builds/T-04.05-outros-ecossistemas.md) | T-04.02 | pendente |
| T-04.06 | [Versões de runtime](F04-dependencias-e-builds/T-04.06-versoes-de-runtime.md) | T-02.03 | pendente |

## F05 — Git

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-05.01 | [Inventário de repositórios](F05-git/T-05.01-inventario-de-repositorios.md) | T-02.03 | pendente |
| T-05.02 | [Merge por sinais locais](F05-git/T-05.02-merge-por-sinais-locais.md) | T-05.01 | pendente |
| T-05.03 | [Merge via GitHub](F05-git/T-05.03-merge-via-github.md) | T-05.02 | pendente |
| T-05.04 | [Branches locais](F05-git/T-05.04-branches-locais.md) | T-05.02, T-05.03, T-09.04 | pendente |
| T-05.05 | [Worktrees](F05-git/T-05.05-worktrees.md) | T-05.02, T-05.03, T-09.04 | pendente |
| T-05.06 | [Stashes antigos](F05-git/T-05.06-stashes.md) | T-05.01, T-09.04 | pendente |

## F06 — Ferramentas de IA e editores

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-06.01 | [Worktrees de agentes](F06-ferramentas-de-ia-e-ides/T-06.01-worktrees-de-agentes.md) | T-05.05 | pendente |
| T-06.02 | [Modelos locais de IA](F06-ferramentas-de-ia-e-ides/T-06.02-modelos-locais-de-ia.md) | T-01.04 | pendente |
| T-06.03 | [Navegadores de automação](F06-ferramentas-de-ia-e-ides/T-06.03-navegadores-de-automacao.md) | T-02.03 | pendente |
| T-06.04 | [Editores](F06-ferramentas-de-ia-e-ides/T-06.04-editores.md) | T-01.04 | pendente |
| T-06.05 | [Sessões de agentes](F06-ferramentas-de-ia-e-ides/T-06.05-sessoes-de-agentes.md) | T-09.04 | pendente |

## F07 — Arquivos gerais

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-07.01 | [Downloads](F07-arquivos-gerais/T-07.01-downloads.md) | T-01.04, T-09.04 | pendente |
| T-07.02 | [Temporários e crash dumps](F07-arquivos-gerais/T-07.02-temporarios-e-crash-dumps.md) | T-01.04 | pendente |
| T-07.03 | [Lixeira do sistema](F07-arquivos-gerais/T-07.03-lixeira-do-sistema.md) | T-01.04 | pendente |

## F08 — Interface

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-08.01 | [Painel principal](F08-interface/T-08.01-painel-principal.md) | T-01.05, T-08.06, T-08.07 | pendente |
| T-08.02 | [Ranking](F08-interface/T-08.02-ranking.md) | T-01.03, T-08.01 | pendente |
| T-08.03 | [Visões por projeto e categoria](F08-interface/T-08.03-visoes-por-projeto-e-categoria.md) | T-08.02 | pendente |
| T-08.04 | [Detalhe do item](F08-interface/T-08.04-detalhe-do-item.md) | T-08.02 | pendente |
| T-08.05 | [Seleção e resumo](F08-interface/T-08.05-selecao-e-resumo.md) | T-08.02, T-01.02 | pendente |
| T-08.06 | [Progresso e estados](F08-interface/T-08.06-progresso-e-estados.md) | T-01.05 | pendente |
| T-08.07 | [Textos e tradução](F08-interface/T-08.07-textos-e-traducao.md) | — | pendente |

## F09 — Limpeza e quarentena

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-09.01 | [Confirmação](F09-limpeza-e-quarentena/T-09.01-confirmacao.md) | T-08.05 | pendente |
| T-09.02 | [Revalidação](F09-limpeza-e-quarentena/T-09.02-revalidacao.md) | T-01.04, T-09.01 | pendente |
| T-09.03 | [Execução e relatório](F09-limpeza-e-quarentena/T-09.03-execucao-e-relatorio.md) | T-09.02 | pendente |
| T-09.04 | [Entrada na quarentena](F09-limpeza-e-quarentena/T-09.04-entrada-na-quarentena.md) | T-01.04 | pendente |
| T-09.05 | [Restauração](F09-limpeza-e-quarentena/T-09.05-restauracao.md) | T-09.04 | pendente |
| T-09.06 | [Expiração e purga](F09-limpeza-e-quarentena/T-09.06-expiracao-e-purga.md) | T-09.04 | pendente |

## F10 — Automação

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-10.01 | [Bandeja e segundo plano](F10-automacao/T-10.01-bandeja-e-segundo-plano.md) | T-02.05 | pendente |
| T-10.02 | [Análise agendada](F10-automacao/T-10.02-analise-agendada.md) | T-10.01, T-01.05 | pendente |
| T-10.03 | [Limpeza automática](F10-automacao/T-10.03-limpeza-automatica.md) | T-10.02, T-09.03 | pendente |
| T-10.04 | [Alertas](F10-automacao/T-10.04-alertas.md) | T-10.02 | pendente |

## F11 — Histórico

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-11.01 | [Log de ações](F11-historico/T-11.01-log-de-acoes.md) | T-09.03 | pendente |
| T-11.02 | [Evolução e gráficos](F11-historico/T-11.02-evolucao-e-graficos.md) | T-01.05, T-11.01 | pendente |
| T-11.03 | [Retenção](F11-historico/T-11.03-retencao.md) | T-11.01, T-11.02 | pendente |

## F12 — Integração com agentes (Fase 2)

| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-12.01 | [Interface programática](F12-integracao-com-agentes/T-12.01-interface-programatica.md) | MVP | pendente |
| T-12.02 | [Servidor MCP](F12-integracao-com-agentes/T-12.02-servidor-mcp.md) | T-12.01, T-12.04 | pendente |
| T-12.03 | [Evento de fim de tarefa](F12-integracao-com-agentes/T-12.03-evento-de-fim-de-tarefa.md) | T-12.02 | pendente |
| T-12.04 | [Origem dos itens](F12-integracao-com-agentes/T-12.04-origem-dos-itens.md) | T-12.01 | pendente |

## Modelo de tarefa

```markdown
# T-NN.MM — Título

**Feature:** FNN · **Fase:** MVP | 2 · **Depende de:** T-..

## Contexto            (opcional: por que a tarefa existe)
## Regras de negócio   (o que deve acontecer, sem tecnologia)
## Critérios de aceite (- [ ] **CAn** Dado ..., quando ..., então ...)
## Fora do escopo      (opcional)
```
