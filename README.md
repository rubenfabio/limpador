# Limpador

> Encontra, ranqueia e limpa com segurança o lixo que o desenvolvimento acumula no seu computador.

**Status:** 📐 em planejamento. A estrutura do produto, as decisões de arquitetura e as tarefas estão prontas; a implementação ainda não começou.

## O problema

Quem programa acumula lixo no disco: imagens e volumes antigos do Docker, `node_modules` de projetos abandonados, caches de gerenciadores de pacote, worktrees perdidas, branches locais que já foram mergeadas, modelos de IA baixados uma vez e nunca mais usados.

Antes, uma limpeza periódica manual dava conta. **Com IA, o acúmulo acelerou**: uma mesma pessoa conduz vários projetos em paralelo, e agentes criam worktrees, branches, containers e instalações de dependências o tempo todo. O lixo cresce no ritmo da IA; a limpeza continua no ritmo humano.

As ferramentas existentes resolvem pedaços isolados (`npkill`, `docker system prune`, `git-trim`) ou são genéricas e não entendem desenvolvimento. O Limpador olha o ambiente **como um todo e organizado por projeto**.

## O que ele faz

- **Analisa** tudo o que pode ser limpo, sem alterar nada.
- **Ranqueia por ganho seguro**: no topo fica o que libera mais espaço com menos risco.
- **Explica cada item**: o que é, por que pode sair e como recriar.
- **Limpa com rede de segurança**: o que é recriável é apagado; o que pode conter dado único vai para uma **quarentena** restaurável por 14 dias.
- **Trabalha sozinho**: fica na bandeja, analisa periodicamente, limpa apenas o que é 100% seguro e avisa quando o lixo cresce.
- **(Fase 2) Integra com agentes de IA** via CLI e MCP, limpando worktrees e branches quando a tarefa do agente termina, sempre com a sua aprovação para o que não é seguro.

## O que ele encontra

| Categoria | Exemplos |
|---|---|
| **Docker e WSL** | Imagens sem uso, containers parados, volumes órfãos, cache de build e o **espaço fantasma** dos discos virtuais `.vhdx`, que não encolhem sozinhos |
| **Dependências e builds** | `node_modules`, `.next`/`dist`, `target/`, `.venv`, caches do npm/pnpm/yarn/pip/uv/Go/Cargo/Gradle/NuGet, versões antigas de Node e Python |
| **Git** | Branches mergeadas (inclusive por squash e rebase), worktrees concluídas ou órfãs, stashes antigos |
| **IA e editores** | Worktrees de agentes, modelos locais (Ollama, Hugging Face, LM Studio), navegadores do Playwright, caches do VS Code/Cursor, sessões antigas de agentes |
| **Arquivos gerais** | Instaladores e `.zip` antigos em Downloads, temporários, crash dumps |

O ranking completo, com tamanhos típicos e locais no Windows, está no [catálogo de limpeza](docs/catalogo-de-limpeza.md).

## Segurança primeiro

Um app que apaga coisas precisa merecer confiança. Estas regras são inegociáveis:

- A análise **nunca altera nada**.
- Nunca remove alterações não commitadas, commits que só existem localmente, nem arquivos como `.env`.
- Nunca segue links simbólicos ou junções para fora do item.
- Nunca age sobre algo em uso (arquivo aberto, container rodando, editor aberto).
- Revalida cada item imediatamente antes de removê-lo.
- A automação só toca itens 🟢 seguros.
- Tudo fica registrado num log de ações auditável.

Lista completa em [AGENTS.md](AGENTS.md#invariantes-de-segurança-inegociáveis).

## Stack

- [Tauri 2](https://tauri.app): núcleo em **Rust** e interface em **React + TypeScript**.
- O núcleo é uma biblioteca independente da interface, reutilizada pela CLI e pelo servidor MCP.
- **Windows primeiro** (incluindo WSL e Docker Desktop); Mac e Linux depois.

## Qualidade

- **Testes primeiro**: cada critério de aceite vira um teste antes do código.
- Unitários (`cargo test`, Vitest), integração contra uma **máquina falsa** (git real, Docker/WSL/GitHub simulados), e2e da interface (Playwright) e do app real (WebdriverIO + `tauri-driver`).
- **Testes de mutação** (`cargo-mutants`, StrykerJS): ≥ 90% no código que decide o que apagar e ≥ 70% no resto.

Detalhes em [estratégia de testes](docs/estrategia-de-testes.md).

## Como o repositório está organizado

```
AGENTS.md / CLAUDE.md            instruções para agentes de código
docs/visao.md                    problema, decisões de produto, princípios, fases
docs/catalogo-de-limpeza.md      tudo o que pode ser limpo, ranqueado
docs/estrategia-de-testes.md     pirâmide de testes, mutação, jornadas e2e
docs/adr/                        decisões de arquitetura (ADRs)
features/FNN-*/processo.md       plano de processo de cada feature
tarefas/                         tarefas com regras de negócio e critérios de aceite
```

## Roadmap

| Fase | Conteúdo |
|---|---|
| **F00** | Fundação de qualidade: esqueleto, máquina falsa, e2e, mutação, CI |
| **MVP** | Motor de análise, configuração, coletores (Docker/WSL, dependências, Git, IA e editores, arquivos gerais), interface, limpeza e quarentena, automação, histórico |
| **Fase 2** | Integração com agentes de IA: CLI, servidor MCP, limpeza no fim da tarefa |
| **Depois** | Mac e Linux, interface em inglês |

Acompanhe as tarefas em [tarefas/README.md](tarefas/README.md).

## Contribuindo

O projeto ainda está no início. Para quem quiser acompanhar ou contribuir:

1. Leia a [visão](docs/visao.md) e o [AGENTS.md](AGENTS.md).
2. Decisões técnicas são registradas como [ADRs](docs/adr/README.md): proponha uma antes de mudar algo estrutural.
3. Escolha uma tarefa `pendente` cujas dependências estejam concluídas e siga o fluxo de testes primeiro.

## Licença

A definir.
