# Registros de decisão de arquitetura (ADRs)

Cada arquivo registra **uma** decisão: o contexto, o que foi decidido, as alternativas e as consequências. Regras do processo em [ADR-0001](0001-registrar-decisoes-com-adr.md). Modelo em [000-modelo.md](000-modelo.md).

## Como criar um ADR

1. Copie `000-modelo.md` para `NNNN-titulo-em-kebab-case.md`, usando o próximo número.
2. Preencha com status **Proposta**.
3. Apresente as opções ao dono do produto, com uma recomendada.
4. Depois da decisão, mude para **Aceita** (ou **Rejeitada**) e adicione na tabela abaixo.
5. Para mudar uma decisão aceita, crie um ADR novo e marque o antigo como "Substituída por ADR-XXXX". Não apague ADRs.

## Índice

| # | Decisão | Status | Data |
|---|---|---|---|
| [0001](0001-registrar-decisoes-com-adr.md) | Registrar decisões de arquitetura com ADRs | Aceita | 2026-10-17 |
| [0002](0002-tauri-como-stack-desktop.md) | Tauri 2 (Rust + React/TypeScript) como stack desktop | Aceita | 2026-10-16 |
| [0003](0003-nucleo-independente-da-interface.md) | Núcleo como biblioteca independente da interface | Aceita | 2026-10-16 |
| [0004](0004-coletores-plugaveis.md) | Coletores plugáveis com contrato único e dependências injetáveis | Aceita | 2026-10-16 |
| [0005](0005-catalogo-de-locais-orientado-a-dados.md) | Catálogo de locais e ecossistemas orientado a dados | Aceita | 2026-10-16 |
| [0006](0006-modelo-de-risco-e-destino.md) | Modelo de risco em quatro níveis com destino derivado | Aceita | 2026-10-16 |
| [0007](0007-ranking-por-ganho-seguro.md) | Ordenar o ranking por ganho seguro | Aceita | 2026-10-16 |
| [0008](0008-quarentena-por-tipo-no-mesmo-volume.md) | Quarentena por tipo de item, no mesmo volume, com prazo | Aceita | 2026-10-16 |
| [0009](0009-deteccao-de-merge-local-e-github.md) | Detectar merge combinando sinais locais e o GitHub | Aceita | 2026-10-16 |
| [0010](0010-persistencia-local.md) | Persistência apenas local (formato no 0018) | Aceita | 2026-10-16 |
| [0011](0011-estrategia-de-testes.md) | Testes primeiro, mutação por criticidade, e2e em duas camadas | Aceita (execução substituída pelo 0017) | 2026-10-17 |
| [0012](0012-maquina-falsa-e-sandbox.md) | Máquina falsa para testes e sandbox só em builds de teste | Aceita | 2026-10-17 |
| [0013](0013-windows-primeiro.md) | Windows primeiro, com diferenças de plataforma isoladas | Aceita | 2026-10-16 |
| [0014](0014-textos-por-chave-de-traducao.md) | Textos por chave de tradução, inclusive os do núcleo | Aceita | 2026-10-16 |
| [0015](0015-agentes-via-cli-e-mcp-com-aprovacao.md) | Agentes via CLI e MCP, com pedidos de aprovação | Aceita (Fase 2) | 2026-10-16 |
| [0016](0016-estrutura-do-repositorio-e-ferramentas.md) | Estrutura do repositório e ferramentas (pnpm, Biome, comandos únicos) | Aceita | 2026-10-17 |
| [0017](0017-ci-local.md) | CI local via hooks do git e tarefa agendada | Aceita | 2026-10-17 |
| [0018](0018-formato-de-persistencia.md) | Configurações em TOML e histórico em SQLite | Aceita | 2026-10-17 |

## Decisões em aberto (viram ADR quando a tarefa chegar)

- Biblioteca SQLite e esquema do histórico (T-09.04 / T-11.01).
- Biblioteca de estado e roteamento da interface (T-08.01).
- Método de compactação de discos virtuais por edição do Windows (T-03.07).
- Como detectar "arquivo em uso" no Windows (T-04.01, T-09.02).
