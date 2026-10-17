# Limpador — Visão do produto

## Problema

Quem programa acumula lixo no computador e no Docker: imagens e containers antigos, volumes órfãos, `node_modules` de projetos abandonados, caches de gerenciadores de pacotes, worktrees perdidas, branches locais já mergeadas, modelos de IA baixados uma vez e nunca mais usados.

Antes, uma limpeza periódica manual dava conta. **Com IA, o acúmulo acelerou**: uma mesma pessoa conduz vários projetos em paralelo, e agentes criam worktrees, branches, containers e instalações de dependências o tempo todo. O lixo cresce no ritmo da IA; a limpeza continua no ritmo humano.

## Tese

Não existe uma ferramenta que olhe o ambiente de desenvolvimento **como um todo e organizado por projeto**. As soluções atuais resolvem pedaços isolados (`npkill`, `kondo`, `docker system prune`, `git-trim`) ou são genéricas e não entendem desenvolvimento (CCleaner, CleanMyMac).

O Limpador é um **app desktop** que:
1. analisa tudo o que pode ser limpo e **ranqueia por ganho seguro**;
2. explica cada item: o que é, por que pode sair e como recriar;
3. limpa com segurança, mandando para quarentena o que pode ter dado único;
4. roda em segundo plano, limpa sozinho só o que é seguro e avisa quando o lixo cresce;
5. (Fase 2) integra-se aos agentes de IA para limpar na origem, quando a tarefa termina.

## Decisões tomadas

| Tema | Decisão |
|---|---|
| Plataforma | Windows primeiro (incluindo WSL e Docker Desktop); arquitetura preparada para Mac/Linux |
| Público | Uso próprio primeiro, open source depois |
| Forma | App desktop com ícone na bandeja e análise agendada |
| Stack | Tauri (núcleo em Rust + interface web). Ver `AGENTS.md` |
| Autonomia | Manual para tudo; automático **apenas** para itens 🟢 Seguros, por regras |
| Exclusão | Depende do tipo e do risco: recriável → exclusão direta; conteúdo do usuário ou 🔴 → quarentena |
| Quarentena | 14 dias, configurável |
| Inatividade de projeto | 30 dias, configurável |
| Ranking | Por **ganho seguro** (tamanho × risco × inatividade) |
| Descoberta | Pastas raiz configuráveis + sugestão automática na primeira execução |
| Detecção de merge | Sinais locais do git + consulta ao GitHub (pega squash e rebase merge) |
| Escopo | Coletores de dev + "Arquivos gerais" (Downloads, Temp, CrashDumps) como categoria separada |
| Idioma | Interface em pt-BR, com textos centralizados para tradução |
| Histórico | Log auditável de ações + gráfico de evolução do espaço |
| Integração com agentes | Fase 2, logo após o MVP |
| Qualidade | Testes primeiro; unitários, integração, e2e em duas camadas e mutação com metas por criticidade. Ver `docs/estrategia-de-testes.md` |
| Arquitetura | Decisões registradas como ADRs em `docs/adr/` |

## Conceitos centrais

- **Item limpável**: qualquer coisa que o app pode remover (uma pasta, uma imagem Docker, uma branch...). Tem tamanho, risco, motivo, ação e, quando aplicável, projeto associado.
- **Coletor**: módulo que entende um ecossistema (Docker, Git, Node...), encontra itens e sabe limpá-los.
- **Projeto**: pasta reconhecida como projeto de software dentro das pastas raiz.
- **Risco**:
  - 🟢 **Seguro**: recriável automaticamente, sem ação do usuário e sem perda (caches).
  - 🟡 **Moderado**: recriável, mas exige ação ou tempo (reinstalar dependências, baixar de novo).
  - 🔴 **Cuidado**: pode conter dado que só existe ali.
  - ⛔ **Bloqueado**: em uso, protegido ou com trabalho não salvo. Aparece na lista, mas não pode ser limpo.
- **Ganho seguro**: pontuação que ordena o ranking (regra completa em `tarefas/F01-motor-de-analise/T-01.03-ganho-seguro-e-ranking.md`).
- **Quarentena**: área de espera de onde o item pode ser restaurado até expirar.

## Princípios inegociáveis

1. **Analisar nunca altera nada.** A análise é estritamente leitura.
2. **Nunca destruir trabalho não salvo**: alterações não commitadas, commits que só existem localmente, arquivos ignorados que não são recriáveis (ex.: `.env`).
3. **Tudo é explicado**: nenhum item aparece sem motivo legível.
4. **Revalidar antes de apagar**: o estado pode ter mudado desde a análise.
5. **Tudo é registrado** no log de ações.
6. **Automação só toca 🟢.**
7. **Nunca seguir links simbólicos ou junções** para fora do item sendo limpo.

## Fases

- **MVP**: F01 a F11 (motor, configuração, cinco coletores, interface, limpeza e quarentena, automação, histórico).
- **Fase 2**: F12 (integração com agentes de IA: interface programática, servidor MCP, evento de fim de tarefa).
- **Depois**: suporte a Mac/Linux, tradução para inglês, abertura do código.

## Suposições a revisar

Pontos que defini por inferência e que valem uma confirmação:
- Containers parados com camada gravável acima de 100 MB são tratados como 🔴 (quarentena); abaixo disso, 🟡.
- Instaladores e arquivos compactados em Downloads com mais de 30 dias são 🟡 (exclusão direta); outros arquivos pessoais são 🔴.
- A limpeza automática de 🟢 é oferecida na primeira execução, com sugestão de ficar ligada.
- A análise agendada é adiada quando o computador está na bateria.
- A lixeira do sistema nunca é esvaziada automaticamente.
