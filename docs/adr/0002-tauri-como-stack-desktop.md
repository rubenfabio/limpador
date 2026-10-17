# ADR-0002 — Usar Tauri 2 (Rust + React/TypeScript) como stack do app desktop

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-00.01, todas

## Contexto

O produto é um app desktop que varre o disco (muitos arquivos, rápido), conversa com `git`, `docker` e `wsl`, fica na bandeja do sistema e deve ser leve. Começa no Windows e precisa ser portável para Mac e Linux. Seria contraditório um app de limpeza pesar centenas de MB.

## Decisão

Usar **Tauri 2**: núcleo e casca em **Rust**, interface em **React + TypeScript** renderizada pelo WebView do sistema (WebView2 no Windows).

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Tauri 2 (Rust + web)** | Binário pequeno (~10 MB), Rust rápido e seguro para varrer disco, multiplataforma, bandeja e notificações nativas | Rust tem curva maior; o WebView varia por sistema |
| Electron (Node + web) | Ecossistema enorme, fácil | ~150 MB, alto uso de memória; incoerente com o propósito do app |
| Wails (Go + web) | Go simples, SDK oficial do Docker | Comunidade e plugins menores que os do Tauri |
| Nativo (.NET/WinUI) | Integração máxima com o Windows | Prende ao Windows, contra a decisão de portar |

## Consequências

**Positivas**
- App leve e rápido; a mesma base serve Windows, Mac e Linux.
- O núcleo em Rust pode virar CLI e servidor MCP (ADR-0003).

**Negativas / custos aceitos**
- E2E via WebDriver não funciona no macOS (ADR-0011).
- Duas linguagens no projeto.

**O que passa a ser obrigatório**
- Consultar a documentação atual do Tauri antes de usar APIs dele.
