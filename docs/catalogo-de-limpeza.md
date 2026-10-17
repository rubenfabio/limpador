# Catálogo do que pode ser limpo

Mapa de tudo o que o Limpador deve encontrar, **ordenado pelo espaço típico que ocupa** em máquinas de desenvolvimento. Os tamanhos são faixas observadas na prática e variam muito de máquina para máquina. O ranking que o app mostra é calculado na hora, por ganho seguro (ver F01).

Legenda de risco: 🟢 Seguro · 🟡 Moderado · 🔴 Cuidado · ⛔ Bloqueado

## Ranking geral

| # | Item | Espaço típico | Risco | Coletor | Por que acumula com IA |
|---|---|---|---|---|---|
| 1 | **Espaço fantasma dos discos virtuais** (Docker Desktop e distros WSL) | 10–100+ GB | 🟢 | F03 | O disco cresce com cada build e imagem e **não encolhe** sozinho depois do prune |
| 2 | **Imagens Docker** sem uso e cache de build | 5–80 GB | 🟢/🟡 | F03 | Cada projeto e cada agente sobe um ambiente diferente |
| 3 | **Modelos locais de IA** (Ollama, Hugging Face, LM Studio) | 5–100 GB | 🟡 | F06 | Modelos de 4–40 GB baixados para testar e esquecidos |
| 4 | **Worktrees de agentes** (cada uma com seu próprio `node_modules` e build) | 1–30 GB | 🔴 | F05/F06 | Agentes criam uma cópia de trabalho por tarefa |
| 5 | **`node_modules`** de projetos parados | 5–50 GB | 🟡 | F04 | Muitos projetos e protótipos iniciados |
| 6 | **Downloads** (instaladores, .zip, .iso antigos) | 5–50 GB | 🟡/🔴 | F07 | — |
| 7 | **Saídas de build Rust** (`target/`) | 2–50 GB | 🟡 | F04 | — |
| 8 | **Caches de gerenciadores de pacote** (npm, pnpm, yarn, bun, pip, uv, NuGet) | 2–20 GB | 🟢 | F04 | Instalações frequentes em projetos e worktrees |
| 9 | **SDK e emuladores Android** (AVDs antigos) | 5–30 GB | 🟡 | F04 (futuro) | — |
| 10 | **Gradle / Maven / Cargo registry / Go mod** | 2–15 GB | 🟡 | F04 | — |
| 11 | **Volumes Docker órfãos** | 1–20 GB | 🔴 | F03 | Bancos de dados de ambientes que ninguém mais sobe |
| 12 | **Saídas de build JS** (`.next`, `dist`, `.turbo`, `coverage`) | 1–10 GB | 🟡 | F04 | — |
| 13 | **Containers parados** | 0,1–10 GB | 🟡/🔴 | F03 | — |
| 14 | **Navegadores de automação** (Playwright, Puppeteer, Cypress) | 1–5 GB | 🟡 | F06 | Cada versão de Playwright baixa novos navegadores |
| 15 | **Versões antigas de runtime** (Node, Python) | 1–5 GB | 🟡 | F04 | — |
| 16 | **Editores** (caches, workspaceStorage órfão, versões antigas de extensões) | 1–5 GB | 🟢 | F06 | Cada worktree aberta gera um workspaceStorage novo |
| 17 | **Ambientes Python** (`.venv`, `__pycache__`) | 0,5–10 GB | 🟡/🟢 | F04 | — |
| 18 | **Temporários e crash dumps** | 0,5–5 GB | 🟢 | F07 | — |
| 19 | **Lixeira do sistema** | variável | 🟢 | F07 | — |
| 20 | **Histórico de sessões de agentes** (transcrições, snapshots de arquivos) | 0,1–5 GB | 🔴 | F06 | Cresce com cada sessão |
| 21 | **Branches locais mergeadas e stashes antigos** | ~0 GB | 🟢/🔴 | F05 | **Ganho de organização**, não de espaço. Agentes criam uma branch por tarefa |

> Branches e stashes quase não ocupam espaço, mas são parte da dor. O app mostra os dois números: **espaço recuperável** e **itens de organização**.

## Medição real nesta máquina (16/10/2026)

Disco C: 931 GB, com 449 GB usados (49%). Medição só de leitura nos locais conhecidos:

| Item | Tamanho | Observação |
|---|---|---|
| Downloads | 17,8 GB | Ainda não classificado (instaladores vs. arquivos pessoais) |
| Disco virtual do Docker Desktop (`docker_data.vhdx`) | 17,0 GB | Docker estava desligado; não foi possível medir o uso real interno |
| Disco virtual de distro WSL (`ext4.vhdx`) | 16,3 GB | Uso real interno não medido |
| Navegadores do Playwright | 2,5 GB | |
| VS Code (`AppData\Roaming\Code`) | 1,2 GB | |
| Temp | 0,7 GB | |
| `.cache` | 0,7 GB | |
| Cache de build do Go | 0,5 GB | |
| CrashDumps | 0,3 GB | |
| `.claude` | 0,1 GB | |
| `node_modules` em `C:\Projetos` (8 pastas) | ~0,2 GB | |

Na sua máquina, o grosso está nos **discos virtuais (33 GB)** e em **Downloads (18 GB)**, e não em `node_modules`. Isso reforça que a compactação dos discos virtuais (T-03.07) é uma das tarefas de maior valor.

## Locais conhecidos no Windows (ponto de partida para os coletores)

| Item | Local |
|---|---|
| Disco do Docker Desktop | `%LOCALAPPDATA%\Docker\wsl\disk\docker_data.vhdx` (versões antigas: `...\wsl\data\ext4.vhdx`) |
| Discos de distros WSL | `%LOCALAPPDATA%\wsl\{guid}\ext4.vhdx` e `%LOCALAPPDATA%\Packages\*\LocalState\ext4.vhdx` |
| npm | `%LOCALAPPDATA%\npm-cache` |
| pnpm | `%LOCALAPPDATA%\pnpm\store` |
| yarn | `%LOCALAPPDATA%\Yarn\Cache` |
| pip / uv | `%LOCALAPPDATA%\pip\Cache`, `%LOCALAPPDATA%\uv\cache` |
| Go | `%LOCALAPPDATA%\go-build`, `%USERPROFILE%\go\pkg\mod` |
| Cargo | `%USERPROFILE%\.cargo\registry` |
| Gradle / Maven | `%USERPROFILE%\.gradle\caches`, `%USERPROFILE%\.m2\repository` |
| NuGet | `%USERPROFILE%\.nuget\packages` |
| nvm / fnm / volta | `%APPDATA%\nvm`, `%LOCALAPPDATA%\fnm_multishells`, `%LOCALAPPDATA%\Volta` |
| Playwright | `%LOCALAPPDATA%\ms-playwright` |
| Puppeteer | `%USERPROFILE%\.cache\puppeteer` |
| Ollama | `%USERPROFILE%\.ollama\models` |
| Hugging Face | `%USERPROFILE%\.cache\huggingface` |
| LM Studio | `%USERPROFILE%\.lmstudio\models` |
| VS Code | `%APPDATA%\Code\{Cache,CachedData,CachedExtensionVSIXs,User\workspaceStorage}`, `%USERPROFILE%\.vscode\extensions` |
| Cursor | `%APPDATA%\Cursor\...` (mesma estrutura do VS Code), `%USERPROFILE%\.cursor\extensions` |
| JetBrains | `%LOCALAPPDATA%\JetBrains\<Produto><versão>` |
| Claude Code | `<repo>\.claude\worktrees\*`, `%USERPROFILE%\.claude\projects` |
| Temp | `%TEMP%` |
| Crash dumps | `%LOCALAPPDATA%\CrashDumps` |

> Os locais de ferramentas de IA mudam com frequência. Cada coletor deve ler esses caminhos de um catálogo configurável, e não de valores fixos no código. Os caminhos devem ser confirmados durante a implementação.

## Fora do escopo

- Arquivos do sistema operacional (Windows Update, `WinSxS`, hibernação, pagefile): exigem privilégio de administrador e não são lixo de desenvolvimento.
- Desinstalar programas.
- Limpeza de navegadores comuns (histórico, cookies).
- Duplicatas genéricas de fotos e documentos pessoais.
