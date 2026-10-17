# ADR-0019 — Tradução da interface com solução própria sobre Intl

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-08.07, todas as telas

## Contexto

O ADR-0014 exige que todo texto visível use chave de tradução, inclusive os motivos gerados pelo núcleo (chave + parâmetros). A interface começa em pt-BR e ganha inglês depois. Era preciso escolher como traduzir.

## Decisão

Uma solução própria em `ui/src/i18n`:
- um dicionário por idioma (`pt-BR.ts`), com textos `{parâmetro}` e formas de plural escolhidas por `Intl.PluralRules`;
- `t(chave, params)` para textos da interface e `translate(mensagem)` para mensagens do núcleo;
- `formatBytes`, `formatRelative` e `formatDateTime` sobre `Intl.NumberFormat`, `Intl.RelativeTimeFormat` e `Intl.DateTimeFormat`;
- chave ausente mostra a própria chave e avisa no console em desenvolvimento;
- um teste falha se houver texto visível em `.tsx` fora dos dicionários.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Própria com Intl** | Zero dependências; mesmo formato das mensagens do núcleo | Recursos avançados (carregamento sob demanda, gênero) teriam de ser escritos |
| i18next + react-i18next | Padrão de mercado, muitos recursos | Mais dependências e configuração para um app de um idioma por vez |
| FormatJS (react-intl) | Formato ICU completo | Mais pesado |

## Consequências

- Tamanhos usam 1 GB = 1024³ bytes, como o Explorador do Windows mostra ("17,8 GB").
- Se surgir necessidade de ICU completo ou de carregamento sob demanda, um ADR novo avalia a troca.
